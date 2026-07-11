//! Handler-to-PostgreSQL coverage for purpose-bound, session-bound recent authentication.

#[path = "helpers/mod.rs"]
mod helpers;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const JWT_SECRET: &str = "integration-test-secret-32bytes!";

fn password_hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash test password")
        .to_string()
}

async fn insert_account(pool: &sqlx::PgPool, role: &str, password: Option<&str>) -> (i64, String) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("recent-{suffix}@tongji.edu.cn");
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, email_verified_at, handle, role, password_hash) \
         VALUES ($1, now(), $2, $3::identity.account_role, $4) RETURNING id",
    )
    .bind(&email)
    .bind(format!("recent-{suffix}"))
    .bind(role)
    .bind(password.map(password_hash))
    .fetch_one(pool)
    .await
    .expect("insert test account");
    (id, email)
}

async fn create_session_token(
    pool: &sqlx::PgPool,
    account_id: i64,
    is_recent: bool,
) -> (i64, String) {
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, user_agent, expires_at, \
          recent_authenticated_at, recent_auth_method, recent_auth_credential_version) \
         VALUES ($1, $2, $3, 'recent-auth-test', now() + interval '1 day', \
                 CASE WHEN $4 THEN now() ELSE NULL END, \
                 CASE WHEN $4 THEN 'password' ELSE NULL END, \
                 CASE WHEN $4 THEN (SELECT credential_version FROM identity.accounts WHERE id = $1) \
                      ELSE NULL END) RETURNING id",
    )
    .bind(account_id)
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(uuid::Uuid::new_v4())
    .bind(is_recent)
    .fetch_one(pool)
    .await
    .expect("insert test session");
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await
            .expect("read auth version");
    let token = identity::auth::create_session_access_token(
        account_id,
        session_id,
        auth_version,
        JWT_SECRET,
        3600,
    )
    .expect("create session token");
    (session_id, token)
}

fn authenticated_json_request(
    method: Method,
    uri: impl AsRef<str>,
    token: &str,
    body: Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri.as_ref())
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build authenticated request")
}

fn authenticated_empty_request(method: Method, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .expect("build authenticated request")
}

#[tokio::test]
async fn password_step_up_is_session_bound_and_required_for_role_changes() {
    let (pool, app) = helpers::create_test_app().await;
    let password = "correct-horse-battery-staple!";
    let (admin_id, admin_email) = insert_account(&pool, "admin", Some(password)).await;
    let (target_id, _) = insert_account(&pool, "user", None).await;
    let (target_session_id, _) = create_session_token(&pool, target_id, true).await;
    let (admin_session_id, admin_token) = create_session_token(&pool, admin_id, false).await;

    let before = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::PATCH,
            format!("/api/v2/admin/users/{target_id}/role"),
            &admin_token,
            json!({ "role": "mod", "reason": "promote trusted community moderator" }),
        ))
        .await
        .expect("role response before step-up");
    assert_eq!(before.status(), StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(helpers::read_json(before).await["error"]["code"], "RECENT_AUTH_REQUIRED");

    let wrong = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/recent-auth/verify",
            &admin_token,
            json!({ "method": "password", "password": "not-the-password" }),
        ))
        .await
        .expect("wrong password response");
    assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);
    let was_marked: bool = sqlx::query_scalar(
        "SELECT recent_authenticated_at IS NOT NULL FROM identity.sessions WHERE id = $1",
    )
    .bind(admin_session_id)
    .fetch_one(&pool)
    .await
    .expect("read recent state");
    assert!(!was_marked);

    let verified = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/recent-auth/verify",
            &admin_token,
            json!({ "method": "password", "password": password }),
        ))
        .await
        .expect("password step-up response");
    assert_eq!(verified.status(), StatusCode::OK);
    let verified_body = helpers::read_json(verified).await;
    assert_eq!(verified_body["isFresh"], true);
    assert_eq!(verified_body["method"], "password");

    let role_change = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::PATCH,
            format!("/api/v2/admin/users/{target_id}/role"),
            &admin_token,
            json!({ "role": "mod", "reason": "promote trusted community moderator" }),
        ))
        .await
        .expect("role response after step-up");
    assert_eq!(role_change.status(), StatusCode::OK);
    let target_revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity.sessions WHERE id = $1")
            .bind(target_session_id)
            .fetch_one(&pool)
            .await
            .expect("target session state");
    assert!(target_revoked, "role changes must revoke target freshness and access");

    let audit_text: String = sqlx::query_scalar(
        "SELECT reason || COALESCE(metadata::text, '') FROM governance.audit_events \
         WHERE action = 'identity.user.role_changed' AND target_id = $1",
    )
    .bind(target_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("role audit");
    assert!(!audit_text.contains(password));
    assert!(!audit_text.contains(&admin_email));
}

#[tokio::test]
async fn recent_auth_email_code_is_purpose_bound_atomic_and_single_use() {
    let (pool, app) = helpers::create_test_app().await;
    let (account_id, email) = insert_account(&pool, "admin", None).await;
    let (session_id, token) = create_session_token(&pool, account_id, false).await;
    helpers::insert_valid_code_for_purpose(&pool, &email, "246810", "recent_auth").await;

    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..25 {
        let app = app.clone();
        let token = token.clone();
        requests.spawn(async move {
            app.oneshot(authenticated_json_request(
                Method::POST,
                "/api/v2/auth/recent-auth/verify",
                &token,
                json!({ "method": "email_code", "code": "246810" }),
            ))
            .await
            .expect("concurrent verification response")
            .status()
        });
    }
    let mut successes = 0;
    while let Some(result) = requests.join_next().await {
        if result.expect("verification task") == StatusCode::OK {
            successes += 1;
        }
    }
    assert_eq!(successes, 1);

    let session_state: (Option<chrono::DateTime<chrono::Utc>>, Option<String>) = sqlx::query_as(
        "SELECT recent_authenticated_at, recent_auth_method FROM identity.sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .expect("recent session state");
    assert!(session_state.0.is_some());
    assert_eq!(session_state.1.as_deref(), Some("email_code"));
    let consumed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.email_codes \
         WHERE email = $1 AND purpose = 'recent_auth' AND used_at IS NOT NULL",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("consumed code count");
    assert_eq!(consumed_count, 1);

    helpers::insert_valid_code_for_purpose(&pool, &email, "135791", "login").await;
    let (_, cross_purpose_token) = create_session_token(&pool, account_id, false).await;
    let cross_purpose = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/recent-auth/verify",
            &cross_purpose_token,
            json!({ "method": "email_code", "code": "135791" }),
        ))
        .await
        .expect("cross-purpose response");
    assert_eq!(cross_purpose.status(), StatusCode::BAD_REQUEST);
    let login_code_unused: bool = sqlx::query_scalar(
        "SELECT used_at IS NULL FROM identity.email_codes WHERE email = $1 AND purpose = 'login'",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("login code state");
    assert!(login_code_unused);

    let (_, other_email) = insert_account(&pool, "user", None).await;
    helpers::insert_valid_code_for_purpose(&pool, &other_email, "975310", "recent_auth").await;
    let (_, account_bound_token) = create_session_token(&pool, account_id, false).await;
    let other_account_code = app
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/recent-auth/verify",
            &account_bound_token,
            json!({ "method": "email_code", "code": "975310" }),
        ))
        .await
        .expect("other-account code response");
    assert_eq!(other_account_code.status(), StatusCode::BAD_REQUEST);
    let other_code_unused: bool = sqlx::query_scalar(
        "SELECT used_at IS NULL FROM identity.email_codes \
         WHERE email = $1 AND purpose = 'recent_auth'",
    )
    .bind(other_email)
    .fetch_one(&pool)
    .await
    .expect("other-account code state");
    assert!(other_code_unused);
}

#[tokio::test]
async fn legacy_jwt_fails_closed_but_ordinary_silence_does_not_require_step_up() {
    let (pool, app) = helpers::create_test_app().await;
    let (admin_id, _) = insert_account(&pool, "admin", None).await;
    let (role_target_id, _) = insert_account(&pool, "user", None).await;
    let (silence_target_id, _) = insert_account(&pool, "user", None).await;
    let legacy_token = identity::auth::create_access_token(admin_id, JWT_SECRET, 3600)
        .expect("legacy access token");

    let status = app
        .clone()
        .oneshot(authenticated_empty_request(
            Method::GET,
            "/api/v2/auth/recent-auth",
            &legacy_token,
        ))
        .await
        .expect("legacy status response");
    assert_eq!(status.status(), StatusCode::OK);
    let status_body = helpers::read_json(status).await;
    assert_eq!(status_body["sessionBound"], false);
    assert_eq!(status_body["availableMethods"], json!([]));

    let role_change = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::PATCH,
            format!("/api/v2/admin/users/{role_target_id}/role"),
            &legacy_token,
            json!({ "role": "mod", "reason": "legacy token must not authorize promotion" }),
        ))
        .await
        .expect("legacy role response");
    assert_eq!(role_change.status(), StatusCode::PRECONDITION_REQUIRED);

    let silence = app
        .oneshot(authenticated_json_request(
            Method::POST,
            format!("/api/v2/admin/users/{silence_target_id}/silence"),
            &legacy_token,
            json!({ "reason": "bounded ordinary community moderation" }),
        ))
        .await
        .expect("ordinary moderation response");
    assert_eq!(silence.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn revoking_the_session_immediately_invalidates_step_up_and_its_access_token() {
    let (pool, app) = helpers::create_test_app().await;
    let (admin_id, _) = insert_account(&pool, "admin", None).await;
    let (target_id, _) = insert_account(&pool, "user", None).await;
    let (session_id, token) = create_session_token(&pool, admin_id, true).await;

    let before = app
        .clone()
        .oneshot(authenticated_empty_request(Method::GET, "/api/v2/auth/recent-auth", &token))
        .await
        .expect("fresh status response");
    assert_eq!(helpers::read_json(before).await["isFresh"], true);

    sqlx::query("UPDATE identity.sessions SET revoked_at = now() WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await
        .expect("revoke current session");
    let after = app
        .clone()
        .oneshot(authenticated_empty_request(Method::GET, "/api/v2/auth/recent-auth", &token))
        .await
        .expect("revoked status response");
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);

    let forced_revoke = app
        .oneshot(authenticated_json_request(
            Method::POST,
            format!("/api/v2/admin/users/{target_id}/sessions/revoke"),
            &token,
            json!({ "reason": "revoked actor cannot force a logout" }),
        ))
        .await
        .expect("revoked actor response");
    assert_eq!(forced_revoke.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn provider_unaccepted_code_never_updates_recent_auth() {
    let (pool, app) = helpers::create_test_app().await;
    let (account_id, email) = insert_account(&pool, "admin", None).await;
    let (session_id, token) = create_session_token(&pool, account_id, false).await;
    let code_hash = hex::encode(Sha256::digest(b"112233"));
    sqlx::query(
        "INSERT INTO identity.email_codes \
         (email, purpose, request_id, code_hash, expires_at) \
         VALUES ($1, 'recent_auth', $2, $3, now() + interval '10 minutes')",
    )
    .bind(email)
    .bind(uuid::Uuid::new_v4())
    .bind(code_hash)
    .execute(&pool)
    .await
    .expect("insert unaccepted code");

    let response = app
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/recent-auth/verify",
            &token,
            json!({ "method": "email_code", "code": "112233" }),
        ))
        .await
        .expect("unaccepted code response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let recent_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT recent_authenticated_at FROM identity.sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&pool)
            .await
            .expect("recent state after rejected code");
    assert!(recent_at.is_none());
}

#[tokio::test]
async fn refresh_rotation_preserves_bounded_freshness_within_the_same_session_family() {
    let (pool, app) = helpers::create_test_app().await;
    let (account_id, _) = insert_account(&pool, "admin", None).await;
    let random_part = "a".repeat(64);
    let refresh_hash = hex::encode(Sha256::digest(random_part.as_bytes()));
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, user_agent, expires_at, \
          recent_authenticated_at, recent_auth_method) \
         VALUES ($1, $2, $3, 'rotation-test', now() + interval '1 day', now(), 'email_code') \
         RETURNING id",
    )
    .bind(account_id)
    .bind(refresh_hash)
    .bind(uuid::Uuid::new_v4())
    .fetch_one(&pool)
    .await
    .expect("insert refresh session");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refreshToken": format!("{session_id:x}:{random_part}") }).to_string(),
                ))
                .expect("build refresh request"),
        )
        .await
        .expect("refresh response");
    assert_eq!(response.status(), StatusCode::OK);

    let successor: (Option<chrono::DateTime<chrono::Utc>>, Option<String>) = sqlx::query_as(
        "SELECT successor.recent_authenticated_at, successor.recent_auth_method \
         FROM identity.sessions predecessor \
         JOIN identity.sessions successor ON successor.id = predecessor.replaced_by_id \
         WHERE predecessor.id = $1",
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .expect("successor recent-auth state");
    assert!(successor.0.is_some());
    assert_eq!(successor.1.as_deref(), Some("email_code"));
}

#[tokio::test]
async fn suspension_unsuspension_and_forced_logout_enforce_freshness_without_weakening_revocation()
{
    let (pool, app) = helpers::create_test_app().await;
    let (admin_id, _) = insert_account(&pool, "admin", None).await;
    let (target_id, _) = insert_account(&pool, "user", None).await;
    let (logout_target_id, _) = insert_account(&pool, "user", None).await;
    let (admin_session_id, admin_token) = create_session_token(&pool, admin_id, true).await;
    let (target_session_id, _) = create_session_token(&pool, target_id, true).await;
    create_session_token(&pool, logout_target_id, false).await;

    let suspension = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::POST,
            format!("/api/v2/admin/users/{target_id}/suspend"),
            &admin_token,
            json!({ "reason": "credible account takeover investigation" }),
        ))
        .await
        .expect("suspension response");
    assert_eq!(suspension.status(), StatusCode::NO_CONTENT);
    let target_was_revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity.sessions WHERE id = $1")
            .bind(target_session_id)
            .fetch_one(&pool)
            .await
            .expect("suspended target session");
    assert!(target_was_revoked);
    let sanction_id: i64 = sqlx::query_scalar(
        "SELECT id FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'suspend' AND revoked_at IS NULL",
    )
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("active suspension");

    sqlx::query(
        "UPDATE identity.sessions SET recent_authenticated_at = now() - interval '11 minutes' \
         WHERE id = $1",
    )
    .bind(admin_session_id)
    .execute(&pool)
    .await
    .expect("expire administrator freshness");
    let unsuspend = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::POST,
            format!("/api/v2/admin/users/{target_id}/unsanction"),
            &admin_token,
            json!({
                "sanctionId": sanction_id.to_string(),
                "reason": "investigation completed without compromise"
            }),
        ))
        .await
        .expect("stale unsuspension response");
    assert_eq!(unsuspend.status(), StatusCode::PRECONDITION_REQUIRED);
    let forced_logout = app
        .oneshot(authenticated_json_request(
            Method::POST,
            format!("/api/v2/admin/users/{logout_target_id}/sessions/revoke"),
            &admin_token,
            json!({ "reason": "credential reset requested by account owner" }),
        ))
        .await
        .expect("stale forced-logout response");
    assert_eq!(forced_logout.status(), StatusCode::PRECONDITION_REQUIRED);
    let suspension_still_active: bool =
        sqlx::query_scalar("SELECT revoked_at IS NULL FROM identity.sanctions WHERE id = $1")
            .bind(sanction_id)
            .fetch_one(&pool)
            .await
            .expect("suspension after rejected unsanction");
    assert!(suspension_still_active);
}

#[tokio::test]
async fn concurrent_session_revocation_wins_before_a_high_risk_mutation_commits() {
    let (pool, app) = helpers::create_test_app().await;
    let (admin_id, _) = insert_account(&pool, "admin", None).await;
    let (target_id, _) = insert_account(&pool, "user", None).await;
    let (admin_session_id, admin_token) = create_session_token(&pool, admin_id, true).await;

    let mut revocation = pool.begin().await.expect("begin revocation transaction");
    sqlx::query("UPDATE identity.sessions SET revoked_at = now() WHERE id = $1")
        .bind(admin_session_id)
        .execute(&mut *revocation)
        .await
        .expect("stage concurrent revocation");

    let mutation = tokio::spawn(async move {
        app.oneshot(authenticated_json_request(
            Method::PATCH,
            format!("/api/v2/admin/users/{target_id}/role"),
            &admin_token,
            json!({ "role": "mod", "reason": "concurrent revocation safety check" }),
        ))
        .await
        .expect("concurrent role response")
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!mutation.is_finished(), "mutation must wait on the session revocation lock");
    revocation.commit().await.expect("commit concurrent revocation");

    let response = mutation.await.expect("join concurrent mutation");
    assert_eq!(response.status(), StatusCode::PRECONDITION_REQUIRED);
    let persisted_role: String =
        sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1")
            .bind(target_id)
            .fetch_one(&pool)
            .await
            .expect("target role after revocation race");
    assert_eq!(persisted_role, "user");
}

#[tokio::test]
async fn stale_password_proof_cannot_raise_freshness_after_concurrent_credential_change() {
    let (pool, _) = helpers::create_test_app().await;
    let old_password = "old-concurrent-password!42";
    let (account_id, _) = insert_account(&pool, "admin", Some(old_password)).await;
    let (session_id, _) = create_session_token(&pool, account_id, false).await;
    let credential_version: i64 =
        sqlx::query_scalar("SELECT credential_version FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("credential version");
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let change_pool = pool.clone();
    let change_barrier = barrier.clone();
    let new_hash = password_hash("new-concurrent-password!84");
    let change = tokio::spawn(async move {
        change_barrier.wait().await;
        identity::credential_state::replace_password_if_current(
            &change_pool,
            account_id,
            Some(session_id),
            credential_version,
            &new_hash,
        )
        .await
    });
    let mark_pool = pool.clone();
    let mark_barrier = barrier.clone();
    let mark = tokio::spawn(async move {
        mark_barrier.wait().await;
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        identity::credential_state::mark_password_recent_auth_if_current(
            &mark_pool,
            account_id,
            session_id,
            credential_version,
        )
        .await
    });
    barrier.wait().await;

    change.await.expect("password change task").expect("password change wins");
    assert!(mark.await.expect("recent-auth task").is_err());
    let is_fresh: bool = sqlx::query_scalar(
        "SELECT COALESCE( \
           session.recent_authenticated_at IS NOT NULL \
           AND session.recent_auth_credential_version = account.credential_version, FALSE) \
         FROM identity.sessions session \
         JOIN identity.accounts account ON account.id = session.account_id \
         WHERE session.id = $1",
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .expect("final recent-auth state");
    assert!(!is_fresh, "old password proof must never survive a credential epoch change");
}

#[tokio::test]
async fn email_code_constraint_composes_appeal_and_recent_auth_purposes() {
    let (pool, _) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    for purpose in ["appeal", "recent_auth"] {
        sqlx::query(
            "INSERT INTO identity.email_codes \
             (email, code_hash, expires_at, purpose, request_id, delivery_accepted_at) \
             VALUES ($1, $2, now() + interval '10 minutes', $3, $4, now())",
        )
        .bind(format!("purpose-{purpose}-{suffix}@tongji.edu.cn"))
        .bind(hex::encode(Sha256::digest(format!("{purpose}-{suffix}"))))
        .bind(purpose)
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .expect("composed email code purpose must remain valid");
    }
}

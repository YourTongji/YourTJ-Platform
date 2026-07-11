//! Handler-to-PostgreSQL coverage for onboarding, lifecycle closure, and recovery isolation.

#[path = "helpers/mod.rs"]
mod helpers;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

const JWT_SECRET: &str = "integration-test-secret-32bytes!";

fn password_hash(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .expect("hash password")
        .to_string()
}

async fn insert_account(pool: &sqlx::PgPool, password: &str) -> (i64, String) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("lifecycle-{suffix}@tongji.edu.cn");
    let account_id = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, email_verified_at, handle, password_hash) \
         VALUES ($1, now(), $2, $3) RETURNING id",
    )
    .bind(&email)
    .bind(format!("life-{suffix}"))
    .bind(password_hash(password))
    .fetch_one(pool)
    .await
    .expect("insert account");
    (account_id, email)
}

async fn create_session(pool: &sqlx::PgPool, account_id: i64, is_recent: bool) -> String {
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, expires_at, recent_authenticated_at, \
          recent_auth_method, recent_auth_credential_version) \
         VALUES ($1, $2, $3, now() + interval '1 day', \
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
    .expect("insert session");
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await
            .expect("auth version");
    identity::auth::create_session_access_token(
        account_id,
        session_id,
        auth_version,
        JWT_SECRET,
        3600,
    )
    .expect("session access token")
}

fn request(method: Method, uri: &str, token: Option<&str>, body: Value) -> Request<Body> {
    let mut builder =
        Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder.body(Body::from(body.to_string())).expect("build request")
}

#[tokio::test]
async fn onboarding_is_resumable_and_blocks_ordinary_domain_access_until_terms_are_accepted() {
    let (pool, app) = helpers::create_test_app().await;
    let (account_id, _) = insert_account(&pool, "A-secure-onboarding-password!42").await;
    sqlx::query(
        "UPDATE identity.account_onboarding SET accepted_terms_version = NULL, accepted_at = NULL, \
             completed_at = NULL WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("mark onboarding incomplete");
    let token = create_session(&pool, account_id, false).await;

    let me = app
        .clone()
        .oneshot(request(Method::GET, "/api/v2/me", Some(&token), json!(null)))
        .await
        .expect("me response");
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(helpers::read_json(me).await["onboardingRequired"], true);

    let blocked = app
        .clone()
        .oneshot(request(Method::GET, "/api/v2/me/profile", Some(&token), json!(null)))
        .await
        .expect("blocked profile response");
    assert_eq!(blocked.status(), StatusCode::UNAUTHORIZED);

    let stale_terms = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/onboarding",
            Some(&token),
            json!({
                "handle": "explicit-handle",
                "displayName": "同济同学",
                "bio": "第一次来到社区",
                "profileVisibility": "campus",
                "activityVisibility": "only_me",
                "discoverable": true,
                "acceptedTermsVersion": "stale-version"
            }),
        ))
        .await
        .expect("stale terms response");
    assert_eq!(stale_terms.status(), StatusCode::CONFLICT);

    let completed = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/onboarding",
            Some(&token),
            json!({
                "handle": "explicit-handle",
                "displayName": "同济同学",
                "bio": "第一次来到社区",
                "profileVisibility": "campus",
                "activityVisibility": "only_me",
                "discoverable": true,
                "acceptedTermsVersion": "2026-07-12"
            }),
        ))
        .await
        .expect("onboarding response");
    assert_eq!(completed.status(), StatusCode::OK);
    assert_eq!(helpers::read_json(completed).await["required"], false);

    let profile = app
        .oneshot(request(Method::GET, "/api/v2/me/profile", Some(&token), json!(null)))
        .await
        .expect("profile response");
    assert_eq!(profile.status(), StatusCode::OK);
}

#[tokio::test]
async fn deactivation_revokes_sessions_and_recovery_credential_cannot_access_normal_routes() {
    let (pool, app) = helpers::create_test_app().await;
    let password = "A-secure-lifecycle-password!42";
    let (account_id, _) = insert_account(&pool, password).await;
    sqlx::query(
        "UPDATE identity.account_onboarding SET accepted_terms_version = 'legacy-v1', \
             accepted_at = now(), completed_at = now() WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("complete onboarding");
    let token = create_session(&pool, account_id, true).await;

    let deactivated = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/me/lifecycle/deactivate")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("Idempotency-Key", "deactivate-lifecycle-test")
                .body(Body::from(json!({ "confirmation": "DEACTIVATE" }).to_string()))
                .expect("deactivate request"),
        )
        .await
        .expect("deactivate response");
    assert_eq!(deactivated.status(), StatusCode::OK);
    let body = helpers::read_json(deactivated).await;
    assert_eq!(body["lifecycle"]["state"], "deactivated");
    let recovery_token =
        body["recovery"]["recoveryToken"].as_str().expect("recovery token").to_owned();

    let old_session = app
        .clone()
        .oneshot(request(Method::GET, "/api/v2/me", Some(&token), json!(null)))
        .await
        .expect("old session response");
    assert_eq!(old_session.status(), StatusCode::UNAUTHORIZED);

    let scoped_on_normal_route = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/me")
                .header("X-Recovery-Token", &recovery_token)
                .body(Body::empty())
                .expect("scoped request"),
        )
        .await
        .expect("scoped response");
    assert_eq!(scoped_on_normal_route.status(), StatusCode::UNAUTHORIZED);

    let recovered = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/recovery")
                .header("X-Recovery-Token", &recovery_token)
                .body(Body::empty())
                .expect("recover request"),
        )
        .await
        .expect("recover response");
    assert_eq!(recovered.status(), StatusCode::OK);
    assert_eq!(helpers::read_json(recovered).await["state"], "active");

    let replay = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/recovery")
                .header("X-Recovery-Token", recovery_token)
                .body(Body::empty())
                .expect("recover replay request"),
        )
        .await
        .expect("recover replay response");
    assert_eq!(replay.status(), StatusCode::OK);
}

#[tokio::test]
async fn deletion_moves_through_durable_deleted_stage_and_rejects_recovery_after_purge() {
    let (pool, app) = helpers::create_test_app().await;
    let password = "A-secure-delete-password!42";
    let (account_id, email) = insert_account(&pool, password).await;
    helpers::insert_valid_code_for_purpose(&pool, &email, "878787", "login").await;
    sqlx::query(
        "UPDATE identity.account_onboarding SET accepted_terms_version = 'legacy-v1', \
             accepted_at = now(), completed_at = now() WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("complete onboarding");
    let token = create_session(&pool, account_id, true).await;

    let requested = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/me/lifecycle/delete")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("Idempotency-Key", "delete-lifecycle-test")
                .body(Body::from(json!({ "confirmation": "DELETE" }).to_string()))
                .expect("delete request"),
        )
        .await
        .expect("delete response");
    assert_eq!(requested.status(), StatusCode::ACCEPTED);
    let body = helpers::read_json(requested).await;
    let recovery_token =
        body["recovery"]["recoveryToken"].as_str().expect("recovery token").to_owned();
    assert_eq!(body["lifecycle"]["state"], "deletion_requested");

    let mark_job = identity::lifecycle::claim_due_job(&pool)
        .await
        .expect("claim mark-deleted job")
        .expect("mark-deleted job exists");
    assert_eq!(mark_job.job_type, "mark_deleted");
    identity::lifecycle::complete_mark_deleted(&pool, &mark_job)
        .await
        .expect("complete deleted stage");
    assert_eq!(
        identity::lifecycle::get(&pool, account_id).await.expect("lifecycle").state,
        "deleted"
    );

    sqlx::query(
        "UPDATE identity.accounts SET deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 minute' \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("expire recovery window");
    sqlx::query(
        "UPDATE identity.account_lifecycle_jobs SET next_attempt_at = now() \
         WHERE account_id = $1 AND job_type = 'purge'",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("make purge due");
    let purge_job = identity::lifecycle::claim_due_job(&pool)
        .await
        .expect("claim purge job")
        .expect("purge job exists");
    identity::lifecycle::complete_purge(&pool, &purge_job).await.expect("complete purge");

    let tombstone: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT status::text, email::text, email_blind_index \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read tombstone");
    assert_eq!(tombstone.0, "purged");
    assert!(tombstone.1.is_none());
    assert!(tombstone.2.is_none());
    let retained_email_codes: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity.email_codes WHERE email = $1")
            .bind(&email)
            .fetch_one(&pool)
            .await
            .expect("read purged plaintext email codes");
    assert_eq!(retained_email_codes, 0);

    let recovery = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/recovery")
                .header("X-Recovery-Token", recovery_token)
                .body(Body::empty())
                .expect("late recovery request"),
        )
        .await
        .expect("late recovery response");
    assert_eq!(recovery.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn owner_export_is_idempotent_scoped_one_time_and_recovers_an_expired_worker_lease() {
    let (pool, _) = helpers::create_test_app().await;
    let (account_id, _) = insert_account(&pool, "A-secure-export-password!42").await;
    let token = create_session(&pool, account_id, true).await;
    let headers = HeaderMap::from_iter([(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().expect("authorization header"),
    )]);
    let context =
        identity::auth_middleware::authenticate_context(&headers, &pool, JWT_SECRET, None)
            .await
            .expect("authenticated context");

    let first = identity::data_export::create_job(&pool, &context, "owner-export-test-key")
        .await
        .expect("create export");
    let replay = identity::data_export::create_job(&pool, &context, "owner-export-test-key")
        .await
        .expect("replay export");
    assert_eq!(first.id, replay.id);
    let listed = identity::data_export::list_jobs(&pool, account_id).await.expect("list exports");
    assert_eq!(listed.first().map(|job| job.id), Some(first.id));

    sqlx::query(
        "UPDATE identity.account_export_jobs SET status = 'running', locked_at = \
             now() - interval '11 minutes', attempts = 1 WHERE id = $1",
    )
    .bind(first.id)
    .execute(&pool)
    .await
    .expect("simulate abandoned worker");
    let reclaimed = identity::data_export::claim_job(&pool)
        .await
        .expect("claim export")
        .expect("reclaimed export");
    assert_eq!(reclaimed.id, first.id);

    let artifact = json!({ "schemaVersion": "test", "identity": { "handle": "owner" } });
    identity::data_export::complete_job(&pool, reclaimed.id, &artifact)
        .await
        .expect("complete export");
    let stale_token = create_session(&pool, account_id, false).await;
    let stale_headers = HeaderMap::from_iter([(
        header::AUTHORIZATION,
        format!("Bearer {stale_token}").parse().expect("stale authorization header"),
    )]);
    let stale_context =
        identity::auth_middleware::authenticate_context(&stale_headers, &pool, JWT_SECRET, None)
            .await
            .expect("stale authenticated context");
    let stale_grant =
        identity::data_export::issue_download_grant(&pool, &stale_context, first.id).await;
    assert!(matches!(stale_grant, Err(shared::AppError::RecentAuthRequired)));
    let grant = identity::data_export::issue_download_grant(&pool, &context, first.id)
        .await
        .expect("issue grant");
    let downloaded =
        identity::data_export::consume_download_grant(&pool, account_id, first.id, &grant.token)
            .await
            .expect("consume grant");
    assert_eq!(downloaded, artifact);

    let replayed =
        identity::data_export::consume_download_grant(&pool, account_id, first.id, &grant.token)
            .await;
    assert!(matches!(replayed, Err(shared::AppError::Unauthorized)));

    let (other_account_id, _) = insert_account(&pool, "A-secure-other-password!42").await;
    let other_token = create_session(&pool, other_account_id, true).await;
    let other_headers = HeaderMap::from_iter([(
        header::AUTHORIZATION,
        format!("Bearer {other_token}").parse().expect("other authorization header"),
    )]);
    let other_context =
        identity::auth_middleware::authenticate_context(&other_headers, &pool, JWT_SECRET, None)
            .await
            .expect("other authenticated context");
    let cross_owner =
        identity::data_export::issue_download_grant(&pool, &other_context, first.id).await;
    assert!(matches!(cross_owner, Err(shared::AppError::NotFound)));
}

#[tokio::test]
async fn lifecycle_history_rejects_update_delete_and_truncate() {
    let (pool, _) = helpers::create_test_app().await;
    let (account_id, _) = insert_account(&pool, "A-secure-history-password!42").await;
    let event_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.account_lifecycle_events \
         (account_id, actor_kind, from_state, to_state) \
         VALUES ($1, 'account', 'active', 'deactivated') RETURNING id",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("insert lifecycle history");

    assert!(sqlx::query(
        "UPDATE identity.account_lifecycle_events SET to_state = 'deleted' WHERE id = $1",
    )
    .bind(event_id)
    .execute(&pool)
    .await
    .is_err());
    assert!(sqlx::query("DELETE FROM identity.account_lifecycle_events WHERE id = $1")
        .bind(event_id)
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("TRUNCATE identity.account_lifecycle_events")
        .execute(&pool)
        .await
        .is_err());

    let still_present: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity.account_lifecycle_events WHERE id = $1)",
    )
    .bind(event_id)
    .fetch_one(&pool)
    .await
    .expect("read lifecycle history after rejected mutations");
    assert!(still_present);
}

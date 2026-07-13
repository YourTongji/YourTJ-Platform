//! Credential transaction, durable email, and minimal security-event regressions.

#[path = "helpers/mod.rs"]
mod helpers;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use tower::ServiceExt;

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("JSON request")
}

fn authenticated_json_request(
    method: Method,
    uri: &str,
    access_token: &str,
    body: Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("authenticated JSON request")
}

async fn register_code_only_account(pool: &sqlx::PgPool, app: &Router) -> (i64, String, String) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("credential-{suffix}@tongji.edu.cn");
    helpers::insert_valid_code_for_purpose(pool, &email, "123456", "registration").await;
    let response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/email/verify",
            json!({
                "email": email,
                "code": "123456",
                "purpose": "registration",
                "handle": format!("credential-{}", &suffix[..16])
            }),
        ))
        .await
        .expect("registration response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = helpers::read_json(response).await;
    let account_id =
        body["account"]["id"].as_str().expect("account id").parse().expect("numeric account id");
    let access_token = body["accessToken"].as_str().expect("access token").to_owned();
    let refresh_token = body["refreshToken"].as_str().expect("refresh token").to_owned();
    (account_id, access_token, refresh_token)
}

async fn mark_session_fresh(pool: &sqlx::PgPool, refresh_token: &str) -> i64 {
    let session_id =
        i64::from_str_radix(refresh_token.split_once(':').expect("combined refresh token").0, 16)
            .expect("session id");
    sqlx::query(
        "UPDATE identity.sessions SET recent_authenticated_at = now(), \
             recent_auth_method = 'email_code', recent_auth_credential_version = NULL \
         WHERE id = $1",
    )
    .bind(session_id)
    .execute(pool)
    .await
    .expect("mark session fresh");
    session_id
}

#[tokio::test]
async fn concurrent_first_password_setup_commits_once_and_replaces_the_old_session() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let (account_id, access_token, old_refresh) = register_code_only_account(&pool, &app).await;
    mark_session_fresh(&pool, &old_refresh).await;

    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..2 {
        let app = app.clone();
        let access_token = access_token.clone();
        requests.spawn(async move {
            app.oneshot(authenticated_json_request(
                Method::POST,
                "/api/v2/auth/password/set",
                &access_token,
                json!({ "newPassword": "first-correct-horse-battery-staple!" }),
            ))
            .await
            .expect("password-set response")
        });
    }
    let mut successful_body = None;
    while let Some(response) = requests.join_next().await {
        let response = response.expect("password-set task");
        if response.status() == StatusCode::OK {
            assert!(successful_body.is_none(), "only one password setup may commit");
            successful_body = Some(helpers::read_json(response).await);
        } else {
            assert!(matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::CONFLICT | StatusCode::PRECONDITION_REQUIRED
            ));
        }
    }
    let successful_body = successful_body.expect("one password setup succeeds");
    let replacement_access =
        successful_body["accessToken"].as_str().expect("replacement access token");

    let old_access = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/me")
                .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("old access request"),
        )
        .await
        .expect("old access response");
    assert_eq!(old_access.status(), StatusCode::UNAUTHORIZED);
    let old_refresh_response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/refresh",
            json!({ "refreshToken": old_refresh }),
        ))
        .await
        .expect("old refresh response");
    assert_eq!(old_refresh_response.status(), StatusCode::UNAUTHORIZED);
    let replacement_access_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/me")
                .header(header::AUTHORIZATION, format!("Bearer {replacement_access}"))
                .body(Body::empty())
                .expect("replacement access request"),
        )
        .await
        .expect("replacement access response");
    assert_eq!(replacement_access_response.status(), StatusCode::OK);

    let facts: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
           (SELECT COUNT(*) FROM identity.sessions \
            WHERE account_id = $1 AND revoked_at IS NULL), \
           (SELECT COUNT(*) FROM identity.security_events \
            WHERE account_id = $1 AND event_type = 'password_set'), \
           (SELECT COUNT(*) FROM identity.email_delivery_jobs \
            WHERE account_id = $1 AND kind = 'password_set')",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("credential facts");
    assert_eq!(facts, (1, 1, 1));
}

#[tokio::test]
async fn email_enqueue_failure_rolls_back_password_session_and_security_fact() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let (account_id, access_token, refresh_token) = register_code_only_account(&pool, &app).await;
    let original_session_id = mark_session_fresh(&pool, &refresh_token).await;
    sqlx::raw_sql(
        "DROP TRIGGER IF EXISTS test_reject_identity_email_delivery \
           ON identity.email_delivery_jobs; \
         DROP FUNCTION IF EXISTS identity.test_reject_identity_email_delivery(); \
         CREATE FUNCTION identity.test_reject_identity_email_delivery() RETURNS TRIGGER \
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic enqueue failure'; END $$; \
         CREATE TRIGGER test_reject_identity_email_delivery BEFORE INSERT \
           ON identity.email_delivery_jobs FOR EACH ROW \
           EXECUTE FUNCTION identity.test_reject_identity_email_delivery();",
    )
    .execute(&pool)
    .await
    .expect("install enqueue failure trigger");

    let response = app
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/password/set",
            &access_token,
            json!({ "newPassword": "rollback-correct-horse-battery-staple!" }),
        ))
        .await
        .expect("password-set failure response");
    sqlx::raw_sql(
        "DROP TRIGGER test_reject_identity_email_delivery ON identity.email_delivery_jobs; \
         DROP FUNCTION identity.test_reject_identity_email_delivery();",
    )
    .execute(&pool)
    .await
    .expect("remove enqueue failure trigger");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let state: (bool, bool, i64, i64) = sqlx::query_as(
        "SELECT account.password_hash IS NULL, session.revoked_at IS NULL, \
                (SELECT COUNT(*) FROM identity.security_events \
                 WHERE account_id = account.id AND event_type = 'password_set'), \
                (SELECT COUNT(*) FROM identity.email_delivery_jobs \
                 WHERE account_id = account.id) \
         FROM identity.accounts account \
         JOIN identity.sessions session ON session.account_id = account.id \
         WHERE account.id = $1 AND session.id = $2",
    )
    .bind(account_id)
    .bind(original_session_id)
    .fetch_one(&pool)
    .await
    .expect("rolled-back credential state");
    assert_eq!(state, (true, true, 0, 0));
}

#[tokio::test]
async fn credential_change_invalidates_an_older_password_reset_code() {
    const PASSWORD: &str = "yourtj-constant-dummy-password";
    const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("stale-reset-{suffix}@tongji.edu.cn");
    sqlx::query(
        "INSERT INTO identity.accounts (email, handle, password_hash) \
         VALUES ($1, $2, $3)",
    )
    .bind(&email)
    .bind(format!("stale-reset-{}", &suffix[..16]))
    .bind(PASSWORD_HASH)
    .execute(&pool)
    .await
    .expect("password account");
    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/password/login",
            json!({ "email": email, "password": PASSWORD }),
        ))
        .await
        .expect("password login response");
    assert_eq!(login.status(), StatusCode::OK);
    let login = helpers::read_json(login).await;
    let access_token = login["accessToken"].as_str().expect("access token");
    helpers::insert_valid_code_for_purpose(&pool, &email, "654321", "password_reset").await;

    let change = app
        .clone()
        .oneshot(authenticated_json_request(
            Method::POST,
            "/api/v2/auth/password/change",
            access_token,
            json!({
                "currentPassword": PASSWORD,
                "newPassword": "newer-correct-horse-battery-staple!"
            }),
        ))
        .await
        .expect("password change response");
    assert_eq!(change.status(), StatusCode::OK);
    let stale_reset = app
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/password/reset",
            json!({
                "email": email,
                "code": "654321",
                "newPassword": "stale-reset-must-not-win-battery-staple!"
            }),
        ))
        .await
        .expect("stale reset response");
    assert_eq!(stale_reset.status(), StatusCode::BAD_REQUEST);
}

async fn fake_cloudflare(State(attempts): State<Arc<AtomicUsize>>) -> Response {
    if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    Json(json!({
        "success": true,
        "errors": [],
        "result": {
            "message_id": "fake-message-id",
            "delivered": [],
            "queued": [],
            "permanent_bounces": []
        }
    }))
    .into_response()
}

#[tokio::test]
async fn durable_email_retries_provider_failure_without_persisting_message_plaintext() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind fake provider");
    let address = listener.local_addr().expect("fake provider address");
    let provider = Router::new().fallback(post(fake_cloudflare)).with_state(attempts.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, provider).await.expect("fake provider server");
    });
    let mut config = shared::Config::from_env().expect("test config");
    config.email_provider = shared::config::EmailProvider::Cloudflare;
    config.email_from = "security@yourtj.invalid".into();
    config.cloudflare_email_account_id = "fake-account".into();
    config.cloudflare_email_api_token = "fake-token".into();
    config.cloudflare_email_api_base_url = format!("http://{address}");
    let (pool, _, state) = helpers::create_test_app_and_state_with_config(config).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("delivery-{suffix}@tongji.edu.cn");
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(&email)
    .bind(format!("delivery-{}", &suffix[..16]))
    .fetch_one(&pool)
    .await
    .expect("delivery account");
    sqlx::query(
        "INSERT INTO identity.email_delivery_jobs (account_id, kind) \
         VALUES ($1, 'password_changed')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("delivery job");

    assert!(identity::email_delivery::deliver_one_due_email(&state)
        .await
        .expect("first delivery attempt"));
    let retry: (String, i16, Option<String>) = sqlx::query_as(
        "SELECT status, attempts, last_error_code FROM identity.email_delivery_jobs \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("retry state");
    assert_eq!(retry, ("queued".into(), 1, Some("provider_unavailable".into())));
    sqlx::query(
        "UPDATE identity.email_delivery_jobs SET next_attempt_at = now() WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("make retry due");
    assert!(identity::email_delivery::deliver_one_due_email(&state)
        .await
        .expect("second delivery attempt"));
    let completed: (String, i16, bool) = sqlx::query_as(
        "SELECT status, attempts, accepted_at IS NOT NULL \
         FROM identity.email_delivery_jobs WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("completed state");
    assert_eq!(completed, ("succeeded".into(), 2, true));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    let forbidden_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'email_delivery_jobs' \
           AND column_name IN ('email', 'recipient', 'subject', 'body', 'html', 'payload')",
    )
    .fetch_one(&pool)
    .await
    .expect("delivery schema columns");
    assert_eq!(forbidden_columns, 0);
    server.abort();
}

#[tokio::test]
async fn identity_lookup_failure_is_retryable_not_a_recipient_dead_letter() {
    let config = shared::Config::from_env().expect("test config");
    let (pool, _, state) = helpers::create_test_app_and_state_with_config(config).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("lookup-{suffix}@tongji.edu.cn"))
    .bind(format!("lookup-{}", &suffix[..16]))
    .fetch_one(&pool)
    .await
    .expect("lookup account");
    sqlx::query(
        "UPDATE identity.accounts SET email = NULL, email_ciphertext = 'synthetic-ciphertext', \
             email_key_version = 1, email_blind_index = $2, password_email_blind = $2 \
         WHERE id = $1",
    )
    .bind(account_id)
    .bind(format!("synthetic-{suffix}"))
    .execute(&pool)
    .await
    .expect("make recipient temporarily unreadable");
    sqlx::query(
        "INSERT INTO identity.email_delivery_jobs (account_id, kind) \
         VALUES ($1, 'password_reset')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("lookup delivery job");

    assert!(identity::email_delivery::deliver_one_due_email(&state)
        .await
        .expect("lookup delivery attempt"));
    let job: (String, String) = sqlx::query_as(
        "SELECT status, last_error_code FROM identity.email_delivery_jobs \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("lookup retry state");
    assert_eq!(job, ("queued".into(), "identity_unavailable".into()));
}

#[tokio::test]
async fn security_facts_are_owner_exportable_append_only_and_expire_with_delivery_metadata() {
    let (pool, _) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("security-{suffix}@tongji.edu.cn"))
    .bind(format!("security-{}", &suffix[..16]))
    .fetch_one(&pool)
    .await
    .expect("security account");
    sqlx::query("INSERT INTO identity.profiles (account_id) VALUES ($1)")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("owner profile");
    sqlx::query("INSERT INTO identity.profile_privacy (account_id) VALUES ($1)")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("owner privacy");
    let live_event_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.security_events (account_id, event_type) \
         VALUES ($1, 'password_changed') RETURNING id",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("live security event");
    assert!(sqlx::query(
        "UPDATE identity.security_events SET event_type = 'password_reset' WHERE id = $1"
    )
    .bind(live_event_id)
    .execute(&pool)
    .await
    .is_err());
    assert!(sqlx::query("DELETE FROM identity.security_events WHERE id = $1")
        .bind(live_event_id)
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("TRUNCATE identity.security_events").execute(&pool).await.is_err());
    sqlx::query(
        "INSERT INTO identity.security_events \
         (account_id, event_type, created_at, expires_at) \
         VALUES ($1, 'password_reset', now() - interval '366 days', now() - interval '1 day')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("expired security event");
    sqlx::query(
        "INSERT INTO identity.email_delivery_jobs \
         (account_id, kind, status, attempts, last_error_code, accepted_at, updated_at) \
         VALUES ($1, 'password_changed', 'succeeded', 1, NULL, \
                 now() - interval '31 days', now() - interval '31 days'), \
                ($1, 'password_reset', 'dead', 8, 'provider_unavailable', NULL, \
                 now() - interval '91 days')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("expired delivery metadata");
    let export = identity::data_export::snapshot(&pool, None, account_id)
        .await
        .expect("identity owner export");
    let export = serde_json::to_value(export).expect("serialize identity export");
    assert_eq!(export["securityEvents"][0]["eventType"], "password_changed");
    assert!(export["securityEvents"][0].get("subjectSessionId").is_none());

    let removed = identity::email_delivery::purge_expired_email_delivery_data(&pool)
        .await
        .expect("retention cleanup");
    assert!(removed >= 3, "cleanup may also remove expired facts from earlier fixtures");
    let remaining: (i64, i64) = sqlx::query_as(
        "SELECT \
          (SELECT COUNT(*) FROM identity.security_events WHERE account_id = $1), \
          (SELECT COUNT(*) FROM identity.email_delivery_jobs WHERE account_id = $1)",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("remaining retained facts");
    assert_eq!(remaining, (1, 0));
}

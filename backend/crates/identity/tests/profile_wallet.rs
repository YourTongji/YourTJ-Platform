//! Integration tests for the identity domain — profile & wallet operations.

#[path = "helpers/mod.rs"]
mod helpers;

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use base64::Engine;
use helpers::{create_test_app, create_test_app_with_pool};
use ring::signature::KeyPair;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn wait_for_lock_wait(pool: &PgPool, query_prefix: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    let query_pattern = format!("{query_prefix}%");
    loop {
        let is_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE datname = current_database() AND pid <> pg_backend_pid() \
                 AND wait_event_type = 'Lock' AND ltrim(query) LIKE $1 \
             )",
        )
        .bind(&query_pattern)
        .fetch_one(pool)
        .await
        .expect("inspect wallet bind lifecycle wait");
        if is_waiting {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn create_session_token(pool: &PgPool, account_id: i64, is_recent: bool) -> String {
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
    .expect("insert wallet test session");
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await
            .expect("read wallet test auth version");
    identity::auth::create_session_access_token(
        account_id,
        session_id,
        auth_version,
        "integration-test-secret-32bytes!",
        3_600,
    )
    .expect("create wallet test session token")
}

/// ── update my handle ───────────────────────────────────────────────────

#[tokio::test]
async fn test_update_handle_succeeds() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("oliver@tongji.edu.cn")
        .bind("oliver")
        .execute(&pool)
        .await
        .unwrap();

    let (token, account_id) = helpers::create_access_token_for("oliver@tongji.edu.cn", &pool).await;

    let app = create_test_app_with_pool(pool.clone()).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v2/me")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "handle": "oliver_new" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["handle"], "oliver_new");

    let handle: String = sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(handle, "oliver_new");
}

#[tokio::test]
async fn test_update_handle_rejects_invalid_chars() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("pat@tongji.edu.cn")
        .bind("pat")
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("pat@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool.clone()).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v2/me")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "handle": "no spaces!" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_handle_rejects_duplicate() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("quinn@tongji.edu.cn")
        .bind("quinn")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("quinn2@tongji.edu.cn")
        .bind("quinn2")
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("quinn@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v2/me")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "handle": "quinn2" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

/// ── controlled profile media ───────────────────────────────────────────

#[tokio::test]
async fn arbitrary_avatar_url_is_not_persisted() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("rose@tongji.edu.cn")
        .bind("rose")
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("rose@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool.clone()).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v2/me")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "avatarUrl": "https://example.com/avatar.png" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert!(body["avatarUrl"].is_null());
    sqlx::query(
        "UPDATE identity.accounts SET avatar_url = 'https://legacy.example/avatar.png' \
         WHERE handle = 'rose'",
    )
    .execute(&pool)
    .await
    .expect("rolling legacy avatar write is safely retired");
    let stored_avatar: Option<String> =
        sqlx::query_scalar("SELECT avatar_url FROM identity.accounts WHERE handle = 'rose'")
            .fetch_one(&pool)
            .await
            .expect("read retired avatar URL");
    assert!(stored_avatar.is_none());
}

// ── get wallet ─────────────────────────────────────────────────────────
//
// `GET /api/v2/wallet` (read wallet balance) is owned by the credit crate and
// composed into the app in `api::bootstrap`; the identity router does not serve
// it. Balance-read behaviour is covered by `credit::tests::wallet_tests`, so
// there are intentionally no wallet-read tests mounted on `identity::routes`
// here (they would always 404).

/// ── bind key ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_bind_key_valid_ed25519_succeeds() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("uma@tongji.edu.cn")
        .bind("uma")
        .execute(&pool)
        .await
        .unwrap();

    let (_, account_id) = helpers::create_access_token_for("uma@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, true).await;
    let app = create_test_app_with_pool(pool.clone()).await;

    let rng = ring::rand::SystemRandom::new();
    let pkcs8_bytes =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).expect("generate key pair");
    let key_pair =
        ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).expect("parse key pair");
    let public_key_bytes = key_pair.public_key().as_ref();
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "accountId": account_id.to_string(), "publicKey": public_key_b64 })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count: Option<i64> =
        sqlx::query_scalar("SELECT count(*) FROM identity.account_keys WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.unwrap(), 1);
}

#[tokio::test]
async fn legacy_bind_body_can_only_confirm_the_exact_active_key() {
    let (pool, _) = create_test_app().await;
    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("legacy-wallet-bind@tongji.edu.cn")
        .bind("legacy-wallet-bind")
        .execute(&pool)
        .await
        .expect("insert legacy wallet account");
    let (_, account_id) =
        helpers::create_access_token_for("legacy-wallet-bind@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, true).await;
    let app = create_test_app_with_pool(pool.clone()).await;
    let public_key = base64::engine::general_purpose::STANDARD.encode([21_u8; 32]);

    let missing_scope = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "publicKey": public_key }).to_string()))
                .expect("build legacy first-bind request"),
        )
        .await
        .expect("legacy first-bind response");
    assert_eq!(missing_scope.status(), StatusCode::BAD_REQUEST);

    let enrolled = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({
                        "accountId": account_id.to_string(),
                        "publicKey": public_key,
                    })
                    .to_string(),
                ))
                .expect("build account-scoped bind request"),
        )
        .await
        .expect("account-scoped bind response");
    assert_eq!(enrolled.status(), StatusCode::NO_CONTENT);

    let legacy_idempotent = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "publicKey": public_key }).to_string()))
                .expect("build legacy idempotent bind request"),
        )
        .await
        .expect("legacy idempotent bind response");
    assert_eq!(legacy_idempotent.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_bind_key_rejects_account_scope_mismatched_with_bearer() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2), ($3, $4)")
        .bind("wallet-scope-a@tongji.edu.cn")
        .bind("wallet-scope-a")
        .bind("wallet-scope-b@tongji.edu.cn")
        .bind("wallet-scope-b")
        .execute(&pool)
        .await
        .unwrap();

    let (_, account_a) =
        helpers::create_access_token_for("wallet-scope-a@tongji.edu.cn", &pool).await;
    let (_, account_b) =
        helpers::create_access_token_for("wallet-scope-b@tongji.edu.cn", &pool).await;
    let token_b = create_session_token(&pool, account_b, true).await;
    let app = create_test_app_with_pool(pool.clone()).await;
    let public_key = base64::engine::general_purpose::STANDARD.encode([8u8; 32]);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
                .body(Body::from(
                    json!({ "accountId": account_a.to_string(), "publicKey": public_key })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity.account_keys WHERE account_id = ANY($1)")
            .bind(vec![account_a, account_b])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_bind_key_invalid_base64_rejects() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("vera@tongji.edu.cn")
        .bind("vera")
        .execute(&pool)
        .await
        .unwrap();

    let (token, account_id) = helpers::create_access_token_for("vera@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({
                        "accountId": account_id.to_string(),
                        "publicKey": "not-valid-base64!!!"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_bind_key_wrong_length_rejects() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("wendy@tongji.edu.cn")
        .bind("wendy")
        .execute(&pool)
        .await
        .unwrap();

    let (token, account_id) = helpers::create_access_token_for("wendy@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let short_key = base64::engine::general_purpose::STANDARD.encode([0u8; 16]);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "accountId": account_id.to_string(), "publicKey": short_key })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_bind_key_same_canonical_key_is_idempotent() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("xena@tongji.edu.cn")
        .bind("xena")
        .execute(&pool)
        .await
        .unwrap();

    let (_, account_id) = helpers::create_access_token_for("xena@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, true).await;

    let key_b64 = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(account_id)
        .bind(key_b64)
        .execute(&pool)
        .await
        .unwrap();

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "accountId": account_id.to_string(), "publicKey": key_b64 })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_bind_key_requires_fresh_session_authentication() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("yara@tongji.edu.cn")
        .bind("yara")
        .execute(&pool)
        .await
        .unwrap();

    let (_, account_id) = helpers::create_access_token_for("yara@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, false).await;
    let app = create_test_app_with_pool(pool.clone()).await;
    let public_key = base64::engine::general_purpose::STANDARD.encode([3u8; 32]);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "accountId": account_id.to_string(), "publicKey": public_key })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PRECONDITION_REQUIRED);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity.account_keys WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_bind_key_rejects_session_only_rotation() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("zora@tongji.edu.cn")
        .bind("zora")
        .execute(&pool)
        .await
        .unwrap();

    let (_, account_id) = helpers::create_access_token_for("zora@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, true).await;
    let first_key = base64::engine::general_purpose::STANDARD.encode([4u8; 32]);
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(account_id)
        .bind(&first_key)
        .execute(&pool)
        .await
        .unwrap();
    let different_key = base64::engine::general_purpose::STANDARD.encode([5u8; 32]);
    let app = create_test_app_with_pool(pool.clone()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "accountId": account_id.to_string(), "publicKey": different_key })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT public_key FROM identity.account_keys WHERE account_id = $1 ORDER BY public_key",
    )
    .bind(account_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(keys, vec![first_key]);
}

#[tokio::test]
async fn test_bind_key_serializes_concurrent_first_enrollment() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("concurrent-wallet@tongji.edu.cn")
        .bind("concurrent-wallet")
        .execute(&pool)
        .await
        .unwrap();

    let (_, account_id) =
        helpers::create_access_token_for("concurrent-wallet@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, true).await;
    let app = create_test_app_with_pool(pool.clone()).await;
    let request = |key_byte: u8| {
        Request::builder()
            .method(Method::POST)
            .uri("/api/v2/wallet/bind")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(
                json!({
                    "accountId": account_id.to_string(),
                    "publicKey": base64::engine::general_purpose::STANDARD.encode([key_byte; 32])
                })
                .to_string(),
            ))
            .unwrap()
    };

    let (first, second) =
        tokio::join!(app.clone().oneshot(request(6)), app.clone().oneshot(request(7)),);
    let mut statuses = vec![first.unwrap().status(), second.unwrap().status()];
    statuses.sort_by_key(|status| status.as_u16());

    assert_eq!(statuses, vec![StatusCode::NO_CONTENT, StatusCode::CONFLICT]);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity.account_keys WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_bind_key_rechecks_account_state_after_waiting_for_lifecycle() {
    let (pool, _) = create_test_app().await;
    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("wallet-bind-lifecycle@tongji.edu.cn")
        .bind("wallet-bind-lifecycle")
        .execute(&pool)
        .await
        .expect("insert lifecycle wallet account");
    let (_, account_id) =
        helpers::create_access_token_for("wallet-bind-lifecycle@tongji.edu.cn", &pool).await;
    let token = create_session_token(&pool, account_id, true).await;
    let app = create_test_app_with_pool(pool.clone()).await;

    let mut lifecycle_tx = pool.begin().await.expect("begin wallet lifecycle transition");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deletion_requested', \
                deletion_requested_at = now(), \
                deletion_recover_until = now() + interval '30 days' \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(&mut *lifecycle_tx)
    .await
    .expect("stage wallet lifecycle transition");

    let public_key = base64::engine::general_purpose::STANDARD.encode([29_u8; 32]);
    let request_task = tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "accountId": account_id.to_string(), "publicKey": public_key })
                        .to_string(),
                ))
                .expect("build lifecycle wallet bind request"),
        )
        .await
        .expect("lifecycle wallet bind response")
    });
    assert!(
        wait_for_lock_wait(
            &pool,
            "SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE",
        )
        .await,
        "wallet bind did not wait on the lifecycle account lock"
    );

    lifecycle_tx.commit().await.expect("commit wallet lifecycle transition");
    let response = request_task.await.expect("join lifecycle wallet bind request");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let key_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity.account_keys WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("count lifecycle wallet keys");
    assert_eq!(key_count, 0);
}

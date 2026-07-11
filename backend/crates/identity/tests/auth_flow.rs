//! Integration tests for the identity domain — auth flow.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{
    create_test_app, create_test_app_with_pool, create_test_app_with_pool_and_encryption,
};
use serde_json::{json, Value};
use sha2::Digest as _;
use tower::ServiceExt;

/// ── request-code ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_request_code_creates_record() {
    let (pool, app) = create_test_app().await;
    let email = "alice@tongji.edu.cn";

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "captchaToken": "request-alice" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let count: Option<i64> = sqlx::query_scalar(
        "SELECT count(*) FROM identity.email_codes WHERE email = $1 AND attempts < 5",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(count.unwrap() > 0);
}

#[tokio::test]
async fn test_request_code_rate_limited() {
    let (_, app) = create_test_app().await;
    let email = format!("bob-{}@tongji.edu.cn", uuid::Uuid::new_v4());

    let make_req = |captcha_token: &str| {
        Request::builder()
            .method(Method::POST)
            .uri("/api/v2/auth/email/request-code")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "email": email, "captchaToken": captcha_token }).to_string()))
            .unwrap()
    };

    let r1 = app.clone().oneshot(make_req("request-bob-1")).await.unwrap();
    assert_eq!(r1.status(), StatusCode::NO_CONTENT);

    let r2 = app.oneshot(make_req("request-bob-2")).await.unwrap();
    assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// ── verify-email ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_verify_correct_code_creates_account() {
    let (pool, app) = create_test_app().await;
    let email = "charlie@tongji.edu.cn";

    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "captchaToken": "request-charlie" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let code_hash: String = sqlx::query_scalar(
        "SELECT code_hash FROM identity.email_codes \
         WHERE email = $1 AND expires_at > now() AND attempts < 5 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .unwrap();

    let correct_code = helpers::brute_force_code(&code_hash);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": email, "code": correct_code }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = helpers::read_json(resp).await;
    assert!(body.get("accessToken").is_some());
    assert!(body.get("refreshToken").is_some());
    assert_eq!(body["account"]["handle"], "charlie");
    assert_eq!(body["account"]["role"], "user");
}

#[tokio::test]
async fn test_email_flow_stores_only_encrypted_account_email() {
    let (pool, _) = create_test_app().await;
    let encryption = shared::email_crypto::EmailEncryption::from_keys(
        1,
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f",
        &[],
    )
    .unwrap()
    .unwrap();
    let app =
        create_test_app_with_pool_and_encryption(pool.clone(), Some(encryption.clone())).await;
    let email = "encrypted@tongji.edu.cn";

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "captchaToken": "request-encrypted" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let blind_index = encryption.blind_index(email);
    let code_hash: String = sqlx::query_scalar(
        "SELECT code_hash FROM identity.email_codes WHERE email_blind_index = $1",
    )
    .bind(&blind_index)
    .fetch_one(&pool)
    .await
    .unwrap();
    let stored_code_email: Option<String> = sqlx::query_scalar(
        "SELECT email::text FROM identity.email_codes WHERE email_blind_index = $1",
    )
    .bind(&blind_index)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(stored_code_email.is_none());

    let code = helpers::brute_force_code(&code_hash);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": email, "code": code }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let stored_account: (Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT email::text, email_ciphertext, email_blind_index \
         FROM identity.accounts WHERE email_blind_index = $1",
    )
    .bind(blind_index)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(stored_account.0.is_none());
    assert!(stored_account.1.is_some());
    assert!(stored_account.2.is_some());
}

#[tokio::test]
async fn test_verify_wrong_code_fails() {
    let (_, app) = create_test_app().await;

    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/request-code")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "dave@tongji.edu.cn", "captchaToken": "request-dave" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "dave@tongji.edu.cn", "code": "000000" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_verify_code_expired_rejects() {
    let (pool, _) = create_test_app().await;
    let email = "eve@tongji.edu.cn";

    sqlx::query(
        "INSERT INTO identity.email_codes (email, code_hash, expires_at, attempts) \
         VALUES ($1, $2, now() - interval '1 hour', 0)",
    )
    .bind(email)
    .bind(hex::encode(sha2::Sha256::digest("123456")))
    .execute(&pool)
    .await
    .unwrap();

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": email, "code": "123456" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_verify_existing_account_returns_tokens() {
    let (pool, _) = create_test_app().await;
    let email = "frank@tongji.edu.cn";

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind(email)
        .bind("frank")
        .execute(&pool)
        .await
        .unwrap();

    helpers::insert_valid_code(&pool, email, "654321").await;

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": email, "code": "654321" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["account"]["handle"], "frank");
}

#[tokio::test]
async fn test_verify_code_exhausted_after_5_attempts() {
    let (pool, _) = create_test_app().await;
    let email = "grace@tongji.edu.cn";

    let code_hash = hex::encode(sha2::Sha256::digest("111111"));
    sqlx::query(
        "INSERT INTO identity.email_codes (email, code_hash, expires_at, attempts) \
         VALUES ($1, $2, now() + interval '10 minutes', 5)",
    )
    .bind(email)
    .bind(&code_hash)
    .execute(&pool)
    .await
    .unwrap();

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": email, "code": "111111" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// ── refresh ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_refresh_valid_token_returns_new_pair() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("helen@tongji.edu.cn")
        .bind("helen")
        .execute(&pool)
        .await
        .unwrap();

    let account_id: i64 = sqlx::query_scalar("SELECT id FROM identity.accounts WHERE email = $1")
        .bind("helen@tongji.edu.cn")
        .fetch_one(&pool)
        .await
        .unwrap();

    let random_hex = "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234";
    let refresh_hash = hex::encode(sha2::Sha256::digest(random_hex));
    let sid: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions (account_id, refresh_hash, expires_at) \
         VALUES ($1, $2, now() + interval '7 days') RETURNING id",
    )
    .bind(account_id)
    .bind(&refresh_hash)
    .fetch_one(&pool)
    .await
    .unwrap();

    let app = create_test_app_with_pool(pool).await;
    let refresh_token = format!("{sid:x}:{random_hex}");

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "refreshToken": refresh_token }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert!(body.get("accessToken").is_some());
    assert!(body.get("refreshToken").is_some());
}

#[tokio::test]
async fn test_refresh_expired_token_rejects() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("irene@tongji.edu.cn")
        .bind("irene")
        .execute(&pool)
        .await
        .unwrap();

    let account_id: i64 = sqlx::query_scalar("SELECT id FROM identity.accounts WHERE email = $1")
        .bind("irene@tongji.edu.cn")
        .fetch_one(&pool)
        .await
        .unwrap();

    let random_hex = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let refresh_hash = hex::encode(sha2::Sha256::digest(random_hex));
    let sid: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions (account_id, refresh_hash, expires_at) \
         VALUES ($1, $2, now() - interval '1 hour') RETURNING id",
    )
    .bind(account_id)
    .bind(&refresh_hash)
    .fetch_one(&pool)
    .await
    .unwrap();

    let app = create_test_app_with_pool(pool).await;
    let refresh_token = format!("{sid:x}:{random_hex}");

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "refreshToken": refresh_token }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_refresh_revoked_token_rejects() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("jack@tongji.edu.cn")
        .bind("jack")
        .execute(&pool)
        .await
        .unwrap();

    let account_id: i64 = sqlx::query_scalar("SELECT id FROM identity.accounts WHERE email = $1")
        .bind("jack@tongji.edu.cn")
        .fetch_one(&pool)
        .await
        .unwrap();

    let random_hex = "facecafe1234facecafe1234facecafe1234facecafe1234facecafe1234facecafe";
    let refresh_hash = hex::encode(sha2::Sha256::digest(random_hex));
    let sid: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions (account_id, refresh_hash, expires_at, revoked_at) \
         VALUES ($1, $2, now() + interval '7 days', now()) RETURNING id",
    )
    .bind(account_id)
    .bind(&refresh_hash)
    .fetch_one(&pool)
    .await
    .unwrap();

    let app = create_test_app_with_pool(pool).await;
    let refresh_token = format!("{sid:x}:{random_hex}");

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "refreshToken": refresh_token }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// ── logout / me ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_logout_revokes_session() {
    let (pool, _) = create_test_app().await;

    helpers::insert_valid_code(&pool, "kate@tongji.edu.cn", "111111").await;
    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("kate@tongji.edu.cn")
        .bind("kate")
        .execute(&pool)
        .await
        .unwrap();

    let app = create_test_app_with_pool(pool.clone()).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "kate@tongji.edu.cn", "code": "111111" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: Value = helpers::read_json(resp).await;
    let access_token = body["accessToken"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/logout")
                .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let active: Option<i64> = sqlx::query_scalar(
        "SELECT count(*) FROM identity.sessions \
         WHERE account_id = (SELECT id FROM identity.accounts WHERE email = $1) \
         AND revoked_at IS NULL",
    )
    .bind("kate@tongji.edu.cn")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(active.unwrap(), 0);
}

#[tokio::test]
async fn test_me_returns_account() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("leo@tongji.edu.cn")
        .bind("leo")
        .execute(&pool)
        .await
        .unwrap();

    helpers::insert_valid_code(&pool, "leo@tongji.edu.cn", "222222").await;

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "leo@tongji.edu.cn", "code": "222222" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: Value = helpers::read_json(resp).await;
    let access_token = body["accessToken"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/me")
                .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["handle"], "leo");
}

#[tokio::test]
async fn test_me_rejects_invalid_token() {
    let (_, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/me")
                .header(header::AUTHORIZATION, "Bearer not.a.valid.token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_new_account_has_user_role() {
    let (pool, _) = create_test_app().await;

    helpers::insert_valid_code(&pool, "mike@tongji.edu.cn", "333333").await;

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "mike@tongji.edu.cn", "code": "333333" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["account"]["role"], "user");
}

#[tokio::test]
async fn test_handle_auto_generated_on_first_login() {
    let (pool, _) = create_test_app().await;

    helpers::insert_valid_code(&pool, "nancy@tongji.edu.cn", "444444").await;

    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": "nancy@tongji.edu.cn", "code": "444444" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["account"]["handle"], "nancy");
}

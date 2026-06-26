//! Integration tests for the identity domain — profile & wallet operations.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_app, create_test_app_with_pool};
use ring::signature::KeyPair;
use serde_json::{json, Value};
use tower::ServiceExt;

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
    let app = create_test_app_with_pool(pool).await;

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

/// ── update avatar ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_avatar_url_succeeds() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("rose@tongji.edu.cn")
        .bind("rose")
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("rose@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

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
    assert_eq!(body["avatarUrl"], "https://example.com/avatar.png");
}

/// ── get wallet ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_wallet_returns_balance() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("sam@tongji.edu.cn")
        .bind("sam")
        .execute(&pool)
        .await
        .unwrap();

    let account_id: i64 = sqlx::query_scalar("SELECT id FROM identity.accounts WHERE email = $1")
        .bind("sam@tongji.edu.cn")
        .fetch_one(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO credit.wallets (account_id, balance) VALUES ($1, 100)")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("sam@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/wallet")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["balance"], 100);
}

#[tokio::test]
async fn test_get_wallet_for_new_account_returns_zero() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("tina@tongji.edu.cn")
        .bind("tina")
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("tina@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/wallet")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = helpers::read_json(resp).await;
    assert_eq!(body["balance"], 0);
}

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

    let (token, account_id) = helpers::create_access_token_for("uma@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool.clone()).await;

    let rng = ring::rand::SystemRandom::new();
    let pkcs8_bytes =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).expect("generate key pair");
    let key_pair =
        ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).expect("parse key pair");
    let public_key_bytes = key_pair.public_key().as_ref();
    let public_key_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, public_key_bytes);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "publicKey": public_key_b64 }).to_string()))
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
async fn test_bind_key_invalid_base64_rejects() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("vera@tongji.edu.cn")
        .bind("vera")
        .execute(&pool)
        .await
        .unwrap();

    let (token, _) = helpers::create_access_token_for("vera@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "publicKey": "not-valid-base64!!!" }).to_string()))
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

    let (token, _) = helpers::create_access_token_for("wendy@tongji.edu.cn", &pool).await;
    let app = create_test_app_with_pool(pool).await;

    let short_key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 16]);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/wallet/bind")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(json!({ "publicKey": short_key }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_bind_key_duplicate_rejects() {
    let (pool, _) = create_test_app().await;

    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2)")
        .bind("xena@tongji.edu.cn")
        .bind("xena")
        .execute(&pool)
        .await
        .unwrap();

    let (token, account_id) = helpers::create_access_token_for("xena@tongji.edu.cn", &pool).await;

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
                .body(Body::from(json!({ "publicKey": key_b64 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

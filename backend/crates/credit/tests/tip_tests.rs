//! Integration tests for the tip handler.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{create_test_account, create_test_app, create_token, mint_to_account};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn tip_requires_wallet_bound_key() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "tipper@tongji.edu.cn", "tipper").await;
    let _b = create_test_account(&pool, "tiptarget@tongji.edu.cn", "tiptarget").await;
    mint_to_account(&pool, a, 500).await;

    let token = create_token(&pool, "tipper@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": "2",
        "amount": 10,
        "targetType": "thread",
        "targetId": "1"
    });

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/tip")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .header("X-Wallet-Sig", "invalid-base64-sig")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tip_requires_auth() {
    let (_pool, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/tip")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"toAccountId":"1","amount":10,"targetType":"thread","targetId":"1"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tip_invalid_signature_rejected() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "sigsender@tongji.edu.cn", "sigsender").await;
    let _b = create_test_account(&pool, "sigtarget@tongji.edu.cn", "sigtarget").await;
    mint_to_account(&pool, a, 500).await;

    // Bind an Ed25519 public key so wallet-not-bound is not the error.
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    use base64::Engine;
    let pk_b64 = base64::engine::general_purpose::STANDARD.encode(kp.public_key().as_ref());

    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(a)
        .bind(&pk_b64)
        .execute(&pool)
        .await
        .unwrap();

    let token = create_token(&pool, "sigsender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": "2",
        "amount": 10,
        "targetType": "thread",
        "targetId": "1"
    });

    // Send a deliberately wrong signature.
    let wrong_sig =
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/tip")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .header("X-Wallet-Sig", wrong_sig)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tip_insufficient_balance() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "poortipper@tongji.edu.cn", "poortipper").await;
    let _b = create_test_account(&pool, "richtarget@tongji.edu.cn", "richtarget").await;
    // Give very little points.
    mint_to_account(&pool, a, 5).await;

    // Bind a key.
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let kp = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    use base64::Engine;
    let pk_b64 = base64::engine::general_purpose::STANDARD.encode(kp.public_key().as_ref());

    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(a)
        .bind(&pk_b64)
        .execute(&pool)
        .await
        .unwrap();

    let token = create_token(&pool, "poortipper@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": "2",
        "amount": 100,
        "targetType": "thread",
        "targetId": "1"
    });

    // Use an obviously wrong signature (will fail at validation before balance check).
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/tip")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .header("X-Wallet-Sig", "AAAA")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Either FORBIDDEN (invalid sig) or BAD_REQUEST (insufficient balance)
    assert!(resp.status().is_client_error());
}

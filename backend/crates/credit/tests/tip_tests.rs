//! Integration tests for the tip handler.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{
    create_test_account, create_test_app, create_token, mint_to_account, read_json,
    signed_post_request,
};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn tip_requires_wallet_bound_key() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "tipper@tongji.edu.cn", "tipper").await;
    let b = create_test_account(&pool, "tiptarget@tongji.edu.cn", "tiptarget").await;
    mint_to_account(&pool, a, 500).await;

    let token = create_token(&pool, "tipper@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": b.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": "1"
    });

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Idempotency-Key", uuid::Uuid::new_v4().to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "action": "credit.tip", "request": body }).to_string()))
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
    let b = create_test_account(&pool, "sigtarget@tongji.edu.cn", "sigtarget").await;
    mint_to_account(&pool, a, 500).await;

    let token = create_token(&pool, "sigsender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": b.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": "1"
    });

    let mut request = signed_post_request(
        &app,
        &pool,
        &token,
        a,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    request.headers_mut().insert("X-Wallet-Sig", "AAAA".parse().unwrap());
    let resp = app.oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tip_insufficient_balance() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "poortipper@tongji.edu.cn", "poortipper").await;
    let b = create_test_account(&pool, "richtarget@tongji.edu.cn", "richtarget").await;
    // Give very little points.
    mint_to_account(&pool, a, 5).await;

    let token = create_token(&pool, "poortipper@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": b.to_string(),
        "amount": 100,
        "targetType": "thread",
        "targetId": "1"
    });

    let request = signed_post_request(
        &app,
        &pool,
        &token,
        a,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    let resp = app.oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tip_signature_is_bound_to_exact_ledger_entry() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(&pool, "exactsender@tongji.edu.cn", "exactsender").await;
    let recipient =
        create_test_account(&pool, "exactrecipient@tongji.edu.cn", "exactrecipient").await;
    mint_to_account(&pool, sender, 50).await;
    let token = create_token(&pool, "exactsender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": "42"
    });
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let verify = app
        .oneshot(
            Request::builder().uri("/api/v2/wallet/ledger/verify").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert!(read_json(verify).await["ok"].as_bool().unwrap());
}

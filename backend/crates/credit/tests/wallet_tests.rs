//! Integration tests for wallet balance and reconciliation.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{create_test_account, create_test_app, create_token, mint_to_account, read_json};
use tower::ServiceExt;

#[tokio::test]
async fn wallet_returns_zero_for_new_account() {
    let (pool, app) = create_test_app().await;
    create_test_account(&pool, "walletzero@tongji.edu.cn", "walletzero").await;

    let token = create_token(&pool, "walletzero@tongji.edu.cn").await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert_eq!(json["balance"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn wallet_reflects_minted_points() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "minted@tongji.edu.cn", "minted").await;
    mint_to_account(&pool, account_id, 200).await;

    let token = create_token(&pool, "minted@tongji.edu.cn").await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert_eq!(json["balance"].as_i64().unwrap(), 200);
}

#[tokio::test]
async fn wallet_requires_auth() {
    let (_pool, app) = create_test_app().await;

    let resp = app
        .oneshot(Request::builder().uri("/api/v2/wallet").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wallet_balance_reconciliation() {
    let (pool, _app) = create_test_app().await;
    let a = create_test_account(&pool, "recon1@tongji.edu.cn", "recon1").await;

    mint_to_account(&pool, a, 100).await;
    mint_to_account(&pool, a, 50).await;

    // Directly verify the wallet row matches ledger sum.
    let wallet_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(a)
            .fetch_one(&pool)
            .await
            .unwrap();

    // Total minted = 150.
    assert_eq!(wallet_balance, 150);
}

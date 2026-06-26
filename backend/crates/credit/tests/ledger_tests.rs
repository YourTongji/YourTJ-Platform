//! Integration tests for the credit ledger: append, hash chain, and verify.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{create_test_account, create_test_app, mint_to_account, read_json};
use tower::ServiceExt;

#[tokio::test]
async fn ledger_append_creates_entry() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "ledger1@tongji.edu.cn", "ledger1").await;

    // Mint points which appends a ledger entry.
    mint_to_account(&pool, account_id, 100).await;

    // Verify we can read the ledger.
    let token = helpers::create_token(&pool, "ledger1@tongji.edu.cn").await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert!(json["hasMore"].as_bool().is_some());
    let items = json["items"].as_array().unwrap();
    assert!(!items.is_empty());
    let entry = &items[0];
    assert_eq!(entry["type"], "mint");
    assert_eq!(entry["amount"].as_i64().unwrap(), 100);
}

#[tokio::test]
async fn ledger_verify_empty_is_ok() {
    let (_pool, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder().uri("/api/v2/wallet/ledger/verify").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert!(json["ok"].as_bool().unwrap());
}

#[tokio::test]
async fn ledger_verify_with_entries() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "verifier@tongji.edu.cn", "verifier").await;
    mint_to_account(&pool, account_id, 50).await;
    mint_to_account(&pool, account_id, 30).await;

    let resp = app
        .oneshot(
            Request::builder().uri("/api/v2/wallet/ledger/verify").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert!(json["ok"].as_bool().unwrap(), "ledger verification failed: {json}");
}

#[tokio::test]
async fn ledger_hash_chain_is_linear() {
    let (pool, _app) = create_test_app().await;
    let account_id = create_test_account(&pool, "chainer@tongji.edu.cn", "chainer").await;

    // Mint several entries.
    for _ in 0..3 {
        mint_to_account(&pool, account_id, 10).await;
    }

    // Read ledger rows directly.
    let rows: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT seq, prev_hash, hash FROM credit.ledger ORDER BY seq ASC")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(rows.len(), 3);
    for i in 1..rows.len() {
        assert_eq!(
            rows[i].1,
            rows[i - 1].2,
            "hash chain broken at seq {}: prev_hash does not match previous hash",
            rows[i].0
        );
    }
}

#[tokio::test]
async fn ledger_requires_auth() {
    let (_pool, app) = create_test_app().await;

    let resp = app
        .oneshot(Request::builder().uri("/api/v2/wallet/ledger").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

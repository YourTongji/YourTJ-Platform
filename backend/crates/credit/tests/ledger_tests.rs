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

#[tokio::test]
async fn ledger_pagination_is_strict_and_reports_has_more_exactly() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "pager@tongji.edu.cn", "pager").await;
    for _ in 0..3 {
        mint_to_account(&pool, account_id, 10).await;
    }
    let token = helpers::create_token(&pool, "pager@tongji.edu.cn").await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger?limit=2")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first = read_json(first).await;
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    assert_eq!(first["hasMore"], true);
    let cursor = first["nextCursor"].as_str().unwrap();

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/wallet/ledger?limit=2&cursor={cursor}"))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second = read_json(second).await;
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["hasMore"], false);
    assert!(second["nextCursor"].is_null());

    for query in ["limit=0", "limit=101", "limit=2&cursor=not-an-id", "limit=2&cursor=0"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v2/wallet/ledger?{query}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "query={query}");
    }
}

#[tokio::test]
async fn ledger_rows_cannot_be_updated_or_deleted() {
    let (pool, _app) = create_test_app().await;
    let account_id = create_test_account(&pool, "appendonly@tongji.edu.cn", "appendonly").await;
    mint_to_account(&pool, account_id, 10).await;

    let update = sqlx::query("UPDATE credit.ledger SET amount = amount + 1").execute(&pool).await;
    assert!(update.is_err());
    let delete = sqlx::query("DELETE FROM credit.ledger").execute(&pool).await;
    assert!(delete.is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn integrity_migration_preserves_historical_anomaly_but_blocks_new_ones() {
    let (pool, _app) = create_test_app().await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(
        "DROP TRIGGER credit_ledger_reject_mutation ON credit.ledger; \
         ALTER TABLE credit.products DROP CONSTRAINT credit_products_stock_nonnegative; \
         ALTER TABLE credit.tasks DROP CONSTRAINT credit_tasks_no_self_accept; \
         ALTER TABLE credit.purchases DROP CONSTRAINT credit_purchases_distinct_parties; \
         ALTER TABLE credit.ledger DROP CONSTRAINT credit_ledger_controlled_flow_type;",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO credit.ledger \
         (tx_id, type, amount, nonce, signer, signature, prev_hash, hash) \
         VALUES ('historical-anomaly', 'admin_adjust', 1, 'legacy-nonce', \
                 'system', 'legacy-signature', repeat('0', 64), 'legacy-hash')",
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    sqlx::raw_sql(include_str!("../../../migrations/0032_credit_integrity_constraints.sql"))
        .execute(&mut *tx)
        .await
        .unwrap();
    let validated: bool = sqlx::query_scalar(
        "SELECT convalidated FROM pg_constraint \
         WHERE conname = 'credit_ledger_controlled_flow_type'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(!validated);
    let new_anomaly = sqlx::query(
        "INSERT INTO credit.ledger \
         (tx_id, type, amount, nonce, signer, signature, prev_hash, hash) \
         VALUES ('new-anomaly', 'admin_adjust', 1, 'new-nonce', \
                 'system', 'new-signature', repeat('0', 64), 'new-hash')",
    )
    .execute(&mut *tx)
    .await;
    assert!(new_anomaly.is_err());
    tx.rollback().await.unwrap();
}

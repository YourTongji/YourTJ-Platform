//! Integration tests for wallet balance and reconciliation.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine as _;
use helpers::{
    create_test_account, create_test_app, create_tip_thread, create_token, mint_to_account,
    read_json,
};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn appeal_scoped_token_is_rejected_by_every_credit_surface() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "appealcredit@tongji.edu.cn", "appealcredit").await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("promote scoped-token test account");
    let token = identity::auth::create_appeal_access_token(
        account_id,
        "integration-test-secret-32bytes!",
        3_600,
    )
    .expect("create appeal access token");

    let requests = [
        Request::builder().uri("/api/v2/wallet").body(Body::empty()).expect("wallet request"),
        Request::builder()
            .uri("/api/v2/wallet/ledger")
            .body(Body::empty())
            .expect("ledger request"),
        Request::builder()
            .uri("/api/v2/credit/purchases")
            .body(Body::empty())
            .expect("purchase request"),
        Request::builder()
            .uri("/api/v2/admin/credit/reconciliations")
            .body(Body::empty())
            .expect("admin credit request"),
        Request::builder()
            .method("POST")
            .uri("/api/v2/credit/products")
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "must not be created",
                    "description": "appeal credentials cannot mutate credit",
                    "price": 1,
                    "stock": 1
                })
                .to_string(),
            ))
            .expect("product request"),
    ];
    for mut request in requests {
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {token}").parse().expect("authorization header"),
        );
        let response = app.clone().oneshot(request).await.expect("credit response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let product_count: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM credit.products")
        .fetch_one(&pool)
        .await
        .expect("product count");
    assert_eq!(product_count, 0);
}

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
async fn signing_intent_uses_the_only_active_wallet_key() {
    let (pool, app) = create_test_app().await;
    let sender_id =
        create_test_account(&pool, "canonical-key@tongji.edu.cn", "canonical-key").await;
    let recipient_id =
        create_test_account(&pool, "canonical-target@tongji.edu.cn", "canonical-target").await;
    mint_to_account(&pool, sender_id, 20).await;
    let thread_id = create_tip_thread(&pool, recipient_id).await;

    let historical_public_key = base64::engine::general_purpose::STANDARD
        .encode(credit::ledger::derive_public_key(&[8u8; 32]));
    let canonical_public_key = base64::engine::general_purpose::STANDARD
        .encode(credit::ledger::derive_public_key(&[9u8; 32]));
    sqlx::query(
        "INSERT INTO identity.account_keys (account_id, public_key, created_at, revoked_at) \
         VALUES ($1, $2, now() - interval '1 day', now()), ($1, $3, now(), NULL)",
    )
    .bind(sender_id)
    .bind(&historical_public_key)
    .bind(&canonical_public_key)
    .execute(&pool)
    .await
    .expect("insert canonical and later wallet keys");

    let token = create_token(&pool, "canonical-key@tongji.edu.cn").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Idempotency-Key", uuid::Uuid::new_v4().to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "credit.tip",
                        "request": {
                            "toAccountId": recipient_id.to_string(),
                            "amount": 1,
                            "targetType": "thread",
                            "targetId": thread_id.to_string()
                        }
                    })
                    .to_string(),
                ))
                .expect("build signing-intent request"),
        )
        .await
        .expect("signing-intent response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let signing_bytes: serde_json::Value =
        serde_json::from_str(body["signingBytes"].as_str().expect("signing bytes"))
            .expect("parse signing bytes");
    assert_eq!(signing_bytes["publicKey"], canonical_public_key);
    assert_ne!(signing_bytes["publicKey"], historical_public_key);
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

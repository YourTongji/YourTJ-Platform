//! Integration tests for the escrow market: tasks and products.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{create_test_account, create_test_app, create_token, mint_to_account, read_json};
use serde_json::json;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn task_create_requires_balance() {
    let (pool, app) = create_test_app().await;
    create_test_account(&pool, "taskcreator@tongji.edu.cn", "taskcreator").await;
    // No points minted.

    let token = create_token(&pool, "taskcreator@tongji.edu.cn").await;
    let body = json!({
        "title": "Test Task",
        "rewardAmount": 100
    });

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn task_create_and_list() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "tasklister@tongji.edu.cn", "tasklister").await;
    mint_to_account(&pool, a, 500).await;

    let token = create_token(&pool, "tasklister@tongji.edu.cn").await;
    let body = json!({
        "title": "My Bounty",
        "rewardAmount": 100
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert_eq!(json["title"], "My Bounty");
    assert_eq!(json["rewardAmount"].as_i64().unwrap(), 100);
    assert_eq!(json["status"], "open");

    // List tasks.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    let items = json["items"].as_array().unwrap();
    assert!(!items.is_empty());
}

#[tokio::test]
async fn task_list_requires_auth() {
    let (_pool, app) = create_test_app().await;

    let resp = app
        .oneshot(Request::builder().uri("/api/v2/credit/tasks").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn task_accept_and_submit_flow() {
    let (pool, app) = create_test_app().await;
    let creator = create_test_account(&pool, "flowcreator@tongji.edu.cn", "flowcreator").await;
    let acceptor = create_test_account(&pool, "flowacceptor@tongji.edu.cn", "flowacceptor").await;
    mint_to_account(&pool, creator, 500).await;

    // Create task.
    let token = create_token(&pool, "flowcreator@tongji.edu.cn").await;
    let body = json!({
        "title": "Flow Bounty",
        "rewardAmount": 200
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let task = read_json(resp).await;
    let task_id = task["id"].as_str().unwrap();

    // Creator's balance should be reduced by 200.
    let creator_wallet: (i64,) =
        sqlx::query_as("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(creator)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(creator_wallet.0, 300);

    // Accept as the other user.
    let acceptor_token = create_token(&pool, "flowacceptor@tongji.edu.cn").await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/tasks/{task_id}/accept"))
                .method("POST")
                .header("Authorization", format!("Bearer {acceptor_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Submit the task.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/tasks/{task_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {acceptor_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"submit"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Confirm as creator (releases escrow to acceptor).
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/tasks/{task_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"confirm"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Acceptor should now have 200 points.
    let acceptor_wallet: (i64,) =
        sqlx::query_as("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(acceptor)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(acceptor_wallet.0, 200);
}

// ---------------------------------------------------------------------------
// Products
// ---------------------------------------------------------------------------

#[tokio::test]
async fn product_create_and_list() {
    let (pool, app) = create_test_app().await;
    let _seller = create_test_account(&pool, "seller@tongji.edu.cn", "seller").await;

    let token = create_token(&pool, "seller@tongji.edu.cn").await;
    let body = json!({
        "title": "Test Product",
        "price": 50,
        "stock": 10
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/products")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let product = read_json(resp).await;
    assert_eq!(product["title"], "Test Product");
    assert_eq!(product["price"].as_i64().unwrap(), 50);
    assert_eq!(product["status"], "on_sale");

    // List.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/products?limit=10")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = read_json(resp).await;
    assert!(!json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn purchase_flow_releases_escrow() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(&pool, "pseller@tongji.edu.cn", "pseller").await;
    let buyer = create_test_account(&pool, "pbuyer@tongji.edu.cn", "pbuyer").await;
    mint_to_account(&pool, buyer, 500).await;

    // Create product.
    let seller_token = create_token(&pool, "pseller@tongji.edu.cn").await;
    let body = json!({
        "title": "Buyable Product",
        "price": 100,
        "stock": 1
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/products")
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let product = read_json(resp).await;
    let product_id = product["id"].as_str().unwrap();

    // Purchase as buyer.
    let buyer_token = create_token(&pool, "pbuyer@tongji.edu.cn").await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/products/{product_id}/purchase"))
                .method("POST")
                .header("Authorization", format!("Bearer {buyer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let purchase = read_json(resp).await;
    let purchase_id = purchase["id"].as_str().unwrap();
    assert_eq!(purchase["status"], "pending");

    // Buyer balance should have decreased.
    let buyer_balance: (i64,) =
        sqlx::query_as("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(buyer)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(buyer_balance.0, 400);

    // Accept → deliver → confirm
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/purchases/{purchase_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"accept"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/purchases/{purchase_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"deliver"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/purchases/{purchase_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {buyer_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"confirm"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Seller should have received 100 points.
    let seller_balance: (i64,) =
        sqlx::query_as("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(seller)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seller_balance.0, 100);
}

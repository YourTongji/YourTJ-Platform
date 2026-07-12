//! Integration tests for the escrow market: tasks and products.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{
    create_test_account, create_test_app, create_token, mint_to_account, read_json,
    signed_post_request,
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn assert_ledger_and_wallet_projection(
    pool: &PgPool,
    app: &axum::Router,
    account_ids: &[i64],
) {
    let response = app
        .clone()
        .oneshot(
            Request::builder().uri("/api/v2/wallet/ledger/verify").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(read_json(response).await["ok"].as_bool().unwrap());

    for account_id in account_ids {
        let projected: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM( \
               CASE WHEN to_account = $1 THEN amount ELSE 0 END - \
               CASE WHEN from_account = $1 THEN amount ELSE 0 END \
             ), 0)::bigint FROM credit.ledger",
        )
        .bind(account_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let cached: i64 =
            sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
                .bind(account_id)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(cached, projected, "wallet projection mismatch for {account_id}");
    }
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn task_create_requires_balance() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "taskcreator@tongji.edu.cn", "taskcreator").await;
    // No points minted.

    let token = create_token(&pool, "taskcreator@tongji.edu.cn").await;
    let body = json!({
        "title": "Test Task",
        "rewardAmount": 100
    });

    let request = signed_post_request(
        &app,
        &pool,
        &token,
        account_id,
        "/api/v2/credit/tasks",
        "credit.task.create",
        body.clone(),
        Some(body),
    )
    .await;
    let resp = app.oneshot(request).await.unwrap();

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

    let request = signed_post_request(
        &app,
        &pool,
        &token,
        a,
        "/api/v2/credit/tasks",
        "credit.task.create",
        body.clone(),
        Some(body),
    )
    .await;
    let resp = app.clone().oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
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
async fn task_list_treats_all_as_no_filter_and_rejects_unknown_status() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "task-filter@tongji.edu.cn", "task-filter").await;
    let token = create_token(&pool, "task-filter@tongji.edu.cn").await;
    sqlx::query(
        "INSERT INTO credit.tasks \
         (creator_id, title, reward_amount, hold_tx_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(account_id)
    .bind("Filterable task")
    .bind(10_i64)
    .bind("filter-test-hold")
    .execute(&pool)
    .await
    .expect("seed filterable task");

    let all_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks?status=all")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("all tasks request"),
        )
        .await
        .expect("all tasks response");
    assert_eq!(all_response.status(), StatusCode::OK);
    let all_tasks = read_json(all_response).await;
    assert_eq!(all_tasks["items"].as_array().expect("task items").len(), 1);

    let invalid_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks?status=unknown")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("invalid tasks request"),
        )
        .await
        .expect("invalid tasks response");
    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);
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

    let request = signed_post_request(
        &app,
        &pool,
        &token,
        creator,
        "/api/v2/credit/tasks",
        "credit.task.create",
        body.clone(),
        Some(body),
    )
    .await;
    let resp = app.clone().oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
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
    let action = json!({ "action": "confirm" });
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        creator,
        &format!("/api/v2/credit/tasks/{task_id}/action"),
        "credit.task.action",
        json!({ "id": task_id, "action": "confirm" }),
        Some(action),
    )
    .await;
    let resp = app.oneshot(request).await.unwrap();

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

    assert_eq!(resp.status(), StatusCode::CREATED);
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

    assert_eq!(resp.status(), StatusCode::CREATED);
    let product = read_json(resp).await;
    let product_id = product["id"].as_str().unwrap();

    // Purchase as buyer.
    let buyer_token = create_token(&pool, "pbuyer@tongji.edu.cn").await;
    let request = signed_post_request(
        &app,
        &pool,
        &buyer_token,
        buyer,
        &format!("/api/v2/credit/products/{product_id}/purchase"),
        "credit.product.purchase",
        json!({ "productId": product_id }),
        None,
    )
    .await;
    let resp = app.clone().oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
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

    let action = json!({ "action": "confirm" });
    let request = signed_post_request(
        &app,
        &pool,
        &buyer_token,
        buyer,
        &format!("/api/v2/credit/purchases/{purchase_id}/action"),
        "credit.purchase.action",
        json!({ "id": purchase_id, "action": "confirm" }),
        Some(action),
    )
    .await;
    let resp = app.oneshot(request).await.unwrap();
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

#[tokio::test]
async fn task_delete_open_refunds_escrow_and_removes_row() {
    let (pool, app) = create_test_app().await;
    let creator = create_test_account(&pool, "delcreator@tongji.edu.cn", "delcreator").await;
    mint_to_account(&pool, creator, 500).await;

    // Create a task — escrow_hold deducts 150.
    let token = create_token(&pool, "delcreator@tongji.edu.cn").await;
    let body = json!({ "title": "Deletable Bounty", "rewardAmount": 150 });
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        creator,
        "/api/v2/credit/tasks",
        "credit.task.create",
        body.clone(),
        Some(body),
    )
    .await;
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let task = read_json(resp).await;
    let task_id = task["id"].as_str().unwrap().to_string();

    let held: (i64,) = sqlx::query_as("SELECT balance FROM credit.wallets WHERE account_id = $1")
        .bind(creator)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(held.0, 350);

    // Delete the open task — must refund the escrow and remove the row.
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        creator,
        &format!("/api/v2/credit/tasks/{task_id}/action"),
        "credit.task.action",
        json!({ "id": task_id, "action": "delete" }),
        Some(json!({ "action": "delete" })),
    )
    .await;
    let resp = app.clone().oneshot(request).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Creator is made whole again.
    let refunded: (i64,) =
        sqlx::query_as("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(creator)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(refunded.0, 500);

    // The task row is gone.
    let remaining: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM credit.tasks WHERE id = $1::bigint")
            .bind(task_id.parse::<i64>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining.0, 0);

    // Ledger still verifies after a hold + release pair.
    let resp = app
        .oneshot(
            Request::builder().uri("/api/v2/wallet/ledger/verify").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let verify = read_json(resp).await;
    assert!(verify["ok"].as_bool().unwrap(), "ledger verify failed: {verify}");
}

#[tokio::test]
async fn purchase_rejects_sold_out_product_without_charging() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(&pool, "soldoutseller@tongji.edu.cn", "soldoutseller").await;
    let buyer = create_test_account(&pool, "soldoutbuyer@tongji.edu.cn", "soldoutbuyer").await;
    mint_to_account(&pool, buyer, 100).await;
    let product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products (seller_id, title, price, stock, status) \
         VALUES ($1, 'Sold out', 40, 0, 'sold_out') RETURNING id",
    )
    .bind(seller)
    .fetch_one(&pool)
    .await
    .unwrap();
    let token = create_token(&pool, "soldoutbuyer@tongji.edu.cn").await;
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        buyer,
        &format!("/api/v2/credit/products/{product_id}/purchase"),
        "credit.product.purchase",
        json!({ "productId": product_id.to_string() }),
        None,
    )
    .await;

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(buyer)
            .fetch_one(&pool)
            .await
            .unwrap();
    let purchases: i64 =
        sqlx::query_scalar("SELECT count(*) FROM credit.purchases WHERE buyer_id = $1")
            .bind(buyer)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(balance, 100);
    assert_eq!(purchases, 0);
}

#[tokio::test]
async fn concurrent_purchase_confirm_releases_escrow_once() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(&pool, "raceseller@tongji.edu.cn", "raceseller").await;
    let buyer = create_test_account(&pool, "racebuyer@tongji.edu.cn", "racebuyer").await;
    let product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products (seller_id, title, price, stock) \
         VALUES ($1, 'Race product', 25, 1) RETURNING id",
    )
    .bind(seller)
    .fetch_one(&pool)
    .await
    .unwrap();
    let purchase_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.purchases \
         (product_id, buyer_id, seller_id, amount, status, hold_tx_id) \
         VALUES ($1, $2, $3, 25, 'delivered', 'test-hold') RETURNING id",
    )
    .bind(product_id)
    .bind(buyer)
    .bind(seller)
    .fetch_one(&pool)
    .await
    .unwrap();
    let token = create_token(&pool, "racebuyer@tongji.edu.cn").await;
    let uri = format!("/api/v2/credit/purchases/{purchase_id}/action");
    let signing_body = json!({ "id": purchase_id.to_string(), "action": "confirm" });
    let first = signed_post_request(
        &app,
        &pool,
        &token,
        buyer,
        &uri,
        "credit.purchase.action",
        signing_body.clone(),
        Some(json!({ "action": "confirm" })),
    )
    .await;
    let second = signed_post_request(
        &app,
        &pool,
        &token,
        buyer,
        &uri,
        "credit.purchase.action",
        signing_body,
        Some(json!({ "action": "confirm" })),
    )
    .await;

    let (first_response, second_response) =
        tokio::join!(app.clone().oneshot(first), app.clone().oneshot(second));
    let statuses = [first_response.unwrap().status(), second_response.unwrap().status()];
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::NO_CONTENT).count(), 1);
    let seller_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(seller)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seller_balance, 25);
}

#[tokio::test]
async fn task_rejects_self_accept_and_allows_only_one_concurrent_acceptor() {
    let (pool, app) = create_test_app().await;
    let creator = create_test_account(&pool, "acceptcreator@tongji.edu.cn", "acceptcreator").await;
    let first_acceptor =
        create_test_account(&pool, "firstacceptor@tongji.edu.cn", "firstacceptor").await;
    let second_acceptor =
        create_test_account(&pool, "secondacceptor@tongji.edu.cn", "secondacceptor").await;
    mint_to_account(&pool, creator, 100).await;
    let creator_token = create_token(&pool, "acceptcreator@tongji.edu.cn").await;
    let body = json!({ "title": "Single owner task", "rewardAmount": 50 });
    let create_request = signed_post_request(
        &app,
        &pool,
        &creator_token,
        creator,
        "/api/v2/credit/tasks",
        "credit.task.create",
        body.clone(),
        Some(body),
    )
    .await;
    let created = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let task_id = read_json(created).await["id"].as_str().unwrap().to_string();
    let uri = format!("/api/v2/credit/tasks/{task_id}/accept");

    let self_accept = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
                .method("POST")
                .header("Authorization", format!("Bearer {creator_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(self_accept.status(), StatusCode::BAD_REQUEST);

    let first_token = create_token(&pool, "firstacceptor@tongji.edu.cn").await;
    let second_token = create_token(&pool, "secondacceptor@tongji.edu.cn").await;
    let first_request = Request::builder()
        .uri(&uri)
        .method("POST")
        .header("Authorization", format!("Bearer {first_token}"))
        .body(Body::empty())
        .unwrap();
    let second_request = Request::builder()
        .uri(&uri)
        .method("POST")
        .header("Authorization", format!("Bearer {second_token}"))
        .body(Body::empty())
        .unwrap();
    let (first, second) =
        tokio::join!(app.clone().oneshot(first_request), app.clone().oneshot(second_request));
    let statuses = [first.unwrap().status(), second.unwrap().status()];
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::NO_CONTENT).count(), 1);
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::CONFLICT).count(), 1);

    let accepted_by: i64 = sqlx::query_scalar(
        "SELECT acceptor_id FROM credit.tasks WHERE id = $1::bigint AND status = 'in_progress'",
    )
    .bind(task_id.parse::<i64>().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(accepted_by == first_acceptor || accepted_by == second_acceptor);
}

async fn create_pending_purchase(
    pool: &sqlx::PgPool,
    app: &axum::Router,
    seller_id: i64,
    buyer_id: i64,
    buyer_token: &str,
    title: &str,
) -> (i64, i64) {
    let product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products \
         (seller_id, title, price, stock, delivery_info) \
         VALUES ($1, $2, 30, 1, 'private delivery') RETURNING id",
    )
    .bind(seller_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .unwrap();
    mint_to_account(pool, buyer_id, 100).await;
    let request = signed_post_request(
        app,
        pool,
        buyer_token,
        buyer_id,
        &format!("/api/v2/credit/products/{product_id}/purchase"),
        "credit.product.purchase",
        json!({ "productId": product_id.to_string() }),
        None,
    )
    .await;
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let purchase_id = read_json(response).await["id"].as_str().unwrap().parse().unwrap();
    (product_id, purchase_id)
}

#[tokio::test]
async fn purchase_accept_vs_cancel_is_serialized_without_double_resolution() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(&pool, "acceptseller@tongji.edu.cn", "acceptseller").await;
    let buyer = create_test_account(&pool, "cancelbuyer@tongji.edu.cn", "cancelbuyer").await;
    let buyer_token = create_token(&pool, "cancelbuyer@tongji.edu.cn").await;
    let seller_token = create_token(&pool, "acceptseller@tongji.edu.cn").await;
    let (_product_id, purchase_id) =
        create_pending_purchase(&pool, &app, seller, buyer, &buyer_token, "Accept cancel race")
            .await;
    let uri = format!("/api/v2/credit/purchases/{purchase_id}/action");
    let cancel = signed_post_request(
        &app,
        &pool,
        &buyer_token,
        buyer,
        &uri,
        "credit.purchase.action",
        json!({ "id": purchase_id.to_string(), "action": "cancel" }),
        Some(json!({ "action": "cancel" })),
    )
    .await;
    let accept = Request::builder()
        .uri(&uri)
        .method("POST")
        .header("Authorization", format!("Bearer {seller_token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"action":"accept"}"#))
        .unwrap();

    let (accept, cancel) = tokio::join!(app.clone().oneshot(accept), app.clone().oneshot(cancel));
    let statuses = [accept.unwrap().status(), cancel.unwrap().status()];
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::NO_CONTENT).count(), 1);
    let (status, hold_tx_id): (String, Option<String>) =
        sqlx::query_as("SELECT status::text, hold_tx_id FROM credit.purchases WHERE id = $1")
            .bind(purchase_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(matches!(status.as_str(), "accepted" | "cancelled"));
    assert_eq!(hold_tx_id.is_none(), status == "cancelled");
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE type = 'escrow_release' AND metadata->>'purchase_id' = $1",
    )
    .bind(purchase_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(release_count, if status == "cancelled" { 1 } else { 0 });
    assert_ledger_and_wallet_projection(&pool, &app, &[buyer, seller]).await;
}

#[tokio::test]
async fn purchase_deliver_vs_cancel_is_serialized_without_double_resolution() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(&pool, "deliverseller@tongji.edu.cn", "deliverseller").await;
    let buyer = create_test_account(&pool, "deliverbuyer@tongji.edu.cn", "deliverbuyer").await;
    let buyer_token = create_token(&pool, "deliverbuyer@tongji.edu.cn").await;
    let seller_token = create_token(&pool, "deliverseller@tongji.edu.cn").await;
    let (_product_id, purchase_id) =
        create_pending_purchase(&pool, &app, seller, buyer, &buyer_token, "Deliver cancel race")
            .await;
    let uri = format!("/api/v2/credit/purchases/{purchase_id}/action");
    let accept = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"accept"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accept.status(), StatusCode::NO_CONTENT);
    let cancel = signed_post_request(
        &app,
        &pool,
        &buyer_token,
        buyer,
        &uri,
        "credit.purchase.action",
        json!({ "id": purchase_id.to_string(), "action": "cancel" }),
        Some(json!({ "action": "cancel" })),
    )
    .await;
    let deliver = Request::builder()
        .uri(&uri)
        .method("POST")
        .header("Authorization", format!("Bearer {seller_token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"action":"deliver"}"#))
        .unwrap();

    let (deliver, cancel) = tokio::join!(app.clone().oneshot(deliver), app.clone().oneshot(cancel));
    let statuses = [deliver.unwrap().status(), cancel.unwrap().status()];
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::NO_CONTENT).count(), 1);
    let (status, hold_tx_id): (String, Option<String>) =
        sqlx::query_as("SELECT status::text, hold_tx_id FROM credit.purchases WHERE id = $1")
            .bind(purchase_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(matches!(status.as_str(), "delivered" | "cancelled"));
    assert_eq!(hold_tx_id.is_none(), status == "cancelled");
    assert_ledger_and_wallet_projection(&pool, &app, &[buyer, seller]).await;
}

#[tokio::test]
async fn product_delivery_info_is_private_to_purchase_parties() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(&pool, "privateseller@tongji.edu.cn", "privateseller").await;
    let buyer = create_test_account(&pool, "privatebuyer@tongji.edu.cn", "privatebuyer").await;
    let outsider =
        create_test_account(&pool, "privateoutsider@tongji.edu.cn", "privateoutsider").await;
    let seller_token = create_token(&pool, "privateseller@tongji.edu.cn").await;
    let buyer_token = create_token(&pool, "privatebuyer@tongji.edu.cn").await;
    let outsider_token = create_token(&pool, "privateoutsider@tongji.edu.cn").await;
    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/products")
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "Private instructions",
                        "price": 30,
                        "stock": 1,
                        "deliveryInfo": "secret pickup code",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let product = read_json(create).await;
    assert!(product.get("deliveryInfo").is_none());
    let product_id = product["id"].as_str().unwrap();

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/products")
                .header("Authorization", format!("Bearer {outsider_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = read_json(list).await;
    assert!(list["items"][0].get("deliveryInfo").is_none());

    mint_to_account(&pool, buyer, 50).await;
    let purchase_request = signed_post_request(
        &app,
        &pool,
        &buyer_token,
        buyer,
        &format!("/api/v2/credit/products/{product_id}/purchase"),
        "credit.product.purchase",
        json!({ "productId": product_id }),
        None,
    )
    .await;
    let purchase = app.clone().oneshot(purchase_request).await.unwrap();
    assert_eq!(purchase.status(), StatusCode::CREATED);
    let purchase = read_json(purchase).await;
    assert_eq!(purchase["deliveryInfo"], "secret pickup code");
    assert!(purchase["createdAt"].as_i64().is_some());

    for token in [&buyer_token, &seller_token] {
        let orders = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v2/credit/purchases")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let orders = read_json(orders).await;
        assert_eq!(orders["items"][0]["deliveryInfo"], "secret pickup code");
    }
    let outsider_orders = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/purchases")
                .header("Authorization", format!("Bearer {outsider_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(read_json(outsider_orders).await["items"].as_array().unwrap().is_empty());

    let negative_stock = sqlx::query(
        "INSERT INTO credit.products (seller_id, title, price, stock) \
         VALUES ($1, 'Invalid stock', 1, -1)",
    )
    .bind(seller)
    .execute(&pool)
    .await;
    assert!(negative_stock.is_err());
    let _ = outsider;
}

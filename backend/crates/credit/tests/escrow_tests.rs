//! Integration tests for the escrow market: tasks and products.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use helpers::{
    create_test_account, create_test_app, create_token, mint_to_account, read_json,
    signed_post_request,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

fn signing_intent_id(request: &Request<Body>) -> uuid::Uuid {
    request
        .headers()
        .get("x-wallet-intent")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .expect("signed request intent id")
}

fn assert_json_omits_free_text(value: &Value, forbidden_value: &str) {
    match value {
        Value::Object(fields) => {
            assert!(!fields.contains_key("title"), "proof unexpectedly contains a title field");
            for field_value in fields.values() {
                assert_json_omits_free_text(field_value, forbidden_value);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_json_omits_free_text(item, forbidden_value);
            }
        }
        Value::String(field_value) => assert_ne!(field_value, forbidden_value),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

async fn assert_minimized_intent_proof(
    pool: &PgPool,
    intent_id: uuid::Uuid,
    expected_metadata: &Value,
    forbidden_value: &str,
) {
    let (signing_bytes, snapshot, ledger_entry, ledger_canonical): (
        String,
        Value,
        Option<Value>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT signing_bytes, snapshot, ledger_entry, ledger_canonical \
         FROM credit.signing_intents WHERE id = $1",
    )
    .bind(intent_id)
    .fetch_one(pool)
    .await
    .expect("load signing intent proof");
    let signing_envelope: Value =
        serde_json::from_str(&signing_bytes).expect("parse signing envelope");
    let ledger_entry = ledger_entry.expect("prepared ledger entry");
    let ledger_canonical: Value =
        serde_json::from_str(ledger_canonical.as_deref().expect("prepared ledger canonical form"))
            .expect("parse prepared ledger canonical form");

    assert_eq!(signing_envelope["snapshot"], snapshot);
    assert_eq!(signing_envelope["ledgerEntry"], ledger_entry);
    assert_eq!(ledger_canonical, ledger_entry);
    assert_eq!(ledger_entry["metadata"], *expected_metadata);
    assert!(
        ledger_entry["timestamp"].as_i64().expect("ledger timestamp")
            <= signing_envelope["expiresAt"].as_i64().expect("intent expiry")
    );
    assert!(snapshot.get("title").is_none());
    assert_json_omits_free_text(&signing_envelope, forbidden_value);
    assert_json_omits_free_text(&snapshot, forbidden_value);
    assert_json_omits_free_text(&ledger_entry, forbidden_value);
    assert_json_omits_free_text(&ledger_canonical, forbidden_value);
}

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

#[tokio::test]
async fn task_creator_cannot_issue_or_execute_an_acceptor_reject_action() {
    let (pool, app) = create_test_app().await;
    let creator =
        create_test_account(&pool, "reject-role-creator@tongji.edu.cn", "reject-role-creator")
            .await;
    let creator_token = create_token(&pool, "reject-role-creator@tongji.edu.cn").await;
    let public_key = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        credit::ledger::derive_public_key(&[22_u8; 32]),
    );
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(creator)
        .bind(public_key)
        .execute(&pool)
        .await
        .expect("bind creator wallet key");
    let task_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.tasks (creator_id, title, reward_amount, hold_tx_id) \
         VALUES ($1, 'Reject role task', 20, 'reject-role-hold') RETURNING id",
    )
    .bind(creator)
    .fetch_one(&pool)
    .await
    .expect("insert reject-role task");

    let intent_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {creator_token}"))
                .header("Idempotency-Key", "creator-cannot-reject")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "credit.task.action",
                        "request": { "id": task_id.to_string(), "action": "reject" },
                    })
                    .to_string(),
                ))
                .expect("build creator reject intent request"),
        )
        .await
        .expect("creator reject intent response");
    assert_eq!(intent_response.status(), StatusCode::BAD_REQUEST);

    let action_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/tasks/{task_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {creator_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "action": "reject" }).to_string()))
                .expect("build creator reject action request"),
        )
        .await
        .expect("creator reject action response");
    assert_eq!(action_response.status(), StatusCode::BAD_REQUEST);

    let (task_status, intent_count): (String, i64) = sqlx::query_as(
        "SELECT status::text, \
                (SELECT COUNT(*) FROM credit.signing_intents WHERE account_id = $2) \
         FROM credit.tasks WHERE id = $1",
    )
    .bind(task_id)
    .bind(creator)
    .fetch_one(&pool)
    .await
    .expect("load task and rejected intent count");
    assert_eq!(task_status, "open");
    assert_eq!(intent_count, 0);
}

#[tokio::test]
async fn task_accept_rejects_deleted_or_suspended_creator_without_side_effects() {
    let (pool, app) = create_test_app().await;
    let acceptor =
        create_test_account(&pool, "eligibleacceptor@tongji.edu.cn", "eligibleacceptor").await;
    let deleted_creator =
        create_test_account(&pool, "deletedcreator@tongji.edu.cn", "deletedcreator").await;
    let suspended_creator =
        create_test_account(&pool, "suspendedcreator@tongji.edu.cn", "suspendedcreator").await;
    let acceptor_token = create_token(&pool, "eligibleacceptor@tongji.edu.cn").await;

    sqlx::query(
        "UPDATE identity.accounts SET \
           status = 'deleted', deletion_requested_at = now() - interval '31 days', \
           deletion_recover_until = now() - interval '1 day', deleted_at = now() \
         WHERE id = $1",
    )
    .bind(deleted_creator)
    .execute(&pool)
    .await
    .expect("mark task creator deleted");
    sqlx::query("UPDATE identity.accounts SET status = 'suspended' WHERE id = $1")
        .bind(suspended_creator)
        .execute(&pool)
        .await
        .expect("mark task creator suspended");
    let deleted_task: i64 = sqlx::query_scalar(
        "INSERT INTO credit.tasks (creator_id, title, reward_amount, hold_tx_id) \
         VALUES ($1, 'Deleted creator task', 20, 'deleted-creator-hold') RETURNING id",
    )
    .bind(deleted_creator)
    .fetch_one(&pool)
    .await
    .expect("create deleted creator task");
    let suspended_task: i64 = sqlx::query_scalar(
        "INSERT INTO credit.tasks (creator_id, title, reward_amount, hold_tx_id) \
         VALUES ($1, 'Suspended creator task', 20, 'suspended-creator-hold') RETURNING id",
    )
    .bind(suspended_creator)
    .fetch_one(&pool)
    .await
    .expect("create suspended creator task");

    for task_id in [deleted_task, suspended_task] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v2/credit/tasks/{task_id}/accept"))
                    .method("POST")
                    .header("Authorization", format!("Bearer {acceptor_token}"))
                    .body(Body::empty())
                    .expect("build task accept request"),
            )
            .await
            .expect("task accept response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let task_states: Vec<(i64, String, Option<i64>)> = sqlx::query_as(
        "SELECT id, status::text, acceptor_id FROM credit.tasks \
         WHERE id = ANY($1) ORDER BY id",
    )
    .bind(vec![deleted_task, suspended_task])
    .fetch_all(&pool)
    .await
    .expect("load rejected task states");
    assert_eq!(task_states.len(), 2);
    assert!(task_states
        .iter()
        .all(|(_, status, acceptor_id)| status == "open" && acceptor_id.is_none()));
    let ledger_entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger")
        .fetch_one(&pool)
        .await
        .expect("count ledger entries");
    let signing_intents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.signing_intents")
        .fetch_one(&pool)
        .await
        .expect("count signing intents");
    assert_eq!(ledger_entries, 0);
    assert_eq!(signing_intents, 0);
    let acceptor_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(acceptor)
            .fetch_one(&pool)
            .await
            .expect("load acceptor wallet");
    assert_eq!(acceptor_balance, 0);
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
async fn task_and_product_proofs_omit_free_text_and_minimize_ledger_metadata() {
    const TASK_TITLE: &str = "TASK_PRIVATE_SENTINEL_5c61d6c2";
    const PRODUCT_TITLE: &str = "PRODUCT_PRIVATE_SENTINEL_80e7113c";

    let (pool, app) = create_test_app().await;
    let task_creator =
        create_test_account(&pool, "proofcreator@tongji.edu.cn", "proofcreator").await;
    let seller = create_test_account(&pool, "proofseller@tongji.edu.cn", "proofseller").await;
    let buyer = create_test_account(&pool, "proofbuyer@tongji.edu.cn", "proofbuyer").await;
    mint_to_account(&pool, task_creator, 100).await;
    mint_to_account(&pool, buyer, 100).await;

    let creator_token = create_token(&pool, "proofcreator@tongji.edu.cn").await;
    let task_body = json!({ "title": TASK_TITLE, "rewardAmount": 20 });
    let task_request = signed_post_request(
        &app,
        &pool,
        &creator_token,
        task_creator,
        "/api/v2/credit/tasks",
        "credit.task.create",
        task_body.clone(),
        Some(task_body),
    )
    .await;
    let task_intent_id = signing_intent_id(&task_request);
    let task_metadata = json!({ "signing_intent_id": task_intent_id.to_string() });
    assert_minimized_intent_proof(&pool, task_intent_id, &task_metadata, TASK_TITLE).await;

    let task_response = app.clone().oneshot(task_request).await.expect("create task response");
    assert_eq!(task_response.status(), StatusCode::CREATED);
    let task_ledger_metadata: Option<Value> = sqlx::query_scalar(
        "SELECT metadata FROM credit.ledger WHERE metadata->>'signing_intent_id' = $1",
    )
    .bind(task_intent_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("load task ledger metadata");
    let task_ledger_metadata = task_ledger_metadata.expect("task ledger metadata");
    assert_eq!(task_ledger_metadata, task_metadata);
    assert_json_omits_free_text(&task_ledger_metadata, TASK_TITLE);

    let product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products (seller_id, title, price, stock) \
         VALUES ($1, $2, 30, 1) RETURNING id",
    )
    .bind(seller)
    .bind(PRODUCT_TITLE)
    .fetch_one(&pool)
    .await
    .expect("create sentinel product");
    let buyer_token = create_token(&pool, "proofbuyer@tongji.edu.cn").await;
    let product_request = signed_post_request(
        &app,
        &pool,
        &buyer_token,
        buyer,
        &format!("/api/v2/credit/products/{product_id}/purchase"),
        "credit.product.purchase",
        json!({ "productId": product_id.to_string() }),
        None,
    )
    .await;
    let product_intent_id = signing_intent_id(&product_request);
    let product_metadata = json!({
        "product_id": product_id.to_string(),
        "signing_intent_id": product_intent_id.to_string(),
    });
    assert_minimized_intent_proof(&pool, product_intent_id, &product_metadata, PRODUCT_TITLE).await;

    let product_response = app.oneshot(product_request).await.expect("purchase product response");
    assert_eq!(product_response.status(), StatusCode::CREATED);
    let product_ledger_metadata: Option<Value> = sqlx::query_scalar(
        "SELECT metadata FROM credit.ledger WHERE metadata->>'signing_intent_id' = $1",
    )
    .bind(product_intent_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("load product ledger metadata");
    let product_ledger_metadata = product_ledger_metadata.expect("product ledger metadata");
    assert_eq!(product_ledger_metadata, product_metadata);
    assert_json_omits_free_text(&product_ledger_metadata, PRODUCT_TITLE);
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
async fn task_delete_cancelled_requires_no_wallet_signature_and_rejects_replay() {
    let (pool, app) = create_test_app().await;
    let creator =
        create_test_account(&pool, "cancelled-delete@tongji.edu.cn", "cancelled-delete").await;
    mint_to_account(&pool, creator, 100).await;
    let token = create_token(&pool, "cancelled-delete@tongji.edu.cn").await;

    let task_body = json!({ "title": "Cancelled task deletion", "rewardAmount": 30 });
    let create_request = signed_post_request(
        &app,
        &pool,
        &token,
        creator,
        "/api/v2/credit/tasks",
        "credit.task.create",
        task_body.clone(),
        Some(task_body),
    )
    .await;
    let created = app.clone().oneshot(create_request).await.expect("create task response");
    assert_eq!(created.status(), StatusCode::CREATED);
    let task_id = read_json(created).await["id"].as_str().expect("task id").to_string();
    let action_uri = format!("/api/v2/credit/tasks/{task_id}/action");

    let cancel_request = signed_post_request(
        &app,
        &pool,
        &token,
        creator,
        &action_uri,
        "credit.task.action",
        json!({ "id": task_id, "action": "cancel" }),
        Some(json!({ "action": "cancel" })),
    )
    .await;
    let cancelled = app.clone().oneshot(cancel_request).await.expect("cancel task response");
    assert_eq!(cancelled.status(), StatusCode::NO_CONTENT);
    let (status, hold_tx_id): (String, Option<String>) =
        sqlx::query_as("SELECT status::text, hold_tx_id FROM credit.tasks WHERE id = $1::bigint")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .expect("load cancelled task");
    assert_eq!(status, "cancelled");
    assert!(hold_tx_id.is_none());
    let ledger_entries_before_delete: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger")
            .fetch_one(&pool)
            .await
            .expect("count ledger entries before delete");
    let intents_before_delete: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM credit.signing_intents")
            .fetch_one(&pool)
            .await
            .expect("count signing intents before delete");

    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&action_uri)
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "action": "delete" }).to_string()))
                .expect("build unsigned cancelled-task delete request"),
        )
        .await
        .expect("delete cancelled task response");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let replay_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&action_uri)
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "action": "delete" }).to_string()))
                .expect("build replay request"),
        )
        .await
        .expect("replay response");
    assert_eq!(replay_response.status(), StatusCode::NOT_FOUND);

    let remaining_tasks: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM credit.tasks WHERE id = $1::bigint")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .expect("count remaining tasks");
    let ledger_entries_after_replay: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger")
        .fetch_one(&pool)
        .await
        .expect("count ledger entries after replay");
    let intents_after_replay: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM credit.signing_intents")
            .fetch_one(&pool)
            .await
            .expect("count signing intents after replay");
    assert_eq!(remaining_tasks, 0);
    assert_eq!(ledger_entries_after_replay, ledger_entries_before_delete);
    assert_eq!(intents_after_replay, intents_before_delete);
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
async fn purchase_rejects_deleted_or_suspended_seller_without_side_effects() {
    let (pool, app) = create_test_app().await;
    let buyer = create_test_account(&pool, "eligiblebuyer@tongji.edu.cn", "eligiblebuyer").await;
    let deleted_seller =
        create_test_account(&pool, "deletedseller@tongji.edu.cn", "deletedseller").await;
    let suspended_seller =
        create_test_account(&pool, "suspendedseller@tongji.edu.cn", "suspendedseller").await;
    mint_to_account(&pool, buyer, 200).await;
    let buyer_token = create_token(&pool, "eligiblebuyer@tongji.edu.cn").await;

    sqlx::query(
        "UPDATE identity.accounts SET \
           status = 'deleted', deletion_requested_at = now() - interval '31 days', \
           deletion_recover_until = now() - interval '1 day', deleted_at = now() \
         WHERE id = $1",
    )
    .bind(deleted_seller)
    .execute(&pool)
    .await
    .expect("mark product seller deleted");
    sqlx::query("UPDATE identity.accounts SET status = 'suspended' WHERE id = $1")
        .bind(suspended_seller)
        .execute(&pool)
        .await
        .expect("mark product seller suspended");
    let deleted_product: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products (seller_id, title, price, stock) \
         VALUES ($1, 'Deleted seller product', 30, 2) RETURNING id",
    )
    .bind(deleted_seller)
    .fetch_one(&pool)
    .await
    .expect("create deleted seller product");
    let suspended_product: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products (seller_id, title, price, stock) \
         VALUES ($1, 'Suspended seller product', 30, 2) RETURNING id",
    )
    .bind(suspended_seller)
    .fetch_one(&pool)
    .await
    .expect("create suspended seller product");

    let mut intent_ids = Vec::new();
    for product_id in [deleted_product, suspended_product] {
        let request = signed_post_request(
            &app,
            &pool,
            &buyer_token,
            buyer,
            &format!("/api/v2/credit/products/{product_id}/purchase"),
            "credit.product.purchase",
            json!({ "productId": product_id.to_string() }),
            None,
        )
        .await;
        intent_ids.push(signing_intent_id(&request));
        let response = app.clone().oneshot(request).await.expect("product purchase response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let product_states: Vec<(i64, i32, String)> = sqlx::query_as(
        "SELECT id, stock, status::text FROM credit.products \
         WHERE id = ANY($1) ORDER BY id",
    )
    .bind(vec![deleted_product, suspended_product])
    .fetch_all(&pool)
    .await
    .expect("load rejected product states");
    assert_eq!(product_states.len(), 2);
    assert!(product_states.iter().all(|(_, stock, status)| *stock == 2 && status == "on_sale"));
    let purchases: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.purchases")
        .fetch_one(&pool)
        .await
        .expect("count purchases");
    let ledger_entries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credit.ledger")
        .fetch_one(&pool)
        .await
        .expect("count ledger entries");
    let buyer_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(buyer)
            .fetch_one(&pool)
            .await
            .expect("load buyer wallet");
    assert_eq!(purchases, 0);
    assert_eq!(ledger_entries, 1);
    assert_eq!(buyer_balance, 200);
    for intent_id in intent_ids {
        let consumed: bool = sqlx::query_scalar(
            "SELECT consumed_at IS NOT NULL FROM credit.signing_intents WHERE id = $1",
        )
        .bind(intent_id)
        .fetch_one(&pool)
        .await
        .expect("load signing intent consumption state");
        assert!(!consumed);
    }
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
async fn purchase_signing_and_action_routes_hide_orders_from_non_parties() {
    let (pool, app) = create_test_app().await;
    let seller =
        create_test_account(&pool, "private-order-seller@tongji.edu.cn", "private-order-seller")
            .await;
    let buyer =
        create_test_account(&pool, "private-order-buyer@tongji.edu.cn", "private-order-buyer")
            .await;
    let outsider = create_test_account(
        &pool,
        "private-order-outsider@tongji.edu.cn",
        "private-order-outsider",
    )
    .await;
    let buyer_token = create_token(&pool, "private-order-buyer@tongji.edu.cn").await;
    let seller_token = create_token(&pool, "private-order-seller@tongji.edu.cn").await;
    let outsider_token = create_token(&pool, "private-order-outsider@tongji.edu.cn").await;
    for (account_id, seed_byte) in [(buyer, 17_u8), (seller, 18_u8), (outsider, 19_u8)] {
        let public_key = credit::ledger::derive_public_key(&[seed_byte; 32]);
        let public_key =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, public_key);
        sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
            .bind(account_id)
            .bind(public_key)
            .execute(&pool)
            .await
            .expect("bind test wallet key");
    }

    let product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products (seller_id, title, price, stock) \
         VALUES ($1, 'Private order product', 25, 1) RETURNING id",
    )
    .bind(seller)
    .fetch_one(&pool)
    .await
    .expect("insert product");
    let purchase_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.purchases (product_id, buyer_id, seller_id, amount, status) \
         VALUES ($1, $2, $3, 25, 'pending') RETURNING id",
    )
    .bind(product_id)
    .bind(buyer)
    .bind(seller)
    .fetch_one(&pool)
    .await
    .expect("insert private purchase");

    let outsider_intent = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {outsider_token}"))
                .header("Idempotency-Key", "private-order-outsider-intent")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "credit.purchase.action",
                        "request": { "id": purchase_id.to_string(), "action": "cancel" },
                    })
                    .to_string(),
                ))
                .expect("build outsider signing intent request"),
        )
        .await
        .expect("outsider signing intent response");
    assert_eq!(outsider_intent.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json(outsider_intent).await["error"]["code"], "NOT_FOUND");

    let outsider_action = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v2/credit/purchases/{purchase_id}/action"))
                .method("POST")
                .header("Authorization", format!("Bearer {outsider_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "action": "accept" }).to_string()))
                .expect("build outsider purchase action request"),
        )
        .await
        .expect("outsider purchase action response");
    assert_eq!(outsider_action.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json(outsider_action).await["error"]["code"], "NOT_FOUND");

    let buyer_wrong_state = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {buyer_token}"))
                .header("Idempotency-Key", "private-order-buyer-confirm")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "credit.purchase.action",
                        "request": { "id": purchase_id.to_string(), "action": "confirm" },
                    })
                    .to_string(),
                ))
                .expect("build wrong-state signing intent request"),
        )
        .await
        .expect("wrong-state signing intent response");
    assert_eq!(buyer_wrong_state.status(), StatusCode::CONFLICT);

    let seller_wrong_role = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Idempotency-Key", "private-order-seller-cancel")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "action": "credit.purchase.action",
                        "request": { "id": purchase_id.to_string(), "action": "cancel" },
                    })
                    .to_string(),
                ))
                .expect("build wrong-role signing intent request"),
        )
        .await
        .expect("wrong-role signing intent response");
    assert_eq!(seller_wrong_role.status(), StatusCode::BAD_REQUEST);

    let rejected_intents: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.signing_intents \
         WHERE account_id = ANY($1) AND action = 'credit.purchase.action'",
    )
    .bind(vec![buyer, seller, outsider])
    .fetch_one(&pool)
    .await
    .expect("count rejected signing intents");
    assert_eq!(rejected_intents, 0);
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
async fn pending_purchase_cancel_restores_last_unit_and_reopens_product() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(
        &pool,
        "pending-restock-seller@tongji.edu.cn",
        "pending-restock-seller",
    )
    .await;
    let buyer =
        create_test_account(&pool, "pending-restock-buyer@tongji.edu.cn", "pending-restock-buyer")
            .await;
    let buyer_token = create_token(&pool, "pending-restock-buyer@tongji.edu.cn").await;
    let (product_id, purchase_id) = create_pending_purchase(
        &pool,
        &app,
        seller,
        buyer,
        &buyer_token,
        "Pending cancellation restock",
    )
    .await;
    let reserved: (i32, String) =
        sqlx::query_as("SELECT stock, status::text FROM credit.products WHERE id = $1")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .expect("read reserved product state");
    assert_eq!(reserved, (0, "sold_out".into()));

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
    let response = app.clone().oneshot(cancel).await.expect("cancel pending purchase");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let restored: (i32, String) =
        sqlx::query_as("SELECT stock, status::text FROM credit.products WHERE id = $1")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .expect("read restored product state");
    assert_eq!(restored, (1, "on_sale".into()));
    let purchase: (String, Option<String>) =
        sqlx::query_as("SELECT status::text, hold_tx_id FROM credit.purchases WHERE id = $1")
            .bind(purchase_id)
            .fetch_one(&pool)
            .await
            .expect("read cancelled pending purchase");
    assert_eq!(purchase, ("cancelled".into(), None));
    assert_ledger_and_wallet_projection(&pool, &app, &[buyer, seller]).await;
}

#[tokio::test]
async fn accepted_purchase_cancel_restores_stock_without_reopening_off_sale_product() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(
        &pool,
        "accepted-restock-seller@tongji.edu.cn",
        "accepted-restock-seller",
    )
    .await;
    let buyer = create_test_account(
        &pool,
        "accepted-restock-buyer@tongji.edu.cn",
        "accepted-restock-buyer",
    )
    .await;
    let seller_token = create_token(&pool, "accepted-restock-seller@tongji.edu.cn").await;
    let buyer_token = create_token(&pool, "accepted-restock-buyer@tongji.edu.cn").await;
    let (product_id, purchase_id) = create_pending_purchase(
        &pool,
        &app,
        seller,
        buyer,
        &buyer_token,
        "Accepted cancellation restock",
    )
    .await;
    let uri = format!("/api/v2/credit/purchases/{purchase_id}/action");
    let accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
                .method("POST")
                .header("Authorization", format!("Bearer {seller_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"action":"accept"}"#))
                .expect("build accept request"),
        )
        .await
        .expect("accept purchase");
    assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
    sqlx::query("UPDATE credit.products SET status = 'off_sale' WHERE id = $1")
        .bind(product_id)
        .execute(&pool)
        .await
        .expect("take reserved product off sale");

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
    let response = app.clone().oneshot(cancel).await.expect("cancel accepted purchase");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let restored: (i32, String) =
        sqlx::query_as("SELECT stock, status::text FROM credit.products WHERE id = $1")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .expect("read off-sale restored product");
    assert_eq!(restored, (1, "off_sale".into()));
    assert_ledger_and_wallet_projection(&pool, &app, &[buyer, seller]).await;
}

#[tokio::test]
async fn concurrent_purchase_cancels_restore_and_refund_exactly_once() {
    let (pool, app) = create_test_app().await;
    let seller = create_test_account(
        &pool,
        "concurrent-restock-seller@tongji.edu.cn",
        "concurrent-restock-seller",
    )
    .await;
    let buyer = create_test_account(
        &pool,
        "concurrent-restock-buyer@tongji.edu.cn",
        "concurrent-restock-buyer",
    )
    .await;
    let buyer_token = create_token(&pool, "concurrent-restock-buyer@tongji.edu.cn").await;
    let (product_id, purchase_id) = create_pending_purchase(
        &pool,
        &app,
        seller,
        buyer,
        &buyer_token,
        "Concurrent cancellation restock",
    )
    .await;
    let uri = format!("/api/v2/credit/purchases/{purchase_id}/action");
    let first = signed_post_request(
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
    let second = signed_post_request(
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

    let (first, second) = tokio::join!(app.clone().oneshot(first), app.clone().oneshot(second));
    let statuses = [
        first.expect("first concurrent cancellation").status(),
        second.expect("second concurrent cancellation").status(),
    ];
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::NO_CONTENT).count(), 1);
    assert_eq!(statuses.iter().filter(|status| **status == StatusCode::BAD_REQUEST).count(), 1);

    let product: (i32, String) =
        sqlx::query_as("SELECT stock, status::text FROM credit.products WHERE id = $1")
            .bind(product_id)
            .fetch_one(&pool)
            .await
            .expect("read product after concurrent cancellations");
    assert_eq!(product, (1, "on_sale".into()));
    let release_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE type = 'escrow_release' AND metadata->>'purchase_id' = $1",
    )
    .bind(purchase_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count concurrent cancellation refunds");
    assert_eq!(release_count, 1);
    assert_ledger_and_wallet_projection(&pool, &app, &[buyer, seller]).await;
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

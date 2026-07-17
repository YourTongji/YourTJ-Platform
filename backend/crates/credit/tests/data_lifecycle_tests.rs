//! Integration coverage for Credit-owned account export and private-data purge.

mod helpers;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine as _;
use credit::account_eligibility::{AccountEligibilityFuture, AccountEligibilityResolver};
use helpers::{
    create_test_account, create_test_app, create_test_app_with_account_eligibility, create_token,
    mint_to_account, read_json,
};
use serde_json::Value;
use sqlx::PgConnection;
use tokio::sync::Notify;
use tower::ServiceExt;

struct PausingAccountEligibilityResolver {
    should_pause: AtomicBool,
    entered: Notify,
    resume: Notify,
}

impl PausingAccountEligibilityResolver {
    fn new() -> Self {
        Self { should_pause: AtomicBool::new(true), entered: Notify::new(), resume: Notify::new() }
    }

    async fn wait_until_entered(&self) {
        tokio::time::timeout(Duration::from_secs(5), self.entered.notified())
            .await
            .expect("credit writer must acquire its account lifecycle barrier");
    }

    fn resume(&self) {
        self.resume.notify_one();
    }
}

impl AccountEligibilityResolver for PausingAccountEligibilityResolver {
    fn is_eligible_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_id: i64,
    ) -> AccountEligibilityFuture<'a> {
        Box::pin(identity::public_accounts::is_credit_recipient_eligible(conn, account_id))
    }

    fn are_eligible_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_ids: &'a [i64],
    ) -> AccountEligibilityFuture<'a> {
        Box::pin(async move {
            let is_eligible =
                identity::public_accounts::lock_active_interaction_accounts(conn, account_ids)
                    .await?;
            if self.should_pause.swap(false, Ordering::SeqCst) {
                self.entered.notify_one();
                self.resume.notified().await;
            }
            Ok(is_eligible)
        })
    }
}

fn item_with_string_id(items: &Value, item_id: i64) -> &Value {
    let item_id = item_id.to_string();
    items
        .as_array()
        .expect("response items")
        .iter()
        .find(|item| item["id"].as_str() == Some(item_id.as_str()))
        .expect("response item by id")
}

fn item_with_numeric_id(items: &Value, item_id: i64) -> &Value {
    items
        .as_array()
        .expect("export items")
        .iter()
        .find(|item| item["id"].as_i64() == Some(item_id))
        .expect("export item by id")
}

#[tokio::test]
async fn lifecycle_transition_waits_for_signing_intent_then_purge_removes_it() {
    let barrier = Arc::new(PausingAccountEligibilityResolver::new());
    let (pool, app) = create_test_app_with_account_eligibility(barrier.clone()).await;
    let account_id =
        create_test_account(&pool, "intent-purge-race@tongji.edu.cn", "intent-purge-race").await;
    let token = create_token(&pool, "intent-purge-race@tongji.edu.cn").await;
    let public_key = base64::engine::general_purpose::STANDARD.encode([37_u8; 32]);
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(account_id)
        .bind(public_key)
        .execute(&pool)
        .await
        .expect("bind lifecycle intent key");

    let request_app = app.clone();
    let request_token = token.clone();
    let idempotency_key = format!("lifecycle-intent-{}", uuid::Uuid::new_v4());
    let request_handle = tokio::spawn(async move {
        request_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v2/credit/signing-intents")
                    .header("Authorization", format!("Bearer {request_token}"))
                    .header("Content-Type", "application/json")
                    .header("Idempotency-Key", idempotency_key)
                    .body(Body::from(
                        serde_json::json!({
                            "action": "credit.task.create",
                            "request": {
                                "title": "Lifecycle barrier task",
                                "rewardAmount": 1,
                            }
                        })
                        .to_string(),
                    ))
                    .expect("build lifecycle intent request"),
            )
            .await
            .expect("lifecycle intent response")
    });
    barrier.wait_until_entered().await;

    let lifecycle_pool = pool.clone();
    let mut lifecycle_handle = tokio::spawn(async move {
        sqlx::query(
            "UPDATE identity.accounts SET status = 'deletion_requested', \
                    deletion_requested_at = now(), \
                    deletion_recover_until = now() + interval '30 days' \
             WHERE id = $1",
        )
        .bind(account_id)
        .execute(&lifecycle_pool)
        .await
        .expect("mark lifecycle intent account deleted");
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut lifecycle_handle).await.is_err(),
        "lifecycle transition must wait for the signing-intent transaction"
    );

    barrier.resume();
    let response = tokio::time::timeout(Duration::from_secs(5), request_handle)
        .await
        .expect("signing intent request must finish")
        .expect("join signing intent request");
    assert_eq!(response.status(), StatusCode::OK);
    tokio::time::timeout(Duration::from_secs(5), lifecycle_handle)
        .await
        .expect("lifecycle transition must finish after intent commit")
        .expect("join lifecycle intent transition");

    credit::data_export::purge_account_private_data(&pool, account_id)
        .await
        .expect("purge lifecycle intent");
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.signing_intents WHERE account_id = $1 AND consumed_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count lifecycle intents");
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn lifecycle_transition_waits_for_private_product_then_purge_clears_it() {
    let barrier = Arc::new(PausingAccountEligibilityResolver::new());
    let (pool, app) = create_test_app_with_account_eligibility(barrier.clone()).await;
    let account_id =
        create_test_account(&pool, "product-purge-race@tongji.edu.cn", "product-purge-race").await;
    let token = create_token(&pool, "product-purge-race@tongji.edu.cn").await;

    let request_handle = tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/credit/products")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Lifecycle barrier product",
                        "description": "private writer race",
                        "price": 1,
                        "stock": 1,
                        "deliveryInfo": "owner-only delivery secret",
                    })
                    .to_string(),
                ))
                .expect("build lifecycle product request"),
        )
        .await
        .expect("lifecycle product response")
    });
    barrier.wait_until_entered().await;

    let lifecycle_pool = pool.clone();
    let mut lifecycle_handle = tokio::spawn(async move {
        sqlx::query(
            "UPDATE identity.accounts SET status = 'deletion_requested', \
                    deletion_requested_at = now(), \
                    deletion_recover_until = now() + interval '30 days' \
             WHERE id = $1",
        )
        .bind(account_id)
        .execute(&lifecycle_pool)
        .await
        .expect("mark lifecycle product account deleted");
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(150), &mut lifecycle_handle).await.is_err(),
        "lifecycle transition must wait for the product transaction"
    );

    barrier.resume();
    let response = tokio::time::timeout(Duration::from_secs(5), request_handle)
        .await
        .expect("product request must finish")
        .expect("join product request");
    assert_eq!(response.status(), StatusCode::CREATED);
    tokio::time::timeout(Duration::from_secs(5), lifecycle_handle)
        .await
        .expect("lifecycle transition must finish after product commit")
        .expect("join lifecycle product transition");

    credit::data_export::purge_account_private_data(&pool, account_id)
        .await
        .expect("purge lifecycle product");
    let retained: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, \
                COUNT(*) FILTER (WHERE delivery_info IS NOT NULL)::bigint \
         FROM credit.products WHERE seller_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("inspect lifecycle product");
    assert_eq!(retained, (1, 0));
}

#[tokio::test]
async fn owner_export_and_purge_cover_escrow_private_fields() {
    let (pool, app) = create_test_app().await;
    let owner_id =
        create_test_account(&pool, "lifecycle-owner@tongji.edu.cn", "lifecycle-owner").await;
    let counterparty_id = create_test_account(
        &pool,
        "lifecycle-counterparty@tongji.edu.cn",
        "lifecycle-counterparty",
    )
    .await;
    let counterparty_token = create_token(&pool, "lifecycle-counterparty@tongji.edu.cn").await;
    let owner_token = create_token(&pool, "lifecycle-owner@tongji.edu.cn").await;
    mint_to_account(&pool, owner_id, 100).await;

    let owner_task_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.tasks \
         (creator_id, acceptor_id, title, description, reward_amount, contact_info, status) \
         VALUES ($1, $2, 'Owner task', 'Owner task details', 20, \
                 'owner task contact secret', 'in_progress') RETURNING id",
    )
    .bind(owner_id)
    .bind(counterparty_id)
    .fetch_one(&pool)
    .await
    .expect("insert owner task");
    let counterparty_task_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.tasks \
         (creator_id, acceptor_id, title, description, reward_amount, contact_info, status) \
         VALUES ($1, $2, 'Counterparty task', 'Counterparty task details', 15, \
                 'counterparty task contact secret', 'in_progress') RETURNING id",
    )
    .bind(counterparty_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("insert counterparty task");

    let owner_product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products \
         (seller_id, title, description, price, stock, delivery_info) \
         VALUES ($1, 'Owner product', 'Owner product details', 30, 1, \
                 'owner delivery secret') RETURNING id",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("insert owner product");
    let counterparty_product_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.products \
         (seller_id, title, description, price, stock, delivery_info) \
         VALUES ($1, 'Counterparty product', 'Counterparty product details', 40, 1, \
                 'counterparty delivery secret') RETURNING id",
    )
    .bind(counterparty_id)
    .fetch_one(&pool)
    .await
    .expect("insert counterparty product");

    let owner_sale_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.purchases (product_id, buyer_id, seller_id, amount) \
         VALUES ($1, $2, $3, 30) RETURNING id",
    )
    .bind(owner_product_id)
    .bind(counterparty_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("insert owner sale");
    let owner_purchase_id: i64 = sqlx::query_scalar(
        "INSERT INTO credit.purchases (product_id, buyer_id, seller_id, amount) \
         VALUES ($1, $2, $3, 40) RETURNING id",
    )
    .bind(counterparty_product_id)
    .bind(owner_id)
    .bind(counterparty_id)
    .fetch_one(&pool)
    .await
    .expect("insert owner purchase");
    let pending_intent_id = uuid::Uuid::new_v4();
    let consumed_intent_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO credit.signing_intents \
         (id, account_id, public_key, action, request_hash, snapshot, idempotency_key, \
          signing_bytes, expires_at, consumed_at) \
         VALUES \
           ($1, $3, 'fixture-key', 'credit.tip', repeat('0', 64), '{}'::jsonb, $4, \
            'pending-proof', now() + interval '5 minutes', NULL), \
           ($2, $3, 'fixture-key', 'credit.tip', repeat('1', 64), '{}'::jsonb, $5, \
            'consumed-proof', now() - interval '5 minutes', now())",
    )
    .bind(pending_intent_id)
    .bind(consumed_intent_id)
    .bind(owner_id)
    .bind(format!("pending-{pending_intent_id}"))
    .bind(format!("consumed-{consumed_intent_id}"))
    .execute(&pool)
    .await
    .expect("insert lifecycle signing intents");

    let export =
        credit::data_export::snapshot(&pool, owner_id).await.expect("snapshot owner credit data");
    let export = serde_json::to_value(export).expect("serialize owner credit data");
    assert_eq!(export["balance"], 100);
    assert_eq!(export["ledger"].as_array().expect("owner ledger").len(), 1);
    assert_eq!(export["createdTasks"].as_array().expect("created tasks").len(), 1);
    assert_eq!(
        item_with_numeric_id(&export["createdTasks"], owner_task_id)["contactInfo"],
        "owner task contact secret"
    );
    assert_eq!(export["acceptedTasks"].as_array().expect("accepted tasks").len(), 1);
    assert_eq!(
        item_with_numeric_id(&export["acceptedTasks"], counterparty_task_id)["contactInfo"],
        "counterparty task contact secret"
    );
    assert_eq!(export["createdProducts"].as_array().expect("created products").len(), 1);
    assert_eq!(
        item_with_numeric_id(&export["createdProducts"], owner_product_id)["deliveryInfo"],
        "owner delivery secret"
    );
    assert_eq!(export["purchases"].as_array().expect("party purchases").len(), 2);
    assert_eq!(
        item_with_numeric_id(&export["purchases"], owner_sale_id)["deliveryInfo"],
        "owner delivery secret"
    );
    assert_eq!(
        item_with_numeric_id(&export["purchases"], owner_purchase_id)["deliveryInfo"],
        "counterparty delivery secret"
    );

    credit::data_export::purge_account_private_data(&pool, owner_id)
        .await
        .expect("purge owner credit private data");
    credit::data_export::purge_account_private_data(&pool, owner_id)
        .await
        .expect("retry owner credit private-data purge");

    let retained: (Option<String>, Option<String>, Option<String>, Option<String>, i64, i64) =
        sqlx::query_as(
            "SELECT \
               (SELECT contact_info FROM credit.tasks WHERE id = $1), \
               (SELECT contact_info FROM credit.tasks WHERE id = $2), \
               (SELECT delivery_info FROM credit.products WHERE id = $3), \
               (SELECT delivery_info FROM credit.products WHERE id = $4), \
               (SELECT COUNT(*) FROM credit.purchases \
                WHERE buyer_id = $5 OR seller_id = $5), \
               (SELECT COUNT(*) FROM credit.ledger \
                WHERE from_account = $5 OR to_account = $5)",
        )
        .bind(owner_task_id)
        .bind(counterparty_task_id)
        .bind(owner_product_id)
        .bind(counterparty_product_id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("read retained credit facts");
    assert_eq!(retained.0, None);
    assert_eq!(retained.1.as_deref(), Some("counterparty task contact secret"));
    assert_eq!(retained.2, None);
    assert_eq!(retained.3.as_deref(), Some("counterparty delivery secret"));
    assert_eq!(retained.4, 2);
    assert_eq!(retained.5, 1);
    let intent_counts: (i64, i64) = sqlx::query_as(
        "SELECT \
           COUNT(*) FILTER (WHERE consumed_at IS NULL)::bigint, \
           COUNT(*) FILTER (WHERE consumed_at IS NOT NULL)::bigint \
         FROM credit.signing_intents WHERE account_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("read retained signing intent counts");
    assert_eq!(intent_counts, (0, 1));

    let retained_outcome = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/credit/signing-intent-outcome")
                .header("Authorization", format!("Bearer {owner_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "intentId": consumed_intent_id.to_string() }).to_string(),
                ))
                .expect("build retained intent outcome request"),
        )
        .await
        .expect("retained intent outcome response");
    assert_eq!(retained_outcome.status(), StatusCode::OK);
    assert_eq!(read_json(retained_outcome).await["status"], "committed");
    let removed_outcome = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/credit/signing-intent-outcome")
                .header("Authorization", format!("Bearer {owner_token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "intentId": pending_intent_id.to_string() }).to_string(),
                ))
                .expect("build removed intent outcome request"),
        )
        .await
        .expect("removed intent outcome response");
    assert_eq!(removed_outcome.status(), StatusCode::NOT_FOUND);

    let tasks_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/tasks")
                .header("Authorization", format!("Bearer {counterparty_token}"))
                .body(Body::empty())
                .expect("build counterparty task request"),
        )
        .await
        .expect("list tasks as counterparty");
    assert_eq!(tasks_response.status(), StatusCode::OK);
    let tasks = read_json(tasks_response).await;
    assert!(item_with_string_id(&tasks["items"], owner_task_id)["contactInfo"].is_null());
    assert_eq!(
        item_with_string_id(&tasks["items"], counterparty_task_id)["contactInfo"],
        "counterparty task contact secret"
    );

    let purchases_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/purchases")
                .header("Authorization", format!("Bearer {counterparty_token}"))
                .body(Body::empty())
                .expect("build counterparty purchase request"),
        )
        .await
        .expect("list purchases as counterparty");
    assert_eq!(purchases_response.status(), StatusCode::OK);
    let purchases = read_json(purchases_response).await;
    assert!(item_with_string_id(&purchases["items"], owner_sale_id)["deliveryInfo"].is_null());
    assert_eq!(
        item_with_string_id(&purchases["items"], owner_purchase_id)["deliveryInfo"],
        "counterparty delivery secret"
    );

    let purged_export = credit::data_export::snapshot(&pool, owner_id)
        .await
        .expect("snapshot purged owner credit data");
    let purged_export =
        serde_json::to_value(purged_export).expect("serialize purged owner credit data");
    assert!(item_with_numeric_id(&purged_export["createdTasks"], owner_task_id)["contactInfo"]
        .is_null());
    assert_eq!(
        item_with_numeric_id(&purged_export["acceptedTasks"], counterparty_task_id)["contactInfo"],
        "counterparty task contact secret"
    );
    assert!(item_with_numeric_id(&purged_export["createdProducts"], owner_product_id)
        ["deliveryInfo"]
        .is_null());
    assert_eq!(purged_export["purchases"].as_array().expect("retained purchases").len(), 2);
    assert_eq!(purged_export["ledger"].as_array().expect("retained ledger").len(), 1);
}

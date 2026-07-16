//! Owner and row-lock semantics for signing-intent outcomes.

mod helpers;

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use base64::Engine as _;
use credit::dto::SigningIntentInput;
use helpers::{
    create_test_account, create_test_app, create_token, mint_to_account, read_json,
    AccountWalletKeyResolver, IdentityAccountEligibilityResolver,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt as _;

async fn get_outcome(app: &axum::Router, token: Option<&str>, intent_id: &str) -> Response<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v2/credit/signing-intent-outcome")
        .header("Content-Type", "application/json");
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(
            builder
                .body(Body::from(json!({ "intentId": intent_id }).to_string()))
                .expect("build outcome request"),
        )
        .await
        .expect("intent outcome response")
}

async fn create_signing_intent(app: &axum::Router, token: Option<&str>) -> Response<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v2/credit/signing-intents")
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", format!("auth-boundary-{}", uuid::Uuid::new_v4()));
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(
            builder
                .body(Body::from(
                    json!({
                        "action": "credit.task.create",
                        "request": { "title": "Authentication boundary", "rewardAmount": 1 },
                    })
                    .to_string(),
                ))
                .expect("build signing intent request"),
        )
        .await
        .expect("signing intent response")
}

async fn insert_intent(
    pool: &PgPool,
    account_id: i64,
    expires_at: chrono::DateTime<chrono::Utc>,
    is_consumed: bool,
) -> uuid::Uuid {
    let intent_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO credit.signing_intents \
         (id, account_id, public_key, action, request_hash, snapshot, idempotency_key, \
          signing_bytes, expires_at, consumed_at) \
         VALUES ($1, $2, 'fixture-key', 'credit.tip', repeat('0', 64), '{}'::jsonb, $3, \
                 'fixture-signing-bytes', $4, CASE WHEN $5 THEN now() ELSE NULL END)",
    )
    .bind(intent_id)
    .bind(account_id)
    .bind(format!("fixture-{intent_id}"))
    .bind(expires_at)
    .bind(is_consumed)
    .execute(pool)
    .await
    .expect("insert signing intent fixture");
    intent_id
}

async fn wait_for_outcome_lock(pool: &PgPool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let is_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE datname = current_database() AND pid <> pg_backend_pid() \
                 AND wait_event_type = 'Lock' \
                 AND query LIKE '%FROM credit.signing_intents%FOR SHARE%' \
             )",
        )
        .fetch_one(pool)
        .await
        .expect("inspect outcome lock wait");
        if is_waiting {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_for_consumer_lock(pool: &PgPool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let is_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE datname = current_database() AND pid <> pg_backend_pid() \
                 AND wait_event_type = 'Lock' \
                 AND query LIKE '%FROM credit.signing_intents WHERE id = $1 FOR UPDATE%' \
             )",
        )
        .fetch_one(pool)
        .await
        .expect("inspect consumer lock wait");
        if is_waiting {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn intent_outcome_is_owner_only_and_uses_typed_terminal_precedence() {
    let (pool, app) = create_test_app().await;
    let owner = create_test_account(&pool, "intent-owner@tongji.edu.cn", "intent-owner").await;
    let _other = create_test_account(&pool, "intent-other@tongji.edu.cn", "intent-other").await;
    let owner_token = create_token(&pool, "intent-owner@tongji.edu.cn").await;
    let other_token = create_token(&pool, "intent-other@tongji.edu.cn").await;
    let now = chrono::Utc::now();
    let pending = insert_intent(&pool, owner, now + chrono::Duration::minutes(5), false).await;
    let expired = insert_intent(&pool, owner, now - chrono::Duration::minutes(1), false).await;
    let committed = insert_intent(&pool, owner, now - chrono::Duration::minutes(1), true).await;

    for (intent_id, expected_status) in
        [(pending, "pending"), (expired, "expired"), (committed, "committed")]
    {
        let response = get_outcome(&app, Some(&owner_token), &intent_id.to_string()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = read_json(response).await;
        assert_eq!(body["intentId"], intent_id.to_string());
        assert_eq!(body["status"], expected_status);
        assert!(body["expiresAt"].as_i64().is_some());
    }

    let cross_account = get_outcome(&app, Some(&other_token), &pending.to_string()).await;
    let nonexistent =
        get_outcome(&app, Some(&owner_token), &uuid::Uuid::new_v4().to_string()).await;
    let invalid = get_outcome(&app, Some(&owner_token), "not-a-uuid").await;
    assert_eq!(cross_account.status(), StatusCode::NOT_FOUND);
    assert_eq!(nonexistent.status(), StatusCode::NOT_FOUND);
    assert_eq!(invalid.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json(cross_account).await["error"]["code"], "NOT_FOUND");
    assert_eq!(read_json(nonexistent).await["error"]["code"], "NOT_FOUND");

    let unauthenticated = get_outcome(&app, None, &pending.to_string()).await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn signing_intent_endpoints_distinguish_future_and_active_suspensions() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "intent-auth@tongji.edu.cn", "intent-auth").await;
    let token = create_token(&pool, "intent-auth@tongji.edu.cn").await;
    let unknown_intent_id = uuid::Uuid::new_v4().to_string();
    let sanction_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sanctions \
         (account_id, kind, reason, starts_at, ends_at) \
         VALUES ($1, 'suspend', 'signing endpoint auth boundary', \
                 clock_timestamp() + interval '1 hour', \
                 clock_timestamp() + interval '2 hours') RETURNING id",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("insert future signing endpoint suspension");

    let unauthenticated_create = create_signing_intent(&app, None).await;
    let unauthenticated_outcome = get_outcome(&app, None, &unknown_intent_id).await;
    assert_eq!(unauthenticated_create.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(unauthenticated_outcome.status(), StatusCode::UNAUTHORIZED);

    let future_create = create_signing_intent(&app, Some(&token)).await;
    let future_outcome = get_outcome(&app, Some(&token), &unknown_intent_id).await;
    assert_eq!(future_create.status(), StatusCode::BAD_REQUEST);
    assert_eq!(future_outcome.status(), StatusCode::NOT_FOUND);

    sqlx::query("UPDATE identity.sanctions SET starts_at = clock_timestamp() WHERE id = $1")
        .bind(sanction_id)
        .execute(&pool)
        .await
        .expect("activate signing endpoint suspension");
    let suspended_create = create_signing_intent(&app, Some(&token)).await;
    let suspended_outcome = get_outcome(&app, Some(&token), &unknown_intent_id).await;
    assert_eq!(suspended_create.status(), StatusCode::FORBIDDEN);
    assert_eq!(suspended_outcome.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn intent_outcome_waits_for_consume_commit_and_rollback() {
    let (pool, app) = create_test_app().await;
    let owner =
        create_test_account(&pool, "intent-lock-owner@tongji.edu.cn", "intent-lock-owner").await;
    let recipient =
        create_test_account(&pool, "intent-lock-recipient@tongji.edu.cn", "intent-lock-recipient")
            .await;
    mint_to_account(&pool, owner, 10).await;
    let owner_token = create_token(&pool, "intent-lock-owner@tongji.edu.cn").await;
    let seed = [94u8; 32];
    let public_key =
        base64::engine::general_purpose::STANDARD.encode(credit::ledger::derive_public_key(&seed));
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(owner)
        .bind(public_key)
        .execute(&pool)
        .await
        .expect("bind outcome-test wallet key");
    let request: Value = json!({
        "toAccountId": recipient.to_string(),
        "amount": 1,
        "targetType": "thread",
        "targetId": "1",
    });
    let input = SigningIntentInput { action: "credit.tip".into(), request: request.clone() };
    let idempotency_key = uuid::Uuid::new_v4().to_string();
    let intent = credit::signing::create_intent(
        &pool,
        &IdentityAccountEligibilityResolver,
        &AccountWalletKeyResolver,
        owner,
        &input,
        &idempotency_key,
    )
    .await
    .expect("create consumable intent");
    let signature = credit::ledger::sign_with_seed(&intent.signing_bytes, &seed);
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-wallet-intent",
        HeaderValue::from_str(&intent.intent_id).expect("intent header"),
    );
    headers.insert("x-wallet-sig", HeaderValue::from_str(&signature).expect("signature header"));
    headers.insert(
        "idempotency-key",
        HeaderValue::from_str(&idempotency_key).expect("idempotency header"),
    );

    let mut consume_tx = pool.begin().await.expect("begin consume transaction");
    credit::signing::consume_intent(
        &mut consume_tx,
        &AccountWalletKeyResolver,
        &headers,
        owner,
        "credit.tip",
        &request,
    )
    .await
    .expect("consume intent without committing");
    let app_for_commit = app.clone();
    let token_for_commit = owner_token.clone();
    let intent_id_for_commit = intent.intent_id.clone();
    let mut committed_outcome = tokio::spawn(async move {
        get_outcome(&app_for_commit, Some(&token_for_commit), &intent_id_for_commit).await
    });
    assert!(wait_for_outcome_lock(&pool).await, "outcome read did not wait on consume lock");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut committed_outcome).await.is_err(),
        "outcome returned before the consume transaction committed"
    );
    consume_tx.commit().await.expect("commit consumed intent");
    let committed_response = tokio::time::timeout(Duration::from_secs(3), committed_outcome)
        .await
        .expect("committed outcome timeout")
        .expect("join committed outcome");
    assert_eq!(committed_response.status(), StatusCode::OK);
    assert_eq!(read_json(committed_response).await["status"], "committed");

    let expired_id =
        insert_intent(&pool, owner, chrono::Utc::now() - chrono::Duration::minutes(1), false).await;
    let mut rollback_tx = pool.begin().await.expect("begin rollback transaction");
    sqlx::query("UPDATE credit.signing_intents SET consumed_at = now() WHERE id = $1")
        .bind(expired_id)
        .execute(&mut *rollback_tx)
        .await
        .expect("stage consumed marker for rollback");
    let app_for_rollback = app.clone();
    let token_for_rollback = owner_token.clone();
    let mut rolled_back_outcome = tokio::spawn(async move {
        get_outcome(&app_for_rollback, Some(&token_for_rollback), &expired_id.to_string()).await
    });
    assert!(wait_for_outcome_lock(&pool).await, "rollback outcome did not wait on row lock");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut rolled_back_outcome).await.is_err(),
        "outcome returned before the consuming transaction rolled back"
    );
    rollback_tx.rollback().await.expect("rollback consumed marker");
    let rolled_back_response = tokio::time::timeout(Duration::from_secs(3), rolled_back_outcome)
        .await
        .expect("rolled-back outcome timeout")
        .expect("join rolled-back outcome");
    assert_eq!(rolled_back_response.status(), StatusCode::OK);
    assert_eq!(read_json(rolled_back_response).await["status"], "expired");
}

#[tokio::test]
async fn consumer_rechecks_database_expiry_after_waiting_for_the_intent_lock() {
    let (pool, _app) = create_test_app().await;
    let owner =
        create_test_account(&pool, "intent-expiry-owner@tongji.edu.cn", "intent-expiry-owner")
            .await;
    let recipient = create_test_account(
        &pool,
        "intent-expiry-recipient@tongji.edu.cn",
        "intent-expiry-recipient",
    )
    .await;
    mint_to_account(&pool, owner, 10).await;
    let seed = [95u8; 32];
    let public_key =
        base64::engine::general_purpose::STANDARD.encode(credit::ledger::derive_public_key(&seed));
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(owner)
        .bind(public_key)
        .execute(&pool)
        .await
        .expect("bind expiry-test wallet key");
    let request: Value = json!({
        "toAccountId": recipient.to_string(),
        "amount": 1,
        "targetType": "thread",
        "targetId": "1",
    });
    let input = SigningIntentInput { action: "credit.tip".into(), request: request.clone() };
    let idempotency_key = uuid::Uuid::new_v4().to_string();
    let intent = credit::signing::create_intent(
        &pool,
        &IdentityAccountEligibilityResolver,
        &AccountWalletKeyResolver,
        owner,
        &input,
        &idempotency_key,
    )
    .await
    .expect("create expiry-test intent");
    let expires_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT date_trunc('second', clock_timestamp()) + interval '2 seconds'")
            .fetch_one(&pool)
            .await
            .expect("choose database expiry");
    let mut signing_envelope: Value =
        serde_json::from_str(&intent.signing_bytes).expect("parse signing envelope");
    signing_envelope["expiresAt"] = json!(expires_at.timestamp());
    let signing_bytes = credit::ledger::canonicalize(&signing_envelope);
    let signature = credit::ledger::sign_with_seed(&signing_bytes, &seed);
    let intent_id = intent.intent_id.parse::<uuid::Uuid>().expect("intent UUID");
    sqlx::query(
        "UPDATE credit.signing_intents SET signing_bytes = $2, expires_at = $3 WHERE id = $1",
    )
    .bind(intent_id)
    .bind(&signing_bytes)
    .bind(expires_at)
    .execute(&pool)
    .await
    .expect("set near-term database expiry");

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-wallet-intent",
        HeaderValue::from_str(&intent.intent_id).expect("intent header"),
    );
    headers.insert("x-wallet-sig", HeaderValue::from_str(&signature).expect("signature header"));
    headers.insert(
        "idempotency-key",
        HeaderValue::from_str(&idempotency_key).expect("idempotency header"),
    );

    let mut locker = pool.begin().await.expect("begin intent locker");
    sqlx::query("SELECT id FROM credit.signing_intents WHERE id = $1 FOR UPDATE")
        .bind(intent_id)
        .fetch_one(&mut *locker)
        .await
        .expect("lock intent before expiry");
    let pool_for_consumer = pool.clone();
    let mut consumer = tokio::spawn(async move {
        let mut tx = pool_for_consumer.begin().await.expect("begin waiting consumer");
        let result = credit::signing::consume_intent(
            &mut tx,
            &AccountWalletKeyResolver,
            &headers,
            owner,
            "credit.tip",
            &request,
        )
        .await
        .map(|_| ());
        tx.rollback().await.expect("rollback waiting consumer");
        result
    });
    assert!(wait_for_consumer_lock(&pool).await, "consumer did not wait on intent row lock");
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut consumer).await.is_err(),
        "consumer returned before the row lock was released"
    );
    let expiry_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let is_expired: bool = sqlx::query_scalar("SELECT clock_timestamp() >= $1")
            .bind(expires_at)
            .fetch_one(&pool)
            .await
            .expect("check database expiry");
        if is_expired {
            break;
        }
        assert!(Instant::now() < expiry_deadline, "database expiry was not reached");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    locker.rollback().await.expect("release intent lock after expiry");
    let consume_result = tokio::time::timeout(Duration::from_secs(3), consumer)
        .await
        .expect("waiting consumer timeout")
        .expect("join waiting consumer");
    assert!(matches!(consume_result, Err(shared::AppError::Forbidden)));
    let consumed_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT consumed_at FROM credit.signing_intents WHERE id = $1")
            .bind(intent_id)
            .fetch_one(&pool)
            .await
            .expect("read rejected intent marker");
    assert!(consumed_at.is_none());
}

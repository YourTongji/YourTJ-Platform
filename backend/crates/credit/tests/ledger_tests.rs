//! Integration tests for the credit ledger: append, hash chain, and verify.

mod helpers;

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine as _;
use helpers::{
    create_test_account, create_test_app, create_tip_thread, create_token, mint_to_account,
    read_json, signed_post_request,
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const LEDGER_APPEND_LOCK_KEY: i64 = 42;

async fn wait_for_advisory_waiter(pool: &PgPool, lock_holder_pid: i32) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let has_waiter: bool = sqlx::query_scalar(
                "SELECT EXISTS( \
                   SELECT 1 FROM pg_locks waiting \
                   JOIN pg_locks held \
                     ON held.locktype = waiting.locktype \
                    AND held.database IS NOT DISTINCT FROM waiting.database \
                    AND held.classid IS NOT DISTINCT FROM waiting.classid \
                    AND held.objid IS NOT DISTINCT FROM waiting.objid \
                    AND held.objsubid IS NOT DISTINCT FROM waiting.objsubid \
                   WHERE held.pid = $1 \
                     AND held.locktype = 'advisory' \
                     AND held.granted \
                     AND NOT waiting.granted \
                     AND waiting.pid <> held.pid \
                 )",
            )
            .bind(lock_holder_pid)
            .fetch_one(pool)
            .await
            .expect("inspect advisory lock waiters");
            if has_waiter {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("debit transaction did not queue for the ledger lock");
}

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
async fn concurrent_release_invalidates_a_stale_debit_balance_snapshot_without_deadlock() {
    let (pool, app) = create_test_app().await;
    let account_id = create_test_account(&pool, "lock-order@tongji.edu.cn", "lock-order").await;
    mint_to_account(&pool, account_id, 100).await;
    let token = create_token(&pool, "lock-order@tongji.edu.cn").await;
    let debit_body = json!({
        "title": "Concurrent debit",
        "rewardAmount": 40
    });
    let debit_request = signed_post_request(
        &app,
        &pool,
        &token,
        account_id,
        "/api/v2/credit/tasks",
        "credit.task.create",
        debit_body.clone(),
        Some(debit_body),
    )
    .await;

    let mut release_transaction = pool.begin().await.expect("begin release transaction");
    let release_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *release_transaction)
        .await
        .expect("read release connection pid");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(LEDGER_APPEND_LOCK_KEY)
        .execute(&mut *release_transaction)
        .await
        .expect("lock ledger for release");

    let debit_app = app.clone();
    let debit_handle = tokio::spawn(async move {
        debit_app.oneshot(debit_request).await.expect("run concurrent debit request")
    });
    wait_for_advisory_waiter(&pool, release_pid).await;

    let release_tx_id = uuid::Uuid::new_v4().to_string();
    let release_nonce = uuid::Uuid::new_v4().to_string();
    let release_metadata = json!({ "task_id": "lock-order-regression" });
    let release_created_at = chrono::Utc::now().timestamp();
    let release_canonical = credit::ledger::build_ledger_canonical(
        &release_tx_id,
        "escrow_release",
        None,
        Some(account_id),
        50,
        &release_nonce,
        Some(&release_metadata),
        "system",
        release_created_at,
    );
    let release_signature = credit::ledger::sign_with_seed(&release_canonical, &[0u8; 32]);
    let release_row = credit::repo::append_ledger_entry_tx(
        &mut release_transaction,
        &release_tx_id,
        "escrow_release",
        None,
        Some(account_id),
        50,
        &release_nonce,
        Some(&release_metadata),
        "system",
        &release_signature,
        release_created_at,
    )
    .await
    .expect("append release while debit waits for the ledger lock");
    release_transaction.commit().await.expect("commit release transaction");

    let debit_response = tokio::time::timeout(Duration::from_secs(5), debit_handle)
        .await
        .expect("concurrent debit timed out")
        .expect("join concurrent debit");
    assert_eq!(debit_response.status(), StatusCode::FORBIDDEN);

    let wallet_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("read wallet balance after concurrent value moves");
    let projected_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM( \
           CASE WHEN to_account = $1 THEN amount ELSE 0 END - \
           CASE WHEN from_account = $1 THEN amount ELSE 0 END \
         ), 0)::bigint FROM credit.ledger",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("derive balance after concurrent value moves");
    let hold_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE type = 'escrow_hold' AND from_account = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count rejected concurrent debit entries");

    assert_eq!(wallet_balance, 150);
    assert_eq!(projected_balance, wallet_balance);
    assert_eq!(hold_count, 0);
    assert!(release_row.seq > 0);

    let verify_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger/verify")
                .body(Body::empty())
                .expect("build ledger verification request"),
        )
        .await
        .expect("verify concurrent ledger entries");
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verification = read_json(verify_response).await;
    assert!(
        verification["ok"].as_bool().unwrap_or(false),
        "ledger verification failed: {verification}"
    );
}

#[tokio::test]
async fn ledger_verification_uses_revoked_keys_for_historical_signatures() {
    let (pool, app) = create_test_app().await;
    let signer =
        create_test_account(&pool, "historical-signer@tongji.edu.cn", "historical-signer").await;
    let recipient =
        create_test_account(&pool, "historical-recipient@tongji.edu.cn", "historical-recipient")
            .await;
    mint_to_account(&pool, signer, 10).await;

    let seed = [77u8; 32];
    let public_key =
        base64::engine::general_purpose::STANDARD.encode(credit::ledger::derive_public_key(&seed));
    sqlx::query(
        "INSERT INTO identity.account_keys (account_id, public_key, revoked_at) \
         VALUES ($1, $2, now())",
    )
    .bind(signer)
    .bind(public_key)
    .execute(&pool)
    .await
    .expect("insert historical verification key");

    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().timestamp();
    let signer_id = signer.to_string();
    let canonical = credit::ledger::build_ledger_canonical(
        &tx_id,
        "tip",
        Some(signer),
        Some(recipient),
        3,
        &nonce,
        None,
        &signer_id,
        created_at,
    );
    let signature = credit::ledger::sign_with_seed(&canonical, &seed);
    let mut ledger_tx = pool.begin().await.expect("begin historical ledger append");
    credit::repo::append_ledger_entry_tx(
        &mut ledger_tx,
        &tx_id,
        "tip",
        Some(signer),
        Some(recipient),
        3,
        &nonce,
        None,
        &signer_id,
        &signature,
        created_at,
    )
    .await
    .expect("append historical user-signed ledger row");
    ledger_tx.commit().await.expect("commit historical ledger append");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger/verify")
                .body(Body::empty())
                .expect("build historical verification request"),
        )
        .await
        .expect("historical verification response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert!(body["ok"].as_bool().unwrap_or(false), "historical ledger verification failed: {body}");
}

#[tokio::test]
async fn ledger_verification_rejects_an_intent_rekeyed_outside_identity() {
    let (pool, app) = create_test_app().await;
    let sender =
        create_test_account(&pool, "intent-tamper-sender@tongji.edu.cn", "intent-tamper-sender")
            .await;
    let recipient = create_test_account(
        &pool,
        "intent-tamper-recipient@tongji.edu.cn",
        "intent-tamper-recipient",
    )
    .await;
    mint_to_account(&pool, sender, 20).await;
    let target_id = create_tip_thread(&pool, recipient).await;
    let token = create_token(&pool, "intent-tamper-sender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 5,
        "targetType": "thread",
        "targetId": target_id.to_string(),
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
    let response = app.clone().oneshot(request).await.expect("create signed tip");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let (intent_id, original_hash): (uuid::Uuid, String) = sqlx::query_as(
        "SELECT (metadata->>'signing_intent_id')::uuid, hash FROM credit.ledger \
         WHERE type = 'tip' AND from_account = $1",
    )
    .bind(sender)
    .fetch_one(&pool)
    .await
    .expect("read intent-backed ledger row");
    let original_signing_bytes: String =
        sqlx::query_scalar("SELECT signing_bytes FROM credit.signing_intents WHERE id = $1")
            .bind(intent_id)
            .fetch_one(&pool)
            .await
            .expect("read original intent signing bytes");
    let attacker_seed = [93u8; 32];
    let attacker_public_key = base64::engine::general_purpose::STANDARD
        .encode(credit::ledger::derive_public_key(&attacker_seed));
    let mut tampered_intent: serde_json::Value =
        serde_json::from_str(&original_signing_bytes).expect("parse intent signing bytes");
    tampered_intent["publicKey"] = json!(attacker_public_key);
    let tampered_signing_bytes = credit::ledger::canonicalize(&tampered_intent);
    let tampered_signature =
        credit::ledger::sign_with_seed(&tampered_signing_bytes, &attacker_seed);
    sqlx::query(
        "UPDATE credit.signing_intents SET public_key = $2, signing_bytes = $3 WHERE id = $1",
    )
    .bind(intent_id)
    .bind(&attacker_public_key)
    .bind(&tampered_signing_bytes)
    .execute(&pool)
    .await
    .expect("tamper intent key and signing bytes");
    sqlx::query("ALTER TABLE credit.ledger DISABLE TRIGGER credit_ledger_reject_mutation")
        .execute(&pool)
        .await
        .expect("disable append-only trigger for tamper fixture");
    sqlx::query(
        "UPDATE credit.ledger SET signature = $2 \
         WHERE metadata->>'signing_intent_id' = $1",
    )
    .bind(intent_id.to_string())
    .bind(&tampered_signature)
    .execute(&pool)
    .await
    .expect("tamper ledger signature");
    sqlx::query("ALTER TABLE credit.ledger ENABLE TRIGGER credit_ledger_reject_mutation")
        .execute(&pool)
        .await
        .expect("restore append-only trigger");
    let retained_hash: String = sqlx::query_scalar(
        "SELECT hash FROM credit.ledger WHERE type = 'tip' AND from_account = $1",
    )
    .bind(sender)
    .fetch_one(&pool)
    .await
    .expect("read retained ledger hash");
    assert_eq!(retained_hash, original_hash, "signature is outside the ledger hash canonical");

    let verify = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger/verify")
                .body(Body::empty())
                .expect("build tampered intent verification request"),
        )
        .await
        .expect("tampered intent verification response");
    assert_eq!(verify.status(), StatusCode::OK);
    assert!(!read_json(verify).await["ok"].as_bool().unwrap_or(true));
}

#[tokio::test]
async fn ledger_verification_rejects_proof_columns_changed_without_resigning() {
    let (pool, app) = create_test_app().await;
    let sender =
        create_test_account(&pool, "proof-column-sender@tongji.edu.cn", "proof-column-sender")
            .await;
    let recipient = create_test_account(
        &pool,
        "proof-column-recipient@tongji.edu.cn",
        "proof-column-recipient",
    )
    .await;
    mint_to_account(&pool, sender, 20).await;
    let target_id = create_tip_thread(&pool, recipient).await;
    let token = create_token(&pool, "proof-column-sender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 5,
        "targetType": "thread",
        "targetId": target_id.to_string(),
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
    let response = app.clone().oneshot(request).await.expect("create signed tip");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let (intent_id, previous_hash, mut ledger_entry): (uuid::Uuid, String, serde_json::Value) =
        sqlx::query_as(
            "SELECT (ledger.metadata->>'signing_intent_id')::uuid, ledger.prev_hash, \
                    intent.ledger_entry \
             FROM credit.ledger ledger \
             JOIN credit.signing_intents intent \
               ON intent.id = (ledger.metadata->>'signing_intent_id')::uuid \
             WHERE ledger.type = 'tip' AND ledger.from_account = $1",
        )
        .bind(sender)
        .fetch_one(&pool)
        .await
        .expect("read intent-backed ledger proof");
    ledger_entry["amount"] = json!(6);
    let tampered_canonical = credit::ledger::canonicalize(&ledger_entry);
    let tampered_hash = credit::ledger::compute_hash(&tampered_canonical, &previous_hash);
    sqlx::query(
        "UPDATE credit.signing_intents \
         SET ledger_entry = $2, ledger_canonical = $3 WHERE id = $1",
    )
    .bind(intent_id)
    .bind(&ledger_entry)
    .bind(&tampered_canonical)
    .execute(&pool)
    .await
    .expect("tamper persisted proof columns");
    sqlx::query("ALTER TABLE credit.ledger DISABLE TRIGGER credit_ledger_reject_mutation")
        .execute(&pool)
        .await
        .expect("disable append-only trigger for tamper fixture");
    sqlx::query(
        "UPDATE credit.ledger SET amount = 6, hash = $2 \
         WHERE metadata->>'signing_intent_id' = $1",
    )
    .bind(intent_id.to_string())
    .bind(tampered_hash)
    .execute(&pool)
    .await
    .expect("tamper ledger to match unresigned proof columns");
    sqlx::query("ALTER TABLE credit.ledger ENABLE TRIGGER credit_ledger_reject_mutation")
        .execute(&pool)
        .await
        .expect("restore append-only trigger");

    let verify = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger/verify")
                .body(Body::empty())
                .expect("build proof verification request"),
        )
        .await
        .expect("proof verification response");
    assert_eq!(verify.status(), StatusCode::OK);
    assert!(!read_json(verify).await["ok"].as_bool().unwrap_or(true));
}

#[tokio::test]
async fn ledger_verification_rejects_unconsumed_intent_proof() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(
        &pool,
        "unconsumed-proof-sender@tongji.edu.cn",
        "unconsumed-proof-sender",
    )
    .await;
    let recipient = create_test_account(
        &pool,
        "unconsumed-proof-recipient@tongji.edu.cn",
        "unconsumed-proof-recipient",
    )
    .await;
    mint_to_account(&pool, sender, 20).await;
    let target_id = create_tip_thread(&pool, recipient).await;
    let token = create_token(&pool, "unconsumed-proof-sender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 5,
        "targetType": "thread",
        "targetId": target_id.to_string(),
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
    let response = app.clone().oneshot(request).await.expect("create signed tip");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    sqlx::query(
        "UPDATE credit.signing_intents SET consumed_at = NULL \
         WHERE id = (SELECT (metadata->>'signing_intent_id')::uuid \
                     FROM credit.ledger WHERE type = 'tip' AND from_account = $1)",
    )
    .bind(sender)
    .execute(&pool)
    .await
    .expect("remove consumed proof marker");

    let verify = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/ledger/verify")
                .body(Body::empty())
                .expect("build consumed-proof verification request"),
        )
        .await
        .expect("consumed-proof verification response");
    assert_eq!(verify.status(), StatusCode::OK);
    assert!(!read_json(verify).await["ok"].as_bool().unwrap_or(true));
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

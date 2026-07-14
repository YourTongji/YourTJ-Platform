//! Integration coverage for the legacy-wallet claim transaction.

#[path = "helpers/mod.rs"]
mod helpers;

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use base64::Engine as _;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt as _;

struct PreparedClaim {
    account_id: i64,
    challenge_id: String,
    legacy_user_hash: String,
    review_id: i64,
    signature: String,
    token: String,
}

impl PreparedClaim {
    fn request(&self) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/api/v2/wallet/claim")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .body(Body::from(
                json!({
                    "legacyUserHash": self.legacy_user_hash,
                    "challengeId": self.challenge_id,
                    "signature": self.signature,
                })
                .to_string(),
            ))
            .expect("build wallet claim request")
    }
}

async fn prepare_claim(pool: &PgPool, app: &axum::Router, legacy_balance: i64) -> PreparedClaim {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("wallet-claim-{suffix}@tongji.edu.cn");
    let handle = format!("wallet-claim-{}", &suffix[..12]);
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(&email)
    .bind(&handle)
    .fetch_one(pool)
    .await
    .expect("create wallet claim account");
    let (token, token_account_id) = helpers::create_access_token_for(&email, pool).await;
    assert_eq!(token_account_id, account_id);

    let legacy_user_hash = format!("synthetic-legacy-wallet-{suffix}");
    let legacy_seed = [17u8; 32];
    let legacy_public_key = base64::engine::general_purpose::STANDARD
        .encode(credit::ledger::derive_public_key(&legacy_seed));
    sqlx::query(
        "INSERT INTO identity.legacy_wallet_links \
         (legacy_user_hash, legacy_public_key, legacy_balance) VALUES ($1, $2, $3)",
    )
    .bind(&legacy_user_hash)
    .bind(&legacy_public_key)
    .bind(legacy_balance)
    .execute(pool)
    .await
    .expect("create legacy wallet link");
    let course_id: i64 = sqlx::query_scalar(
        "INSERT INTO courses.courses (code, name) VALUES ($1, 'Wallet claim fixture') RETURNING id",
    )
    .bind(format!("CLAIM-{suffix}"))
    .fetch_one(pool)
    .await
    .expect("create wallet claim course");
    let review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews \
         (course_id, rating, comment, reviewer_name, wallet_user_hash, is_legacy) \
         VALUES ($1, 5, 'Synthetic legacy review', 'legacy-reviewer', $2, 1) RETURNING id",
    )
    .bind(course_id)
    .bind(&legacy_user_hash)
    .fetch_one(pool)
    .await
    .expect("create legacy review");

    let challenge_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v2/wallet/claim-challenge")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build wallet claim challenge request"),
        )
        .await
        .expect("request wallet claim challenge");
    assert_eq!(challenge_response.status(), StatusCode::OK);
    let challenge = helpers::read_json(challenge_response).await;
    let challenge_id = challenge["challengeId"].as_str().expect("claim challenge id").to_string();
    let nonce = challenge["nonce"].as_str().expect("claim challenge nonce");
    let canonical = serde_json::to_string(&json!({
        "accountId": account_id.to_string(),
        "challengeId": challenge_id,
        "legacyUserHash": legacy_user_hash,
        "nonce": nonce,
    }))
    .expect("serialize wallet claim payload");
    let signature = credit::ledger::sign_with_seed(&canonical, &legacy_seed);

    PreparedClaim { account_id, challenge_id, legacy_user_hash, review_id, signature, token }
}

fn system_public_key() -> String {
    base64::engine::general_purpose::STANDARD.encode(credit::ledger::derive_public_key(&[0u8; 32]))
}

async fn wait_for_lock_wait(pool: &PgPool, query_prefix: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    let query_pattern = format!("{query_prefix}%");
    loop {
        let is_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE datname = current_database() \
                 AND pid <> pg_backend_pid() \
                 AND wait_event_type = 'Lock' \
                 AND ltrim(query) LIKE $1 \
             )",
        )
        .bind(&query_pattern)
        .fetch_one(pool)
        .await
        .expect("inspect database lock waits");
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
async fn claim_verifies_ledger_and_advances_wallet_last_seq() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let prepared = prepare_claim(&pool, &app, 75).await;

    let response = app.oneshot(prepared.request()).await.expect("claim legacy wallet response");
    assert_eq!(response.status(), StatusCode::OK);
    let response_body = helpers::read_json(response).await;
    assert_eq!(response_body["accountId"], prepared.account_id.to_string());
    assert_eq!(response_body["balance"], 75);

    let (ledger_seq, tx_id, metadata): (i64, String, Option<Value>) = sqlx::query_as(
        "SELECT seq, tx_id, metadata FROM credit.ledger \
         WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
    )
    .bind(prepared.account_id)
    .fetch_one(&pool)
    .await
    .expect("read claim ledger entry");
    assert!(uuid::Uuid::parse_str(&tx_id).is_ok());
    assert!(!tx_id.contains(&prepared.legacy_user_hash));
    assert_eq!(metadata, Some(json!({ "reason": "legacy_wallet_claim" })));

    let (balance, last_seq): (i64, i64) =
        sqlx::query_as("SELECT balance, last_seq FROM credit.wallets WHERE account_id = $1")
            .bind(prepared.account_id)
            .fetch_one(&pool)
            .await
            .expect("read claimed wallet projection");
    assert_eq!(balance, 75);
    assert_eq!(last_seq, ledger_seq);
    let review_account_id: Option<i64> =
        sqlx::query_scalar("SELECT account_id FROM reviews.reviews WHERE id = $1")
            .bind(prepared.review_id)
            .fetch_one(&pool)
            .await
            .expect("read claimed legacy review");
    assert_eq!(review_account_id, Some(prepared.account_id));

    let verification = credit::repo::verify_full_ledger(&pool, &system_public_key())
        .await
        .expect("verify ledger after wallet claim");
    assert!(verification.ok);
    assert_eq!(verification.latest_seq, Some(ledger_seq));
}

#[tokio::test]
async fn review_owner_failure_rolls_back_the_entire_claim() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let prepared = prepare_claim(&pool, &app, 45).await;
    sqlx::raw_sql(
        "DROP TRIGGER IF EXISTS test_reject_legacy_review_claim ON reviews.reviews; \
         DROP FUNCTION IF EXISTS reviews.test_reject_legacy_review_claim(); \
         CREATE FUNCTION reviews.test_reject_legacy_review_claim() RETURNS trigger \
         LANGUAGE plpgsql AS $$ \
         BEGIN \
           RAISE EXCEPTION 'synthetic review claim failure'; \
         END \
         $$; \
         CREATE TRIGGER test_reject_legacy_review_claim \
         BEFORE UPDATE OF account_id ON reviews.reviews \
         FOR EACH ROW EXECUTE FUNCTION reviews.test_reject_legacy_review_claim();",
    )
    .execute(&pool)
    .await
    .expect("install review claim failure trigger");

    let response = app.oneshot(prepared.request()).await.expect("failed claim response");

    sqlx::raw_sql(
        "DROP TRIGGER test_reject_legacy_review_claim ON reviews.reviews; \
         DROP FUNCTION reviews.test_reject_legacy_review_claim();",
    )
    .execute(&pool)
    .await
    .expect("remove review claim failure trigger");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let (linked_account_id, was_claimed): (Option<i64>, bool) = sqlx::query_as(
        "SELECT account_id, claimed_at IS NOT NULL \
         FROM identity.legacy_wallet_links WHERE legacy_user_hash = $1",
    )
    .bind(&prepared.legacy_user_hash)
    .fetch_one(&pool)
    .await
    .expect("read rolled-back legacy wallet link");
    assert_eq!(linked_account_id, None);
    assert!(!was_claimed);
    let challenge_was_used: bool = sqlx::query_scalar(
        "SELECT used_at IS NOT NULL FROM identity.wallet_claim_challenges WHERE id = $1",
    )
    .bind(&prepared.challenge_id)
    .fetch_one(&pool)
    .await
    .expect("read rolled-back claim challenge");
    assert!(!challenge_was_used);
    let review_account_id: Option<i64> =
        sqlx::query_scalar("SELECT account_id FROM reviews.reviews WHERE id = $1")
            .bind(prepared.review_id)
            .fetch_one(&pool)
            .await
            .expect("read rolled-back review owner");
    assert_eq!(review_account_id, None);
    let claim_ledger_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
    )
    .bind(prepared.account_id)
    .fetch_one(&pool)
    .await
    .expect("count rolled-back claim ledger entries");
    assert_eq!(claim_ledger_entries, 0);
}

#[tokio::test]
async fn claim_and_regular_append_share_one_hash_chain() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let prepared = prepare_claim(&pool, &app, 70).await;

    let mut table_lock = pool.begin().await.expect("begin ledger table lock transaction");
    sqlx::query("LOCK TABLE credit.ledger IN SHARE MODE")
        .execute(&mut *table_lock)
        .await
        .expect("block ledger inserts while allowing the previous hash read");

    let claim_account_id = prepared.account_id;
    let claim_app = app.clone();
    let claim_request = prepared.request();
    let claim_task = tokio::spawn(async move { claim_app.oneshot(claim_request).await });
    let claim_reached_insert = wait_for_lock_wait(&pool, "INSERT INTO credit.ledger").await;

    let regular_tx_id = uuid::Uuid::new_v4().to_string();
    let regular_task_tx_id = regular_tx_id.clone();
    let regular_pool = pool.clone();
    let regular_append = tokio::spawn(async move {
        credit::repo::mint_points_with_tx_id(
            &regular_pool,
            claim_account_id,
            30,
            &regular_task_tx_id,
            "concurrent claim regression",
            &[0u8; 32],
        )
        .await
    });
    let regular_waited_for_claim = wait_for_lock_wait(&pool, "SELECT pg_advisory_xact_lock").await;

    table_lock.commit().await.expect("release ledger table lock");
    let claim_response =
        claim_task.await.expect("join wallet claim task").expect("wallet claim service response");
    let regular_entry =
        regular_append.await.expect("join regular ledger append").expect("regular ledger append");

    assert!(claim_reached_insert, "claim did not reach the blocked ledger insert");
    assert!(
        regular_waited_for_claim,
        "regular append did not wait on the claim's Credit advisory lock"
    );
    assert_eq!(claim_response.status(), StatusCode::OK);

    let (claim_seq, claim_prev_hash, claim_hash): (i64, String, String) = sqlx::query_as(
        "SELECT seq, prev_hash, hash FROM credit.ledger \
         WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
    )
    .bind(prepared.account_id)
    .fetch_one(&pool)
    .await
    .expect("read concurrent claim ledger entry");
    let entries_are_linked = (claim_seq < regular_entry.seq
        && regular_entry.prev_hash == claim_hash)
        || (regular_entry.seq < claim_seq && claim_prev_hash == regular_entry.hash);
    assert!(entries_are_linked, "concurrent appends did not form one linear chain");

    let (balance, last_seq): (i64, i64) =
        sqlx::query_as("SELECT balance, last_seq FROM credit.wallets WHERE account_id = $1")
            .bind(prepared.account_id)
            .fetch_one(&pool)
            .await
            .expect("read concurrent wallet projection");
    assert_eq!(balance, 100);
    assert_eq!(last_seq, claim_seq.max(regular_entry.seq));

    let verification = credit::repo::verify_full_ledger(&pool, &system_public_key())
        .await
        .expect("verify ledger after concurrent append");
    assert!(verification.ok);
    assert_eq!(verification.latest_seq, Some(last_seq));

    let stored_regular_tx_id: String =
        sqlx::query_scalar("SELECT tx_id FROM credit.ledger WHERE tx_id = $1")
            .bind(&regular_tx_id)
            .fetch_one(&pool)
            .await
            .expect("read regular ledger transaction");
    assert_eq!(stored_regular_tx_id, regular_tx_id);
}

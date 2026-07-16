//! Integration coverage for the legacy-wallet claim transaction.

#[path = "helpers/mod.rs"]
mod helpers;

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use base64::Engine as _;
use serde_json::{json, Value};
use sha2::Digest as _;
use sqlx::{PgConnection, PgPool};
use tower::ServiceExt as _;

struct AccountWalletKeyResolver;

impl credit::wallet_keys::WalletKeyResolver for AccountWalletKeyResolver {
    fn active_public_key<'a>(
        &'a self,
        pool: &'a PgPool,
        account_id: i64,
    ) -> credit::wallet_keys::WalletKeyFuture<'a, Option<String>> {
        Box::pin(identity::wallet_keys::active_public_key(pool, account_id))
    }

    fn active_public_key_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_id: i64,
    ) -> credit::wallet_keys::WalletKeyFuture<'a, Option<String>> {
        Box::pin(identity::wallet_keys::active_public_key_on(conn, account_id))
    }

    fn verification_public_keys_on<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        account_ids: &'a [i64],
    ) -> credit::wallet_keys::WalletKeyFuture<'a, credit::wallet_keys::VerificationPublicKeys> {
        Box::pin(identity::wallet_keys::verification_public_keys_on(conn, account_ids))
    }
}

struct PreparedClaim {
    account_id: i64,
    challenge_id: String,
    edit_token: String,
    legacy_user_hash: String,
    review_id: i64,
    signature: String,
    token: String,
}

impl PreparedClaim {
    fn request(&self) -> Request<Body> {
        self.request_with(&self.legacy_user_hash, &self.challenge_id, &self.signature)
    }

    fn request_with(
        &self,
        legacy_user_hash: &str,
        challenge_id: &str,
        signature: &str,
    ) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/api/v2/wallet/claim")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header("x-forwarded-for", format!("2001:db8::{:x}", self.account_id))
            .body(Body::from(
                json!({
                    "legacyUserHash": legacy_user_hash,
                    "challengeId": challenge_id,
                    "signature": signature,
                })
                .to_string(),
            ))
            .expect("build wallet claim request")
    }
}

fn synthetic_legacy_user_hash(label: &str) -> String {
    hex::encode(sha2::Sha256::digest(label.as_bytes()))
}

fn invalid_ed25519_signature() -> String {
    base64::engine::general_purpose::STANDARD.encode([0u8; 64])
}

fn assert_generic_claim_rejection(body: &Value) {
    assert_eq!(body["error"]["code"], "BAD_REQUEST");
    assert_eq!(body["error"]["message"], "wallet claim proof is invalid");
}

fn claim_challenge_request(token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/api/v2/wallet/claim-challenge")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .expect("build wallet claim challenge request")
}

async fn create_claim_account(pool: &PgPool, suffix: &str) -> (i64, String) {
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
    (account_id, token)
}

async fn prepare_claim(pool: &PgPool, app: &axum::Router, legacy_balance: i64) -> PreparedClaim {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (account_id, token) = create_claim_account(pool, &suffix).await;

    let legacy_user_hash = synthetic_legacy_user_hash(&format!("synthetic-legacy-wallet-{suffix}"));
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
    let edit_token = format!("legacy-edit-token-{suffix}");
    let review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews \
         (course_id, rating, comment, reviewer_name, wallet_user_hash, edit_token, is_legacy) \
         VALUES ($1, 5, 'Synthetic legacy review', 'legacy-reviewer', $2, $3, 1) RETURNING id",
    )
    .bind(course_id)
    .bind(&legacy_user_hash)
    .bind(&edit_token)
    .fetch_one(pool)
    .await
    .expect("create legacy review");

    let challenge_response = app
        .clone()
        .oneshot(claim_challenge_request(&token))
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

    PreparedClaim {
        account_id,
        challenge_id,
        edit_token,
        legacy_user_hash,
        review_id,
        signature,
        token,
    }
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
async fn claim_challenge_rechecks_account_state_after_waiting_for_lifecycle() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (account_id, token) = create_claim_account(&pool, &suffix).await;

    let mut lifecycle_tx = pool.begin().await.expect("begin challenge lifecycle transition");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deletion_requested', \
                deletion_requested_at = now(), \
                deletion_recover_until = now() + interval '30 days' \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(&mut *lifecycle_tx)
    .await
    .expect("stage challenge lifecycle transition");

    let request_task = tokio::spawn(async move {
        app.oneshot(claim_challenge_request(&token)).await.expect("lifecycle challenge response")
    });
    assert!(
        wait_for_lock_wait(
            &pool,
            "SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE",
        )
        .await,
        "wallet challenge did not wait on the lifecycle account lock"
    );

    lifecycle_tx.commit().await.expect("commit challenge lifecycle transition");
    let response = request_task.await.expect("join lifecycle challenge request");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let challenge_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.wallet_claim_challenges WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count lifecycle wallet challenges");
    assert_eq!(challenge_count, 0);
}

#[tokio::test]
async fn issuing_a_new_challenge_invalidates_the_previous_challenge() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let prepared = prepare_claim(&pool, &app, 35).await;

    let replacement_response = app
        .clone()
        .oneshot(claim_challenge_request(&prepared.token))
        .await
        .expect("replacement challenge response");
    assert_eq!(replacement_response.status(), StatusCode::OK);
    let replacement = helpers::read_json(replacement_response).await;
    let replacement_id = replacement["challengeId"].as_str().expect("replacement challenge id");
    assert_ne!(replacement_id, prepared.challenge_id);

    let stored_challenges: Vec<String> =
        sqlx::query_scalar("SELECT id FROM identity.wallet_claim_challenges WHERE account_id = $1")
            .bind(prepared.account_id)
            .fetch_all(&pool)
            .await
            .expect("read replaced wallet challenges");
    assert_eq!(stored_challenges, vec![replacement_id.to_string()]);

    let stale_claim = app.oneshot(prepared.request()).await.expect("stale wallet claim response");
    assert_eq!(stale_claim.status(), StatusCode::BAD_REQUEST);
    let claim_ledger_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
    )
    .bind(prepared.account_id)
    .fetch_one(&pool)
    .await
    .expect("count stale claim ledger entries");
    assert_eq!(claim_ledger_entries, 0);
}

#[tokio::test]
async fn concurrent_challenge_issuance_keeps_only_one_account_challenge() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (account_id, token) = create_claim_account(&pool, &suffix).await;

    let (first, second) = tokio::join!(
        app.clone().oneshot(claim_challenge_request(&token)),
        app.clone().oneshot(claim_challenge_request(&token)),
    );
    let first = first.expect("first concurrent challenge response");
    let second = second.expect("second concurrent challenge response");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let first = helpers::read_json(first).await;
    let second = helpers::read_json(second).await;
    let issued_ids = [
        first["challengeId"].as_str().expect("first challenge id"),
        second["challengeId"].as_str().expect("second challenge id"),
    ];

    let stored_challenges: Vec<String> =
        sqlx::query_scalar("SELECT id FROM identity.wallet_claim_challenges WHERE account_id = $1")
            .bind(account_id)
            .fetch_all(&pool)
            .await
            .expect("read concurrent wallet challenges");
    assert_eq!(stored_challenges.len(), 1);
    assert!(issued_ids.contains(&stored_challenges[0].as_str()));
}

#[tokio::test]
async fn suspended_account_cannot_replace_a_wallet_claim_challenge() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (account_id, token) = create_claim_account(&pool, &suffix).await;
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason) \
         VALUES ($1, 'suspend', 'wallet claim test suspension')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("suspend wallet claim account");

    let response =
        app.oneshot(claim_challenge_request(&token)).await.expect("suspended challenge response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let challenge_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.wallet_claim_challenges WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count suspended wallet challenges");
    assert_eq!(challenge_count, 0);
}

#[tokio::test]
async fn challenge_issuance_is_rate_limited_when_redis_is_available() {
    let Some((pool, app, redis_pool)) = helpers::create_test_app_with_redis_if_available().await
    else {
        return;
    };
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (account_id, token) = create_claim_account(&pool, &suffix).await;
    let rate_limit_key = format!("rl:wallet_claim_challenge:{account_id}");
    let mut redis_connection = redis_pool.get().await.expect("acquire wallet claim test Redis");
    let _: () = redis::cmd("DEL")
        .arg(&rate_limit_key)
        .query_async(&mut redis_connection)
        .await
        .expect("clear wallet claim rate limit");
    drop(redis_connection);

    for _ in 0..10 {
        let response = app
            .clone()
            .oneshot(claim_challenge_request(&token))
            .await
            .expect("rate-limited challenge response");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let limited =
        app.oneshot(claim_challenge_request(&token)).await.expect("over-limit challenge response");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let challenge_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.wallet_claim_challenges WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count rate-limited wallet challenges");
    assert_eq!(challenge_count, 1);
}

#[tokio::test]
async fn malformed_claim_fields_are_rejected_without_consuming_the_challenge() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let prepared = prepare_claim(&pool, &app, 35).await;
    let oversized_hash = "a".repeat(256 * 1024);
    let invalid_requests = [
        prepared.request_with(&oversized_hash, &prepared.challenge_id, &prepared.signature),
        prepared.request_with(&prepared.legacy_user_hash, "not-a-uuid", &prepared.signature),
        prepared.request_with(&prepared.legacy_user_hash, &prepared.challenge_id, "not-base64"),
    ];

    let mut rejection_body: Option<Value> = None;
    for request in invalid_requests {
        let response = app.clone().oneshot(request).await.expect("malformed claim response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = helpers::read_json(response).await;
        assert_generic_claim_rejection(&body);
        if let Some(expected) = rejection_body.as_ref() {
            assert_eq!(&body, expected);
        } else {
            rejection_body = Some(body);
        }
    }

    let challenge_was_used: bool = sqlx::query_scalar(
        "SELECT used_at IS NOT NULL FROM identity.wallet_claim_challenges WHERE id = $1",
    )
    .bind(&prepared.challenge_id)
    .fetch_one(&pool)
    .await
    .expect("read challenge after malformed requests");
    assert!(!challenge_was_used);
    let linked_account_id: Option<i64> = sqlx::query_scalar(
        "SELECT account_id FROM identity.legacy_wallet_links WHERE legacy_user_hash = $1",
    )
    .bind(&prepared.legacy_user_hash)
    .fetch_one(&pool)
    .await
    .expect("read link after malformed requests");
    assert_eq!(linked_account_id, None);
    let claim_ledger_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
    )
    .bind(prepared.account_id)
    .fetch_one(&pool)
    .await
    .expect("count claim entries after malformed requests");
    assert_eq!(claim_ledger_entries, 0);
}

#[tokio::test]
async fn legacy_proof_failures_are_neutral_and_consume_the_challenge() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let mut rejection_body: Option<Value> = None;

    for failure_kind in ["missing", "claimed", "keyless", "signature"] {
        let prepared = prepare_claim(&pool, &app, 35).await;
        match failure_kind {
            "missing" => {
                sqlx::query("DELETE FROM identity.legacy_wallet_links WHERE legacy_user_hash = $1")
                    .bind(&prepared.legacy_user_hash)
                    .execute(&pool)
                    .await
                    .expect("delete legacy link fixture");
            }
            "claimed" => {
                sqlx::query(
                    "UPDATE identity.legacy_wallet_links \
                     SET account_id = $2, claimed_at = now() WHERE legacy_user_hash = $1",
                )
                .bind(&prepared.legacy_user_hash)
                .bind(prepared.account_id)
                .execute(&pool)
                .await
                .expect("pre-claim legacy link fixture");
            }
            "keyless" => {
                sqlx::query(
                    "UPDATE identity.legacy_wallet_links SET legacy_public_key = NULL \
                     WHERE legacy_user_hash = $1",
                )
                .bind(&prepared.legacy_user_hash)
                .execute(&pool)
                .await
                .expect("remove legacy public key fixture");
            }
            "signature" => {}
            _ => unreachable!("test enumerates all proof failure kinds"),
        }
        let invalid_signature = invalid_ed25519_signature();
        let signature =
            if failure_kind == "signature" { &invalid_signature } else { &prepared.signature };
        let request =
            prepared.request_with(&prepared.legacy_user_hash, &prepared.challenge_id, signature);
        let response = app.clone().oneshot(request).await.expect("invalid proof response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = helpers::read_json(response).await;
        assert_generic_claim_rejection(&body);
        if let Some(expected) = rejection_body.as_ref() {
            assert_eq!(&body, expected);
        } else {
            rejection_body = Some(body.clone());
        }

        let challenge_was_used: bool = sqlx::query_scalar(
            "SELECT used_at IS NOT NULL FROM identity.wallet_claim_challenges WHERE id = $1",
        )
        .bind(&prepared.challenge_id)
        .fetch_one(&pool)
        .await
        .expect("read challenge after invalid legacy proof");
        assert!(challenge_was_used);

        let retry =
            app.clone().oneshot(prepared.request()).await.expect("used challenge retry response");
        assert_eq!(retry.status(), StatusCode::BAD_REQUEST);
        let retry_body = helpers::read_json(retry).await;
        assert_eq!(retry_body, body);
        let claim_ledger_entries: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM credit.ledger \
             WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
        )
        .bind(prepared.account_id)
        .fetch_one(&pool)
        .await
        .expect("count entries after invalid legacy proof");
        assert_eq!(claim_ledger_entries, 0);
    }
}

#[tokio::test]
async fn claim_attempts_are_rate_limited_when_redis_is_available() {
    let Some((pool, app, redis_pool)) = helpers::create_test_app_with_redis_if_available().await
    else {
        return;
    };
    let prepared = prepare_claim(&pool, &app, 35).await;
    let rate_limit_key = format!("rl:wallet_claim_account:{}", prepared.account_id);
    let mut redis_connection = redis_pool.get().await.expect("acquire claim attempt Redis");
    let _: () = redis::cmd("DEL")
        .arg(&rate_limit_key)
        .query_async(&mut redis_connection)
        .await
        .expect("clear claim attempt rate limit");
    drop(redis_connection);

    let invalid_signature = invalid_ed25519_signature();
    let mut challenge_id = prepared.challenge_id.clone();
    for attempt in 0..10 {
        let request =
            prepared.request_with(&prepared.legacy_user_hash, &challenge_id, &invalid_signature);
        let response = app.clone().oneshot(request).await.expect("bounded claim response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_generic_claim_rejection(&helpers::read_json(response).await);

        if attempt < 9 {
            let challenge_response = app
                .clone()
                .oneshot(claim_challenge_request(&prepared.token))
                .await
                .expect("replacement challenge response");
            assert_eq!(challenge_response.status(), StatusCode::OK);
            let challenge = helpers::read_json(challenge_response).await;
            challenge_id =
                challenge["challengeId"].as_str().expect("replacement challenge id").to_string();
        }
    }

    let over_limit_challenge = uuid::Uuid::new_v4().to_string();
    let limited = app
        .oneshot(prepared.request_with(
            &prepared.legacy_user_hash,
            &over_limit_challenge,
            &invalid_signature,
        ))
        .await
        .expect("over-limit claim response");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    let linked_account_id: Option<i64> = sqlx::query_scalar(
        "SELECT account_id FROM identity.legacy_wallet_links WHERE legacy_user_hash = $1",
    )
    .bind(&prepared.legacy_user_hash)
    .fetch_one(&pool)
    .await
    .expect("read rate-limited legacy link");
    assert_eq!(linked_account_id, None);
}

#[tokio::test]
async fn claim_rechecks_account_state_after_waiting_for_lifecycle() {
    let (pool, app) = helpers::create_test_app_without_redis().await;
    let prepared = prepare_claim(&pool, &app, 55).await;

    let mut lifecycle_tx = pool.begin().await.expect("begin claim lifecycle transition");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deletion_requested', \
                deletion_requested_at = now(), \
                deletion_recover_until = now() + interval '30 days' \
         WHERE id = $1",
    )
    .bind(prepared.account_id)
    .execute(&mut *lifecycle_tx)
    .await
    .expect("stage claim lifecycle transition");

    let request = prepared.request();
    let request_task = tokio::spawn(async move {
        app.oneshot(request).await.expect("lifecycle wallet claim response")
    });
    assert!(
        wait_for_lock_wait(&pool, "SELECT account.id FROM identity.accounts AS account").await,
        "wallet claim did not wait on the lifecycle account lock"
    );

    lifecycle_tx.commit().await.expect("commit claim lifecycle transition");
    let response = request_task.await.expect("join lifecycle wallet claim request");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let (linked_account_id, was_claimed): (Option<i64>, bool) = sqlx::query_as(
        "SELECT account_id, claimed_at IS NOT NULL \
         FROM identity.legacy_wallet_links WHERE legacy_user_hash = $1",
    )
    .bind(&prepared.legacy_user_hash)
    .fetch_one(&pool)
    .await
    .expect("read lifecycle-blocked legacy wallet link");
    assert_eq!(linked_account_id, None);
    assert!(!was_claimed);
    let challenge_was_used: bool = sqlx::query_scalar(
        "SELECT used_at IS NOT NULL FROM identity.wallet_claim_challenges WHERE id = $1",
    )
    .bind(&prepared.challenge_id)
    .fetch_one(&pool)
    .await
    .expect("read lifecycle-blocked claim challenge");
    assert!(!challenge_was_used);
    let review_owner_and_credentials: (Option<i64>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT account_id, wallet_user_hash, edit_token \
             FROM reviews.reviews WHERE id = $1",
        )
        .bind(prepared.review_id)
        .fetch_one(&pool)
        .await
        .expect("read lifecycle-blocked legacy review");
    assert_eq!(
        review_owner_and_credentials,
        (None, Some(prepared.legacy_user_hash.clone()), Some(prepared.edit_token.clone()))
    );
    let claim_ledger_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger \
         WHERE to_account = $1 AND metadata->>'reason' = 'legacy_wallet_claim'",
    )
    .bind(prepared.account_id)
    .fetch_one(&pool)
    .await
    .expect("count lifecycle-blocked claim ledger entries");
    assert_eq!(claim_ledger_entries, 0);
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
    assert!(response_body["activePublicKey"].is_null());

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
    let review_owner_and_credentials: (Option<i64>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT account_id, wallet_user_hash, edit_token \
             FROM reviews.reviews WHERE id = $1",
        )
        .bind(prepared.review_id)
        .fetch_one(&pool)
        .await
        .expect("read claimed legacy review");
    assert_eq!(review_owner_and_credentials, (Some(prepared.account_id), None, None));

    let verification =
        credit::repo::verify_full_ledger(&pool, &system_public_key(), &AccountWalletKeyResolver)
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
    let review_owner_and_credentials: (Option<i64>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT account_id, wallet_user_hash, edit_token \
             FROM reviews.reviews WHERE id = $1",
        )
        .bind(prepared.review_id)
        .fetch_one(&pool)
        .await
        .expect("read rolled-back review owner");
    assert_eq!(
        review_owner_and_credentials,
        (None, Some(prepared.legacy_user_hash.clone()), Some(prepared.edit_token.clone()))
    );
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

    let verification =
        credit::repo::verify_full_ledger(&pool, &system_public_key(), &AccountWalletKeyResolver)
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

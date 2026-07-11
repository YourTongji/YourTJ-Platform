//! Shared test helpers for the credit integration test suite.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, Response, StatusCode};
use credit::tip_targets::{ResolvedTipTarget, TipTargetResolver};
use serde_json::Value;
use shared::{AppResult, AppState};
use sqlx::{PgConnection, PgPool};

struct ContentTipTargetResolver;

impl TipTargetResolver for ContentTipTargetResolver {
    fn resolve<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        target_type: &'a str,
        target_id: i64,
    ) -> Pin<Box<dyn Future<Output = AppResult<Option<ResolvedTipTarget>>> + Send + 'a>> {
        Box::pin(async move {
            let target =
                match target_type {
                    "review" => reviews::tip_targets::resolve_tip_target(conn, target_id)
                        .await?
                        .map(|target| ResolvedTipTarget {
                            canonical_type: target.canonical_type.to_string(),
                            canonical_id: target.canonical_id,
                            author_id: target.author_id,
                        }),
                    "thread" | "comment" => {
                        forum::tip_targets::resolve_tip_target(conn, target_type, target_id)
                            .await?
                            .map(|target| ResolvedTipTarget {
                                canonical_type: target.canonical_type.to_string(),
                                canonical_id: target.canonical_id,
                                author_id: target.author_id,
                            })
                    }
                    _ => None,
                };
            let Some(target) = target else {
                return Ok(None);
            };
            if !identity::public_accounts::is_credit_recipient_eligible(conn, target.author_id)
                .await?
            {
                return Ok(None);
            }
            Ok(Some(target))
        })
    }
}

/// Create a complete test application with credit routes.
pub async fn create_test_app() -> (PgPool, axum::Router) {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".to_string());

    let pool = PgPool::connect(&url).await.expect("failed to connect to test database");

    run_migrations(&pool).await;

    // Deterministic test system key: seed [0u8; 32]. The public key must be the
    // real Ed25519 public key derived from that seed, otherwise system-signed
    // ledger entries fail verification in `/wallet/ledger/verify`.
    let seed = [0u8; 32];
    let public_key_bytes = credit::ledger::derive_public_key(&seed);
    let public_key_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &public_key_bytes);

    let state = AppState {
        db: pool.clone(),
        config: shared::Config::from_env().expect("test Config::from_env"),
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis: None,
        system_private_key: seed.to_vec(),
        system_public_key_b64: public_key_b64,
        email_encryption: None,
        captcha_verifier: None,
        sse_tx: None,
    };

    let router = credit::routes(state, Arc::new(ContentTipTargetResolver));
    (pool, router)
}

/// Run the DDL and clean test data.
async fn run_migrations(pool: &PgPool) {
    let is_fresh = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if is_fresh {
        let migrations: [&str; 16] = [
            include_str!("../../../../migrations/0001_init.sql"),
            include_str!("../../../../migrations/0002_escrow_selection.sql"),
            include_str!("../../../../migrations/0003_platform.sql"),
            include_str!("../../../../migrations/0004_review_remediation.sql"),
            include_str!("../../../../migrations/0005_forum_parity.sql"),
            include_str!("../../../../migrations/0006_forum_f2_f3.sql"),
            include_str!("../../../../migrations/0007_badges_feature.sql"),
            include_str!("../../../../migrations/0008_badge_mint_bridge.sql"),
            include_str!("../../../../migrations/0009_selection_raw_pk.sql"),
            include_str!("../../../../migrations/0010_selection_raw_normalized.sql"),
            include_str!("../../../../migrations/0011_password_auth.sql"),
            include_str!("../../../../migrations/0012_natural_key_upsert.sql"),
            include_str!("../../../../migrations/0013_teacher_names.sql"),
            include_str!("../../../../migrations/0014_credit_signing_intents.sql"),
            include_str!("../../../../migrations/0017_credit_prepared_ledger.sql"),
            include_str!("../../../../migrations/0032_credit_integrity_constraints.sql"),
        ];
        for (i, sql) in migrations.iter().enumerate() {
            sqlx::raw_sql(sql)
                .execute(pool)
                .await
                .unwrap_or_else(|_| panic!("migration {:03} failed", i + 1));
        }
    }

    let has_integrity_constraints: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM pg_constraint \
           WHERE conname = 'credit_ledger_controlled_flow_type' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_integrity_constraints {
        sqlx::raw_sql(include_str!("../../../../migrations/0032_credit_integrity_constraints.sql"))
            .execute(pool)
            .await
            .expect("migration 0032 failed");
    }

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM credit.purchases").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.products").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.tasks").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.signing_intents").execute(pool).await.ok();
    sqlx::query("TRUNCATE credit.ledger RESTART IDENTITY").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("TRUNCATE courses.courses RESTART IDENTITY CASCADE").execute(pool).await.ok();
    // TRUNCATE ... CASCADE removes accounts and every row referencing them
    // (across crates), so leftover FK references never block cleanup and cause
    // cross-suite email collisions. Plain DELETE silently fails on such refs.
    sqlx::query("TRUNCATE identity.accounts CASCADE").execute(pool).await.ok();
}

/// Build a wallet-signed POST request using the production signing-intent protocol.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)] // reason: integration requests keep transport and signed payload explicit.
pub async fn signed_post_request(
    app: &axum::Router,
    pool: &PgPool,
    token: &str,
    account_id: i64,
    uri: &str,
    action: &str,
    signing_request: Value,
    body: Option<Value>,
) -> Request<Body> {
    use tower::ServiceExt as _;

    let seed = test_wallet_seed(account_id);
    let public_key = credit::ledger::derive_public_key(&seed);
    let public_key_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, public_key);
    sqlx::query(
        "INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2) \
         ON CONFLICT (public_key) DO NOTHING",
    )
    .bind(account_id)
    .bind(public_key_b64)
    .execute(pool)
    .await
    .expect("bind test wallet key");

    let idempotency_key = uuid::Uuid::new_v4().to_string();
    let intent_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Idempotency-Key", &idempotency_key)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "action": action, "request": signing_request }).to_string(),
                ))
                .expect("build signing intent request"),
        )
        .await
        .expect("create signing intent response");
    assert_eq!(intent_response.status(), StatusCode::OK);
    let intent = read_json(intent_response).await;
    let signing_bytes = intent["signingBytes"].as_str().expect("intent signingBytes");
    let signature = credit::ledger::sign_with_seed(signing_bytes, &seed);

    let mut builder = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", idempotency_key)
        .header("X-Wallet-Intent", intent["intentId"].as_str().expect("intent id"))
        .header("X-Wallet-Sig", signature);
    let request_body = match body {
        Some(body) => {
            builder = builder.header("Content-Type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    builder.body(request_body).expect("build signed request")
}

#[allow(dead_code)]
fn test_wallet_seed(account_id: i64) -> [u8; 32] {
    use sha2::Digest as _;

    sha2::Sha256::digest(format!("yourtj-test-wallet-{account_id}").as_bytes()).into()
}

/// Read the JSON body from a response.
#[allow(dead_code)]
pub async fn read_json(resp: Response<Body>) -> Value {
    let bytes =
        to_bytes(resp.into_body(), 10 * 1024 * 1024).await.expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("failed to parse JSON response")
}

/// Insert a test account and return its id.
pub async fn create_test_account(pool: &PgPool, email: &str, handle: &str) -> i64 {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO identity.accounts (email, handle, role, status) \
         VALUES ($1, $2, 'user'::identity.account_role, 'active'::identity.account_status) \
         RETURNING id",
    )
    .bind(email)
    .bind(handle)
    .fetch_one(pool)
    .await
    .expect("create test account");

    // Ensure wallet exists.
    sqlx::query(
        "INSERT INTO credit.wallets (account_id, balance, last_seq) \
         VALUES ($1, 0, 0) ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(row.0)
    .execute(pool)
    .await
    .ok();

    row.0
}

/// Insert a visible forum thread owned by `author_id` for tip tests.
#[allow(dead_code)]
pub async fn create_tip_thread(pool: &PgPool, author_id: i64) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.threads (author_id, title, body, status) \
         VALUES ($1, 'Tip target', 'Visible body', 'visible') RETURNING id",
    )
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("create tip thread")
}

/// Insert a visible forum comment owned by `author_id` for tip tests.
#[allow(dead_code)]
pub async fn create_tip_comment(pool: &PgPool, author_id: i64) -> i64 {
    let thread_id = create_tip_thread(pool, author_id).await;
    sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body) \
         VALUES ($1, $2, 'Visible comment') RETURNING id",
    )
    .bind(thread_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("create tip comment")
}

/// Insert a visible course review owned by `author_id` for tip tests.
#[allow(dead_code)]
pub async fn create_tip_review(pool: &PgPool, author_id: i64) -> i64 {
    let course_id: i64 = sqlx::query_scalar(
        "INSERT INTO courses.courses (code, name) \
         VALUES ($1, 'Tip target course') RETURNING id",
    )
    .bind(format!("TIP-{author_id}-{}", uuid::Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("create tip course");
    sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
         VALUES ($1, $2, 5, 'Visible review', 'visible') RETURNING id",
    )
    .bind(course_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("create tip review")
}

/// Mint points to an account via the production system-signed mint path.
///
/// Uses the deterministic test key seed (`[0u8; 32]`) so the resulting ledger
/// entry verifies against the system public key wired into `create_test_app`.
/// `mint_points` appends the hash-chained, signed ledger entry and updates the
/// wallet balance in one transaction.
pub async fn mint_to_account(pool: &PgPool, account_id: i64, amount: i64) {
    let seed = [0u8; 32];
    credit::repo::mint_points(pool, account_id, amount, "test mint", &seed)
        .await
        .expect("mint test points");
}

/// Create a JWT access token for the given email.
pub async fn create_token(pool: &PgPool, email: &str) -> String {
    use identity::auth::create_access_token;
    let account_id: i64 = sqlx::query_scalar("SELECT id FROM identity.accounts WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("find test account");

    create_access_token(account_id, "integration-test-secret-32bytes!", 3600)
        .expect("create test access token")
}

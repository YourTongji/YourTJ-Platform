//! Shared test helpers for the credit integration test suite.

use axum::body::{to_bytes, Body};
use axum::http::Response;
use serde_json::Value;
use shared::AppState;
use sqlx::PgPool;

/// Create a complete test application with credit routes.
pub async fn create_test_app() -> (PgPool, axum::Router) {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".to_string());

    let pool = PgPool::connect(&url).await.expect("failed to connect to test database");

    run_migrations(&pool).await;

    let state = AppState {
        db: pool.clone(),
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis: None,
    };

    let router = credit::routes(state);
    (pool, router)
}

/// Run the DDL and clean test data.
async fn run_migrations(pool: &PgPool) {
    let exists: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')"
    ).fetch_one(pool).await.ok().flatten();
    if exists != Some(true) {
        // Apply migrations if not already done by docker-compose initdb.
        let sql = include_str!("../../../../migrations/0001_init.sql");
        sqlx::raw_sql(sql).execute(pool).await.expect("migration 0001 failed");

        let sql2 = include_str!("../../../../migrations/0002_escrow_selection.sql");
        sqlx::raw_sql(sql2).execute(pool).await.expect("migration 0002 failed");
    }

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM credit.purchases").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.products").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.tasks").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.ledger").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.accounts").execute(pool).await.ok();
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

/// Mint points to an account via the system-signed mint ledger entry.
pub async fn mint_to_account(pool: &PgPool, account_id: i64, amount: i64) {
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "tx_id": tx_id,
        "type": "mint",
        "from_account": null,
        "to_account": account_id.to_string(),
        "amount": amount,
        "nonce": nonce,
        "metadata": {"reason": "test mint"},
        "signer": "system",
        "timestamp": chrono::Utc::now().timestamp(),
    });
    let canonical = credit::ledger::canonicalize(&payload);
    let prev_hash: Option<String> =
        sqlx::query_scalar("SELECT hash FROM credit.ledger ORDER BY seq DESC LIMIT 1")
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
    let prev_hash = prev_hash.unwrap_or_else(|| {
        "0000000000000000000000000000000000000000000000000000000000000000".to_string()
    });
    let hash = credit::ledger::compute_hash(&canonical, &prev_hash);

    let metadata = serde_json::json!({"reason": "test mint"});

    sqlx::query(
        "INSERT INTO credit.ledger \
         (tx_id, type, from_account, to_account, amount, nonce, metadata, \
          signer, signature, prev_hash, hash) \
         VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&tx_id)
    .bind("mint")
    .bind(account_id)
    .bind(amount)
    .bind(&nonce)
    .bind(&metadata)
    .bind("system")
    .bind("system-signed")
    .bind(&prev_hash)
    .bind(&hash)
    .execute(pool)
    .await
    .expect("mint test points");

    // Update wallet balance.
    sqlx::query(
        "INSERT INTO credit.wallets (account_id, balance, last_seq) \
         VALUES ($1, $2, 1) \
         ON CONFLICT (account_id) \
         DO UPDATE SET balance = credit.wallets.balance + $2",
    )
    .bind(account_id)
    .bind(amount)
    .execute(pool)
    .await
    .ok();
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

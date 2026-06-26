//! Shared test helpers for the identity integration test suite.

use axum::body::{to_bytes, Body};
use axum::http::Response;
use serde_json::Value;
use sha2::{Digest, Sha256};
use shared::AppState;
use sqlx::PgPool;

/// Create a complete test application with a fresh DB connection.
///
/// Reads `DATABASE_URL` from the environment; falls back to a local
/// default if not set.
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
    };

    let router = identity::routes(state);
    (pool, router)
}

/// Same as `create_test_app` but reuses an existing pool.
pub async fn create_test_app_with_pool(pool: PgPool) -> axum::Router {
    let state = AppState {
        db: pool,
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
    };
    identity::routes(state)
}

/// Run the DDL from migrations to set up the test database.
async fn run_migrations(pool: &PgPool) {
    let sql = include_str!("../../../../migrations/0001_init.sql");
    sqlx::query(sql).execute(pool).await.expect("migration 0001 failed");

    let sql2 = include_str!("../../../../migrations/0002_escrow_selection.sql");
    sqlx::query(sql2).execute(pool).await.expect("migration 0002 failed");

    // Clean test data from previous runs.
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.ledger").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.accounts").execute(pool).await.ok();
}

/// Read the JSON body from a response.
pub async fn read_json(resp: Response<Body>) -> Value {
    let bytes =
        to_bytes(resp.into_body(), 10 * 1024 * 1024).await.expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("failed to parse JSON response")
}

/// Brute-force a 6-digit code that matches the given SHA-256 hex hash.
/// For testing only — iterates 000000..999999.
#[allow(dead_code)]
pub fn brute_force_code(code_hash: &str) -> String {
    for n in 0..1_000_000 {
        let candidate = format!("{n:06}");
        let h = hex::encode(Sha256::digest(candidate.as_bytes()));
        if h == code_hash {
            return candidate;
        }
    }
    panic!("could not brute-force code matching hash {code_hash}");
}

/// Insert a valid verification code for an email into the test DB.
#[allow(dead_code)]
pub async fn insert_valid_code(pool: &PgPool, email: &str, code: &str) {
    let code_hash = hex::encode(Sha256::digest(code));
    sqlx::query(
        "INSERT INTO identity.email_codes (email, code_hash, expires_at) \
         VALUES ($1, $2, now() + interval '10 minutes')",
    )
    .bind(email)
    .bind(&code_hash)
    .execute(pool)
    .await
    .expect("insert test code");
}

/// Create a JWT access token for a given email, returning (token, account_id).
#[allow(dead_code)]
pub async fn create_access_token_for(email: &str, pool: &PgPool) -> (String, i64) {
    let account_id: i64 = sqlx::query_scalar("SELECT id FROM identity.accounts WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("find test account");

    let token =
        identity::auth::create_access_token(account_id, "integration-test-secret-32bytes!", 3600)
            .expect("create test access token");
    (token, account_id)
}

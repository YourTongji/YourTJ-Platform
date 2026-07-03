//! Shared test helpers for the reviews integration test suite.

use axum::body::{to_bytes, Body};
use axum::http::Request;
use axum::http::Response;
use serde_json::Value;
use shared::AppState;
use sqlx::PgPool;

/// Create a complete test application for the reviews domain.
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
        config: shared::Config::from_env().expect("test Config::from_env"),
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis: None,
        system_private_key: vec![0u8; 32],
        system_public_key_b64: "test-public-key".into(),
    };

    let router = reviews::routes(state);
    (pool, router)
}

/// Run the DDL from migrations and clean review-related tables.
async fn run_migrations(pool: &PgPool) {
    let exists: Option<bool> = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(pool)
    .await
    .ok()
    .flatten();
    if exists != Some(true) {
        // Apply migrations if not already done by docker-compose initdb.
        let sql = include_str!("../../../../migrations/0001_init.sql");
        sqlx::raw_sql(sql).execute(pool).await.expect("migration 0001 failed");

        let sql2 = include_str!("../../../../migrations/0002_escrow_selection.sql");
        sqlx::raw_sql(sql2).execute(pool).await.expect("migration 0002 failed");
    }

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM reviews.review_reports").execute(pool).await.ok();
    sqlx::query("DELETE FROM reviews.review_likes").execute(pool).await.ok();
    sqlx::query("DELETE FROM reviews.reviews").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.ledger").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.accounts").execute(pool).await.ok();
    sqlx::query("DELETE FROM courses.course_aliases").execute(pool).await.ok();
    // Reset course stats.
    sqlx::query("UPDATE courses.courses SET review_count = 0, review_avg = 0")
        .execute(pool)
        .await
        .ok();
}

/// Read the JSON body from a response.
pub async fn read_json(resp: Response<Body>) -> Value {
    let bytes =
        to_bytes(resp.into_body(), 10 * 1024 * 1024).await.expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("failed to parse JSON response")
}

/// Seed a test account, returning (account_id, email, handle).
pub async fn seed_account(pool: &PgPool, email: &str, handle: &str) -> i64 {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(email)
    .bind(handle)
    .fetch_one(pool)
    .await
    .expect("seed account");
    row.0
}

/// Seed a test course, returning course_id.
pub async fn seed_course(pool: &PgPool, code: &str, name: &str) -> i64 {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO courses.courses (code, name) VALUES ($1, $2) \
         ON CONFLICT (code) DO UPDATE SET name = $2 RETURNING id",
    )
    .bind(code)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed course");
    row.0
}

/// Create a JWT access token for a given account_id.
pub fn create_access_token_for(account_id: i64) -> String {
    use identity::auth::create_access_token;
    create_access_token(account_id, "integration-test-secret-32bytes!", 3600)
        .expect("create test access token")
}

/// Make an authenticated JSON request.
pub fn auth_req(method: axum::http::Method, uri: &str, body: Value, token: &str) -> Request<Body> {
    use axum::http::header;
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .expect("build request")
}

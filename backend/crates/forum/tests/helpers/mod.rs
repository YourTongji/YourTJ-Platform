//! Shared test helpers for the forum integration test suite.

use axum::body::{to_bytes, Body};
use axum::http::Response;
use serde_json::Value;
use shared::AppState;
use sqlx::PgPool;

/// Create a complete test application with a fresh DB connection.
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
        system_public_key_b64: String::new(),
        sse_tx: None,
    };

    let router = forum::routes(state);
    (pool, router)
}

/// Run the DDL from migrations to set up the test database.
async fn run_migrations(pool: &PgPool) {
    let is_fresh = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if is_fresh {
        let migrations: [&str; 6] = [
            include_str!("../../../../migrations/0001_init.sql"),
            include_str!("../../../../migrations/0002_escrow_selection.sql"),
            include_str!("../../../../migrations/0003_platform.sql"),
            include_str!("../../../../migrations/0004_review_remediation.sql"),
            include_str!("../../../../migrations/0005_forum_parity.sql"),
            include_str!("../../../../migrations/0006_forum_f2_f3.sql"),
        ];
        for (i, sql) in migrations.iter().enumerate() {
            sqlx::raw_sql(sql)
                .execute(pool)
                .await
                .unwrap_or_else(|_| panic!("migration {:03} failed", i + 1));
        }
    }

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM forum.comments").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.threads").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.boards").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.accounts").execute(pool).await.ok();

    // Seed a default board.
    sqlx::query("INSERT INTO forum.boards (slug, name) VALUES ('general', 'General')")
        .execute(pool)
        .await
        .expect("seed board");
}

/// Read the JSON body from a response.
#[allow(dead_code)]
pub async fn read_json(resp: Response<Body>) -> Value {
    let bytes =
        to_bytes(resp.into_body(), 10 * 1024 * 1024).await.expect("failed to read response body");
    serde_json::from_slice(&bytes).expect("failed to parse JSON response")
}

/// Create a test account and return (account_id, access_token).
pub async fn create_test_account(pool: &PgPool, email: &str, handle: &str) -> (i64, String) {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(email)
    .bind(handle)
    .fetch_one(pool)
    .await
    .expect("insert test account");

    let token =
        identity::auth::create_access_token(row.0, "integration-test-secret-32bytes!", 3600)
            .expect("create test access token");

    (row.0, token)
}

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
        email_encryption: None,
        captcha_verifier: None,
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
        let migrations: [&str; 11] = [
            include_str!("../../../../migrations/0001_init.sql"),
            include_str!("../../../../migrations/0002_escrow_selection.sql"),
            include_str!("../../../../migrations/0003_platform.sql"),
            include_str!("../../../../migrations/0004_review_remediation.sql"),
            include_str!("../../../../migrations/0005_forum_parity.sql"),
            include_str!("../../../../migrations/0006_forum_f2_f3.sql"),
            include_str!("../../../../migrations/0020_activity.sql"),
            include_str!("../../../../migrations/0021_dm_moderation.sql"),
            include_str!("../../../../migrations/0022_governance.sql"),
            include_str!("../../../../migrations/0025_moderation_state.sql"),
            include_str!("../../../../migrations/0026_forum_flag_attempts.sql"),
        ];
        for (i, sql) in migrations.iter().enumerate() {
            sqlx::raw_sql(sql)
                .execute(pool)
                .await
                .unwrap_or_else(|_| panic!("migration {:03} failed", i + 1));
        }
    }

    let has_password_hash: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'accounts' \
           AND column_name = 'password_hash')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_password_hash {
        sqlx::raw_sql(include_str!("../../../../migrations/0011_password_auth.sql"))
            .execute(pool)
            .await
            .expect("migration 0011 failed");
    }

    let has_email_blind_index: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'email_codes' \
           AND column_name = 'email_blind_index')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_email_blind_index {
        sqlx::raw_sql(include_str!("../../../../migrations/0016_email_encryption.sql"))
            .execute(pool)
            .await
            .expect("migration 0016 failed");
    }

    let has_auth_hardening: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'email_codes' \
           AND column_name = 'purpose')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_auth_hardening {
        sqlx::raw_sql(include_str!("../../../../migrations/0033_identity_auth_hardening.sql"))
            .execute(pool)
            .await
            .expect("migration 0033 failed");
    }

    let has_activity_schema: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'activity')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_activity_schema {
        sqlx::raw_sql(include_str!("../../../../migrations/0020_activity.sql"))
            .execute(pool)
            .await
            .expect("migration 0020 failed");
    }

    let has_dm_moderation: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'dm_conversations' \
             AND column_name = 'account_low_id' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_dm_moderation {
        sqlx::raw_sql(include_str!("../../../../migrations/0021_dm_moderation.sql"))
            .execute(pool)
            .await
            .expect("migration 0021 failed");
    }

    let has_dm_participant_lifecycle: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'dm_participants' \
             AND column_name = 'muted_at' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_dm_participant_lifecycle {
        sqlx::raw_sql(include_str!("../../../../migrations/0036_dm_participant_lifecycle.sql"))
            .execute(pool)
            .await
            .expect("migration 0036 failed");
    }

    let has_governance_schema: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'governance')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_governance_schema {
        sqlx::raw_sql(include_str!("../../../../migrations/0022_governance.sql"))
            .execute(pool)
            .await
            .expect("migration 0022 failed");
    }

    let has_moderation_state: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'flags' \
             AND column_name = 'resolution_note' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_moderation_state {
        sqlx::raw_sql(include_str!("../../../../migrations/0025_moderation_state.sql"))
            .execute(pool)
            .await
            .expect("migration 0025 failed");
    }

    let has_open_flag_index: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM pg_indexes \
           WHERE schemaname = 'forum' AND tablename = 'flags' \
             AND indexname = 'flags_one_open_report_per_reporter' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_open_flag_index {
        sqlx::raw_sql(include_str!("../../../../migrations/0026_forum_flag_attempts.sql"))
            .execute(pool)
            .await
            .expect("migration 0026 failed");
    }

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM forum.comments").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.threads").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.boards").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    // TRUNCATE ... CASCADE removes accounts and every row referencing them
    // (forum.user_stats, forum.subscriptions, votes, etc.), so leftover FK
    // references never block cleanup and leak accounts into other test suites.
    sqlx::query("TRUNCATE identity.accounts CASCADE").execute(pool).await.ok();

    // Seed a default board with a deterministic id. `forum.boards.id` is
    // GENERATED ALWAYS AS IDENTITY and the sequence is not reset by DELETE, so
    // restart it to 1 before seeding. The forum tests reference `board_id = 1`,
    // which only holds if the seeded board is reliably id 1 on every run.
    sqlx::query("ALTER TABLE forum.boards ALTER COLUMN id RESTART WITH 1")
        .execute(pool)
        .await
        .expect("restart boards identity");
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

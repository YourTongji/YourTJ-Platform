//! Shared test helpers for the reviews integration test suite.

use axum::body::{to_bytes, Body};
use axum::http::Request;
use axum::http::Response;
use serde_json::Value;
use shared::AppState;
use sqlx::PgPool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

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
        redis: test_redis_pool(),
        system_private_key: vec![0u8; 32],
        system_public_key_b64: String::new(),
        email_encryption: None,
        captcha_verifier: Some(std::sync::Arc::new(shared::captcha::FakeCaptcha)),
        sse_tx: None,
    };

    let router = reviews::routes(state);
    (pool, router)
}

fn test_redis_pool() -> Option<deadpool_redis::Pool> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    deadpool_redis::Config::from_url(url).create_pool(Some(deadpool_redis::Runtime::Tokio1)).ok()
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
        MIGRATOR.run(pool).await.expect("review test migrations failed");
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

    let has_forum_parity: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'accounts' \
           AND column_name = 'trust_level')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_forum_parity {
        sqlx::raw_sql(include_str!("../../../../migrations/0005_forum_parity.sql"))
            .execute(pool)
            .await
            .expect("migration 0005 failed");
    }

    let has_selection_raw: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'selection' AND table_name = 'pk_course_details')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_selection_raw {
        sqlx::raw_sql(include_str!("../../../../migrations/0009_selection_raw_pk.sql"))
            .execute(pool)
            .await
            .expect("migration 0009 failed");
    }

    let has_review_attribution: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'reviews' AND table_name = 'reviews' \
           AND column_name = 'reviewer_name')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_review_attribution {
        sqlx::raw_sql(include_str!("../../../../migrations/0010_selection_raw_normalized.sql"))
            .execute(pool)
            .await
            .expect("migration 0010 failed");
    }

    let has_legacy_interactions: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'reviews' AND table_name = 'legacy_review_likes')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_legacy_interactions {
        sqlx::raw_sql(include_str!("../../../../migrations/0019_legacy_review_interactions.sql"))
            .execute(pool)
            .await
            .expect("migration 0019 failed");
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

    let has_report_status_constraint: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_constraint WHERE conname = 'review_reports_status_check')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_report_status_constraint {
        sqlx::raw_sql(include_str!("../../../../migrations/0023_review_moderation_decisions.sql"))
            .execute(pool)
            .await
            .expect("migration 0023 failed");
    }

    let review_course_delete_rule: Option<String> = sqlx::query_scalar(
        "SELECT delete_rule FROM information_schema.referential_constraints \
         WHERE constraint_schema = 'reviews' AND constraint_name = 'reviews_course_id_fkey'",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    if review_course_delete_rule.as_deref() == Some("CASCADE") {
        sqlx::raw_sql(include_str!("../../../../migrations/0028_review_course_restrict.sql"))
            .execute(pool)
            .await
            .expect("migration 0028 failed");
    }

    let has_open_report_index: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_indexes \
         WHERE schemaname = 'reviews' \
           AND indexname = 'review_reports_one_open_per_reporter_idx')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_open_report_index {
        sqlx::raw_sql(include_str!(
            "../../../../migrations/0029_review_report_open_uniqueness.sql"
        ))
        .execute(pool)
        .await
        .expect("migration 0029 failed");
    }

    let has_review_idempotency: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'reviews' AND table_name = 'review_create_idempotency')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_review_idempotency {
        sqlx::raw_sql(include_str!("../../../../migrations/0030_review_create_idempotency.sql"))
            .execute(pool)
            .await
            .expect("migration 0030 failed");
    }

    let database_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await
        .expect("test db name");
    assert!(database_name.ends_with("_test"), "refuse destructive cleanup outside a test database");

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM reviews.review_reports").execute(pool).await.ok();
    sqlx::query("DELETE FROM reviews.review_create_idempotency").execute(pool).await.ok();
    sqlx::query("DELETE FROM reviews.review_likes").execute(pool).await.ok();
    sqlx::query("DELETE FROM reviews.reviews").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("TRUNCATE credit.ledger RESTART IDENTITY").execute(pool).await.ok();
    retire_test_accounts(pool).await;
    sqlx::query("DELETE FROM courses.course_aliases").execute(pool).await.ok();
    // Reset course stats.
    sqlx::query("UPDATE courses.courses SET review_count = 0, review_avg = 0")
        .execute(pool)
        .await
        .ok();
}

async fn retire_test_accounts(pool: &PgPool) {
    sqlx::query(
        "UPDATE identity.accounts SET \
           status = 'deleted', \
           email = ('retired-' || id || '@test.invalid')::citext, \
           handle = ('retired-' || id)::citext, \
           email_ciphertext = NULL, email_key_version = NULL, \
           email_blind_index = NULL, password_email_blind = NULL",
    )
    .execute(pool)
    .await
    .expect("retire prior test accounts without truncating append-only governance history");
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
///
/// `courses.courses.code` is intentionally not unique (production upserts with
/// `ON CONFLICT DO NOTHING`), so an `ON CONFLICT (code)` upsert is invalid here.
/// Reuse an existing row for the code if present, otherwise insert one, to stay
/// idempotent across test runs (the cleanup does not delete courses). The
/// existing row is reused as-is — the courses-test suite seeds some of these
/// codes and asserts their names, so this helper must not mutate them.
pub async fn seed_course(pool: &PgPool, code: &str, name: &str) -> i64 {
    if let Some(row) =
        sqlx::query_as::<_, (i64,)>("SELECT id FROM courses.courses WHERE code = $1 LIMIT 1")
            .bind(code)
            .fetch_optional(pool)
            .await
            .expect("lookup course")
    {
        return row.0;
    }

    // Other suites (courses tests) insert courses with explicit ids via
    // OVERRIDING SYSTEM VALUE, which leaves the IDENTITY sequence behind the
    // real MAX(id). Resync it so the IDENTITY insert below does not collide.
    sqlx::query(
        "SELECT setval(pg_get_serial_sequence('courses.courses', 'id'), \
         GREATEST((SELECT COALESCE(MAX(id), 0) FROM courses.courses), 1))",
    )
    .execute(pool)
    .await
    .expect("resync courses id sequence");

    let row: (i64,) =
        sqlx::query_as("INSERT INTO courses.courses (code, name) VALUES ($1, $2) RETURNING id")
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
pub fn auth_req(
    method: axum::http::Method,
    uri: &str,
    mut body: Value,
    token: &str,
) -> Request<Body> {
    use axum::http::header;
    if method == axum::http::Method::POST {
        if let Some(object) = body.as_object_mut() {
            object
                .entry("captchaToken")
                .or_insert_with(|| Value::String(format!("review-test-{}", uuid::Uuid::new_v4())));
        }
    }
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .expect("build request")
}

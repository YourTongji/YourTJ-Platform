//! Shared test helpers for the identity integration test suite.

use axum::body::{to_bytes, Body};
use axum::http::Response;
use serde_json::Value;
use sha2::{Digest, Sha256};
use shared::email_crypto::EmailEncryption;
use shared::AppState;
use sqlx::PgPool;

/// Create a complete test application with a fresh DB connection.
///
/// Reads `DATABASE_URL` from the environment; falls back to a local
/// default if not set.
pub async fn create_test_app() -> (PgPool, axum::Router) {
    let test_config = shared::Config::from_env().expect("test Config::from_env");
    create_test_app_with_config(test_config).await
}

pub async fn create_test_app_with_config(test_config: shared::Config) -> (PgPool, axum::Router) {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".to_string());

    let pool = PgPool::connect(&url).await.expect("failed to connect to test database");

    run_migrations(&pool).await;

    let redis = test_redis_pool();

    let state = AppState {
        db: pool.clone(),
        config: test_config.clone(),
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis,
        system_private_key: vec![0u8; 32],
        system_public_key_b64: String::new(),
        email_encryption: None,
        captcha_verifier: Some(std::sync::Arc::new(shared::captcha::FakeCaptcha)),
        sse_tx: None,
    };

    let router = identity::routes(state);
    (pool, router)
}

#[allow(dead_code)]
pub async fn create_test_app_with_pool(pool: PgPool) -> axum::Router {
    create_test_app_with_pool_and_encryption(pool, None).await
}

#[allow(dead_code)]
pub async fn create_test_app_with_pool_and_encryption(
    pool: PgPool,
    email_encryption: Option<EmailEncryption>,
) -> axum::Router {
    let test_config = shared::Config::from_env().expect("test Config::from_env");
    let redis = test_redis_pool();
    let state = AppState {
        db: pool,
        config: test_config,
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis,
        system_private_key: vec![0u8; 32],
        system_public_key_b64: String::new(),
        email_encryption,
        captcha_verifier: Some(std::sync::Arc::new(shared::captcha::FakeCaptcha)),
        sse_tx: None,
    };
    identity::routes(state)
}

fn test_redis_pool() -> Option<deadpool_redis::Pool> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    deadpool_redis::Config::from_url(url).create_pool(Some(deadpool_redis::Runtime::Tokio1)).ok()
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

    let has_encrypted_email: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'accounts' \
           AND column_name = 'email_ciphertext')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_encrypted_email {
        sqlx::raw_sql(include_str!("../../../../migrations/0016_email_encryption.sql"))
            .execute(pool)
            .await
            .expect("migration 0016 failed");
        sqlx::raw_sql(include_str!("../../../../migrations/0018_email_encrypted_storage.sql"))
            .execute(pool)
            .await
            .expect("migration 0018 failed");
    }

    let has_governance: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'governance')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_governance {
        sqlx::raw_sql(include_str!("../../../../migrations/0022_governance.sql"))
            .execute(pool)
            .await
            .expect("migration 0022 failed");
    }

    let has_invitation_expiry: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'accounts' \
           AND column_name = 'invitation_expires_at')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_invitation_expiry {
        sqlx::raw_sql(include_str!("../../../../migrations/0024_invitation_expiry.sql"))
            .execute(pool)
            .await
            .expect("migration 0024 failed");
    }

    let has_moderation_state: bool = sqlx::query_scalar(
        "SELECT COALESCE(( \
           SELECT is_nullable = 'YES' FROM information_schema.columns \
           WHERE table_schema = 'identity' AND table_name = 'sanctions' \
             AND column_name = 'issued_by' \
         ), false)",
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

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM governance.audit_events").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.ledger").execute(pool).await.ok();
    // TRUNCATE ... CASCADE removes accounts and every row referencing them
    // (across crates), so leftover FK references never block cleanup and cause
    // cross-suite email collisions. Plain DELETE silently fails on such refs.
    sqlx::query("TRUNCATE identity.accounts CASCADE").execute(pool).await.ok();
}

/// Read the JSON body from a response.
#[allow(dead_code)] // reason: each integration-test binary compiles this shared helper independently
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
    let account_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE email = $1)")
            .bind(email)
            .fetch_one(pool)
            .await
            .expect("check test account");
    let purpose = if account_exists { "login" } else { "registration" };
    insert_valid_code_for_purpose(pool, email, code, purpose).await;
}

/// Insert a delivered code bound to an explicit security purpose.
#[allow(dead_code)]
pub async fn insert_valid_code_for_purpose(pool: &PgPool, email: &str, code: &str, purpose: &str) {
    let code_hash = hex::encode(Sha256::digest(code));
    sqlx::query(
        "INSERT INTO identity.email_codes \
         (email, purpose, request_id, code_hash, expires_at, delivery_accepted_at) \
         VALUES ($1, $2, $3, $4, now() + interval '10 minutes', now())",
    )
    .bind(email)
    .bind(purpose)
    .bind(uuid::Uuid::new_v4())
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

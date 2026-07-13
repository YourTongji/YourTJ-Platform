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
    create_test_app_with_config_and_redis(test_config, test_redis_pool()).await
}

#[allow(dead_code)] // reason: durable-worker integration tests need the same state as the HTTP router
pub async fn create_test_app_and_state_with_config(
    test_config: shared::Config,
) -> (PgPool, axum::Router, AppState) {
    let (pool, state) =
        create_test_state_with_config_and_redis(test_config, test_redis_pool()).await;
    let router = identity::routes(state.clone());
    (pool, router, state)
}

#[allow(dead_code)] // reason: concurrency tests that do not exercise CAPTCHA or limits avoid shared Redis state
pub async fn create_test_app_without_redis() -> (PgPool, axum::Router) {
    let test_config = shared::Config::from_env().expect("test Config::from_env");
    create_test_app_with_config_and_redis(test_config, None).await
}

async fn create_test_app_with_config_and_redis(
    test_config: shared::Config,
    redis: Option<deadpool_redis::Pool>,
) -> (PgPool, axum::Router) {
    let (pool, state) = create_test_state_with_config_and_redis(test_config, redis).await;
    let router = identity::routes(state);
    (pool, router)
}

async fn create_test_state_with_config_and_redis(
    test_config: shared::Config,
    redis: Option<deadpool_redis::Pool>,
) -> (PgPool, AppState) {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".to_string());

    let pool = PgPool::connect(&url).await.expect("failed to connect to test database");

    run_migrations(&pool).await;

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

    (pool, state)
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

    let has_governance_notices: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'governance' AND table_name = 'notices')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_governance_notices {
        sqlx::raw_sql(include_str!("../../../../migrations/0047_governance_appeals.sql"))
            .execute(pool)
            .await
            .expect("migration 0047 failed");
    }

    let has_social_privacy: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'identity' AND table_name = 'profile_privacy')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_social_privacy {
        sqlx::raw_sql(include_str!("../../../../migrations/0034_social_identity_privacy.sql"))
            .execute(pool)
            .await
            .expect("migration 0034 failed");
    }

    let has_activity_privacy: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'profile_privacy' \
           AND column_name = 'activity_visibility')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_activity_privacy {
        sqlx::raw_sql(include_str!("../../../../migrations/0050_activity_and_mention_privacy.sql"))
            .execute(pool)
            .await
            .expect("migration 0050 failed");
    }

    let has_recent_auth: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'sessions' \
           AND column_name = 'recent_authenticated_at')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_recent_auth {
        sqlx::raw_sql(include_str!("../../../../migrations/0048_recent_auth.sql"))
            .execute(pool)
            .await
            .expect("migration 0048 failed");
    }

    let has_composed_code_purposes: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM pg_constraint \
           WHERE conrelid = 'identity.email_codes'::regclass \
             AND conname = 'email_codes_purpose_check' \
             AND pg_get_constraintdef(oid) LIKE '%appeal%' \
             AND pg_get_constraintdef(oid) LIKE '%recent_auth%' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_composed_code_purposes {
        sqlx::raw_sql(include_str!("../../../../migrations/0052_email_code_purpose_union.sql"))
            .execute(pool)
            .await
            .expect("migration 0052 failed");
    }

    let has_account_lifecycle: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'accounts' \
           AND column_name = 'lifecycle_version')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_account_lifecycle {
        sqlx::raw_sql(include_str!(
            "../../../../migrations/0053_account_lifecycle_and_exports.sql"
        ))
        .execute(pool)
        .await
        .expect("migration 0053 failed");
    }

    let has_lifecycle_job_lease: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'account_lifecycle_jobs' \
           AND column_name = 'lease_token')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_lifecycle_job_lease {
        sqlx::raw_sql(include_str!("../../../../migrations/0058_account_lifecycle_job_leases.sql"))
            .execute(pool)
            .await
            .expect("migration 0058 failed");
    }

    let has_identity_delivery: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'identity' AND table_name = 'email_delivery_jobs')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_identity_delivery {
        sqlx::raw_sql(include_str!("../../../../migrations/0062_identity_security_delivery.sql"))
            .execute(pool)
            .await
            .expect("migration 0062 failed");
    }

    let has_profile_school: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'profiles' \
           AND column_name = 'school')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_profile_school {
        sqlx::raw_sql(include_str!("../../../../migrations/0064_identity_profile_school.sql"))
            .execute(pool)
            .await
            .expect("migration 0064 failed");
    }

    let database_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await
        .expect("test db name");
    assert!(database_name.ends_with("_test"), "refuse destructive cleanup outside a test database");

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM identity.email_delivery_jobs").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_export_download_grants").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_export_jobs").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_recovery_credentials").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_lifecycle_jobs").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.wallets").execute(pool).await.ok();
    sqlx::query("DELETE FROM credit.ledger").execute(pool).await.ok();
    retire_test_accounts(pool).await;
}

async fn retire_test_accounts(pool: &PgPool) {
    sqlx::query(
        "UPDATE identity.accounts SET \
           status = 'purged', \
           email = ('retired-' || id || '@test.invalid')::citext, \
           handle = ('retired-' || id)::citext, \
           email_ciphertext = NULL, email_key_version = NULL, \
           email_blind_index = NULL, password_hash = NULL, password_email_blind = NULL, \
           deactivated_at = NULL, deletion_requested_at = now() - interval '31 days', \
           deletion_recover_until = now() - interval '1 day', deleted_at = now() - interval '1 day', \
           purge_started_at = now(), purged_at = now(), \
           tombstone_id = COALESCE(tombstone_id, gen_random_uuid()), \
           lifecycle_version = lifecycle_version + 1, \
           credential_version = credential_version + 1, auth_version = auth_version + 1",
    )
    .execute(pool)
    .await
    .expect("retire prior test accounts without truncating append-only governance history");
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
    let credential_version: Option<i64> = if purpose == "password_reset" {
        sqlx::query_scalar("SELECT credential_version FROM identity.accounts WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
            .expect("read reset-code credential version")
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO identity.email_codes \
         (email, purpose, request_id, code_hash, expires_at, delivery_accepted_at, \
          credential_version) \
         VALUES ($1, $2, $3, $4, now() + interval '10 minutes', now(), $5)",
    )
    .bind(email)
    .bind(purpose)
    .bind(uuid::Uuid::new_v4())
    .bind(&code_hash)
    .bind(credential_version)
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

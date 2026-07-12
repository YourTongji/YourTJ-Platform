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
        let migrations: [&str; 12] = [
            include_str!("../../../../migrations/0001_init.sql"),
            include_str!("../../../../migrations/0002_escrow_selection.sql"),
            include_str!("../../../../migrations/0003_platform.sql"),
            include_str!("../../../../migrations/0004_review_remediation.sql"),
            include_str!("../../../../migrations/0005_forum_parity.sql"),
            include_str!("../../../../migrations/0006_forum_f2_f3.sql"),
            include_str!("../../../../migrations/0015_media_upload_intents.sql"),
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

    let has_social_privacy: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'forum' AND table_name = 'user_follows')",
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

    let has_content_formats: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'threads' \
             AND column_name = 'content_format' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_content_formats {
        sqlx::raw_sql(include_str!("../../../../migrations/0039_forum_content_formats.sql"))
            .execute(pool)
            .await
            .expect("migration 0039 failed");
    }

    let has_profile_media_usage: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'media' AND table_name = 'uploads' \
             AND column_name = 'usage' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_profile_media_usage {
        sqlx::raw_sql(include_str!("../../../../migrations/0040_profile_media_usage.sql"))
            .execute(pool)
            .await
            .expect("migration 0040 failed");
    }

    let has_draft_versions: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'drafts' \
             AND column_name = 'version' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_draft_versions {
        sqlx::raw_sql(include_str!("../../../../migrations/0041_forum_draft_versions.sql"))
            .execute(pool)
            .await
            .expect("migration 0041 failed");
    }

    let has_content_versions: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'threads' \
             AND column_name = 'content_version' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_content_versions {
        sqlx::raw_sql(include_str!("../../../../migrations/0043_forum_content_versions.sql"))
            .execute(pool)
            .await
            .expect("migration 0043 failed");
    }

    let has_dm_message_requests: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'forum' AND table_name = 'dm_conversations' \
             AND column_name = 'request_status' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_dm_message_requests {
        sqlx::raw_sql(include_str!("../../../../migrations/0044_dm_message_requests.sql"))
            .execute(pool)
            .await
            .expect("migration 0044 failed");
    }

    let has_pending_badge_mints: bool =
        sqlx::query_scalar("SELECT to_regclass('platform.pending_mints') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_pending_badge_mints {
        sqlx::raw_sql(include_str!("../../../../migrations/0008_badge_mint_bridge.sql"))
            .execute(pool)
            .await
            .expect("migration 0008 failed");
    }

    let has_achievement_operations: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'platform' AND table_name = 'badges' \
           AND column_name = 'icon_token')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_achievement_operations {
        sqlx::raw_sql(include_str!("../../../../migrations/0045_achievement_operations.sql"))
            .execute(pool)
            .await
            .expect("migration 0045 failed");
    }

    let has_forum_media_bindings: bool =
        sqlx::query_scalar("SELECT to_regclass('media.asset_usages') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_forum_media_bindings {
        sqlx::raw_sql(include_str!("../../../../migrations/0046_forum_media_attachments.sql"))
            .execute(pool)
            .await
            .expect("migration 0046 failed");
    }

    let has_achievement_operations: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'platform' AND table_name = 'account_badges' \
             AND column_name = 'revoked_at' \
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_achievement_operations {
        sqlx::raw_sql(include_str!("../../../../migrations/0045_achievement_operations.sql"))
            .execute(pool)
            .await
            .expect("migration 0045 failed");
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

    let has_governance_appeals: bool =
        sqlx::query_scalar("SELECT to_regclass('governance.notices') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_governance_appeals {
        sqlx::raw_sql(include_str!("../../../../migrations/0047_governance_appeals.sql"))
            .execute(pool)
            .await
            .expect("migration 0047 failed");
    }

    let has_verification_credentials: bool =
        sqlx::query_scalar("SELECT to_regclass('platform.verification_grants') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_verification_credentials {
        sqlx::raw_sql(include_str!("../../../../migrations/0037_verification_credentials.sql"))
            .execute(pool)
            .await
            .expect("migration 0037 failed");
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

    let has_platform_promotions: bool =
        sqlx::query_scalar("SELECT to_regclass('platform.promotions') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_platform_promotions {
        sqlx::raw_sql(include_str!(
            "../../../../migrations/0035_platform_announcements_promotions.sql"
        ))
        .execute(pool)
        .await
        .expect("migration 0035 failed");
    }

    let has_notification_outbox: bool =
        sqlx::query_scalar("SELECT to_regclass('platform.outbox_events') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_notification_outbox {
        sqlx::raw_sql(include_str!("../../../../migrations/0054_durable_notification_outbox.sql"))
            .execute(pool)
            .await
            .expect("migration 0054 failed");
    }

    let has_media_deletion_jobs: bool =
        sqlx::query_scalar("SELECT to_regclass('media.object_deletion_jobs') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_media_deletion_jobs {
        sqlx::raw_sql(include_str!("../../../../migrations/0056_media_moderation_integrity.sql"))
            .execute(pool)
            .await
            .expect("migration 0056 failed");
    }

    let has_media_retention_bindings: bool =
        sqlx::query_scalar("SELECT to_regclass('media.asset_bindings') IS NOT NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(false);
    if !has_media_retention_bindings {
        sqlx::raw_sql(include_str!("../../../../migrations/0057_media_bindings_retention_gc.sql"))
            .execute(pool)
            .await
            .expect("migration 0057 failed");
    }


    let has_trust_levels: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'activity' AND table_name = 'trust_level_policies')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if !has_trust_levels {
        sqlx::raw_sql(include_str!("../../../../migrations/0059_activity_trust_levels.sql"))
            .execute(pool)
            .await
            .expect("migration 0059 failed");
    }

    let database_name: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await
        .expect("test db name");
    assert!(database_name.ends_with("_test"), "refuse destructive cleanup outside a test database");

    // Clean test data from previous runs (always run, even if migrations were skipped).
    sqlx::query("DELETE FROM forum.notification_delivery_receipts").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.notifications").execute(pool).await.ok();
    sqlx::query("DELETE FROM platform.outbox_events").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.dm_message_reports").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.dm_messages").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.dm_participants").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.dm_conversations").execute(pool).await.ok();
    sqlx::query("DELETE FROM activity.events").execute(pool).await.ok();
    sqlx::query("DELETE FROM activity.daily_counts").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.post_revisions").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.comments").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.threads").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.tags").execute(pool).await.ok();
    sqlx::query("DELETE FROM forum.watched_words").execute(pool).await.ok();
    forum::watched_words::reload_watched_words(pool).await.expect("reload empty watched words");
    sqlx::query("DELETE FROM forum.boards").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.sessions").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.email_codes").execute(pool).await.ok();
    sqlx::query("DELETE FROM identity.account_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM activity.trust_level_events").execute(pool).await.ok();
    sqlx::query("DELETE FROM activity.account_trust_progress").execute(pool).await.ok();
    sqlx::query("DELETE FROM activity.account_totals").execute(pool).await.ok();
    retire_test_accounts(pool).await;

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

async fn retire_test_accounts(pool: &PgPool) {
    let has_account_lifecycle: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'identity' AND table_name = 'accounts' \
           AND column_name = 'lifecycle_version')",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    if has_account_lifecycle {
        sqlx::query(
            "UPDATE identity.accounts SET \
               status = 'purged', \
               email = ('retired-' || id || '@test.invalid')::citext, \
               handle = ('retired-' || id)::citext, \
               email_ciphertext = NULL, email_key_version = NULL, \
               email_blind_index = NULL, password_hash = NULL, password_email_blind = NULL, \
               deactivated_at = NULL, deletion_requested_at = now() - interval '31 days', \
               deletion_recover_until = now() - interval '1 day', \
               deleted_at = now() - interval '1 day', purge_started_at = now(), \
               purged_at = now(), tombstone_id = COALESCE(tombstone_id, gen_random_uuid()), \
               lifecycle_version = lifecycle_version + 1, \
               credential_version = credential_version + 1, auth_version = auth_version + 1",
        )
        .execute(pool)
        .await
        .expect("retire lifecycle-aware test accounts");
        return;
    }
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

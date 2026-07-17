//! Populated upgrade coverage for media retention migration 0057.

use std::borrow::Cow;
use std::str::FromStr;

use sha2::{Digest, Sha256};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

fn migrations_matching(predicate: impl Fn(i64) -> bool) -> Migrator {
    Migrator {
        migrations: Cow::Owned(
            MIGRATOR.iter().filter(|migration| predicate(migration.version)).cloned().collect(),
        ),
        ignore_missing: true,
        locking: true,
        no_tx: false,
    }
}

async fn seed_upload(
    pool: &PgPool,
    account_id: i64,
    suffix: &str,
    label: &str,
    status: &str,
    usage: Option<&str>,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at) \
         VALUES ($1, 'image', $2, $3, 128, 'image/png', $4, $5, $6, \
                 now() - interval '400 days') RETURNING id",
    )
    .bind(account_id)
    .bind(format!("uploads/{account_id}/image/{suffix}-{label}.png"))
    .bind(format!("https://cdn.example.test/{suffix}-{label}.png"))
    .bind("a".repeat(64))
    .bind(status)
    .bind(usage)
    .fetch_one(pool)
    .await
    .expect("seed media upgrade upload")
}

#[tokio::test]
async fn populated_0056_upgrade_backfills_triggers_and_redacts_historical_deletions() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for migration upgrade");
    let base_options = PgConnectOptions::from_str(&database_url).expect("parse migration DB URL");
    let admin_options = base_options.clone().database("postgres");
    let mut admin = PgConnection::connect_with(&admin_options)
        .await
        .expect("connect migration database administrator");
    let database_name = format!("yourtj_media_0057_{}_test", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("create isolated media upgrade database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_options.database(&database_name))
        .await
        .expect("connect isolated media upgrade database");

    migrations_matching(|version| version < 57)
        .run(&pool)
        .await
        .expect("migrate populated fixture through 0056");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("media-upgrade-owner-{suffix}@tongji.edu.cn"))
    .bind(format!("media-upgrade-owner-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert migration owner");
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("media-upgrade-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("media-upgrade-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert migration admin");
    let first_profile_id =
        seed_upload(&pool, owner_id, &suffix, "profile-a", "clean", Some("profile_avatar")).await;
    let second_profile_id =
        seed_upload(&pool, owner_id, &suffix, "profile-b", "clean", Some("profile_avatar")).await;
    let banner_id =
        seed_upload(&pool, owner_id, &suffix, "banner", "clean", Some("profile_banner")).await;
    let promotion_id = seed_upload(&pool, owner_id, &suffix, "promotion", "clean", None).await;
    let draft_id =
        seed_upload(&pool, owner_id, &suffix, "draft-a", "clean", Some("forum_thread")).await;
    let second_draft_id =
        seed_upload(&pool, owner_id, &suffix, "draft-b", "clean", Some("forum_thread")).await;
    let quarantined_draft_id = seed_upload(
        &pool,
        owner_id,
        &suffix,
        "draft-quarantined",
        "quarantined",
        Some("forum_thread"),
    )
    .await;
    let legacy_blocked_id =
        seed_upload(&pool, owner_id, &suffix, "legacy-blocked", "blocked", None).await;
    let succeeded_blocked_id =
        seed_upload(&pool, owner_id, &suffix, "job-blocked", "blocked", None).await;

    sqlx::query(
        "INSERT INTO identity.profiles (account_id, avatar_asset_id, banner_asset_id) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (account_id) DO UPDATE \
         SET avatar_asset_id = EXCLUDED.avatar_asset_id, banner_asset_id = EXCLUDED.banner_asset_id",
    )
    .bind(owner_id)
    .bind(first_profile_id)
    .bind(banner_id)
    .execute(&pool)
    .await
    .expect("seed historical profile media");
    let promotion_record_id: i64 = sqlx::query_scalar(
        "INSERT INTO platform.promotions \
         (placement, title, target_url, asset_id, status, priority, audience, created_by, updated_by) \
         VALUES ('home-left-primary', 'Upgrade promotion', '/forum', $1, 'published', 1, \
                 'all', $2, $2) RETURNING id",
    )
    .bind(promotion_id)
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("seed historical promotion media");
    sqlx::query(
        "INSERT INTO forum.drafts (account_id, draft_key, payload) VALUES \
         ($1, 'thread:valid', $2), \
         ($1, 'thread:not-array', $3), \
         ($1, 'thread:object', $4), \
         ($1, 'thread:overflow', $5)",
    )
    .bind(owner_id)
    .bind(serde_json::json!({
        "kind": "thread",
        "attachmentAssetIds": [draft_id.to_string()],
    }))
    .bind(serde_json::json!({ "kind": "thread", "attachmentAssetIds": "not-an-array" }))
    .bind(serde_json::json!({ "kind": "thread", "attachmentAssetIds": { "id": draft_id } }))
    .bind(serde_json::json!({
        "kind": "thread",
        "attachmentAssetIds": ["999999999999999999999999999999999999"],
    }))
    .execute(&pool)
    .await
    .expect("seed historical draft variants");
    sqlx::query(
        "INSERT INTO media.object_deletion_jobs \
         (upload_id, requested_by, requested_role, reason, previous_status, status, \
          attempt_count, completed_at) \
         VALUES ($1, $2, 'admin', 'historical successful deletion', 'clean', 'succeeded', 1, \
                 now() - interval '10 days')",
    )
    .bind(succeeded_blocked_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("seed historical succeeded deletion job");
    for (label, upload_id) in [("legacy", legacy_blocked_id), ("succeeded", succeeded_blocked_id)] {
        sqlx::query(
            "INSERT INTO media.upload_intents \
             (id, account_id, kind, oss_key, content_type, max_bytes, callback_token, expires_at, \
              consumed_at, upload_id) \
             VALUES ($1, $2, 'image', $3, 'image/png', 128, $4, now() - interval '20 days', \
                     now() - interval '20 days', $5)",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(owner_id)
        .bind(format!("uploads/{owner_id}/image/{suffix}-{label}-intent.png"))
        .bind(uuid::Uuid::new_v4().simple().to_string())
        .bind(upload_id)
        .execute(&pool)
        .await
        .expect("seed historical consumed intent");
    }
    let active_intent_id = uuid::Uuid::new_v4();
    let active_callback_token = uuid::Uuid::new_v4().simple().to_string();
    sqlx::query(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, max_bytes, callback_token, expires_at) \
         VALUES ($1, $2, 'image', $3, 'image/png', 128, $4, now() + interval '10 minutes')",
    )
    .bind(active_intent_id)
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-active-intent.png"))
    .bind(&active_callback_token)
    .execute(&pool)
    .await
    .expect("seed active plaintext callback intent");

    let mut old_writer = pool.begin().await.expect("begin pre-trigger profile writer");
    sqlx::query("UPDATE identity.profiles SET avatar_asset_id = $2 WHERE account_id = $1")
        .bind(owner_id)
        .bind(second_profile_id)
        .execute(&mut *old_writer)
        .await
        .expect("stage old-writer profile replacement");
    let migration_started_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&pool)
            .await
            .expect("read migration database clock");
    let migration_pool = pool.clone();
    let migration_task = tokio::spawn(async move {
        migrations_matching(|version| version == 57).run(&migration_pool).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!migration_task.is_finished(), "migration should wait for an in-flight old writer");
    old_writer.commit().await.expect("commit old writer before trigger installation");
    migration_task
        .await
        .expect("join migration 0057")
        .expect("apply migration 0057 to populated database");

    let active_profile_assets: Vec<(String, i64)> = sqlx::query_as(
        "SELECT target_type, asset_id FROM media.asset_bindings \
         WHERE target_id = $1 AND target_type IN ('profile_avatar', 'profile_banner') \
           AND detached_at IS NULL ORDER BY target_type",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("backfilled profile bindings");
    assert_eq!(
        active_profile_assets,
        vec![("profile_avatar".into(), second_profile_id), ("profile_banner".into(), banner_id)]
    );
    let active_promotion_asset: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.asset_bindings \
         WHERE target_type = 'platform_promotion' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(promotion_record_id)
    .fetch_one(&pool)
    .await
    .expect("backfilled promotion binding");
    assert_eq!(active_promotion_asset, promotion_id);
    let valid_draft_asset: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.draft_asset_references \
         WHERE account_id = $1 AND draft_key = 'thread:valid'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("backfilled valid draft reference");
    assert_eq!(valid_draft_asset, draft_id);
    let malformed_reference_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.draft_asset_references \
         WHERE account_id = $1 AND draft_key <> 'thread:valid'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("malformed draft reference count");
    assert_eq!(malformed_reference_count, 0);
    let cleaned_after_rollout: bool = sqlx::query_scalar(
        "SELECT bool_and(cleaned_at >= $1) FROM media.uploads WHERE status = 'clean'",
    )
    .bind(migration_started_at)
    .fetch_one(&pool)
    .await
    .expect("historical approval timestamps");
    assert!(cleaned_after_rollout);
    let redacted_rows: Vec<(i64, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, oss_key, url, sha256, redacted_at IS NOT NULL FROM media.uploads \
         WHERE id = ANY($1) ORDER BY id",
    )
    .bind(vec![legacy_blocked_id, succeeded_blocked_id])
    .fetch_all(&pool)
    .await
    .expect("historical blocked redaction");
    assert!(redacted_rows.iter().all(|(id, key, url, hash, redacted)| {
        key == &format!("redacted/{id}") && url.is_empty() && hash.is_empty() && *redacted
    }));
    let retained_blocked_intents: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.upload_intents WHERE upload_id = ANY($1)",
    )
    .bind(vec![legacy_blocked_id, succeeded_blocked_id])
    .fetch_one(&pool)
    .await
    .expect("historical blocked intent redaction");
    assert_eq!(retained_blocked_intents, 0);
    let migrated_callback_hash: Vec<u8> =
        sqlx::query_scalar("SELECT callback_token_hash FROM media.upload_intents WHERE id = $1")
            .bind(active_intent_id)
            .fetch_one(&pool)
            .await
            .expect("migrated callback token digest");
    let expected_callback_hash: [u8; 32] = Sha256::digest(active_callback_token.as_bytes()).into();
    assert_eq!(migrated_callback_hash, expected_callback_hash);
    let plaintext_callback_column_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM information_schema.columns \
           WHERE table_schema = 'media' AND table_name = 'upload_intents' \
             AND column_name = 'callback_token' \
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("callback token column inventory");
    assert!(!plaintext_callback_column_exists);

    sqlx::query("UPDATE platform.promotions SET asset_id = $2 WHERE id = $1")
        .bind(promotion_record_id)
        .bind(second_profile_id)
        .execute(&pool)
        .await
        .expect("legacy promotion writer uses trigger");
    let trigger_promotion_asset: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.asset_bindings \
         WHERE target_type = 'platform_promotion' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(promotion_record_id)
    .fetch_one(&pool)
    .await
    .expect("trigger-maintained promotion binding");
    assert_eq!(trigger_promotion_asset, second_profile_id);
    sqlx::query(
        "UPDATE platform.promotions SET status = 'archived', archived_at = now() WHERE id = $1",
    )
    .bind(promotion_record_id)
    .execute(&pool)
    .await
    .expect("legacy promotion archive uses trigger");
    let active_archived_binding: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM media.asset_bindings \
         WHERE target_type = 'platform_promotion' AND target_id = $1 AND detached_at IS NULL)",
    )
    .bind(promotion_record_id)
    .fetch_one(&pool)
    .await
    .expect("archived promotion binding state");
    assert!(!active_archived_binding);

    sqlx::query(
        "UPDATE forum.drafts SET payload = $3 \
         WHERE account_id = $1 AND draft_key = $2",
    )
    .bind(owner_id)
    .bind("thread:valid")
    .bind(serde_json::json!({
        "kind": "thread",
        "attachmentAssetIds": [second_draft_id.to_string()],
    }))
    .execute(&pool)
    .await
    .expect("legacy draft writer uses trigger");
    let trigger_draft_asset: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.draft_asset_references \
         WHERE account_id = $1 AND draft_key = 'thread:valid'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("trigger-maintained draft reference");
    assert_eq!(trigger_draft_asset, second_draft_id);
    let invalid_draft_update = sqlx::query(
        "UPDATE forum.drafts SET payload = $3 \
         WHERE account_id = $1 AND draft_key = $2",
    )
    .bind(owner_id)
    .bind("thread:valid")
    .bind(serde_json::json!({
        "kind": "thread",
        "attachmentAssetIds": [quarantined_draft_id.to_string()],
    }))
    .execute(&pool)
    .await;
    assert!(invalid_draft_update.is_err());

    sqlx::query("DELETE FROM identity.profiles WHERE account_id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("profile deletion uses retention trigger");
    let remaining_active_profile_bindings: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.asset_bindings \
         WHERE target_id = $1 AND target_type IN ('profile_avatar', 'profile_banner') \
           AND detached_at IS NULL",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("profile delete binding state");
    assert_eq!(remaining_active_profile_bindings, 0);
    let delayed_profile_grace: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.asset_bindings \
         WHERE target_id = $1 AND target_type IN ('profile_avatar', 'profile_banner') \
           AND gc_eligible_at > now()",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("profile delete GC grace state");
    assert_eq!(delayed_profile_grace, 0);

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE \"{database_name}\""))
        .execute(&mut admin)
        .await
        .expect("drop isolated media upgrade database");
}

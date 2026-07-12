//! Retention-aware scheduling of unreferenced media for durable provider deletion.

use std::time::Duration;

use shared::{AppResult, AppState};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::bindings::detach_account_profile_bindings;

const GC_BATCH_SIZE: i64 = 50;
const GC_IDLE_SECONDS: u64 = 60;
const ACCOUNT_PURGE_BATCH_SIZE: i64 = 50;
const UPLOAD_INTENT_CLEANUP_BUFFER_MINUTES: i64 = 10;

/// Bounded account-media cleanup progress used by the durable account lifecycle worker.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct AccountMediaPurgeProgress {
    pub scheduled: u64,
    pub has_more: bool,
    pub pending_deletions: i64,
    pub dead_letter_deletions: i64,
    pub retained_assets: i64,
    pub missing_deletion_jobs: i64,
}

#[derive(Debug, FromRow)]
struct GcCandidate {
    id: i64,
    status: String,
}

#[derive(Debug, FromRow)]
struct UploadIntentCleanupCandidate {
    id: Uuid,
    account_id: i64,
    kind: String,
    oss_key: String,
    content_type: String,
    usage: Option<String>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

async fn enqueue_upload_intent_cleanup(
    connection: &mut PgConnection,
    candidates: Vec<UploadIntentCleanupCandidate>,
    request_source: &str,
) -> AppResult<u64> {
    let mut scheduled = 0_u64;
    for candidate in candidates {
        let upload_id: Option<i64> = sqlx::query_scalar(
            "INSERT INTO media.uploads \
             (account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, created_at, \
              is_cleanup_tombstone) \
             SELECT $2, $3, $4, '', 0, $5, '', 'quarantined', $6, now(), TRUE \
             WHERE EXISTS (SELECT 1 FROM media.upload_intents intent \
                           WHERE intent.id = $1 AND intent.upload_id IS NULL) \
             RETURNING id",
        )
        .bind(candidate.id)
        .bind(candidate.account_id)
        .bind(&candidate.kind)
        .bind(&candidate.oss_key)
        .bind(&candidate.content_type)
        .bind(&candidate.usage)
        .fetch_optional(&mut *connection)
        .await?;
        let Some(upload_id) = upload_id else {
            continue;
        };
        let changed = sqlx::query(
            "UPDATE media.upload_intents \
             SET revoked_at = COALESCE(revoked_at, now()), upload_id = $2 \
             WHERE id = $1 AND upload_id IS NULL",
        )
        .bind(candidate.id)
        .bind(upload_id)
        .execute(&mut *connection)
        .await?
        .rows_affected();
        if changed != 1 {
            return Err(shared::AppError::Internal(anyhow::anyhow!(
                "upload intent cleanup lost its locked intent"
            )));
        }
        sqlx::query(
            "INSERT INTO media.object_deletion_jobs \
             (upload_id, requested_by, requested_role, request_source, reason, previous_status, \
              available_at) \
             VALUES ($1, NULL, NULL, $2, 'expired or revoked upload intent cleanup', 'pending', \
                     GREATEST($3 + ($4 * interval '1 minute'), now()))",
        )
        .bind(upload_id)
        .bind(request_source)
        .bind(candidate.expires_at)
        .bind(UPLOAD_INTENT_CLEANUP_BUFFER_MINUTES)
        .execute(&mut *connection)
        .await?;
        governance::record_system_event_tx(
            connection,
            "media.upload_intent.cleanup_queued",
            "upload_intent",
            &candidate.id.to_string(),
            "expired or revoked upload intent cleanup queued",
            Some(&serde_json::json!({
                "uploadId": upload_id.to_string(),
                "requestSource": request_source,
            })),
        )
        .await?;
        scheduled += 1;
    }
    Ok(scheduled)
}

async fn enqueue_candidates(
    connection: &mut PgConnection,
    candidates: Vec<GcCandidate>,
    request_source: &str,
    reason: &str,
    respect_active_hold: bool,
) -> AppResult<u64> {
    let mut scheduled = 0_u64;
    for candidate in candidates {
        let changed = sqlx::query(
            "UPDATE media.uploads AS upload SET status = 'quarantined' \
             WHERE upload.id = $1 AND upload.status = $2 \
               AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                               WHERE usage.asset_id = upload.id \
                                 AND usage.detached_at IS NULL) \
               AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                               WHERE binding.asset_id = upload.id \
                                 AND binding.detached_at IS NULL) \
               AND NOT EXISTS (SELECT 1 FROM media.draft_asset_references draft_reference \
                               WHERE draft_reference.asset_id = upload.id) \
               AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                               WHERE usage.asset_id = upload.id \
                                 AND usage.gc_eligible_at > now()) \
               AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                               WHERE binding.asset_id = upload.id \
                                 AND binding.gc_eligible_at > now()) \
               AND (NOT $3 OR NOT EXISTS ( \
                 SELECT 1 FROM media.asset_retention_holds hold \
                 WHERE hold.asset_id = upload.id \
                   AND hold.released_at IS NULL AND hold.expires_at > now() \
               ))",
        )
        .bind(candidate.id)
        .bind(&candidate.status)
        .bind(respect_active_hold)
        .execute(&mut *connection)
        .await?
        .rows_affected();
        if changed != 1 {
            continue;
        }
        sqlx::query(
            "INSERT INTO media.object_deletion_jobs \
             (upload_id, requested_by, requested_role, request_source, reason, previous_status) \
             VALUES ($1, NULL, NULL, $2, $3, $4)",
        )
        .bind(candidate.id)
        .bind(request_source)
        .bind(reason)
        .bind(&candidate.status)
        .execute(&mut *connection)
        .await?;
        governance::record_system_event_tx(
            connection,
            "media.upload.gc_queued",
            "upload",
            &candidate.id.to_string(),
            reason,
            Some(&serde_json::json!({
                "oldStatus": candidate.status,
                "newStatus": "quarantined",
                "requestSource": request_source,
            })),
        )
        .await?;
        scheduled += 1;
    }
    Ok(scheduled)
}

/// Quarantine a bounded batch whose live references and retention grace have ended.
pub async fn schedule_retention_gc_batch(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let limit = limit.clamp(1, 100);
    let mut transaction = pool.begin().await?;
    let candidates = sqlx::query_as::<_, GcCandidate>(
        "SELECT upload.id, upload.status \
         FROM media.uploads upload \
         WHERE upload.status = 'clean' \
           AND upload.cleaned_at <= now() - interval '30 days' \
           AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                           WHERE usage.asset_id = upload.id AND usage.detached_at IS NULL) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                           WHERE binding.asset_id = upload.id AND binding.detached_at IS NULL) \
           AND NOT EXISTS (SELECT 1 FROM media.draft_asset_references draft_reference \
                           WHERE draft_reference.asset_id = upload.id) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                           WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                             AND hold.expires_at > now()) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                           WHERE usage.asset_id = upload.id AND usage.gc_eligible_at > now()) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                           WHERE binding.asset_id = upload.id AND binding.gc_eligible_at > now()) \
         ORDER BY upload.cleaned_at, upload.id \
         FOR UPDATE OF upload SKIP LOCKED LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&mut *transaction)
    .await?;
    let scheduled = enqueue_candidates(
        &mut transaction,
        candidates,
        "retention_gc",
        "media retention grace elapsed",
        true,
    )
    .await?;
    transaction.commit().await?;
    Ok(scheduled)
}

/// Queue exact-key cleanup for expired upload intents that never completed a callback.
pub async fn schedule_expired_upload_intent_cleanup_batch(
    pool: &PgPool,
    limit: i64,
) -> AppResult<u64> {
    let limit = limit.clamp(1, 100);
    let mut transaction = pool.begin().await?;
    let candidates = sqlx::query_as::<_, UploadIntentCleanupCandidate>(
        "SELECT id, account_id, kind, oss_key, content_type, usage, expires_at \
         FROM media.upload_intents \
         WHERE upload_id IS NULL \
           AND expires_at <= now() - ($1 * interval '1 minute') \
         ORDER BY expires_at, id FOR UPDATE SKIP LOCKED LIMIT $2",
    )
    .bind(UPLOAD_INTENT_CLEANUP_BUFFER_MINUTES)
    .bind(limit)
    .fetch_all(&mut *transaction)
    .await?;
    let scheduled =
        enqueue_upload_intent_cleanup(&mut transaction, candidates, "intent_cleanup").await?;
    transaction.commit().await?;
    Ok(scheduled)
}

/// Detach account profile slots and durably queue unreferenced assets before account tombstoning.
pub async fn prepare_account_media_purge(
    pool: &PgPool,
    account_id: i64,
    system_enqueue_enabled: bool,
) -> AppResult<AccountMediaPurgeProgress> {
    let mut transaction = pool.begin().await?;
    detach_account_profile_bindings(&mut transaction, account_id).await?;
    sqlx::query("DELETE FROM media.draft_asset_references WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE media.upload_intents SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1 AND upload_id IS NULL",
    )
    .bind(account_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "DELETE FROM media.upload_intents \
         WHERE account_id = $1 AND upload_id IS NOT NULL AND revoked_at IS NULL",
    )
    .bind(account_id)
    .execute(&mut *transaction)
    .await?;
    let candidates = if system_enqueue_enabled {
        sqlx::query_as::<_, GcCandidate>(
            "SELECT upload.id, upload.status FROM media.uploads upload \
         WHERE upload.account_id = $1 AND upload.status IN ('pending', 'clean') \
           AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                           WHERE usage.asset_id = upload.id AND usage.detached_at IS NULL) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                           WHERE binding.asset_id = upload.id AND binding.detached_at IS NULL) \
           AND NOT EXISTS (SELECT 1 FROM media.draft_asset_references draft_reference \
                           WHERE draft_reference.asset_id = upload.id) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                           WHERE usage.asset_id = upload.id AND usage.gc_eligible_at > now()) \
           AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                           WHERE binding.asset_id = upload.id AND binding.gc_eligible_at > now()) \
         ORDER BY upload.id FOR UPDATE OF upload SKIP LOCKED LIMIT $2",
        )
        .bind(account_id)
        .bind(ACCOUNT_PURGE_BATCH_SIZE)
        .fetch_all(&mut *transaction)
        .await?
    } else {
        Vec::new()
    };
    let scheduled_assets = enqueue_candidates(
        &mut transaction,
        candidates,
        "account_purge",
        "account media purge after recovery window",
        false,
    )
    .await?;
    let intent_candidates = if system_enqueue_enabled {
        sqlx::query_as::<_, UploadIntentCleanupCandidate>(
            "SELECT id, account_id, kind, oss_key, content_type, usage, expires_at \
             FROM media.upload_intents \
             WHERE account_id = $1 AND upload_id IS NULL \
             ORDER BY expires_at, id FOR UPDATE SKIP LOCKED LIMIT $2",
        )
        .bind(account_id)
        .bind(ACCOUNT_PURGE_BATCH_SIZE)
        .fetch_all(&mut *transaction)
        .await?
    } else {
        Vec::new()
    };
    let scheduled_intents =
        enqueue_upload_intent_cleanup(&mut transaction, intent_candidates, "account_purge").await?;
    let scheduled = scheduled_assets.saturating_add(scheduled_intents);
    let has_more: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM media.uploads upload \
           WHERE upload.account_id = $1 AND upload.status IN ('pending', 'clean') \
             AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                             WHERE usage.asset_id = upload.id AND usage.detached_at IS NULL) \
             AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                             WHERE binding.asset_id = upload.id AND binding.detached_at IS NULL) \
             AND NOT EXISTS (SELECT 1 FROM media.draft_asset_references draft_reference \
                             WHERE draft_reference.asset_id = upload.id) \
             AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                             WHERE usage.asset_id = upload.id AND usage.gc_eligible_at > now()) \
             AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                             WHERE binding.asset_id = upload.id AND binding.gc_eligible_at > now()) \
         ) OR EXISTS( \
           SELECT 1 FROM media.upload_intents intent \
           WHERE intent.account_id = $1 AND intent.upload_id IS NULL \
         )",
    )
    .bind(account_id)
    .fetch_one(&mut *transaction)
    .await?;
    let (pending_deletions, dead_letter_deletions): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER ( \
                  WHERE job.status IN ('queued', 'leased') \
                    AND NOT EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                                    WHERE hold.asset_id = upload.id \
                                      AND hold.released_at IS NULL \
                                      AND hold.expires_at > now()) \
                )::bigint, \
                count(*) FILTER (WHERE job.status = 'dead_letter')::bigint \
         FROM media.object_deletion_jobs job \
         JOIN media.uploads upload ON upload.id = job.upload_id \
         WHERE upload.account_id = $1 AND job.status <> 'succeeded'",
    )
    .bind(account_id)
    .fetch_one(&mut *transaction)
    .await?;
    let retained_assets: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.uploads upload \
         WHERE upload.account_id = $1 AND upload.status IN ('pending', 'clean', 'quarantined') \
           AND ( \
             EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                     WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                       AND hold.expires_at > now()) \
             OR (upload.status IN ('pending', 'clean') AND ( \
               EXISTS (SELECT 1 FROM media.asset_usages usage \
                       WHERE usage.asset_id = upload.id \
                         AND (usage.detached_at IS NULL OR usage.gc_eligible_at > now())) \
               OR EXISTS (SELECT 1 FROM media.asset_bindings binding \
                          WHERE binding.asset_id = upload.id \
                            AND (binding.detached_at IS NULL OR binding.gc_eligible_at > now())) \
               OR EXISTS (SELECT 1 FROM media.draft_asset_references draft_reference \
                          WHERE draft_reference.asset_id = upload.id) \
             )) \
           )",
    )
    .bind(account_id)
    .fetch_one(&mut *transaction)
    .await?;
    let missing_deletion_jobs: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.uploads upload \
         WHERE upload.account_id = $1 AND upload.status = 'quarantined' \
           AND NOT EXISTS (SELECT 1 FROM media.object_deletion_jobs job \
                           WHERE job.upload_id = upload.id \
                             AND job.status <> 'succeeded')",
    )
    .bind(account_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(AccountMediaPurgeProgress {
        scheduled,
        has_more,
        pending_deletions,
        dead_letter_deletions,
        retained_assets,
        missing_deletion_jobs,
    })
}

/// Run the media retention scheduler for the lifetime of the API process.
pub async fn run_retention_gc_worker(state: AppState) {
    loop {
        match schedule_retention_gc_batch(&state.db, GC_BATCH_SIZE).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "media retention GC queued assets");
                continue;
            }
            Ok(_) => tokio::time::sleep(Duration::from_secs(GC_IDLE_SECONDS)).await,
            Err(error) => {
                tracing::warn!(?error, "media retention GC scheduling failed");
                tokio::time::sleep(Duration::from_secs(GC_IDLE_SECONDS)).await;
            }
        }
    }
}

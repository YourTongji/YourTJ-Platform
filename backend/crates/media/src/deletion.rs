//! Durable quarantine and external-object deletion.

use std::sync::Arc;
use std::time::Duration;

use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::dto::DeletionJobDto;
use crate::error::MediaError;
use crate::moderation::authorize_moderation;
use crate::quarantine::{AliyunUploadObjectStore, DeliveryPurgeTaskState, UploadObjectStore};

const MAX_DELETE_ATTEMPTS: i32 = 8;
const DELETE_LEASE_SECONDS: i64 = 120;
const PURGE_POLL_SECONDS: i64 = 30;
const PURGE_TASK_DEADLINE_SECONDS: i64 = 600;
const PROCESSING_IDLE_POLL_SECONDS: u64 = 30;
const CLEANUP_IDLE_POLL_SECONDS: u64 = 5;

#[derive(Debug, FromRow)]
struct ClaimedDeletionJob {
    id: i64,
    upload_id: i64,
    requested_by: Option<i64>,
    requested_role: Option<String>,
    request_source: String,
    reason: String,
    previous_status: String,
    self_review: bool,
    attempt_count: i32,
    lease_token: Uuid,
}

#[derive(Debug, FromRow)]
struct ClaimedCleanupStep {
    id: i64,
    step_kind: String,
    object_key: String,
    provider_task_id: Option<String>,
    provider_task_submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    attempt_count: i32,
    lease_token: Uuid,
}

#[derive(Debug, FromRow)]
struct OperationsDeletionJobRow {
    id: i64,
    upload_id: i64,
    account_id: i64,
    upload_status: String,
    request_source: String,
    reason: String,
    status: String,
    attempt_count: i32,
    last_error_code: Option<String>,
    available_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) async fn enqueue_variant_cleanup_steps(
    connection: &mut PgConnection,
    upload_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO media.object_cleanup_steps (deletion_job_id, step_kind, object_key) \
         SELECT job.id, step.step_kind, variant.object_key \
         FROM media.object_deletion_jobs job \
         JOIN media.asset_variants variant ON variant.asset_id = job.upload_id \
         CROSS JOIN (VALUES ('cdn_purge'::text), ('delivery_delete'::text)) step(step_kind) \
         WHERE job.upload_id = $1 AND variant.status <> 'deleted' \
         ON CONFLICT DO NOTHING",
    )
    .bind(upload_id)
    .execute(connection)
    .await?;
    Ok(())
}

/// Immediately hide an upload and durably enqueue its provider object for deletion.
pub(crate) async fn schedule_upload_deletion(
    state: &AppState,
    auth_context: &identity::auth_middleware::AuthenticatedContext,
    upload_id: i64,
    reason: &str,
    self_review_confirmed: bool,
) -> AppResult<()> {
    let mut transaction = state.db.begin().await?;
    let (owner_id, owner) = crate::locking::lock_upload_owner(&mut transaction, upload_id).await?;
    let upload: Option<(String, i64)> =
        sqlx::query_as("SELECT status, account_id FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_optional(&mut *transaction)
            .await?;
    let (current_status, locked_owner_id) = upload.ok_or(MediaError::NotFound)?;
    if locked_owner_id != owner_id {
        return Err(AppError::Internal(anyhow::anyhow!("locked media owner changed")));
    }
    let authorization =
        authorize_moderation(&auth_context.account, owner_id, &owner.role, self_review_confirmed)?;
    if authorization.is_self_review {
        identity::auth_middleware::require_recent_auth_tx(auth_context, &mut transaction).await?;
    }
    let action = match current_status.as_str() {
        "pending" | "clean" => {
            sqlx::query("UPDATE media.uploads SET status = 'quarantined' WHERE id = $1")
                .bind(upload_id)
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "INSERT INTO media.object_deletion_jobs \
                 (upload_id, requested_by, requested_role, request_source, reason, previous_status, \
                  self_review) \
                 VALUES ($1, $2, $3, 'moderation', $4, $5, $6)",
            )
            .bind(upload_id)
            .bind(auth_context.account.id)
            .bind(&auth_context.account.role)
            .bind(reason)
            .bind(&current_status)
            .bind(authorization.is_self_review)
            .execute(&mut *transaction)
            .await?;
            enqueue_variant_cleanup_steps(&mut transaction, upload_id).await?;
            "media.upload.quarantined"
        }
        "quarantined" => {
            let deletion: Option<(String, String)> = sqlx::query_as(
                "SELECT status, request_source FROM media.object_deletion_jobs \
                 WHERE upload_id = $1 FOR UPDATE",
            )
            .bind(upload_id)
            .fetch_optional(&mut *transaction)
            .await?;
            let (deletion_status, request_source) = deletion.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "quarantined upload has no durable deletion job"
                ))
            })?;
            if request_source != "moderation" {
                return Err(AppError::Conflict(
                    "system deletion jobs require the operations workflow".into(),
                ));
            }
            let affected = sqlx::query(
                "UPDATE media.object_deletion_jobs \
                 SET status = 'queued', requested_by = $2, requested_role = $3, \
                     request_source = 'moderation', reason = $4, self_review = $5, \
                     attempt_count = 0, available_at = now(), lease_token = NULL, \
                     lease_expires_at = NULL, last_error_code = NULL, updated_at = now() \
                 WHERE upload_id = $1 AND status = 'dead_letter' \
                   AND request_source = 'moderation'",
            )
            .bind(upload_id)
            .bind(auth_context.account.id)
            .bind(&auth_context.account.role)
            .bind(reason)
            .bind(authorization.is_self_review)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
            if affected == 0 {
                if !matches!(deletion_status.as_str(), "queued" | "leased") {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "quarantined upload has no active deletion job"
                    )));
                }
                transaction.commit().await?;
                return Ok(());
            }
            sqlx::query(
                "UPDATE media.object_cleanup_steps step \
                 SET status = 'queued', attempt_count = 0, available_at = now(), \
                     lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
                     provider_task_id = NULL, provider_task_submitted_at = NULL, \
                     completed_at = NULL, updated_at = now() \
                 FROM media.object_deletion_jobs job \
                 WHERE job.upload_id = $1 AND step.deletion_job_id = job.id \
                   AND step.status = 'dead_letter'",
            )
            .bind(upload_id)
            .execute(&mut *transaction)
            .await?;
            "media.upload.deletion_requeued"
        }
        "blocked" => return Err(AppError::Conflict("upload is already blocked".into())),
        _ => {
            return Err(AppError::Conflict(format!(
                "upload cannot be quarantined from {current_status}"
            )))
        }
    };

    let metadata = serde_json::json!({
        "oldStatus": current_status,
        "newStatus": "quarantined",
        "deletion": "queued",
        "requestSource": "moderation",
        "selfReview": authorization.is_self_review,
    });
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        action,
        "upload",
        &upload_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn claim_deletion_job(
    pool: &PgPool,
    upload_id: Option<i64>,
) -> AppResult<Option<ClaimedDeletionJob>> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "UPDATE media.variant_processing_jobs processing \
         SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'asset_left_clean_state', updated_at = now() \
         FROM media.uploads upload \
         WHERE processing.asset_id = upload.id AND upload.status IN ('quarantined', 'blocked') \
           AND processing.status = 'leased' AND processing.lease_expires_at <= now()",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE media.object_cleanup_steps \
         SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'lease_expired_after_max_attempts', updated_at = now() \
         WHERE status = 'leased' AND lease_expires_at <= now() AND attempt_count >= $1",
    )
    .bind(MAX_DELETE_ATTEMPTS)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE media.object_deletion_jobs job \
         SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'cleanup_step_lease_expired_after_max_attempts', \
             updated_at = now() \
         WHERE job.status IN ('queued', 'leased') AND EXISTS ( \
           SELECT 1 FROM media.object_cleanup_steps step \
           WHERE step.deletion_job_id = job.id AND step.status = 'dead_letter' \
         )",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'lease_expired_after_max_attempts', updated_at = now() \
         WHERE status = 'leased' AND lease_expires_at <= now() AND attempt_count >= $1",
    )
    .bind(MAX_DELETE_ATTEMPTS)
    .execute(&mut *transaction)
    .await?;

    let candidate_upload_id: Option<i64> = sqlx::query_scalar(
        "SELECT upload.id FROM media.object_deletion_jobs job \
         JOIN media.uploads upload ON upload.id = job.upload_id \
         WHERE upload.status IN ('quarantined', 'blocked') AND job.attempt_count < $1 \
           AND NOT EXISTS ( \
             SELECT 1 FROM media.variant_processing_jobs processing \
             WHERE processing.asset_id = upload.id AND processing.status = 'leased' \
           ) \
           AND ($2::bigint IS NULL OR job.upload_id = $2) \
           AND ((job.status = 'queued' AND job.available_at <= now()) \
                OR (job.status = 'leased' AND job.lease_expires_at <= now())) \
           AND (EXISTS ( \
             SELECT 1 FROM media.object_cleanup_steps step \
             WHERE step.deletion_job_id = job.id AND step.attempt_count < $1 \
               AND ((step.status = 'queued' AND step.available_at <= now()) \
                    OR (step.status = 'leased' AND step.lease_expires_at <= now())) \
               AND (step.step_kind <> 'ingest_delete' OR NOT EXISTS ( \
                 SELECT 1 FROM media.asset_retention_holds hold \
                 WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                   AND hold.expires_at > now() \
               )) \
               AND ( \
                 step.step_kind = 'cdn_purge' \
                 OR (step.step_kind = 'delivery_delete' AND NOT EXISTS ( \
                   SELECT 1 FROM media.object_cleanup_steps predecessor \
                   WHERE predecessor.deletion_job_id = job.id \
                     AND predecessor.step_kind = 'cdn_purge' \
                     AND predecessor.object_key = step.object_key \
                     AND predecessor.status <> 'succeeded' \
                 )) \
                 OR (step.step_kind = 'ingest_delete' AND NOT EXISTS ( \
                   SELECT 1 FROM media.object_cleanup_steps predecessor \
                   WHERE predecessor.deletion_job_id = job.id \
                     AND predecessor.step_kind <> 'ingest_delete' \
                     AND predecessor.status <> 'succeeded' \
                 )) \
               ) \
           ) OR NOT EXISTS ( \
             SELECT 1 FROM media.object_cleanup_steps step \
             WHERE step.deletion_job_id = job.id AND step.status <> 'succeeded' \
           )) \
         ORDER BY job.available_at, job.id \
         FOR UPDATE OF upload SKIP LOCKED LIMIT 1",
    )
    .bind(MAX_DELETE_ATTEMPTS)
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(candidate_upload_id) = candidate_upload_id else {
        transaction.commit().await?;
        return Ok(None);
    };
    let job_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM media.object_deletion_jobs \
         WHERE upload_id = $1 AND attempt_count < $2 \
           AND ((status = 'queued' AND available_at <= now()) \
                OR (status = 'leased' AND lease_expires_at <= now())) \
         FOR UPDATE SKIP LOCKED",
    )
    .bind(candidate_upload_id)
    .bind(MAX_DELETE_ATTEMPTS)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(job_id) = job_id else {
        transaction.commit().await?;
        return Ok(None);
    };
    let lease_token = Uuid::new_v4();
    let job = sqlx::query_as::<_, ClaimedDeletionJob>(
        "UPDATE media.object_deletion_jobs job \
         SET status = 'leased', lease_token = $2, \
             lease_expires_at = now() + ($3 * interval '1 second'), updated_at = now() \
         FROM media.uploads upload \
         WHERE job.id = $1 AND upload.id = job.upload_id \
           AND upload.status IN ('quarantined', 'blocked') \
         RETURNING job.id, job.upload_id, job.requested_by, \
                   job.requested_role, job.request_source, job.reason, job.previous_status, \
                   job.self_review, \
                   job.attempt_count, job.lease_token",
    )
    .bind(job_id)
    .bind(lease_token)
    .bind(DELETE_LEASE_SECONDS)
    .fetch_optional(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(job)
}

async fn claim_cleanup_step(
    pool: &PgPool,
    job: &ClaimedDeletionJob,
) -> AppResult<Option<ClaimedCleanupStep>> {
    let mut transaction = pool.begin().await?;
    let upload_is_held: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM media.asset_retention_holds hold \
         WHERE hold.asset_id = $1 AND hold.released_at IS NULL AND hold.expires_at > now())",
    )
    .bind(job.upload_id)
    .fetch_one(&mut *transaction)
    .await?;
    let processing_is_active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM media.variant_processing_jobs \
         WHERE asset_id = $1 AND status = 'leased')",
    )
    .bind(job.upload_id)
    .fetch_one(&mut *transaction)
    .await?;
    if processing_is_active {
        transaction.commit().await?;
        return Ok(None);
    }
    let step_id: Option<i64> = sqlx::query_scalar(
        "SELECT step.id FROM media.object_cleanup_steps step \
         WHERE step.deletion_job_id = $1 AND step.attempt_count < $2 \
           AND ((step.status = 'queued' AND step.available_at <= now()) \
                OR (step.status = 'leased' AND step.lease_expires_at <= now())) \
           AND (step.step_kind <> 'ingest_delete' OR NOT $3) \
           AND ( \
             step.step_kind = 'cdn_purge' \
             OR (step.step_kind = 'delivery_delete' AND NOT EXISTS ( \
               SELECT 1 FROM media.object_cleanup_steps predecessor \
               WHERE predecessor.deletion_job_id = step.deletion_job_id \
                 AND predecessor.step_kind = 'cdn_purge' \
                 AND predecessor.object_key = step.object_key \
                 AND predecessor.status <> 'succeeded' \
             )) \
             OR (step.step_kind = 'ingest_delete' AND NOT EXISTS ( \
               SELECT 1 FROM media.object_cleanup_steps predecessor \
               WHERE predecessor.deletion_job_id = step.deletion_job_id \
                 AND predecessor.step_kind <> 'ingest_delete' \
                 AND predecessor.status <> 'succeeded' \
             )) \
           ) \
         ORDER BY CASE step.step_kind \
                    WHEN 'cdn_purge' THEN 1 WHEN 'delivery_delete' THEN 2 ELSE 3 END, step.id \
         FOR UPDATE SKIP LOCKED LIMIT 1",
    )
    .bind(job.id)
    .bind(MAX_DELETE_ATTEMPTS)
    .bind(upload_is_held)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(step_id) = step_id else {
        transaction.commit().await?;
        return Ok(None);
    };
    let lease_token = Uuid::new_v4();
    let step = sqlx::query_as::<_, ClaimedCleanupStep>(
        "UPDATE media.object_cleanup_steps \
         SET status = 'leased', attempt_count = attempt_count + 1, lease_token = $2, \
             lease_expires_at = now() + ($3 * interval '1 second'), updated_at = now() \
         WHERE id = $1 \
         RETURNING id, step_kind, object_key, provider_task_id, provider_task_submitted_at, \
                   attempt_count, lease_token",
    )
    .bind(step_id)
    .bind(lease_token)
    .bind(DELETE_LEASE_SECONDS)
    .fetch_optional(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(step)
}

async fn release_deletion_job(pool: &PgPool, job: &ClaimedDeletionJob) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = 'queued', available_at = now(), lease_token = NULL, \
             lease_expires_at = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("media deletion lease was lost".into()));
    }
    Ok(())
}

async fn defer_purge_step(
    pool: &PgPool,
    job: &ClaimedDeletionJob,
    step: &ClaimedCleanupStep,
    submitted_task_id: Option<&str>,
) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    let step_affected = if let Some(provider_task_id) = submitted_task_id {
        sqlx::query(
            "UPDATE media.object_cleanup_steps \
             SET status = 'queued', attempt_count = GREATEST(attempt_count - 1, 0), \
                 available_at = now() + ($3 * interval '1 second'), \
                 lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
                 provider_task_id = $4, provider_task_submitted_at = now(), updated_at = now() \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2",
        )
        .bind(step.id)
        .bind(step.lease_token)
        .bind(PURGE_POLL_SECONDS)
        .bind(provider_task_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
    } else {
        sqlx::query(
            "UPDATE media.object_cleanup_steps \
             SET status = 'queued', attempt_count = GREATEST(attempt_count - 1, 0), \
                 available_at = now() + ($3 * interval '1 second'), \
                 lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
                 updated_at = now() \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2 \
               AND provider_task_id IS NOT NULL",
        )
        .bind(step.id)
        .bind(step.lease_token)
        .bind(PURGE_POLL_SECONDS)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
    };
    if step_affected != 1 {
        return Err(AppError::Conflict("media cleanup-step lease was lost".into()));
    }
    let job_affected = sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = 'queued', available_at = now() + ($3 * interval '1 second'), \
             lease_token = NULL, lease_expires_at = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(PURGE_POLL_SECONDS)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if job_affected != 1 {
        return Err(AppError::Conflict("media deletion lease was lost".into()));
    }
    transaction.commit().await?;
    Ok(())
}

async fn complete_cleanup_step(
    pool: &PgPool,
    job: &ClaimedDeletionJob,
    step: &ClaimedCleanupStep,
) -> AppResult<bool> {
    let mut transaction = pool.begin().await?;
    let affected = sqlx::query(
        "UPDATE media.object_cleanup_steps \
         SET status = 'succeeded', lease_token = NULL, lease_expires_at = NULL, \
             completed_at = now(), last_error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(step.id)
    .bind(step.lease_token)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("media cleanup-step lease was lost".into()));
    }
    if step.step_kind == "delivery_delete" {
        sqlx::query(
            "UPDATE media.asset_variants SET status = 'deleted', deleted_at = now() \
             WHERE asset_id = $1 AND object_key = $2 AND status <> 'deleted'",
        )
        .bind(job.upload_id)
        .bind(&step.object_key)
        .execute(&mut *transaction)
        .await?;
    }
    let all_succeeded: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM media.object_cleanup_steps \
         WHERE deletion_job_id = $1 AND status <> 'succeeded')",
    )
    .bind(job.id)
    .fetch_one(&mut *transaction)
    .await?;
    if !all_succeeded {
        let job_affected = sqlx::query(
            "UPDATE media.object_deletion_jobs \
             SET status = 'queued', available_at = now(), lease_token = NULL, \
                 lease_expires_at = NULL, updated_at = now() \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if job_affected != 1 {
            return Err(AppError::Conflict("media deletion lease was lost".into()));
        }
    }
    transaction.commit().await?;
    Ok(all_succeeded)
}

async fn record_cleanup_failure(
    pool: &PgPool,
    job: &ClaimedDeletionJob,
    step: &ClaimedCleanupStep,
    error_code: &str,
    clear_provider_task: bool,
) -> AppResult<()> {
    let is_dead_letter = step.attempt_count >= MAX_DELETE_ATTEMPTS
        || job.attempt_count.saturating_add(1) >= MAX_DELETE_ATTEMPTS;
    let retry_seconds =
        i64::from(15 * 2_i32.pow(step.attempt_count.clamp(1, 8) as u32 - 1)).min(3_600);
    let mut transaction = pool.begin().await?;
    let step_affected = sqlx::query(
        "UPDATE media.object_cleanup_steps \
         SET status = CASE WHEN $3 THEN 'dead_letter' ELSE 'queued' END, \
             available_at = CASE WHEN $3 THEN available_at \
                                 ELSE now() + ($4 * interval '1 second') END, \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = $5, \
             provider_task_id = CASE WHEN $6 THEN NULL ELSE provider_task_id END, \
             provider_task_submitted_at = CASE WHEN $6 THEN NULL \
                                               ELSE provider_task_submitted_at END, \
             updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(step.id)
    .bind(step.lease_token)
    .bind(is_dead_letter)
    .bind(retry_seconds)
    .bind(error_code)
    .bind(clear_provider_task)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if step_affected != 1 {
        return Err(AppError::Conflict("media cleanup-step lease was lost".into()));
    }
    let job_affected = sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = CASE WHEN $3 THEN 'dead_letter' ELSE 'queued' END, \
             attempt_count = LEAST(attempt_count + 1, $4), \
             available_at = CASE WHEN $3 THEN available_at \
                                 ELSE now() + ($5 * interval '1 second') END, \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = $6, \
             updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(is_dead_letter)
    .bind(MAX_DELETE_ATTEMPTS)
    .bind(retry_seconds)
    .bind(error_code)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if job_affected != 1 {
        return Err(AppError::Conflict("media deletion lease was lost".into()));
    }
    transaction.commit().await?;
    Ok(())
}

async fn complete_deletion(pool: &PgPool, job: &ClaimedDeletionJob) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    let upload_state: Option<(String, bool)> = sqlx::query_as(
        "SELECT status, is_cleanup_tombstone FROM media.uploads WHERE id = $1 FOR UPDATE",
    )
    .bind(job.upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let (upload_status, is_cleanup_tombstone) = upload_state.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("provider-deleted media upload is missing"))
    })?;
    if !matches!(upload_status.as_str(), "quarantined" | "blocked") {
        return Err(AppError::Internal(anyhow::anyhow!(
            "deleted media object is not represented by a quarantined upload"
        )));
    }
    let affected = sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = 'succeeded', lease_token = NULL, lease_expires_at = NULL, \
             completed_at = now(), last_error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("media deletion lease was lost".into()));
    }
    if upload_status == "quarantined" {
        let upload_affected = sqlx::query(
            "UPDATE media.uploads \
             SET status = 'blocked', oss_key = 'redacted/' || id, url = '', bytes = 0, \
                 mime = 'application/octet-stream', sha256 = '', usage = NULL, \
                 image_width = NULL, image_height = NULL, cleaned_at = NULL, redacted_at = now() \
             WHERE id = $1 AND status = 'quarantined'",
        )
        .bind(job.upload_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if upload_affected != 1 {
            return Err(AppError::Internal(anyhow::anyhow!(
                "deleted media object is not represented by a quarantined upload"
            )));
        }
        sqlx::query("DELETE FROM media.upload_intents WHERE upload_id = $1")
            .bind(job.upload_id)
            .execute(&mut *transaction)
            .await?;
    }
    let cleanup_step_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.object_cleanup_steps WHERE deletion_job_id = $1",
    )
    .bind(job.id)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM media.object_cleanup_steps WHERE deletion_job_id = $1")
        .bind(job.id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM media.asset_variants WHERE asset_id = $1")
        .bind(job.upload_id)
        .execute(&mut *transaction)
        .await?;
    let metadata = serde_json::json!({
        "oldStatus": upload_status,
        "newStatus": "blocked",
        "previousPublishedStatus": job.previous_status,
        "deletionAttempts": job.attempt_count,
        "requestSource": job.request_source,
        "selfReview": job.self_review,
        "cleanupStepCount": cleanup_step_count,
    });
    let account_action = if upload_status == "blocked" {
        "media.upload.late_delivery_recleaned"
    } else {
        "media.upload.blocked"
    };
    match (job.requested_by, job.requested_role.as_deref()) {
        (Some(account_id), Some(role)) if job.request_source == "moderation" => {
            governance::record_account_event_tx(
                &mut transaction,
                governance::AccountActor { account_id, role },
                account_action,
                "upload",
                &job.upload_id.to_string(),
                &job.reason,
                Some(&metadata),
            )
            .await?;
        }
        (None, None) if upload_status == "blocked" && job.request_source != "moderation" => {
            governance::record_system_event_tx(
                &mut transaction,
                "media.upload.late_delivery_recleaned",
                "upload",
                &job.upload_id.to_string(),
                "late Delivery objects were purged and deleted after terminal media cleanup",
                Some(&metadata),
            )
            .await?;
        }
        (None, None) if is_cleanup_tombstone && job.request_source != "moderation" => {
            governance::record_system_event_tx(
                &mut transaction,
                "media.upload_intent.object_deleted",
                "upload",
                &job.upload_id.to_string(),
                "expired or revoked upload-intent object deletion completed",
                Some(&metadata),
            )
            .await?;
        }
        (None, None) if job.request_source != "moderation" => {
            governance::record_system_event_tx(
                &mut transaction,
                "media.upload.garbage_collected",
                "upload",
                &job.upload_id.to_string(),
                "retention-authorized media object deletion completed",
                Some(&metadata),
            )
            .await?;
        }
        _ => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "media deletion job actor shape is invalid"
            )))
        }
    }
    transaction.commit().await?;
    Ok(())
}

/// List system-owned deletion jobs independently from moderation target hierarchy.
pub async fn list_system_deletion_jobs(
    connection: &mut PgConnection,
    status: &str,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<DeletionJobDto>> {
    if !matches!(status, "queued" | "leased" | "succeeded" | "dead_letter") {
        return Err(AppError::BadRequest("invalid media deletion job status".into()));
    }
    let rows = sqlx::query_as::<_, OperationsDeletionJobRow>(
        "SELECT job.id, job.upload_id, upload.account_id, upload.status AS upload_status, \
                job.request_source, job.reason, job.status, job.attempt_count, \
                job.last_error_code, job.available_at, job.created_at, job.updated_at \
         FROM media.object_deletion_jobs job \
         JOIN media.uploads upload ON upload.id = job.upload_id \
         WHERE job.request_source <> 'moderation' AND job.status = $1 \
           AND ($2::bigint IS NULL OR job.id < $2) \
         ORDER BY job.id DESC LIMIT $3",
    )
    .bind(status)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(connection)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let visible = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = if has_more { visible.last().map(|row| row.id.to_string()) } else { None };
    let items = visible
        .into_iter()
        .map(|row| DeletionJobDto {
            id: row.id.to_string(),
            upload_id: row.upload_id.to_string(),
            account_id: row.account_id.to_string(),
            upload_status: row.upload_status,
            request_source: row.request_source,
            reason: row.reason,
            status: row.status,
            attempt_count: row.attempt_count,
            last_error_code: row.last_error_code,
            available_at: row.available_at.timestamp(),
            created_at: row.created_at.timestamp(),
            updated_at: row.updated_at.timestamp(),
        })
        .collect();
    Ok(Page::new(items, next_cursor))
}

/// Requeue one exhausted system-owned deletion job under a fresh operations authorization.
pub async fn retry_system_deletion_job(
    pool: &PgPool,
    auth_context: &identity::auth_middleware::AuthenticatedContext,
    job_id: i64,
    reason: &str,
) -> AppResult<()> {
    let upload_id: Option<i64> =
        sqlx::query_scalar("SELECT upload_id FROM media.object_deletion_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_optional(pool)
            .await?;
    let upload_id = upload_id.ok_or(AppError::NotFound)?;
    let mut transaction = pool.begin().await?;
    identity::auth_middleware::require_recent_auth_tx(auth_context, &mut transaction).await?;
    let upload_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(upload_id)
            .fetch_optional(&mut *transaction)
            .await?;
    if upload_status.as_deref() != Some("quarantined") {
        return Err(AppError::Conflict("only quarantined media can be requeued".into()));
    }
    let request_source: Option<String> = sqlx::query_scalar(
        "SELECT request_source FROM media.object_deletion_jobs \
         WHERE id = $1 AND upload_id = $2 AND status = 'dead_letter' FOR UPDATE",
    )
    .bind(job_id)
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let request_source = request_source
        .ok_or_else(|| AppError::Conflict("media deletion job is not dead-lettered".into()))?;
    if request_source == "moderation" {
        return Err(AppError::Conflict("moderation jobs use the moderation queue".into()));
    }
    sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = 'queued', attempt_count = 0, available_at = now(), \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
             updated_at = now() WHERE id = $1",
    )
    .bind(job_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE media.object_cleanup_steps \
         SET status = 'queued', attempt_count = 0, available_at = now(), \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
             provider_task_id = NULL, provider_task_submitted_at = NULL, \
             completed_at = NULL, updated_at = now() \
         WHERE deletion_job_id = $1 AND status = 'dead_letter'",
    )
    .bind(job_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO media.object_deletion_job_retry_events (job_id, actor_id, reason) \
         VALUES ($1, $2, $3)",
    )
    .bind(job_id)
    .bind(auth_context.account.id)
    .bind(reason)
    .execute(&mut *transaction)
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        "media.deletion_job.requeued",
        "media_deletion_job",
        &job_id.to_string(),
        "authorized media system deletion job requeued",
        Some(&serde_json::json!({
            "uploadId": upload_id.to_string(),
            "requestSource": request_source,
        })),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn process_deletion_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
    upload_id: Option<i64>,
) -> AppResult<bool> {
    let Some(job) = claim_deletion_job(pool, upload_id).await? else {
        return Ok(false);
    };
    let Some(step) = claim_cleanup_step(pool, &job).await? else {
        let all_succeeded: bool = sqlx::query_scalar(
            "SELECT NOT EXISTS(SELECT 1 FROM media.object_cleanup_steps \
             WHERE deletion_job_id = $1 AND status <> 'succeeded')",
        )
        .bind(job.id)
        .fetch_one(pool)
        .await?;
        if all_succeeded {
            complete_deletion(pool, &job).await?;
            return Ok(true);
        }
        release_deletion_job(pool, &job).await?;
        return Ok(false);
    };
    if step.step_kind == "cdn_purge" {
        if let Some(provider_task_id) = step.provider_task_id.as_deref() {
            match object_store.delivery_purge_task_state(provider_task_id).await {
                Ok(DeliveryPurgeTaskState::Complete) => {
                    if complete_cleanup_step(pool, &job, &step).await? {
                        complete_deletion(pool, &job).await?;
                    }
                }
                Ok(DeliveryPurgeTaskState::Refreshing) => {
                    let task_timed_out = match step.provider_task_submitted_at {
                        Some(submitted_at) => {
                            chrono::Utc::now().signed_duration_since(submitted_at).num_seconds()
                                >= PURGE_TASK_DEADLINE_SECONDS
                        }
                        None => true,
                    };
                    if task_timed_out {
                        tracing::warn!(
                            upload_id = job.upload_id,
                            attempt = step.attempt_count,
                            "CDN purge task exceeded its completion deadline"
                        );
                        record_cleanup_failure(pool, &job, &step, "cdn_purge_task_timed_out", true)
                            .await?;
                    } else {
                        defer_purge_step(pool, &job, &step, None).await?;
                    }
                }
                Ok(DeliveryPurgeTaskState::Failed) => {
                    tracing::warn!(
                        upload_id = job.upload_id,
                        attempt = step.attempt_count,
                        "CDN purge task reached a terminal failure"
                    );
                    record_cleanup_failure(pool, &job, &step, "cdn_purge_task_failed", true)
                        .await?;
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        upload_id = job.upload_id,
                        attempt = step.attempt_count,
                        "CDN purge status check failed"
                    );
                    record_cleanup_failure(pool, &job, &step, "cdn_purge_status_failed", false)
                        .await?;
                }
            }
        } else {
            match object_store.submit_delivery_purge(&step.object_key).await {
                Ok(provider_task_id) => {
                    defer_purge_step(pool, &job, &step, Some(&provider_task_id)).await?;
                }
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        upload_id = job.upload_id,
                        attempt = step.attempt_count,
                        "CDN purge submission failed"
                    );
                    record_cleanup_failure(pool, &job, &step, "cdn_purge_submit_failed", false)
                        .await?;
                }
            }
        }
        return Ok(true);
    }
    let provider_result = match step.step_kind.as_str() {
        "delivery_delete" => object_store.delete_delivery_object(&step.object_key).await,
        "ingest_delete" => object_store.delete_object(&step.object_key).await,
        _ => Err(AppError::Internal(anyhow::anyhow!("invalid media cleanup step"))),
    };
    match provider_result {
        Ok(()) => {
            if complete_cleanup_step(pool, &job, &step).await? {
                complete_deletion(pool, &job).await?;
            }
        }
        Err(error) => {
            tracing::warn!(
                ?error,
                upload_id = job.upload_id,
                cleanup_step = step.step_kind,
                attempt = step.attempt_count,
                "media cleanup step failed and remains quarantined"
            );
            let error_code = match step.step_kind.as_str() {
                "delivery_delete" => "delivery_delete_failed",
                "ingest_delete" => "ingest_delete_failed",
                _ => "cleanup_step_failed",
            };
            record_cleanup_failure(pool, &job, &step, error_code, false).await?;
        }
    }
    Ok(true)
}

/// Process at most one durable deletion job without holding a database lock across OSS I/O.
pub async fn process_one_deletion_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
) -> AppResult<bool> {
    process_deletion_job(pool, object_store, None).await
}

/// Process one queued deletion for a known upload, used by bounded operational retries.
pub async fn process_upload_deletion_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
    upload_id: i64,
) -> AppResult<bool> {
    process_deletion_job(pool, object_store, Some(upload_id)).await
}

async fn run_variant_loop(pool: PgPool, object_store: Arc<dyn UploadObjectStore>) {
    loop {
        let processed =
            match crate::processing::process_one_variant_job(&pool, object_store.as_ref()).await {
                Ok(processed) => processed,
                Err(error) => {
                    tracing::warn!(?error, "media variant worker iteration failed");
                    false
                }
            };
        if !processed {
            tokio::time::sleep(Duration::from_secs(PROCESSING_IDLE_POLL_SECONDS)).await;
        }
    }
}

async fn run_cleanup_loop(pool: PgPool, object_store: Arc<dyn UploadObjectStore>) {
    loop {
        let processed = match process_one_deletion_job(&pool, object_store.as_ref()).await {
            Ok(processed) => processed,
            Err(error) => {
                tracing::warn!(?error, "media deletion worker iteration failed");
                false
            }
        };
        if !processed {
            tokio::time::sleep(Duration::from_secs(CLEANUP_IDLE_POLL_SECONDS)).await;
        }
    }
}

/// Run independently supervised processing and cleanup loops for the API process lifetime.
pub async fn run_deletion_worker(state: AppState) {
    let object_store: Arc<dyn UploadObjectStore> =
        Arc::new(AliyunUploadObjectStore::from_config(&state.config));
    tokio::join!(
        run_variant_loop(state.db.clone(), object_store.clone()),
        run_cleanup_loop(state.db, object_store),
    );
}

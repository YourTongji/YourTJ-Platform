//! Durable quarantine and external-object deletion.

use std::sync::Arc;
use std::time::Duration;

use shared::{AppError, AppResult, AppState, AuthAccount};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::MediaError;
use crate::moderation::require_strictly_lower_owner;
use crate::quarantine::{AliyunUploadObjectStore, UploadObjectStore};

const MAX_DELETE_ATTEMPTS: i32 = 8;
const DELETE_LEASE_SECONDS: i64 = 120;
const IDLE_POLL_SECONDS: u64 = 30;

#[derive(Debug, FromRow)]
struct ClaimedDeletionJob {
    id: i64,
    upload_id: i64,
    oss_key: String,
    requested_by: i64,
    requested_role: String,
    reason: String,
    previous_status: String,
    attempt_count: i32,
    lease_token: Uuid,
}

/// Immediately hide an upload and durably enqueue its provider object for deletion.
pub(crate) async fn schedule_upload_deletion(
    state: &AppState,
    auth: &AuthAccount,
    upload_id: i64,
    reason: &str,
) -> AppResult<()> {
    let mut transaction = state.db.begin().await?;
    let upload: Option<(String, i64, String)> = sqlx::query_as(
        "SELECT upload.status, upload.account_id, owner.role::text \
         FROM media.uploads upload \
         JOIN identity.accounts owner ON owner.id = upload.account_id \
         WHERE upload.id = $1 \
         FOR UPDATE OF upload, owner",
    )
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let (current_status, owner_id, owner_role) = upload.ok_or(MediaError::NotFound)?;
    require_strictly_lower_owner(auth, owner_id, &owner_role)?;

    let action = match current_status.as_str() {
        "pending" | "clean" => {
            sqlx::query("UPDATE media.uploads SET status = 'quarantined' WHERE id = $1")
                .bind(upload_id)
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "INSERT INTO media.object_deletion_jobs \
                 (upload_id, requested_by, requested_role, reason, previous_status) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(upload_id)
            .bind(auth.id)
            .bind(&auth.role)
            .bind(reason)
            .bind(&current_status)
            .execute(&mut *transaction)
            .await?;
            "media.upload.quarantined"
        }
        "quarantined" => {
            let affected = sqlx::query(
                "UPDATE media.object_deletion_jobs \
                 SET status = 'queued', requested_by = $2, requested_role = $3, reason = $4, \
                     attempt_count = 0, available_at = now(), lease_token = NULL, \
                     lease_expires_at = NULL, last_error_code = NULL, updated_at = now() \
                 WHERE upload_id = $1 AND status = 'dead_letter'",
            )
            .bind(upload_id)
            .bind(auth.id)
            .bind(&auth.role)
            .bind(reason)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
            if affected == 0 {
                let is_active: bool = sqlx::query_scalar(
                    "SELECT EXISTS ( \
                       SELECT 1 FROM media.object_deletion_jobs \
                       WHERE upload_id = $1 AND status IN ('queued', 'leased') \
                     )",
                )
                .bind(upload_id)
                .fetch_one(&mut *transaction)
                .await?;
                if !is_active {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "quarantined upload has no active deletion job"
                    )));
                }
                transaction.commit().await?;
                return Ok(());
            }
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
    });
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
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
        "UPDATE media.object_deletion_jobs \
         SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'lease_expired_after_max_attempts', updated_at = now() \
         WHERE status = 'leased' AND lease_expires_at <= now() AND attempt_count >= $1",
    )
    .bind(MAX_DELETE_ATTEMPTS)
    .execute(&mut *transaction)
    .await?;

    let lease_token = Uuid::new_v4();
    let job = sqlx::query_as::<_, ClaimedDeletionJob>(
        "WITH candidate AS ( \
           SELECT job.id FROM media.object_deletion_jobs job \
           JOIN media.uploads upload ON upload.id = job.upload_id \
           WHERE upload.status = 'quarantined' AND job.attempt_count < $1 \
             AND ($4::bigint IS NULL OR job.upload_id = $4) \
             AND ((job.status = 'queued' AND job.available_at <= now()) \
                  OR (job.status = 'leased' AND job.lease_expires_at <= now())) \
           ORDER BY job.available_at, job.id \
           FOR UPDATE OF job SKIP LOCKED \
           LIMIT 1 \
         ), claimed AS ( \
           UPDATE media.object_deletion_jobs job \
           SET status = 'leased', attempt_count = job.attempt_count + 1, lease_token = $2, \
               lease_expires_at = now() + ($3 * interval '1 second'), updated_at = now() \
           FROM candidate \
           WHERE job.id = candidate.id \
           RETURNING job.id, job.upload_id, job.requested_by, job.requested_role, job.reason, \
                     job.previous_status, job.attempt_count, job.lease_token \
         ) \
         SELECT claimed.id, claimed.upload_id, upload.oss_key, claimed.requested_by, \
                claimed.requested_role, claimed.reason, claimed.previous_status, \
                claimed.attempt_count, claimed.lease_token \
         FROM claimed JOIN media.uploads upload ON upload.id = claimed.upload_id",
    )
    .bind(MAX_DELETE_ATTEMPTS)
    .bind(lease_token)
    .bind(DELETE_LEASE_SECONDS)
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(job)
}

async fn complete_deletion(pool: &PgPool, job: &ClaimedDeletionJob) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
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
    let upload_affected = sqlx::query(
        "UPDATE media.uploads SET status = 'blocked' \
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
    let metadata = serde_json::json!({
        "oldStatus": "quarantined",
        "newStatus": "blocked",
        "previousPublishedStatus": job.previous_status,
        "deletionAttempts": job.attempt_count,
    });
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor { account_id: job.requested_by, role: &job.requested_role },
        "media.upload.blocked",
        "upload",
        &job.upload_id.to_string(),
        &job.reason,
        Some(&metadata),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn record_deletion_failure(pool: &PgPool, job: &ClaimedDeletionJob) -> AppResult<()> {
    let is_dead_letter = job.attempt_count >= MAX_DELETE_ATTEMPTS;
    let retry_seconds =
        i64::from(15 * 2_i32.pow(job.attempt_count.clamp(1, 8) as u32 - 1)).min(3600);
    let affected = sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = CASE WHEN $3 THEN 'dead_letter' ELSE 'queued' END, \
             available_at = CASE WHEN $3 THEN available_at \
                                 ELSE now() + ($4 * interval '1 second') END, \
             lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'provider_delete_failed', updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(is_dead_letter)
    .bind(retry_seconds)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("media deletion lease was lost".into()));
    }
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
    match object_store.delete_object(&job.oss_key).await {
        Ok(()) => complete_deletion(pool, &job).await?,
        Err(error) => {
            tracing::warn!(
                ?error,
                upload_id = job.upload_id,
                attempt = job.attempt_count,
                "media object deletion failed and remains quarantined"
            );
            record_deletion_failure(pool, &job).await?;
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

/// Run the durable media deletion worker for the lifetime of the API process.
pub async fn run_deletion_worker(state: AppState) {
    let object_store: Arc<dyn UploadObjectStore> =
        Arc::new(AliyunUploadObjectStore::from_config(&state.config));
    loop {
        match process_one_deletion_job(&state.db, object_store.as_ref()).await {
            Ok(true) => continue,
            Ok(false) => tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECONDS)).await,
            Err(error) => {
                tracing::warn!(?error, "media deletion worker iteration failed");
                tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECONDS)).await;
            }
        }
    }
}

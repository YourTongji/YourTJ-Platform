//! Durable quarantine and external-object deletion.

use std::sync::Arc;
use std::time::Duration;

use shared::pagination::Page;
use shared::{AppError, AppResult, AppState, AuthAccount};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::dto::DeletionJobDto;
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
    requested_by: Option<i64>,
    requested_role: Option<String>,
    request_source: String,
    reason: String,
    previous_status: String,
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

/// Immediately hide an upload and durably enqueue its provider object for deletion.
pub(crate) async fn schedule_upload_deletion(
    state: &AppState,
    auth: &AuthAccount,
    upload_id: i64,
    reason: &str,
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
    require_strictly_lower_owner(auth, owner_id, &owner.role)?;
    let action = match current_status.as_str() {
        "pending" | "clean" => {
            sqlx::query("UPDATE media.uploads SET status = 'quarantined' WHERE id = $1")
                .bind(upload_id)
                .execute(&mut *transaction)
                .await?;
            sqlx::query(
                "INSERT INTO media.object_deletion_jobs \
                 (upload_id, requested_by, requested_role, request_source, reason, previous_status) \
                 VALUES ($1, $2, $3, 'moderation', $4, $5)",
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
                     request_source = 'moderation', reason = $4, \
                     attempt_count = 0, available_at = now(), lease_token = NULL, \
                     lease_expires_at = NULL, last_error_code = NULL, updated_at = now() \
                 WHERE upload_id = $1 AND status = 'dead_letter' \
                   AND request_source = 'moderation'",
            )
            .bind(upload_id)
            .bind(auth.id)
            .bind(&auth.role)
            .bind(reason)
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

    let candidate_upload_id: Option<i64> = sqlx::query_scalar(
        "SELECT upload.id FROM media.object_deletion_jobs job \
         JOIN media.uploads upload ON upload.id = job.upload_id \
         WHERE upload.status = 'quarantined' AND job.attempt_count < $1 \
           AND NOT EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                           WHERE hold.asset_id = upload.id AND hold.released_at IS NULL \
                             AND hold.expires_at > now()) \
           AND ($2::bigint IS NULL OR job.upload_id = $2) \
           AND ((job.status = 'queued' AND job.available_at <= now()) \
                OR (job.status = 'leased' AND job.lease_expires_at <= now())) \
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
    let has_hold: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM media.asset_retention_holds \
         WHERE asset_id = $1 AND released_at IS NULL AND expires_at > now())",
    )
    .bind(candidate_upload_id)
    .fetch_one(&mut *transaction)
    .await?;
    if has_hold {
        transaction.commit().await?;
        return Ok(None);
    }

    let lease_token = Uuid::new_v4();
    let job = sqlx::query_as::<_, ClaimedDeletionJob>(
        "UPDATE media.object_deletion_jobs job \
         SET status = 'leased', attempt_count = job.attempt_count + 1, lease_token = $2, \
             lease_expires_at = now() + ($3 * interval '1 second'), updated_at = now() \
         FROM media.uploads upload \
         WHERE job.id = $1 AND upload.id = job.upload_id AND upload.status = 'quarantined' \
         RETURNING job.id, job.upload_id, upload.oss_key, job.requested_by, \
                   job.requested_role, job.request_source, job.reason, job.previous_status, \
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
    if upload_status != "quarantined" {
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
    let metadata = serde_json::json!({
        "oldStatus": "quarantined",
        "newStatus": "blocked",
        "previousPublishedStatus": job.previous_status,
        "deletionAttempts": job.attempt_count,
        "requestSource": job.request_source,
    });
    match (job.requested_by, job.requested_role.as_deref()) {
        (Some(account_id), Some(role)) if job.request_source == "moderation" => {
            governance::record_account_event_tx(
                &mut transaction,
                governance::AccountActor { account_id, role },
                "media.upload.blocked",
                "upload",
                &job.upload_id.to_string(),
                &job.reason,
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

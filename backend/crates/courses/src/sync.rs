//! Durable selection projection synchronization.
//!
//! PostgreSQL owns queue state and lease fencing. Workers never hold a row lock
//! across materialization, Meilisearch, or Redis I/O; every transition checks
//! the current UUID lease token.

use std::time::Duration as StdDuration;

use anyhow::Context as _;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

const MAX_ATTEMPTS: i16 = 8;
const LEASE_MINUTES: i64 = 30;

#[derive(Debug, Clone, FromRow)]
pub struct SelectionSyncJob {
    pub id: Uuid,
    pub requested_by: i64,
    pub reason: String,
    pub request_fingerprint: String,
    pub status: String,
    pub step: String,
    pub attempts: i16,
    pub progress_current: i32,
    pub progress_total: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub lease_token: Option<Uuid>,
    pub last_error_code: Option<String>,
    pub result: Value,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const JOB_COLUMNS: &str = "id, requested_by, reason, request_fingerprint, status, step, attempts, \
    progress_current, progress_total, next_attempt_at, lease_token, last_error_code, result, \
    started_at, completed_at, created_at, updated_at";

#[derive(Debug, FromRow)]
struct ClaimedSyncJob {
    id: Uuid,
    attempts: i16,
    lease_token: Uuid,
}

fn unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.code().as_deref() == Some("23505"))
}

pub async fn enqueue_sync_job_tx(
    tx: &mut PgConnection,
    requested_by: i64,
    reason: &str,
    idempotency_key_hash: &str,
    request_fingerprint: &str,
) -> AppResult<(SelectionSyncJob, bool)> {
    let existing_sql = format!(
        "SELECT {JOB_COLUMNS} FROM selection.sync_jobs \
         WHERE requested_by = $1 AND idempotency_key_hash = $2"
    );
    if let Some(existing) = sqlx::query_as::<_, SelectionSyncJob>(&existing_sql)
        .bind(requested_by)
        .bind(idempotency_key_hash)
        .fetch_optional(&mut *tx)
        .await?
    {
        if existing.request_fingerprint != request_fingerprint {
            return Err(AppError::Conflict(
                "idempotency key was already used for a different selection sync request".into(),
            ));
        }
        return Ok((existing, false));
    }

    let id = Uuid::new_v4();
    let insert_sql = format!(
        "INSERT INTO selection.sync_jobs \
         (id, requested_by, reason, idempotency_key_hash, request_fingerprint) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (requested_by, idempotency_key_hash) DO NOTHING \
         RETURNING {JOB_COLUMNS}"
    );
    let result = sqlx::query_as::<_, SelectionSyncJob>(&insert_sql)
        .bind(id)
        .bind(requested_by)
        .bind(reason)
        .bind(idempotency_key_hash)
        .bind(request_fingerprint)
        .fetch_optional(&mut *tx)
        .await;
    match result {
        Ok(Some(job)) => Ok((job, true)),
        Ok(None) => {
            let existing = sqlx::query_as::<_, SelectionSyncJob>(&existing_sql)
                .bind(requested_by)
                .bind(idempotency_key_hash)
                .fetch_one(&mut *tx)
                .await?;
            if existing.request_fingerprint != request_fingerprint {
                return Err(AppError::Conflict(
                    "idempotency key was already used for a different selection sync request"
                        .into(),
                ));
            }
            Ok((existing, false))
        }
        Err(error) if unique_violation(&error) => Err(AppError::Conflict(
            "another selection sync job is already queued or running".into(),
        )),
        Err(error) => Err(error.into()),
    }
}

pub async fn find_sync_job(pool: &PgPool, id: Uuid) -> AppResult<Option<SelectionSyncJob>> {
    let sql = format!("SELECT {JOB_COLUMNS} FROM selection.sync_jobs WHERE id = $1");
    Ok(sqlx::query_as::<_, SelectionSyncJob>(&sql).bind(id).fetch_optional(pool).await?)
}

pub async fn list_sync_jobs(
    pool: &PgPool,
    status: Option<&str>,
    cursor: Option<Uuid>,
    limit: i64,
) -> AppResult<(Vec<SelectionSyncJob>, Option<Uuid>)> {
    let limit = limit.clamp(1, 100);
    let cursor_position: Option<(DateTime<Utc>, Uuid)> = match cursor {
        Some(id) => sqlx::query_as(
            "SELECT created_at, id FROM selection.sync_jobs \
             WHERE id = $1 AND ($2::text IS NULL OR status = $2)",
        )
        .bind(id)
        .bind(status)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
        .map(Some)?,
        None => None,
    };
    let sql = format!(
        "SELECT {JOB_COLUMNS} FROM selection.sync_jobs \
         WHERE ($1::text IS NULL OR status = $1) \
           AND ($2::timestamptz IS NULL OR (created_at, id) < ($2, $3)) \
         ORDER BY created_at DESC, id DESC LIMIT $4"
    );
    let mut rows = sqlx::query_as::<_, SelectionSyncJob>(&sql)
        .bind(status)
        .bind(cursor_position.as_ref().map(|value| value.0))
        .bind(cursor_position.as_ref().map(|value| value.1))
        .bind(limit + 1)
        .fetch_all(pool)
        .await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.pop();
    }
    let next_cursor = has_more.then(|| rows.last().expect("continued job page").id);
    Ok((rows, next_cursor))
}

pub async fn retry_sync_job_tx(tx: &mut PgConnection, id: Uuid) -> AppResult<SelectionSyncJob> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM selection.sync_jobs WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(status) = status else {
        return Err(AppError::NotFound);
    };
    if status != "dead" {
        return Err(AppError::Conflict("only a dead selection sync job can be retried".into()));
    }
    let sql = format!(
        "UPDATE selection.sync_jobs \
         SET status = 'queued', step = 'queued', attempts = 0, progress_current = 0, \
             next_attempt_at = now(), last_error_code = NULL, result = '{{}}'::jsonb, \
             started_at = NULL, completed_at = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'dead' RETURNING {JOB_COLUMNS}"
    );
    match sqlx::query_as::<_, SelectionSyncJob>(&sql).bind(id).fetch_one(&mut *tx).await {
        Ok(job) => Ok(job),
        Err(error) if unique_violation(&error) => Err(AppError::Conflict(
            "another selection sync job is already queued or running".into(),
        )),
        Err(error) => Err(error.into()),
    }
}

async fn claim_due_job(pool: &PgPool) -> AppResult<Option<ClaimedSyncJob>> {
    let mut tx = pool.begin().await?;
    let expired_jobs = sqlx::query_as::<_, (Uuid, i16, String)>(
        "UPDATE selection.sync_jobs \
         SET status = CASE WHEN attempts >= $1 THEN 'dead' ELSE 'queued' END, \
             step = CASE WHEN attempts >= $1 THEN step ELSE 'queued' END, \
             next_attempt_at = now(), locked_at = NULL, lease_token = NULL, \
             lease_expires_at = NULL, last_error_code = 'worker_lease_expired', \
             completed_at = CASE WHEN attempts >= $1 THEN now() ELSE NULL END, \
             updated_at = now() \
         WHERE status = 'running' AND lease_expires_at < now() \
         RETURNING id, attempts, status",
    )
    .bind(MAX_ATTEMPTS)
    .fetch_all(&mut *tx)
    .await?;
    for (id, attempts, next_status) in expired_jobs {
        let metadata = json!({
            "attempts": attempts,
            "errorCode": "worker_lease_expired",
            "nextStatus": next_status
        });
        governance::record_system_event_tx(
            &mut tx,
            if next_status == "dead" {
                "selection.sync.dead"
            } else {
                "selection.sync.lease_expired"
            },
            "selection_sync_job",
            &id.to_string(),
            "selection projection sync worker lease expired",
            Some(&metadata),
        )
        .await?;
    }

    let candidate: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM selection.sync_jobs \
         WHERE status = 'queued' AND attempts < $1 AND next_attempt_at <= now() \
         ORDER BY next_attempt_at, created_at, id FOR UPDATE SKIP LOCKED LIMIT 1",
    )
    .bind(MAX_ATTEMPTS)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = candidate else {
        tx.commit().await?;
        return Ok(None);
    };

    let lease_token = Uuid::new_v4();
    let claimed = sqlx::query_as::<_, ClaimedSyncJob>(
        "UPDATE selection.sync_jobs \
         SET status = 'running', attempts = attempts + 1, locked_at = now(), \
             lease_token = $2, lease_expires_at = now() + ($3::bigint * interval '1 minute'), \
             started_at = COALESCE(started_at, now()), last_error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'queued' \
         RETURNING id, attempts, lease_token",
    )
    .bind(id)
    .bind(lease_token)
    .bind(LEASE_MINUTES)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(claimed)
}

async fn set_progress(
    pool: &PgPool,
    job: &ClaimedSyncJob,
    step: &'static str,
    progress_current: i32,
    result_patch: Option<&Value>,
) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE selection.sync_jobs \
         SET step = $3, progress_current = $4, \
             lease_expires_at = now() + ($5::bigint * interval '1 minute'), \
             result = result || COALESCE($6, '{}'::jsonb), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(step)
    .bind(progress_current)
    .bind(LEASE_MINUTES)
    .bind(result_patch)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("selection sync job lease was lost".into()));
    }
    Ok(())
}

async fn invalidate_selection_caches(state: &AppState) -> anyhow::Result<bool> {
    let Some(redis) = state.redis.as_ref() else {
        return Ok(false);
    };
    for prefix in [
        "selection-calendars",
        "selection-campuses",
        "selection-faculties",
        "selection-grades",
        "selection-majors",
        "selection-natures",
        "selection-offerings",
        "selection-offering-detail",
        "selection-offering-timeslots",
        "selection-latest-update",
    ] {
        shared::cache::bump_version(redis, prefix, "all")
            .await
            .with_context(|| format!("invalidate {prefix}"))?;
    }
    Ok(true)
}

struct PipelineFailure {
    code: &'static str,
    error: anyhow::Error,
}

async fn execute_pipeline(
    state: &AppState,
    job: &ClaimedSyncJob,
) -> Result<Value, PipelineFailure> {
    set_progress(&state.db, job, "catalogue", 0, None).await.map_err(|error| PipelineFailure {
        code: "catalogue_failed",
        error: anyhow::anyhow!(error.to_string()),
    })?;
    sqlx::raw_sql(include_str!("../../../ops/materialize_courses.sql"))
        .execute(&state.db)
        .await
        .context("catalogue materialization")
        .map_err(|error| PipelineFailure { code: "catalogue_failed", error })?;
    let pinyin_rows = crate::pinyin::sync_all_courses_pinyin(&state.db).await.map_err(|error| {
        PipelineFailure { code: "catalogue_failed", error: anyhow::anyhow!(error.to_string()) }
    })?;
    set_progress(
        &state.db,
        job,
        "materialize",
        1,
        Some(&json!({ "cataloguePinyinRows": pinyin_rows })),
    )
    .await
    .map_err(|error| PipelineFailure {
        code: "materialize_failed",
        error: anyhow::anyhow!(error.to_string()),
    })?;

    sqlx::raw_sql(include_str!("../../../ops/materialize_selection.sql"))
        .execute(&state.db)
        .await
        .context("selection materialization")
        .map_err(|error| PipelineFailure { code: "materialize_failed", error })?;
    let (offering_rows, timeslot_rows): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM selection.courses), \
                (SELECT COUNT(*) FROM selection.timeslots)",
    )
    .fetch_one(&state.db)
    .await
    .context("selection materialization counts")
    .map_err(|error| PipelineFailure { code: "materialize_failed", error })?;
    set_progress(
        &state.db,
        job,
        "search",
        2,
        Some(&json!({ "offeringRows": offering_rows, "timeslotRows": timeslot_rows })),
    )
    .await
    .map_err(|error| PipelineFailure {
        code: "search_failed",
        error: anyhow::anyhow!(error.to_string()),
    })?;

    if state.meili_url.trim().is_empty() {
        return Err(PipelineFailure {
            code: "search_failed",
            error: anyhow::anyhow!("Meilisearch is not configured"),
        });
    }
    crate::meili::setup_selection_index(&state.meili_url, &state.meili_master_key).await.map_err(
        |error| PipelineFailure { code: "search_failed", error: anyhow::anyhow!(error) },
    )?;
    let indexed_catalogue = crate::meili::reindex_course_documents(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
    )
    .await
    .map_err(|error| PipelineFailure {
        code: "search_failed",
        error: anyhow::anyhow!(error.to_string()),
    })?;
    let indexed_offerings = crate::meili::sync_selection_courses_to_meili(
        &state.meili_url,
        &state.meili_master_key,
        &state.db,
    )
    .await
    .map_err(|error| PipelineFailure {
        code: "search_failed",
        error: anyhow::anyhow!(error.to_string()),
    })?;
    set_progress(
        &state.db,
        job,
        "cache",
        3,
        Some(&json!({
            "indexedCatalogue": indexed_catalogue,
            "indexedOfferings": indexed_offerings
        })),
    )
    .await
    .map_err(|error| PipelineFailure {
        code: "cache_failed",
        error: anyhow::anyhow!(error.to_string()),
    })?;

    let cache_invalidated = invalidate_selection_caches(state)
        .await
        .map_err(|error| PipelineFailure { code: "cache_failed", error })?;
    Ok(json!({
        "cataloguePinyinRows": pinyin_rows,
        "offeringRows": offering_rows,
        "timeslotRows": timeslot_rows,
        "indexedCatalogue": indexed_catalogue,
        "indexedOfferings": indexed_offerings,
        "cacheInvalidated": cache_invalidated
    }))
}

async fn complete_job(pool: &PgPool, job: &ClaimedSyncJob, result: &Value) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let affected = sqlx::query(
        "UPDATE selection.sync_jobs \
         SET status = 'succeeded', step = 'complete', progress_current = progress_total, \
             result = $3, locked_at = NULL, lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = NULL, completed_at = now(), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(result)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("selection sync job lease was lost".into()));
    }
    let metadata = json!({ "attempts": job.attempts, "result": result });
    governance::record_system_event_tx(
        &mut tx,
        "selection.sync.succeeded",
        "selection_sync_job",
        &job.id.to_string(),
        "selection projection sync completed",
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn fail_job(
    pool: &PgPool,
    job: &ClaimedSyncJob,
    error_code: &'static str,
) -> AppResult<String> {
    let exponent = u32::try_from((job.attempts - 1).clamp(0, 7)).unwrap_or(0);
    let retry_at = Utc::now() + Duration::seconds((30_i64 * 2_i64.pow(exponent)).min(3_600));
    let next_status = if job.attempts >= MAX_ATTEMPTS { "dead" } else { "queued" };
    let mut tx = pool.begin().await?;
    let affected = sqlx::query(
        "UPDATE selection.sync_jobs \
         SET status = $3, next_attempt_at = $4, locked_at = NULL, lease_token = NULL, \
             lease_expires_at = NULL, last_error_code = $5, \
             completed_at = CASE WHEN $3 = 'dead' THEN now() ELSE NULL END, updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(next_status)
    .bind(retry_at)
    .bind(error_code)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("selection sync job lease was lost".into()));
    }
    let metadata = json!({
        "attempt": job.attempts,
        "errorCode": error_code,
        "nextStatus": next_status
    });
    governance::record_system_event_tx(
        &mut tx,
        if next_status == "dead" {
            "selection.sync.dead"
        } else {
            "selection.sync.retry_scheduled"
        },
        "selection_sync_job",
        &job.id.to_string(),
        "selection projection sync did not complete",
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(next_status.into())
}

/// Claim and process at most one due job.
pub async fn process_one_selection_sync_job(state: &AppState) -> AppResult<bool> {
    let Some(job) = claim_due_job(&state.db).await? else {
        return Ok(false);
    };
    match execute_pipeline(state, &job).await {
        Ok(result) => complete_job(&state.db, &job, &result).await?,
        Err(failure) => {
            let next_status = fail_job(&state.db, &job, failure.code).await?;
            tracing::warn!(
                job_id = %job.id,
                attempt = job.attempts,
                error_code = failure.code,
                status = next_status,
                error = %failure.error,
                "selection sync pipeline failed"
            );
        }
    }
    Ok(true)
}

/// Run the durable worker until process shutdown.
pub async fn run_selection_sync_worker(state: AppState) {
    loop {
        match process_one_selection_sync_job(&state).await {
            Ok(true) => continue,
            Ok(false) => tokio::time::sleep(StdDuration::from_millis(500)).await,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    error_code = "worker_iteration_failed",
                    "selection sync worker failed"
                );
                tokio::time::sleep(StdDuration::from_secs(1)).await;
            }
        }
    }
}

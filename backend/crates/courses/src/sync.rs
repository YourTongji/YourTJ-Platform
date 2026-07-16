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
const LEASE_HEARTBEAT_SECONDS: u64 = 60;
const RETENTION_INTERVAL_SECONDS: u64 = 24 * 60 * 60;
const SUCCEEDED_RETENTION_DAYS: i64 = 90;
const DEAD_RETENTION_DAYS: i64 = 365;
const IMPORT_PROVENANCE_RETENTION_DAYS: i64 = 365;

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
    let next_cursor = if has_more { rows.last().map(|job| job.id) } else { None };
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
         WHERE id = $1 AND status = 'running' AND lease_token = $2 \
           AND lease_expires_at > now()",
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

async fn renew_lease(pool: &PgPool, job: &ClaimedSyncJob) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE selection.sync_jobs \
         SET lease_expires_at = now() + ($3::bigint * interval '1 minute'), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2 \
           AND lease_expires_at > now()",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(LEASE_MINUTES)
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

fn projection_failure(error: AppError) -> PipelineFailure {
    let code = if matches!(error, AppError::Conflict(_)) { "lease_lost" } else { "search_failed" };
    PipelineFailure { code, error: anyhow::anyhow!(error.to_string()) }
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
    .map_err(projection_failure)?;

    if state.meili_url.trim().is_empty() {
        return Err(PipelineFailure {
            code: "search_failed",
            error: anyhow::anyhow!("Meilisearch is not configured"),
        });
    }
    let fence = crate::meili::SelectionSyncFence::new(&state.db, job.id, job.lease_token);
    fence.assert_current().await.map_err(|error| PipelineFailure {
        code: "lease_lost",
        error: anyhow::anyhow!(error.to_string()),
    })?;
    crate::meili::setup_selection_index(&state.meili_url, &state.meili_master_key).await.map_err(
        |error| PipelineFailure { code: "search_failed", error: anyhow::anyhow!(error) },
    )?;
    fence.assert_current().await.map_err(|error| PipelineFailure {
        code: "lease_lost",
        error: anyhow::anyhow!(error.to_string()),
    })?;
    let indexed_catalogue = crate::meili::reindex_course_documents_fenced(
        &state.db,
        &state.meili_url,
        &state.meili_master_key,
        &fence,
    )
    .await
    .map_err(projection_failure)?;
    let indexed_offerings = crate::meili::sync_selection_courses_to_meili_fenced(
        &state.meili_url,
        &state.meili_master_key,
        &state.db,
        &fence,
    )
    .await
    .map_err(projection_failure)?;
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

async fn execute_pipeline_with_heartbeat(
    state: &AppState,
    job: &ClaimedSyncJob,
) -> Result<Value, PipelineFailure> {
    renew_lease(&state.db, job).await.map_err(|error| PipelineFailure {
        code: "lease_lost",
        error: anyhow::anyhow!(error.to_string()),
    })?;
    let pipeline = execute_pipeline(state, job);
    tokio::pin!(pipeline);
    let mut heartbeat = tokio::time::interval(StdDuration::from_secs(LEASE_HEARTBEAT_SECONDS));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    heartbeat.tick().await;
    loop {
        tokio::select! {
            result = &mut pipeline => return result,
            _ = heartbeat.tick() => {
                renew_lease(&state.db, job).await.map_err(|error| PipelineFailure {
                    code: "lease_lost",
                    error: anyhow::anyhow!(error.to_string()),
                })?;
            }
        }
    }
}

async fn complete_job(pool: &PgPool, job: &ClaimedSyncJob, result: &Value) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let affected = sqlx::query(
        "UPDATE selection.sync_jobs \
         SET status = 'succeeded', step = 'complete', progress_current = progress_total, \
             result = $3, locked_at = NULL, lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = NULL, completed_at = now(), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2 \
           AND lease_expires_at > now()",
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
    let exponent = u32::try_from((job.attempts - 1).clamp(0, 7))
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    let retry_at = Utc::now() + Duration::seconds((30_i64 * 2_i64.pow(exponent)).min(3_600));
    let next_status = if job.attempts >= MAX_ATTEMPTS { "dead" } else { "queued" };
    let mut tx = pool.begin().await?;
    let affected = sqlx::query(
        "UPDATE selection.sync_jobs \
         SET status = $3, next_attempt_at = $4, locked_at = NULL, lease_token = NULL, \
             lease_expires_at = NULL, last_error_code = $5, \
             completed_at = CASE WHEN $3 = 'dead' THEN now() ELSE NULL END, updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2 \
           AND lease_expires_at > now()",
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
    match execute_pipeline_with_heartbeat(state, &job).await {
        Ok(result) => {
            if let Err(error) = complete_job(&state.db, &job, &result).await {
                if matches!(error, AppError::Conflict(_)) {
                    tracing::warn!(
                        job_id = %job.id,
                        attempt = job.attempts,
                        "stale selection sync worker could not complete after losing its lease"
                    );
                    return Ok(true);
                }
                return Err(error);
            }
        }
        Err(failure) => {
            let next_status = match fail_job(&state.db, &job, failure.code).await {
                Ok(status) => status,
                Err(AppError::Conflict(_)) => {
                    tracing::warn!(
                        job_id = %job.id,
                        attempt = job.attempts,
                        error_code = failure.code,
                        error = %failure.error,
                        "stale selection sync worker stopped after losing its lease"
                    );
                    return Ok(true);
                }
                Err(error) => return Err(error),
            };
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

async fn purge_operation_history(pool: &PgPool) -> AppResult<(u64, u64)> {
    let mut tx = pool.begin().await?;
    let sync_jobs = sqlx::query(
        "DELETE FROM selection.sync_jobs \
         WHERE (status IN ('succeeded', 'cancelled') \
                AND completed_at < now() - ($1::bigint * interval '1 day')) \
            OR (status = 'dead' \
                AND completed_at < now() - ($2::bigint * interval '1 day'))",
    )
    .bind(SUCCEEDED_RETENTION_DAYS)
    .bind(DEAD_RETENTION_DAYS)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let import_runs = sqlx::query(
        "DELETE FROM selection.import_runs \
         WHERE imported_at < now() - ($1::bigint * interval '1 day') \
           AND id <> (SELECT id FROM selection.import_runs \
                      ORDER BY imported_at DESC, id DESC LIMIT 1)",
    )
    .bind(IMPORT_PROVENANCE_RETENTION_DAYS)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok((sync_jobs, import_runs))
}

/// Run the durable worker until process shutdown.
pub async fn run_selection_sync_worker(state: AppState) {
    let mut next_retention = tokio::time::Instant::now();
    loop {
        let now = tokio::time::Instant::now();
        if now >= next_retention {
            next_retention = now + StdDuration::from_secs(RETENTION_INTERVAL_SECONDS);
            match purge_operation_history(&state.db).await {
                Ok((sync_jobs, import_runs)) if sync_jobs > 0 || import_runs > 0 => {
                    tracing::info!(
                        sync_jobs,
                        import_runs,
                        "selection operation history retention completed"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(?error, "selection operation history retention failed");
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serde_json::json;
    use shared::AppError;
    use sqlx::migrate::Migrator;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use sqlx::{Connection, PgConnection};

    use super::{
        claim_due_job, complete_job, enqueue_sync_job_tx, fail_job, purge_operation_history,
        renew_lease, set_progress,
    };

    static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

    #[tokio::test]
    async fn expired_worker_cannot_mutate_before_or_after_reclaim() {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let base_options = PgConnectOptions::from_str(&database_url)
            .expect("parse selection lease-test database URL");
        let mut admin = PgConnection::connect_with(&base_options.clone().database("postgres"))
            .await
            .expect("connect selection lease-test administrator");
        let database_name =
            format!("yourtj_selection_lease_{}_test", uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE \"{database_name}\""))
            .execute(&mut admin)
            .await
            .expect("create isolated selection lease-test database");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(base_options.database(&database_name))
            .await
            .expect("connect isolated selection lease-test database");
        MIGRATOR.run(&pool).await.expect("apply selection lease-test migrations");
        let account_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.accounts (email, handle, role) \
             VALUES ($1, $2, 'admin') RETURNING id",
        )
        .bind(format!("lease-test-{}@tongji.edu.cn", uuid::Uuid::new_v4().simple()))
        .bind(format!("lease-test-{}", uuid::Uuid::new_v4().simple()))
        .fetch_one(&pool)
        .await
        .expect("seed selection lease-test operator");
        let mut tx = pool.begin().await.expect("begin selection lease-test enqueue");
        enqueue_sync_job_tx(
            &mut tx,
            account_id,
            "lease fencing test",
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .await
        .expect("enqueue selection lease-test job");
        tx.commit().await.expect("commit selection lease-test enqueue");

        let first = claim_due_job(&pool)
            .await
            .expect("claim first selection lease")
            .expect("queued selection lease-test job");
        sqlx::query(
            "UPDATE selection.sync_jobs \
             SET lease_expires_at = now() - interval '1 second' WHERE id = $1",
        )
        .bind(first.id)
        .execute(&pool)
        .await
        .expect("expire first selection lease");

        assert!(matches!(renew_lease(&pool, &first).await, Err(AppError::Conflict(_))));
        assert!(matches!(
            set_progress(&pool, &first, "search", 2, None).await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            complete_job(&pool, &first, &json!({"worker": 1})).await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            fail_job(&pool, &first, "search_failed").await,
            Err(AppError::Conflict(_))
        ));

        let second = claim_due_job(&pool)
            .await
            .expect("reclaim expired selection lease")
            .expect("expired selection lease-test job is requeued");
        assert_ne!(first.lease_token, second.lease_token);
        assert!(matches!(renew_lease(&pool, &first).await, Err(AppError::Conflict(_))));
        assert!(matches!(
            set_progress(&pool, &first, "search", 2, None).await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            complete_job(&pool, &first, &json!({"worker": 1})).await,
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            fail_job(&pool, &first, "search_failed").await,
            Err(AppError::Conflict(_))
        ));

        renew_lease(&pool, &second).await.expect("renew current selection lease");
        set_progress(&pool, &second, "cache", 3, None)
            .await
            .expect("advance current selection lease");
        complete_job(&pool, &second, &json!({"worker": 2}))
            .await
            .expect("complete current selection lease");
        let status: String =
            sqlx::query_scalar("SELECT status FROM selection.sync_jobs WHERE id = $1")
                .bind(second.id)
                .fetch_one(&pool)
                .await
                .expect("read completed selection lease-test job");
        assert_eq!(status, "succeeded");

        let recent_succeeded = uuid::Uuid::new_v4();
        let old_dead = uuid::Uuid::new_v4();
        let recent_dead = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO selection.sync_jobs (\
               id, requested_by, reason, idempotency_key_hash, request_fingerprint, status, step, \
               attempts, progress_current, last_error_code, completed_at, created_at, updated_at\
             ) VALUES \
               ($1, $4, 'recent success', $5, $6, 'succeeded', 'complete', 1, 4, NULL, \
                now() - interval '89 days', now() - interval '89 days', now() - interval '89 days'), \
               ($2, $4, 'old failure', $7, $8, 'dead', 'search', 8, 2, 'search_failed', \
                now() - interval '366 days', now() - interval '366 days', now() - interval '366 days'), \
               ($3, $4, 'recent failure', $9, $10, 'dead', 'search', 8, 2, 'search_failed', \
                now() - interval '364 days', now() - interval '364 days', now() - interval '364 days')",
        )
        .bind(recent_succeeded)
        .bind(old_dead)
        .bind(recent_dead)
        .bind(account_id)
        .bind("c".repeat(64))
        .bind("d".repeat(64))
        .bind("e".repeat(64))
        .bind("f".repeat(64))
        .bind("1".repeat(64))
        .bind("2".repeat(64))
        .execute(&pool)
        .await
        .expect("seed selection operation retention jobs");
        sqlx::query(
            "UPDATE selection.sync_jobs \
             SET completed_at = now() - interval '91 days', \
                 created_at = now() - interval '91 days', updated_at = now() - interval '91 days' \
             WHERE id = $1",
        )
        .bind(second.id)
        .execute(&pool)
        .await
        .expect("age completed selection sync job");
        sqlx::query(
            "INSERT INTO selection.import_runs (\
               snapshot_sha256, snapshot_bytes, source_database, imported_by, \
               source_table_counts, target_table_counts, imported_at\
             ) VALUES \
               ($1, 1, 'jcourse-db-backup', 'on-call-role', '{}'::jsonb, '{}'::jsonb, \
                now() - interval '366 days'), \
               ($2, 1, 'jcourse-db-backup', 'on-call-role', '{}'::jsonb, '{}'::jsonb, now())",
        )
        .bind("3".repeat(64))
        .bind("4".repeat(64))
        .execute(&pool)
        .await
        .expect("seed selection import provenance retention rows");

        let purged = purge_operation_history(&pool)
            .await
            .expect("purge expired selection operation history");
        assert_eq!(purged, (2, 1));
        let retained_jobs: Vec<uuid::Uuid> = sqlx::query_scalar(
            "SELECT id FROM selection.sync_jobs \
             WHERE id = ANY($1) ORDER BY id",
        )
        .bind(vec![second.id, recent_succeeded, old_dead, recent_dead])
        .fetch_all(&pool)
        .await
        .expect("read retained selection operation jobs");
        let mut expected_jobs = vec![recent_succeeded, recent_dead];
        expected_jobs.sort_unstable();
        assert_eq!(retained_jobs, expected_jobs);
        let retained_imports: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM selection.import_runs")
                .fetch_one(&pool)
                .await
                .expect("read retained selection import provenance");
        assert_eq!(retained_imports, 1);

        pool.close().await;
        sqlx::query(&format!("DROP DATABASE \"{database_name}\""))
            .execute(&mut admin)
            .await
            .expect("drop isolated selection lease-test database");
    }
}

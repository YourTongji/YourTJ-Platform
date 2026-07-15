//! Operations-only HTTP surface for durable selection synchronization.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, Page};
use uuid::Uuid;

use crate::sync::{self, SelectionSyncJob};

#[derive(Debug, Deserialize)]
pub struct SyncJobInput {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncJobsQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionSyncJobDto {
    pub id: String,
    pub requested_by: String,
    pub status: String,
    pub step: String,
    pub attempts: i16,
    pub progress_current: i32,
    pub progress_total: i32,
    pub next_attempt_at: i64,
    pub last_error_code: Option<String>,
    pub result: serde_json::Value,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn unix(value: DateTime<Utc>) -> i64 {
    value.timestamp()
}

fn job_dto(job: SelectionSyncJob) -> SelectionSyncJobDto {
    SelectionSyncJobDto {
        id: job.id.to_string(),
        requested_by: job.requested_by.to_string(),
        status: job.status,
        step: job.step,
        attempts: job.attempts,
        progress_current: job.progress_current,
        progress_total: job.progress_total,
        next_attempt_at: unix(job.next_attempt_at),
        last_error_code: job.last_error_code,
        result: job.result,
        started_at: job.started_at.map(unix),
        completed_at: job.completed_at.map(unix),
        created_at: unix(job.created_at),
        updated_at: unix(job.updated_at),
    }
}

async fn authenticate_operations(
    headers: &HeaderMap,
    state: &AppState,
) -> AppResult<shared::AuthAccount> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(Capability::RunOperations).map_err(|_| AppError::Forbidden)?;
    Ok(auth)
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) || reason.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "reason must contain 3-500 non-control characters".into(),
        ));
    }
    Ok(reason)
}

fn idempotency_key(headers: &HeaderMap) -> AppResult<&str> {
    let key = headers
        .get("idempotency-key")
        .ok_or_else(|| AppError::BadRequest("Idempotency-Key is required".into()))?
        .to_str()
        .map_err(|_| AppError::BadRequest("Idempotency-Key is invalid".into()))?;
    if !(8..=128).contains(&key.len()) || !key.bytes().all(|byte| (b'!'..=b'~').contains(&byte)) {
        return Err(AppError::BadRequest(
            "Idempotency-Key must be 8-128 visible ASCII characters".into(),
        ));
    }
    Ok(key)
}

fn digest(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())
}

/// POST /api/v2/admin/selection/sync
pub async fn enqueue_sync(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(input): Json<SyncJobInput>,
) -> AppResult<(StatusCode, Json<SelectionSyncJobDto>)> {
    let auth = authenticate_operations(&headers, &state).await?;
    let reason = validate_reason(&input.reason)?;
    let key = idempotency_key(&headers)?;
    let account_id = auth.id.to_string();
    let key_hash = digest(&[&account_id, key]);
    let fingerprint = digest(&["selection_sync", reason]);

    let mut tx = state.db.begin().await?;
    let (job, created) =
        sync::enqueue_sync_job_tx(&mut tx, auth.id, reason, &key_hash, &fingerprint).await?;
    let metadata = serde_json::json!({ "state": job.status, "idempotentReplay": !created });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        if created { "selection.sync.queued" } else { "selection.sync.replayed" },
        "selection_sync_job",
        &job.id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok((if created { StatusCode::ACCEPTED } else { StatusCode::OK }, Json(job_dto(job))))
}

/// GET /api/v2/admin/selection/sync-jobs
pub async fn list_sync_jobs(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SyncJobsQuery>,
) -> AppResult<Json<Page<SelectionSyncJobDto>>> {
    authenticate_operations(&headers, &state).await?;
    if !(1..=100).contains(&query.limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let status = query.status.as_deref();
    if status.is_some_and(|value| {
        !matches!(value, "queued" | "running" | "succeeded" | "dead" | "cancelled")
    }) {
        return Err(AppError::BadRequest("invalid selection sync status".into()));
    }
    let cursor = query
        .cursor
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid selection sync cursor".into()))?;
    let (jobs, next) = sync::list_sync_jobs(&state.db, status, cursor, query.limit).await?;
    Ok(Json(Page::new(
        jobs.into_iter().map(job_dto).collect(),
        next.map(|value| value.to_string()),
    )))
}

/// GET /api/v2/admin/selection/sync-jobs/{id}
pub async fn get_sync_job(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<SelectionSyncJobDto>> {
    authenticate_operations(&headers, &state).await?;
    let id = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("invalid selection sync job id".into()))?;
    let job = sync::find_sync_job(&state.db, id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(job_dto(job)))
}

/// POST /api/v2/admin/selection/sync-jobs/{id}/retry
pub async fn retry_sync_job(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<SyncJobInput>,
) -> AppResult<Json<SelectionSyncJobDto>> {
    let auth = authenticate_operations(&headers, &state).await?;
    let id = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("invalid selection sync job id".into()))?;
    let reason = validate_reason(&input.reason)?;
    let mut tx = state.db.begin().await?;
    let job = sync::retry_sync_job_tx(&mut tx, id).await?;
    let metadata = serde_json::json!({ "state": "queued", "attemptsReset": true });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "selection.sync.retried",
        "selection_sync_job",
        &id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(job_dto(job)))
}

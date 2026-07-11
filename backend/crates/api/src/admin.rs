//! Cross-domain administration endpoints and operational jobs.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::Capability;
use shared::{AppResult, AppState};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AdminOverviewDto {
    pub total_users: i64,
    pub active_users: i64,
    pub suspended_users: i64,
    pub moderators: i64,
    pub administrators: i64,
    pub pending_review_reports: i64,
    pub pending_forum_flags: i64,
    pub pending_dm_reports: i64,
    pub pending_media_uploads: i64,
    pub threads_today: i64,
    pub comments_today: i64,
    pub likes_today: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventsQuery {
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminJobInput {
    pub reason: String,
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(shared::AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
}

async fn audit_job_request(
    state: &AppState,
    auth: &shared::AuthAccount,
    job_id: &str,
    job_kind: &str,
    reason: &str,
) -> AppResult<()> {
    let metadata = serde_json::json!({ "jobKind": job_kind, "state": "queued" });
    governance::record_account_event(
        &state.db,
        AccountActor { account_id: auth.id, role: &auth.role },
        "operations.job.queued",
        "job",
        job_id,
        reason,
        Some(&metadata),
    )
    .await
}

async fn authenticate_staff(
    headers: &HeaderMap,
    state: &AppState,
) -> AppResult<shared::AuthAccount> {
    identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)
}

/// GET /api/v2/admin/overview — operational queue and account counters.
pub async fn overview_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<Json<AdminOverviewDto>> {
    let auth = authenticate_staff(&headers, &state).await?;
    auth.require_capability(Capability::SearchUsers).map_err(|_| shared::AppError::Forbidden)?;

    let overview = sqlx::query_as::<_, AdminOverviewDto>(
        "SELECT \
           (SELECT COUNT(*) FROM identity.accounts WHERE status != 'deleted') AS total_users, \
           (SELECT COUNT(*) FROM identity.accounts accounts \
              WHERE accounts.status = 'active' \
                AND NOT EXISTS (SELECT 1 FROM identity.sanctions sanctions \
                  WHERE sanctions.account_id = accounts.id AND sanctions.kind = 'suspend' \
                    AND sanctions.revoked_at IS NULL \
                    AND (sanctions.ends_at IS NULL OR sanctions.ends_at > now()))) AS active_users, \
           (SELECT COUNT(DISTINCT account_id) FROM identity.sanctions \
              WHERE kind = 'suspend' AND revoked_at IS NULL \
                AND (ends_at IS NULL OR ends_at > now())) AS suspended_users, \
           (SELECT COUNT(*) FROM identity.accounts WHERE role = 'mod' AND status = 'active') AS moderators, \
           (SELECT COUNT(*) FROM identity.accounts WHERE role = 'admin' AND status = 'active') AS administrators, \
           (SELECT COUNT(*) FROM reviews.review_reports WHERE status = 'open') AS pending_review_reports, \
           (SELECT COUNT(*) FROM forum.flags WHERE status = 'open') AS pending_forum_flags, \
           (SELECT COUNT(*) FROM forum.dm_message_reports WHERE status = 'open') AS pending_dm_reports, \
           (SELECT COUNT(*) FROM media.uploads WHERE status = 'pending') AS pending_media_uploads, \
           COALESCE((SELECT SUM(threads_created)::bigint FROM activity.daily_counts \
             WHERE activity_date = timezone('Asia/Shanghai', now())::date), 0) AS threads_today, \
           COALESCE((SELECT SUM(comments_created)::bigint FROM activity.daily_counts \
             WHERE activity_date = timezone('Asia/Shanghai', now())::date), 0) AS comments_today, \
           COALESCE((SELECT SUM(likes_given)::bigint FROM activity.daily_counts \
             WHERE activity_date = timezone('Asia/Shanghai', now())::date), 0) AS likes_today",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(overview))
}

/// GET /api/v2/admin/audit-events — filtered append-only privileged action history.
pub async fn audit_events_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<AuditEventsQuery>,
) -> AppResult<Json<shared::Page<governance::AuditEventDto>>> {
    let auth = authenticate_staff(&headers, &state).await?;
    auth.require_capability(Capability::ReadAudit).map_err(|_| shared::AppError::Forbidden)?;

    let actor_id = query
        .actor_id
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid actorId".into()))?;
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
    let page = governance::list_events(
        &state.db,
        cursor,
        query.limit.unwrap_or(50),
        actor_id,
        query.action.as_deref(),
        query.target_type.as_deref(),
    )
    .await?;
    Ok(Json(page))
}

/// POST /api/v2/admin/selection/sync — trigger selection data sync pipeline
pub async fn selection_sync_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<AdminJobInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_capability(Capability::RunOperations).map_err(|_| shared::AppError::Forbidden)?;

    let job_id = Uuid::new_v4().to_string();
    audit_job_request(&state, &auth, &job_id, "selection_sync", validate_reason(&body.reason)?)
        .await?;
    let job_id_resp = job_id.clone();
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let pool = state.db.clone();
    let redis = state.redis.clone();

    tokio::spawn(async move {
        if let Err(e) =
            courses::sync::run_selection_sync(&pool, &meili_url, &meili_key, redis.as_ref()).await
        {
            tracing::error!(error = %e, job_id, "selection sync failed");
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "queued",
            "message": "selection sync started",
            "jobId": job_id_resp,
        })),
    ))
}

/// POST /api/v2/admin/reviews/reindex — rebuild reviews in Meilisearch
pub async fn reviews_reindex_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<AdminJobInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_capability(Capability::RunOperations).map_err(|_| shared::AppError::Forbidden)?;

    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let pool = state.db.clone();
    let job_id = Uuid::new_v4().to_string();
    audit_job_request(&state, &auth, &job_id, "reviews_reindex", validate_reason(&body.reason)?)
        .await?;
    let job_id_resp = job_id.clone();

    tokio::spawn(async move {
        tracing::info!(%job_id, "review reindex started");
        let rows: Vec<(i64,)> = match sqlx::query_as("SELECT id FROM reviews.reviews ORDER BY id")
            .fetch_all(&pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, %job_id, "review reindex: query failed");
                return;
            }
        };
        tracing::info!(%job_id, count = rows.len(), "review reindex: syncing reviews to meilisearch");
        for (review_id,) in rows {
            courses::meili::sync_review_to_meili(&meili_url, &meili_key, review_id, &pool).await;
        }
        tracing::info!(%job_id, "review reindex completed");
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "queued",
            "message": "review reindex started",
            "jobId": job_id_resp,
        })),
    ))
}

/// POST /api/v2/admin/forum/reindex — rebuild forum_threads Meilisearch index
pub async fn forum_reindex_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<AdminJobInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_capability(Capability::RunOperations).map_err(|_| shared::AppError::Forbidden)?;

    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let pool = state.db.clone();
    let job_id = Uuid::new_v4().to_string();
    audit_job_request(&state, &auth, &job_id, "forum_reindex", validate_reason(&body.reason)?)
        .await?;
    let job_id_response = job_id.clone();

    tokio::spawn(async move {
        if let Err(e) = forum::meili::reindex_forum(&pool, &meili_url, &meili_key).await {
            tracing::error!(error = %e, %job_id, "forum reindex failed");
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "status": "queued", "jobId": job_id_response })),
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// All admin routes (cross-domain stubs only; course admin CRUD moved to courses crate).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/overview", get(overview_handler))
        .route("/api/v2/admin/audit-events", get(audit_events_handler))
        .route("/api/v2/admin/selection/sync", post(selection_sync_handler))
        .route("/api/v2/admin/reviews/reindex", post(reviews_reindex_handler))
        .route("/api/v2/admin/forum/reindex", post(forum_reindex_handler))
        .with_state(state)
}

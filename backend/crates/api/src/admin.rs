//! Admin cross-cutting endpoints: selection sync, review reindex.
//! These are stubs that live in the api crate because they cross domain boundaries.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use shared::{AppResult, AppState};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Selection sync
// ---------------------------------------------------------------------------

/// POST /api/v2/admin/selection/sync — trigger selection data sync pipeline
pub async fn selection_sync_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let job_id = Uuid::new_v4().to_string();
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

/// POST /api/v2/admin/reviews/reindex — stub (queued)
pub async fn reviews_reindex_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({ "status": "queued" }))))
}

/// POST /api/v2/admin/forum/reindex — rebuild forum_threads Meilisearch index
pub async fn forum_reindex_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let pool = state.db.clone();

    tokio::spawn(async move {
        if let Err(e) = forum::meili::reindex_forum(&pool, &meili_url, &meili_key).await {
            tracing::error!(error = %e, "forum reindex failed");
        }
    });

    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({ "status": "queued" }))))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// All admin routes (cross-domain stubs only; course admin CRUD moved to courses crate).
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/selection/sync", post(selection_sync_handler))
        .route("/api/v2/admin/reviews/reindex", post(reviews_reindex_handler))
        .route("/api/v2/admin/forum/reindex", post(forum_reindex_handler))
        .with_state(state)
}

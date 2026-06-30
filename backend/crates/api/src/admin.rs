//! Admin cross-cutting endpoints: selection sync, review reindex.
//! These are stubs that live in the api crate because they cross domain boundaries.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use shared::{AppResult, AppState};

// ---------------------------------------------------------------------------
// Stub handlers
// ---------------------------------------------------------------------------

/// POST /api/v2/admin/selection/sync — stub (queued)
pub async fn selection_sync_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let _auth = identity::auth_middleware::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_| shared::AppError::Unauthorized)?;
    // require_mod is handled by the courses/admin handlers now
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({ "status": "queued" }))))
}

/// POST /api/v2/admin/reviews/reindex — stub (queued)
pub async fn reviews_reindex_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;
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
        .with_state(state)
}

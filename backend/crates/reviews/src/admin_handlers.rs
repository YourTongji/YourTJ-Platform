//! Axum handlers for the admin moderation interface (reviews domain).
//!
//! Every handler requires a `mod` or `admin` role.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppResult, AppState};

use crate::dto::{ReportDto, ReviewDto, ReviewInput};
use crate::repo;

// ---------------------------------------------------------------------------
// Query parameter types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AdminListReviewsQuery {
    pub status: Option<String>,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AdminListReportsQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReportInput {
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fetch a review's course_id and bump the cache version for both the course
/// and its review list, so CDN and app caches see the change immediately.
async fn bump_review_cache(state: &AppState, review_id: i64) {
    if let Ok(Some(course_id)) =
        sqlx::query_scalar::<_, i64>("SELECT course_id FROM reviews.reviews WHERE id = $1")
            .bind(review_id)
            .fetch_optional(&state.db)
            .await
    {
        let cid = course_id.to_string();
        shared::cache::bump_version_opt(state.redis.as_ref(), "course", &cid).await.ok();
        shared::cache::bump_version_opt(state.redis.as_ref(), "reviews", &cid).await.ok();
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /admin/reviews — list all reviews with optional status filter.
/// Returns a cursor-paginated `Page<ReviewDto>` matching the OpenAPI contract.
pub async fn admin_list_reviews(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AdminListReviewsQuery>,
) -> AppResult<Json<Page<ReviewDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let page =
        repo::admin_list_reviews(&state.db, params.status.as_deref(), params.cursor, params.limit)
            .await?;
    Ok(Json(page))
}

/// PATCH /admin/reviews/{id} — mod can edit any review.
pub async fn admin_edit_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReviewInput>,
) -> AppResult<Json<ReviewDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let dto = repo::admin_edit_review(
        &state.db,
        review_id,
        body.rating,
        body.comment.as_deref(),
        body.semester.as_deref(),
        body.score.as_deref(),
    )
    .await?;

    bump_review_cache(&state, review_id).await;
    courses::meili::sync_review_to_meili(
        &state.meili_url,
        &state.meili_master_key,
        review_id,
        &state.db,
    )
    .await;

    Ok(Json(dto))
}

/// DELETE /admin/reviews/{id} — soft-delete a review.
pub async fn admin_delete_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    repo::admin_soft_delete_review(&state.db, review_id).await?;

    bump_review_cache(&state, review_id).await;
    courses::meili::sync_review_to_meili(
        &state.meili_url,
        &state.meili_master_key,
        review_id,
        &state.db,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /admin/reviews/{id}/toggle — toggle visibility.
pub async fn admin_toggle_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    repo::admin_toggle_review_visibility(&state.db, review_id).await?;

    bump_review_cache(&state, review_id).await;
    courses::meili::sync_review_to_meili(
        &state.meili_url,
        &state.meili_master_key,
        review_id,
        &state.db,
    )
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /admin/reports — list reports with optional status filter.
pub async fn admin_list_reports(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AdminListReportsQuery>,
) -> AppResult<Json<Vec<ReportDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let items = repo::list_reports(&state.db, params.status.as_deref()).await?;
    Ok(Json(items))
}

/// POST /admin/reports/{id}/resolve — resolve a report.
pub async fn admin_resolve_report(
    State(state): State<AppState>,
    Path(report_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ResolveReportInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    repo::resolve_report(&state.db, report_id, body.note.as_deref()).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

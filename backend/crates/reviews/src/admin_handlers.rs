//! Axum handlers for the admin moderation interface (reviews domain).
//!
//! Every handler requires a `mod` or `admin` role.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::{AppResult, AppState, AuthAccount};

use crate::dto::{ReportDto, ReviewDto, ReviewInput};
use crate::repo;

// ---------------------------------------------------------------------------
// Query parameter types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AdminListReviewsQuery {
    pub status: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub limit: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
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
pub async fn admin_list_reviews(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AdminListReviewsQuery>,
) -> AppResult<Json<Vec<ReviewDto>>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let items =
        repo::admin_list_reviews(&state.db, params.status.as_deref(), params.page, params.limit)
            .await?;
    Ok(Json(items))
}

/// PATCH /admin/reviews/{id} — mod can edit any review.
pub async fn admin_edit_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReviewInput>,
) -> AppResult<Json<ReviewDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
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

    Ok(Json(dto))
}

/// DELETE /admin/reviews/{id} — soft-delete a review.
pub async fn admin_delete_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    repo::admin_soft_delete_review(&state.db, review_id).await?;

    bump_review_cache(&state, review_id).await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /admin/reviews/{id}/toggle — toggle visibility.
pub async fn admin_toggle_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    repo::admin_toggle_review_visibility(&state.db, review_id).await?;

    bump_review_cache(&state, review_id).await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /admin/reports — list reports with optional status filter.
pub async fn admin_list_reports(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AdminListReportsQuery>,
) -> AppResult<Json<Vec<ReportDto>>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
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
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    repo::resolve_report(&state.db, report_id, body.note.as_deref()).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

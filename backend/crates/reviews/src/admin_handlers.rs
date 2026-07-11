//! Axum handlers for the admin moderation interface (reviews domain).
//!
//! Every handler requires a `mod` or `admin` role.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppResult, AppState};

use crate::dto::{ReportDto, ReviewDto};
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
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReportInput {
    pub action: String,
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReasonInput {
    pub reason: String,
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

fn validate_reason(reason: &str, maximum: usize) -> AppResult<&str> {
    let reason = reason.trim();
    if reason.chars().count() < 3 || reason.chars().count() > maximum {
        return Err(shared::AppError::BadRequest(format!("reason must be 3–{maximum} characters")));
    }
    Ok(reason)
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
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| shared::AppError::Forbidden)?;

    let status = params.status.as_deref().unwrap_or("all");
    if !matches!(status, "visible" | "hidden" | "pending" | "all") {
        return Err(shared::AppError::BadRequest("invalid review status".into()));
    }
    if params.cursor.is_some_and(|cursor| cursor <= 0) {
        return Err(shared::AppError::BadRequest("cursor must be a positive integer".into()));
    }
    if params.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
        return Err(shared::AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let page = repo::admin_list_reviews(
        &state.db,
        (status != "all").then_some(status),
        params.cursor,
        params.limit,
    )
    .await?;
    Ok(Json(page))
}

/// DELETE /admin/reviews/{id} — soft-delete a review.
pub async fn admin_delete_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<AdminReasonInput>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| shared::AppError::Forbidden)?;

    let reason = validate_reason(&body.reason, 500)?;
    repo::admin_soft_delete_review(&state.db, review_id, auth.id, &auth.role, reason).await?;

    bump_review_cache(&state, review_id).await;
    crate::search::sync_search_document(
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
    Json(body): Json<AdminReasonInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| shared::AppError::Forbidden)?;

    let reason = validate_reason(&body.reason, 500)?;
    repo::admin_toggle_review_visibility(&state.db, review_id, auth.id, &auth.role, reason).await?;

    bump_review_cache(&state, review_id).await;
    crate::search::sync_search_document(
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
) -> AppResult<Json<Page<ReportDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| shared::AppError::Forbidden)?;

    let status = params.status.as_deref().unwrap_or("open");
    if !matches!(status, "open" | "upheld" | "rejected" | "ignored" | "all") {
        return Err(shared::AppError::BadRequest("invalid report status".into()));
    }
    let page = repo::list_reports(
        &state.db,
        (status != "all").then_some(status),
        params.cursor,
        params.limit.unwrap_or(20),
    )
    .await?;
    Ok(Json(page))
}

/// POST /admin/reports/{id}/resolve — resolve a report.
pub async fn admin_resolve_report(
    State(state): State<AppState>,
    Path(report_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ResolveReportInput>,
) -> AppResult<Json<ReportDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| shared::AppError::Forbidden)?;

    if !matches!(body.action.as_str(), "uphold" | "reject" | "ignore") {
        return Err(shared::AppError::BadRequest(
            "action must be uphold, reject, or ignore".into(),
        ));
    }
    let note = validate_reason(&body.note, 1000)?;
    let report =
        repo::resolve_report(&state.db, report_id, &body.action, note, auth.id, &auth.role).await?;
    if report.status == "upheld" {
        if let Ok(review_id) = report.review_id.parse::<i64>() {
            bump_review_cache(&state, review_id).await;
            crate::search::sync_search_document(
                &state.meili_url,
                &state.meili_master_key,
                review_id,
                &state.db,
            )
            .await;
        }
    }
    Ok(Json(report))
}

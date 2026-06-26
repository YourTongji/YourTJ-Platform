//! Axum request handlers for the reviews domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use shared::{AppResult, AppState, AuthAccount};

use crate::dto::{ListReviewsQuery, ReportInput, ReviewDto, ReviewInput};
use crate::repo;

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /courses/{id}/reviews — cursor-paginated list of visible reviews.
pub async fn list_reviews(
    State(state): State<AppState>,
    Path(course_id): Path<i64>,
    Query(params): Query<ListReviewsQuery>,
) -> AppResult<Json<Vec<ReviewDto>>> {
    let items = repo::list_reviews(
        &state.db,
        course_id,
        params.sort.as_deref(),
        params.cursor,
        Some(params.limit),
    )
    .await?;
    Ok(Json(items))
}

/// POST /courses/{id}/reviews — create a new review (authenticated).
pub async fn create_review(
    State(state): State<AppState>,
    Path(course_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReviewInput>,
) -> AppResult<(StatusCode, Json<ReviewDto>)> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    let dto = repo::create_review(
        &state.db,
        course_id,
        auth.id,
        body.rating,
        body.comment.as_deref(),
        body.semester.as_deref(),
        body.score.as_deref(),
    )
    .await?;

    Ok((StatusCode::CREATED, Json(dto)))
}

/// PATCH /reviews/{id} — edit own review (authenticated).
pub async fn edit_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReviewInput>,
) -> AppResult<Json<ReviewDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    let dto = repo::update_review(
        &state.db,
        review_id,
        auth.id,
        body.rating,
        body.comment.as_deref(),
        body.semester.as_deref(),
        body.score.as_deref(),
    )
    .await?;

    Ok(Json(dto))
}

/// POST /reviews/{id}/like — like a review (authenticated, idempotent).
pub async fn like_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    repo::like_review(&state.db, review_id, auth.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /reviews/{id}/unlike — unlike a review (authenticated).
pub async fn unlike_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    repo::unlike_review(&state.db, review_id, auth.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /reviews/{id}/report — report a review (authenticated).
pub async fn report_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReportInput>,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    repo::report_review(&state.db, review_id, auth.id, &body.reason).await?;

    Ok(StatusCode::NO_CONTENT)
}

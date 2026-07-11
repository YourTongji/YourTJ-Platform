//! Axum request handlers for the reviews domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use shared::{AppResult, AppState, Page};

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
) -> AppResult<Json<Page<ReviewDto>>> {
    let sort = params.sort.as_deref().unwrap_or("new");
    let c = params.cursor.map(|x| x.to_string()).unwrap_or_default();
    let cache_id = format!("{course_id}:{sort}:{c}");
    let page = shared::cache::cached_json(state.redis.as_ref(), "reviews", &cache_id, 120, async {
        repo::list_reviews(&state.db, course_id, Some(sort), params.cursor, Some(params.limit))
            .await
    })
    .await?;
    Ok(Json(page))
}

/// POST /courses/{id}/reviews — create a new review (authenticated).
pub async fn create_review(
    State(state): State<AppState>,
    Path(course_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReviewInput>,
) -> AppResult<(StatusCode, Json<ReviewDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "review_create",
        body.captcha_token.as_deref().unwrap_or_default(),
    )
    .await?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "review_create",
        &auth.id.to_string(),
        5,
        60,
    )
    .await?;

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

    // Bump cache version so next read goes to DB.
    let cid = course_id.to_string();
    shared::cache::bump_version_opt(state.redis.as_ref(), "course", &cid).await.ok();
    shared::cache::bump_version_opt(state.redis.as_ref(), "reviews", &cid).await.ok();
    if let Ok(review_id) = dto.id.parse::<i64>() {
        courses::meili::sync_review_to_meili(
            &state.meili_url,
            &state.meili_master_key,
            review_id,
            &state.db,
        )
        .await;
    }

    Ok((StatusCode::CREATED, Json(dto)))
}

/// PATCH /reviews/{id} — edit own review (authenticated).
pub async fn edit_review(
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

    // Bump course cache version so next read goes to DB.
    let cid = dto.course_id.clone();
    shared::cache::bump_version_opt(state.redis.as_ref(), "course", &cid).await.ok();

    courses::meili::sync_review_to_meili(
        &state.meili_url,
        &state.meili_master_key,
        review_id,
        &state.db,
    )
    .await;

    Ok(Json(dto))
}

/// POST /reviews/{id}/like — like a review (authenticated, idempotent).
pub async fn like_review(
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

    repo::like_review(&state.db, review_id, auth.id).await?;

    // Fire-and-forget Redis counter increment.
    if let Some(ref redis) = state.redis {
        if let Ok(mut conn) = redis.get().await {
            let key = format!("counters:review:{}:likes", review_id);
            let _ = redis::cmd("INCR").arg(&key).query_async::<()>(&mut conn).await;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /reviews/{id}/unlike — unlike a review (authenticated).
pub async fn unlike_review(
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

    repo::unlike_review(&state.db, review_id, auth.id).await?;

    // Fire-and-forget Redis counter decrement.
    if let Some(ref redis) = state.redis {
        if let Ok(mut conn) = redis.get().await {
            let key = format!("counters:review:{}:likes", review_id);
            let _ = redis::cmd("DECR").arg(&key).query_async::<()>(&mut conn).await;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /reviews/{id}/report — report a review (authenticated).
pub async fn report_review(
    State(state): State<AppState>,
    Path(review_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ReportInput>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;

    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "review_report",
        &body.captcha_token,
    )
    .await?;

    repo::report_review(&state.db, review_id, auth.id, &body.reason).await?;

    Ok(StatusCode::NO_CONTENT)
}

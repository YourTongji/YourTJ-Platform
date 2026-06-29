//! Axum request handlers for the reviews domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::de::DeserializeOwned;
use serde::Serialize;
use shared::{AppError, AppResult, AppState, AuthAccount};

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
    let sort = params.sort.as_deref().unwrap_or("new");
    let c = params.cursor.map(|x| x.to_string()).unwrap_or_default();
    let cache_id = format!("{course_id}:{sort}:{c}");
    let items = cached_json(&state, "reviews", &cache_id, 120, async {
        repo::list_reviews(&state.db, course_id, Some(sort), params.cursor, Some(params.limit))
            .await
            .map_err(|e| e.into())
    })
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

    // Bump cache version so next read goes to DB.
    let cid = course_id.to_string();
    shared::cache::bump_version_opt(state.redis.as_ref(), "course", &cid).await.ok();
    shared::cache::bump_version_opt(state.redis.as_ref(), "reviews", &cid).await.ok();

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

    // Bump course cache version so next read goes to DB.
    let cid = dto.course_id.clone();
    shared::cache::bump_version_opt(state.redis.as_ref(), "course", &cid).await.ok();

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
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
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
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    repo::report_review(&state.db, review_id, auth.id, &body.reason).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn cached_json<T: Serialize + DeserializeOwned>(
    state: &AppState,
    prefix: &str,
    id: &str,
    ttl: u64,
    fetch: impl std::future::Future<Output = Result<T, AppError>>,
) -> Result<T, AppError> {
    if let Some(ref redis) = state.redis {
        if let Ok(Some(cached)) = shared::cache::get_cached(redis, prefix, id).await {
            if let Ok(val) = serde_json::from_str::<T>(&cached) {
                return Ok(val);
            }
        }
    }
    let val = fetch.await?;
    if let Some(ref redis) = state.redis {
        if let Ok(json) = serde_json::to_string(&val) {
            let _ = shared::cache::set_cached(redis, prefix, id, &json, ttl).await;
        }
    }
    Ok(val)
}

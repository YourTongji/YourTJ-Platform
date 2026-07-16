//! Axum request handlers for the reviews domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use sha2::{Digest, Sha256};
use shared::{AppResult, AppState, Page};

use crate::dto::{ListReviewsQuery, ReportInput, ReviewDto, ReviewInput, ReviewSort};
use crate::repo;

fn review_create_idempotency_key(headers: &HeaderMap) -> AppResult<Option<&str>> {
    let Some(value) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| shared::AppError::BadRequest("invalid Idempotency-Key".into()))?;
    if value.is_empty() || value.len() > 128 || value.trim() != value {
        return Err(shared::AppError::BadRequest(
            "Idempotency-Key must be 1–128 visible characters".into(),
        ));
    }
    Ok(Some(value))
}

fn review_create_request_hash(course_id: i64, body: &ReviewInput) -> AppResult<String> {
    let payload = serde_json::json!({
        "courseId": course_id.to_string(),
        "rating": body.rating,
        "comment": body.comment.as_deref(),
        "semester": body.semester.as_deref(),
        "score": body.score.as_deref(),
    });
    let bytes =
        serde_json::to_vec(&payload).map_err(|error| shared::AppError::Internal(error.into()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

async fn is_review_write_allowed(state: &AppState, account_id: i64) -> AppResult<bool> {
    Ok(!identity::sanctions::is_silenced(state.redis.as_ref(), &state.db, account_id).await?)
}

async fn require_review_write_allowed(state: &AppState, account_id: i64) -> AppResult<()> {
    if !is_review_write_allowed(state, account_id).await? {
        return Err(shared::AppError::Forbidden);
    }
    Ok(())
}

async fn verify_review_create_abuse_controls(
    state: &AppState,
    account_id: i64,
    captcha_token: &str,
) -> AppResult<()> {
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "review_create",
        captcha_token,
    )
    .await?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "review_create",
        &account_id.to_string(),
        5,
        60,
    )
    .await?;
    Ok(())
}

async fn invalidate_created_review(state: &AppState, course_id: i64, dto: &ReviewDto) {
    let course_key = course_id.to_string();
    shared::cache::bump_version_opt(state.redis.as_ref(), "course", &course_key).await.ok();
    shared::cache::bump_version_opt(state.redis.as_ref(), "reviews", &course_key).await.ok();
    if let Ok(review_id) = dto.id.parse::<i64>() {
        crate::search::sync_search_document(
            &state.meili_url,
            &state.meili_master_key,
            review_id,
            &state.db,
        )
        .await;
    }
}
// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /courses/{id}/reviews — cursor-paginated list of visible reviews.
pub async fn list_reviews(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(course_id): Path<i64>,
    Query(params): Query<ListReviewsQuery>,
) -> AppResult<Json<Page<ReviewDto>>> {
    if params.cursor.is_some_and(|cursor| cursor <= 0) {
        return Err(shared::AppError::BadRequest("cursor must be a positive review id".into()));
    }
    if !(1..=100).contains(&params.limit) {
        return Err(shared::AppError::BadRequest("limit must be between 1 and 100".into()));
    }

    let viewer = identity::auth_middleware::authenticate_optional(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| shared::AppError::Unauthorized)?;
    let viewer_account_id = viewer.as_ref().map(|account| account.id);
    let viewer_can_write = if let Some(viewer_account_id) = viewer_account_id {
        is_review_write_allowed(&state, viewer_account_id).await?
    } else {
        false
    };
    let sort = params.sort.unwrap_or(ReviewSort::Hot);
    let c = params.cursor.map(|x| x.to_string()).unwrap_or_default();
    let course_key = course_id.to_string();
    let namespace_version =
        shared::cache::current_version_opt(state.redis.as_ref(), "reviews", &course_key).await;
    let cache_id =
        format!("{course_id}:v{namespace_version}:{}:{c}:{}", sort.as_str(), params.limit);
    let page = if viewer_account_id.is_some() {
        repo::list_reviews(
            &state.db,
            course_id,
            sort,
            params.cursor,
            Some(params.limit),
            viewer_account_id,
            viewer_can_write,
        )
        .await?
    } else {
        shared::cache::cached_json(state.redis.as_ref(), "review_page", &cache_id, 120, async {
            repo::list_reviews(
                &state.db,
                course_id,
                sort,
                params.cursor,
                Some(params.limit),
                None,
                false,
            )
            .await
        })
        .await?
    };
    Ok(Json(page))
}

/// GET /reviews/{id} — return one visible review with viewer-specific permissions.
pub async fn get_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(review_id): Path<i64>,
) -> AppResult<Json<ReviewDto>> {
    let viewer = identity::auth_middleware::authenticate_optional(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| shared::AppError::Unauthorized)?;
    let viewer_account_id = viewer.as_ref().map(|account| account.id);
    let viewer_can_write = if let Some(viewer_account_id) = viewer_account_id {
        is_review_write_allowed(&state, viewer_account_id).await?
    } else {
        false
    };
    let review =
        repo::get_visible_review(&state.db, review_id, viewer_account_id, viewer_can_write)
            .await?
            .ok_or(shared::AppError::NotFound)?;
    Ok(Json(review))
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
    let captcha_token = body.captcha_token.as_deref().unwrap_or_default();
    let idempotency_key = review_create_idempotency_key(&headers)?;
    let dto = if let Some(idempotency_key) = idempotency_key {
        let request_hash = review_create_request_hash(course_id, &body)?;
        let mut tx = state.db.begin().await?;
        repo::lock_review_create_idempotency(&mut tx, auth.id, idempotency_key).await?;
        if let Some(mut replay) =
            repo::find_review_create_replay(&mut tx, auth.id, idempotency_key, &request_hash)
                .await?
        {
            replay.viewer_liked = false;
            replay.can_edit = is_review_write_allowed(&state, auth.id).await?;
            replay.can_report = false;
            tx.commit().await?;
            invalidate_created_review(&state, course_id, &replay).await;
            return Ok((StatusCode::CREATED, Json(replay)));
        }
        require_review_write_allowed(&state, auth.id).await?;
        verify_review_create_abuse_controls(&state, auth.id, captcha_token).await?;
        let dto = repo::create_review_tx(
            &mut tx,
            course_id,
            auth.id,
            body.rating,
            body.comment.as_deref(),
            body.semester.as_deref(),
            body.score.as_deref(),
        )
        .await?;
        let review_id = dto.id.parse::<i64>().map_err(|_| {
            shared::AppError::Internal(
                std::io::Error::other("created review id is not numeric").into(),
            )
        })?;
        repo::record_review_create_idempotency(
            &mut tx,
            auth.id,
            idempotency_key,
            &request_hash,
            review_id,
            &dto,
        )
        .await?;
        tx.commit().await?;
        dto
    } else {
        require_review_write_allowed(&state, auth.id).await?;
        verify_review_create_abuse_controls(&state, auth.id, captcha_token).await?;
        repo::create_review(
            &state.db,
            course_id,
            auth.id,
            body.rating,
            body.comment.as_deref(),
            body.semester.as_deref(),
            body.score.as_deref(),
        )
        .await?
    };

    invalidate_created_review(&state, course_id, &dto).await;

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
    if identity::sanctions::is_silenced(state.redis.as_ref(), &state.db, auth.id).await? {
        return Err(shared::AppError::Forbidden);
    }

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
    shared::cache::bump_version_opt(state.redis.as_ref(), "reviews", &cid).await.ok();

    crate::search::sync_search_document(
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
    if identity::sanctions::is_silenced(state.redis.as_ref(), &state.db, auth.id).await? {
        return Err(shared::AppError::Forbidden);
    }

    let was_inserted = repo::like_review(&state.db, review_id, auth.id).await?;

    // Fire-and-forget Redis counter increment.
    if was_inserted {
        if let Some(course_id) = repo::review_course_id(&state.db, review_id).await? {
            shared::cache::bump_version_opt(
                state.redis.as_ref(),
                "reviews",
                &course_id.to_string(),
            )
            .await
            .ok();
        }
        if let Some(ref redis) = state.redis {
            if let Ok(mut conn) = redis.get().await {
                let key = format!("counters:review:{}:likes", review_id);
                let _ = redis::cmd("INCR").arg(&key).query_async::<()>(&mut conn).await;
            }
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

    let was_deleted = repo::unlike_review(&state.db, review_id, auth.id).await?;

    // Fire-and-forget Redis counter decrement.
    if was_deleted {
        if let Some(course_id) = repo::review_course_id(&state.db, review_id).await? {
            shared::cache::bump_version_opt(
                state.redis.as_ref(),
                "reviews",
                &course_id.to_string(),
            )
            .await
            .ok();
        }
        if let Some(ref redis) = state.redis {
            if let Ok(mut conn) = redis.get().await {
                let key = format!("counters:review:{}:likes", review_id);
                let _ = redis::cmd("DECR").arg(&key).query_async::<()>(&mut conn).await;
            }
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

    require_review_write_allowed(&state, auth.id).await?;

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

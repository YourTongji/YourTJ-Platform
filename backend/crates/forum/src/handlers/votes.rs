use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

use crate::repo;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteDeleteQuery {
    post_type: String,
}

/// POST /api/v2/forum/posts/{post_id}/vote — auth required
pub async fn vote_post(
    State(state): State<AppState>,
    Path(post_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::VoteInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let tl = crate::trust_levels::get_trust_level(&state.db, auth.id).await?;
    if tl == 0 {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "vote_tl0",
            &auth.id.to_string(),
            30,
            60,
        )
        .await?;
    } else {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "vote",
            &auth.id.to_string(),
            60,
            60,
        )
        .await?;
    }

    let post_id: i64 = post_id_str.parse().map_err(|_| AppError::NotFound)?;

    if !matches!(body.post_type.as_str(), "thread" | "comment") {
        return Err(AppError::BadRequest("postType must be thread/comment".into()));
    }
    let post_type = body.post_type;

    let outcome = repo::vote_post(&state.db, &post_type, post_id, auth.id, &body.value).await?;

    crate::cache::invalidate_thread_surfaces(
        state.redis.as_ref(),
        outcome.thread_id,
        outcome.board_id,
    )
    .await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "voteCount": outcome.vote_count,
        "viewerVote": outcome.viewer_vote,
    })))
}

/// DELETE /api/v2/forum/posts/{post_id}/vote — auth required
pub async fn remove_vote(
    State(state): State<AppState>,
    Path(post_id_str): Path<String>,
    Query(query): Query<VoteDeleteQuery>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let post_id = post_id_str.parse().map_err(|_| AppError::NotFound)?;
    let outcome = repo::remove_vote(&state.db, &query.post_type, post_id, auth.id).await?;
    crate::cache::invalidate_thread_surfaces(
        state.redis.as_ref(),
        outcome.thread_id,
        outcome.board_id,
    )
    .await;
    Ok(Json(serde_json::json!({
        "ok": true,
        "voteCount": outcome.vote_count,
        "viewerVote": outcome.viewer_vote,
    })))
}

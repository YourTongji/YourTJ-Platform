use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppError, AppResult, AppState};

use crate::dto::PollOptionDto;
use crate::repo;

/// POST /api/v2/forum/polls/{id}/vote — auth required
pub async fn vote_poll_handler(
    State(state): State<AppState>,
    Path(poll_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::PollVoteInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let poll_id: i64 = poll_id_str.parse().map_err(|_| AppError::NotFound)?;
    let option_id: i64 =
        body.option_id.parse().map_err(|_| AppError::BadRequest("invalid optionId".into()))?;

    // Verify the poll exists.
    let poll = repo::get_poll_by_id(&state.db, poll_id).await?.ok_or(AppError::NotFound)?;

    // Verify the option belongs to this poll.
    let option = repo::get_poll_option(&state.db, option_id)
        .await?
        .ok_or(AppError::BadRequest("option not found".into()))?;

    if option.poll_id != poll_id {
        return Err(AppError::BadRequest("option does not belong to this poll".into()));
    }

    repo::vote_option(&state.db, poll_id, poll.multi_select, option_id, auth.id).await?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// GET /api/v2/forum/polls/{id}/results — auth optional
pub async fn poll_results_handler(
    State(state): State<AppState>,
    Path(poll_id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<crate::dto::PollDto>> {
    let poll_id: i64 = poll_id_str.parse().map_err(|_| AppError::NotFound)?;

    // Attempt authentication (optional).
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .ok();

    // Load poll row.
    let poll = repo::get_poll_by_id(&state.db, poll_id).await?.ok_or(AppError::NotFound)?;

    // Load options.
    let options = repo::get_poll_results(&state.db, poll_id).await?;

    // Load user's votes if authenticated.
    let my_votes: Vec<String> = if let Some(ref account) = auth {
        repo::get_voted_option_ids(&state.db, poll_id, account.id)
            .await?
            .into_iter()
            .map(|id| id.to_string())
            .collect()
    } else {
        vec![]
    };

    let option_dtos: Vec<PollOptionDto> = options
        .into_iter()
        .map(|o| PollOptionDto {
            id: o.id.to_string(),
            label: o.label,
            vote_count: o.vote_count,
            position: o.position,
        })
        .collect();

    Ok(Json(crate::dto::PollDto {
        id: poll.id.to_string(),
        question: poll.question,
        multi_select: poll.multi_select,
        closes_at: poll.closes_at.map(|v| v.timestamp()),
        options: option_dtos,
        my_votes,
    }))
}

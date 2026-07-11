//! Read tracking handlers.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

use crate::dto::ThreadDto;
use crate::repo;

use super::{hydrate_thread_tags, thread_to_dto};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTrackingQuery {
    pub cursor: Option<String>,
    #[serde(default = "super::default_limit")]
    pub limit: i64,
}

/// POST /api/v2/forum/threads/{id}/read — report read position
pub async fn report_read(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::ReadTrackingInput>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "read_report",
        &auth.id.to_string(),
        60,
        60,
    )
    .await?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let last_read = body
        .last_read_comment_id
        .as_deref()
        .map(|comment_id| {
            comment_id
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("invalid lastReadCommentId".into()))
        })
        .transpose()?;

    repo::upsert_read_position(&state.db, auth.id, id, last_read).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v2/forum/threads/unread — list unread threads
pub async fn list_unread_threads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ReadTrackingQuery>,
) -> AppResult<Json<shared::pagination::Page<ThreadDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let cursor = q
        .cursor
        .as_deref()
        .map(|cursor| {
            cursor.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into()))
        })
        .transpose()?;
    let (_rows, next_cursor) =
        repo::get_unread_thread_ids(&state.db, auth.id, None, None, q.limit, cursor).await?;

    let unread_counts = _rows.iter().copied().collect::<std::collections::HashMap<_, _>>();
    let thread_ids = _rows.iter().map(|(thread_id, _)| *thread_id).collect::<Vec<_>>();
    let rows = repo::fetch_threads_by_ids(&state.db, &thread_ids, Some(auth.id)).await?;
    let mut items = rows
        .iter()
        .map(thread_to_dto)
        .map(|mut thread| {
            thread.unread_count = thread
                .id
                .parse::<i64>()
                .ok()
                .and_then(|thread_id| unread_counts.get(&thread_id).copied());
            thread
        })
        .collect::<Vec<ThreadDto>>();
    hydrate_thread_tags(&state.db, &mut items).await?;
    let next_str = next_cursor.map(|c| c.to_string());

    Ok(Json(shared::pagination::Page::new(items, next_str)))
}

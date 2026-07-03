//! Read tracking handlers.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

use crate::dto::ThreadFeedDto;
use crate::repo;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTrackingQuery {
    pub feed: Option<String>, // "unread"
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
) -> AppResult<Json<serde_json::Value>> {
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
    let last_read = body.last_read_comment_id.and_then(|s| s.parse::<i64>().ok());

    repo::upsert_read_position(&state.db, auth.id, id, last_read).await?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// GET /api/v2/forum/threads/unread — list unread threads
pub async fn list_unread_threads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ReadTrackingQuery>,
) -> AppResult<Json<shared::pagination::Page<ThreadFeedDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let cursor: Option<i64> = q.cursor.and_then(|c| c.parse().ok());
    let (_rows, next_cursor) =
        repo::get_unread_thread_ids(&state.db, auth.id, q.limit, cursor).await?;

    let items: Vec<ThreadFeedDto> = Vec::new();
    let next_str = next_cursor.map(|c| c.to_string());

    Ok(Json(shared::pagination::Page::new(items, next_str)))
}

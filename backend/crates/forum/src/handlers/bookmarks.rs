//! Bookmark handlers.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::BookmarkDto;

use super::default_limit;

// ---------------------------------------------------------------------------
// query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkListQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// PUT /api/v2/forum/posts/{id}/bookmark
pub async fn set_bookmark(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::BookmarkInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let post_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    crate::repo::upsert_bookmark(&state.db, auth.id, "thread", post_id, body.note.as_deref())
        .await?;

    Ok(Json(json!({"ok": true})))
}

/// DELETE /api/v2/forum/posts/{id}/bookmark
pub async fn remove_bookmark(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
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

    let post_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    crate::repo::delete_bookmark(&state.db, auth.id, "thread", post_id).await?;

    Ok(Json(json!({"ok": true})))
}

/// GET /api/v2/forum/bookmarks
pub async fn list_bookmarks_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<BookmarkListQuery>,
) -> AppResult<Json<Page<BookmarkDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let cursor: Option<i64> = q.cursor.and_then(|c| c.parse().ok());
    let (rows, next_cursor) =
        crate::repo::list_bookmarks(&state.db, auth.id, cursor, q.limit).await?;

    let items: Vec<BookmarkDto> = rows
        .into_iter()
        .map(|r| BookmarkDto {
            target_type: r.target_type,
            target_id: r.target_id.to_string(),
            note: r.note,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    let next_str = next_cursor.map(|c| c.to_string());
    Ok(Json(Page::new(items, next_str)))
}

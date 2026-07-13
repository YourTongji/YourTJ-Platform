//! Bookmark handlers.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkDeleteQuery {
    post_type: String,
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
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let post_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    crate::repo::upsert_bookmark(
        &state.db,
        auth.id,
        &body.post_type,
        post_id,
        body.note.as_deref(),
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/forum/posts/{id}/bookmark
pub async fn remove_bookmark(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    Query(query): Query<BookmarkDeleteQuery>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let post_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    crate::repo::delete_bookmark(&state.db, auth.id, &query.post_type, post_id).await?;

    Ok(StatusCode::NO_CONTENT)
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

    let (rows, next_cursor) =
        crate::repo::list_bookmarks(&state.db, auth.id, q.cursor.as_deref(), q.limit).await?;

    let thread_ids = rows
        .iter()
        .filter_map(|row| (row.target_type == "thread").then_some(row.target_id))
        .collect::<Vec<_>>();
    let comment_ids = rows
        .iter()
        .filter_map(|row| (row.target_type == "comment").then_some(row.target_id))
        .collect::<Vec<_>>();
    let content_rows = crate::repo::profiles::get_visible_profile_content(
        &state.db,
        &thread_ids,
        &comment_ids,
        auth.id,
    )
    .await?;
    let content = super::profiles::hydrate_profile_content(&state, Some(auth.id), content_rows)
        .await?
        .into_iter()
        .map(|item| ((item.target_type.clone(), item.id.clone()), item))
        .collect::<HashMap<_, _>>();
    let mut content = content;
    let items = rows
        .into_iter()
        .filter_map(|row| {
            let target_id = row.target_id.to_string();
            let item = content.remove(&(row.target_type.clone(), target_id.clone()))?;
            Some(BookmarkDto {
                target_type: row.target_type,
                target_id,
                note: row.note,
                created_at: row.created_at.timestamp(),
                content: item,
            })
        })
        .collect();

    Ok(Json(Page::new(items, next_cursor)))
}

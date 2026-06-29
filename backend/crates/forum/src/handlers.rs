//! Axum request handlers for the forum domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState, AuthAccount};

use crate::dto::{
    BoardDto, CommentDto, CommentInput, ThreadDetailDto, ThreadDto, ThreadInput, VoteInput,
};
use crate::repo;

// ---------------------------------------------------------------------------
// query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListQuery {
    #[serde(default = "default_sort")]
    sort: String, // "hot" or "new"
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_sort() -> String {
    "new".into()
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

// ---------------------------------------------------------------------------
// row → dto helpers
// ---------------------------------------------------------------------------

fn thread_to_dto(row: &crate::models::ThreadRowJoined) -> ThreadDto {
    ThreadDto {
        id: row.id.to_string(),
        board_id: row.board_id.to_string(),
        author_handle: row.author_handle.clone(),
        title: row.title.clone(),
        reply_count: row.reply_count,
        vote_count: row.vote_count,
        hot_score: row.hot_score,
        created_at: row.created_at.timestamp(),
        last_activity_at: row.last_activity_at.timestamp(),
    }
}

fn thread_to_detail_dto(row: &crate::models::ThreadRowJoined) -> ThreadDetailDto {
    ThreadDetailDto { base: thread_to_dto(row), body: row.body.clone() }
}

fn comment_to_dto(row: &crate::models::CommentRowJoined) -> CommentDto {
    CommentDto {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        parent_id: row.parent_id.map(|v| v.to_string()),
        path: row.path.clone().unwrap_or_default(),
        author_handle: row.author_handle.clone(),
        body: row.body.clone(),
        vote_count: row.vote_count,
        created_at: row.created_at.timestamp(),
    }
}

fn board_to_dto(row: &crate::models::BoardRow) -> BoardDto {
    BoardDto { id: row.id.to_string(), slug: row.slug.clone(), name: row.name.clone() }
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/forum/boards — public
pub async fn list_boards(State(state): State<AppState>) -> AppResult<Json<Vec<BoardDto>>> {
    let rows = repo::list_boards(&state.db).await?;
    Ok(Json(rows.iter().map(board_to_dto).collect()))
}

/// GET /api/v2/forum/boards/{board_id}/threads — public
pub async fn list_threads(
    State(state): State<AppState>,
    Path(board_id_str): Path<String>,
    Query(q): Query<ThreadListQuery>,
) -> AppResult<Json<Page<ThreadDto>>> {
    let board_id: i64 = board_id_str.parse().map_err(|_| AppError::NotFound)?;
    let c = q.cursor.as_deref().unwrap_or("");
    let cache_id = format!("{board_id}:{}:{}", q.sort, c);
    let page = cached_json(&state, "board", &cache_id, 60, async {
        let (rows, next_cursor) =
            repo::list_threads(&state.db, board_id, &q.sort, q.cursor.as_deref(), q.limit).await?;
        let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        Ok::<_, AppError>(Page::new(items, next_cursor))
    })
    .await?;
    Ok(Json(page))
}

/// GET /api/v2/forum/threads/{id} — public
pub async fn get_thread(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> AppResult<Json<ThreadDetailDto>> {
    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let detail = cached_json(&state, "thread", &id.to_string(), 120, async {
        let row = repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;
        Ok::<_, AppError>(thread_to_detail_dto(&row))
    })
    .await?;
    Ok(Json(detail))
}

/// POST /api/v2/forum/threads — auth required
pub async fn create_thread(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ThreadInput>,
) -> AppResult<Json<ThreadDetailDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;

    let board_id: i64 = body.board_id.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::create_thread(&state.db, board_id, auth.id, &body).await?;

    // Bump board cache version.
    shared::cache::bump_version_opt(state.redis.as_ref(), "board", &board_id.to_string())
        .await
        .ok();

    Ok(Json(thread_to_detail_dto(&row)))
}

/// GET /api/v2/forum/threads/{thread_id}/comments — public
pub async fn list_comments(
    State(state): State<AppState>,
    Path(thread_id_str): Path<String>,
    Query(q): Query<CommentListQuery>,
) -> AppResult<Json<Page<CommentDto>>> {
    let thread_id: i64 = thread_id_str.parse().map_err(|_| AppError::NotFound)?;
    let (rows, next_cursor) =
        repo::list_comments(&state.db, thread_id, q.cursor.as_deref(), q.limit).await?;
    let items: Vec<CommentDto> = rows.iter().map(comment_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/forum/threads/{thread_id}/comments — auth required
pub async fn create_comment(
    State(state): State<AppState>,
    Path(thread_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CommentInput>,
) -> AppResult<Json<CommentDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;

    let thread_id: i64 = thread_id_str.parse().map_err(|_| AppError::NotFound)?;

    let parent_id: Option<i64> = body
        .parent_id
        .as_deref()
        .map(|s| s.parse::<i64>().map_err(|_| AppError::BadRequest("invalid parentId".into())))
        .transpose()?;

    let row = repo::create_comment(&state.db, thread_id, auth.id, &body.body, parent_id).await?;

    // Bump thread cache version.
    shared::cache::bump_version_opt(state.redis.as_ref(), "thread", &thread_id.to_string())
        .await
        .ok();

    Ok(Json(comment_to_dto(&row)))
}

/// POST /api/v2/forum/posts/{post_id}/vote — auth required
pub async fn vote_post(
    State(state): State<AppState>,
    Path(post_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<VoteInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;

    let _post_id: i64 = post_id_str.parse().map_err(|_| AppError::NotFound)?;
    let ok = repo::vote_post(&state.db, _post_id, &body.value).await?;
    if !ok {
        return Err(AppError::NotFound);
    }
    let _ = auth;

    // Fire-and-forget Redis counter update.
    if let Some(ref redis) = state.redis {
        if let Ok(mut conn) = redis.get().await {
            let key = format!("counters:post:{}:votes", _post_id);
            let _ = redis::cmd("INCRBY")
                .arg(&key)
                .arg(if body.value == "up" { 1 } else { -1 })
                .query_async::<()>(&mut conn)
                .await;
        }
    }

    Ok(Json(serde_json::json!({"ok": true})))
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

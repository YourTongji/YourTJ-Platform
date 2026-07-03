use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{RevisionDto, ThreadDetailDto, ThreadDto, ThreadInput, ThreadUpdateInput};
use crate::repo;

use super::{default_limit, default_sort, thread_to_detail_dto, thread_to_dto};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadFeedQuery {
    pub board: Option<String>,
    #[serde(default = "default_sort")]
    pub sort: String,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/forum/boards/{board_id}/threads — public
pub async fn list_threads(
    State(state): State<AppState>,
    Path(board_id_str): Path<String>,
    Query(q): Query<ThreadListQuery>,
) -> AppResult<Json<Page<ThreadDto>>> {
    let board_id: i64 = board_id_str.parse().map_err(|_| AppError::NotFound)?;
    let c = q.cursor.as_deref().unwrap_or("");
    let cache_id = format!("{board_id}:{}:{}", q.sort, c);
    let page = shared::cache::cached_json(state.redis.as_ref(), "board", &cache_id, 60, async {
        let (rows, next_cursor) =
            repo::list_threads(&state.db, board_id, &q.sort, q.cursor.as_deref(), q.limit).await?;
        let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        Ok::<_, AppError>(Page::new(items, next_cursor))
    })
    .await?;
    Ok(Json(page))
}

/// GET /api/v2/forum/threads — global feed (optional board filter)
pub async fn list_threads_feed(
    State(state): State<AppState>,
    Query(q): Query<ThreadFeedQuery>,
) -> AppResult<Json<Page<ThreadDto>>> {
    let board_id: Option<i64> = q.board.as_deref().and_then(|b| b.parse::<i64>().ok());
    let cache_id = format!("feed:{}:{:?}:{}", q.sort, board_id, q.cursor.as_deref().unwrap_or(""));
    let page = shared::cache::cached_json(state.redis.as_ref(), "board", &cache_id, 60, async {
        let (rows, next_cursor) =
            repo::list_threads_feed(&state.db, board_id, &q.sort, q.cursor.as_deref(), q.limit)
                .await?;
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
    let detail =
        shared::cache::cached_json(state.redis.as_ref(), "thread", &id.to_string(), 120, async {
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
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let tl = crate::trust_levels::get_trust_level(state.redis.as_ref(), &state.db, auth.id).await?;
    if tl == 0 {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "thread_create_tl0",
            &auth.id.to_string(),
            2,
            86400,
        )
        .await?;
    } else {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "thread_create",
            &auth.id.to_string(),
            5,
            60,
        )
        .await?;
    }

    let board_id: i64 = body.board_id.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::create_thread(&state.db, board_id, auth.id, &body).await?;

    // Parse @mentions from body and notify mentioned users (fire-and-forget).
    if let Some(ref body_text) = body.body {
        let mention_re = regex::Regex::new(r"@([\p{L}\p{N}_-]+)").unwrap();
        let handles: Vec<String> = mention_re
            .captures_iter(body_text)
            .map(|c| c[1].to_string())
            .filter(|h| {
                let _ = &auth;
                true
            }) // self-mention filter placeholder
            .take(10)
            .collect();

        if !handles.is_empty() {
            let pool = state.db.clone();
            let _ = &auth; // auth used for ownership
            let thread_author = String::new(); // placeholder: will resolve from DB
            let thread_body = row.body.clone().unwrap_or_default();
            let thread_body_excerpt = thread_body.chars().take(100).collect::<String>();
            let thread_id_val = row.id;
            tokio::spawn(async move {
                for handle in handles {
                    let account_id: Option<i64> =
                        sqlx::query_scalar("SELECT id FROM identity.accounts WHERE handle = $1")
                            .bind(&handle)
                            .fetch_optional(&pool)
                            .await
                            .unwrap_or(None);
                    if let Some(aid) = account_id {
                        crate::notification_hooks::create_notification(
                            &pool,
                            aid,
                            "mention",
                            serde_json::json!({
                                "threadId": thread_id_val.to_string(),
                                "authorHandle": thread_author,
                                "handle": handle,
                                "bodyExcerpt": thread_body_excerpt,
                            }),
                        )
                        .await;
                    }
                }
            });
        }
    }

    // Sync to Meilisearch (fire-and-forget).
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let thread_id = row.id;
    let thread_title = row.title.clone();
    let thread_body = row.body.clone().unwrap_or_default();
    let thread_author = row.author_handle.clone();
    let thread_reply = row.reply_count;
    let thread_vote = row.vote_count;
    let thread_status = row.status.clone();
    let thread_created = row.created_at.timestamp();
    tokio::spawn(async move {
        crate::meili::sync_thread_to_meili(
            &meili_url,
            &meili_key,
            &crate::meili::ForumThreadDoc {
                id: thread_id.to_string(),
                title: thread_title,
                body_excerpt: thread_body.chars().take(2048).collect(),
                board: String::new(),
                tags: vec![],
                author_handle: thread_author,
                reply_count: thread_reply,
                vote_count: thread_vote,
                created_at: thread_created,
                status: thread_status,
            },
        )
        .await;
    });

    // Bump board cache version.
    shared::cache::bump_version_opt(state.redis.as_ref(), "board", &board_id.to_string())
        .await
        .ok();

    Ok(Json(thread_to_detail_dto(&row)))
}

/// PATCH /api/v2/forum/threads/{id} — auth required (author or mod)
pub async fn update_thread(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ThreadUpdateInput>,
) -> AppResult<Json<ThreadDetailDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "edit",
        &auth.id.to_string(),
        10,
        60,
    )
    .await?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Check ownership or mod
    let thread = repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;
    if thread.author_id != auth.id && auth.role != "mod" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    // Check grace period: edits within 5 min don't create a revision
    let now = chrono::Utc::now();
    let within_grace = thread.created_at > now - chrono::Duration::minutes(5);

    // Save revision if outside grace period and body/title changed
    if !within_grace {
        if body.title.is_some() || body.body.is_some() {
            let old_title = Some(thread.title.as_str());
            let old_body = thread.body.as_deref().unwrap_or("");
            repo::create_revision(&state.db, "thread", id, auth.id, old_title, old_body).await?;
        }
    }

    // Update the thread
    let row =
        repo::update_thread(&state.db, id, body.title.as_deref(), body.body.as_deref()).await?;

    // Re-sync to Meilisearch (fire-and-forget).
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let thread_id = row.id;
    let thread_title = row.title.clone();
    let thread_body = row.body.clone().unwrap_or_default();
    let thread_author = row.author_handle.clone();
    let thread_reply = row.reply_count;
    let thread_vote = row.vote_count;
    let thread_status = row.status.clone();
    let thread_created = row.created_at.timestamp();
    tokio::spawn(async move {
        crate::meili::sync_thread_to_meili(
            &meili_url,
            &meili_key,
            &crate::meili::ForumThreadDoc {
                id: thread_id.to_string(),
                title: thread_title,
                body_excerpt: thread_body.chars().take(2048).collect(),
                board: String::new(),
                tags: vec![],
                author_handle: thread_author,
                reply_count: thread_reply,
                vote_count: thread_vote,
                created_at: thread_created,
                status: thread_status,
            },
        )
        .await;
    });

    // Bump cache
    shared::cache::bump_version_silent(state.redis.as_ref(), "thread", &id.to_string()).await;

    Ok(Json(thread_to_detail_dto(&row)))
}

/// DELETE /api/v2/forum/threads/{id} — auth required (author only)
pub async fn delete_thread(
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
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let thread = repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if thread.author_id != auth.id {
        return Err(AppError::Forbidden);
    }

    sqlx::query(
        "UPDATE forum.threads SET deleted_at = now(), deleted_by = $1 WHERE id = $2 AND deleted_at IS NULL",
    )
    .bind(auth.id)
    .bind(id)
    .execute(&state.db)
    .await?;

    // Delete from Meilisearch (fire-and-forget).
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let thread_id = id;
    tokio::spawn(async move {
        crate::meili::delete_thread_from_meili(&meili_url, &meili_key, thread_id).await;
    });

    shared::cache::bump_version_silent(state.redis.as_ref(), "board", &thread.board_id.to_string())
        .await;

    Ok(Json(json!({"ok": true})))
}

/// GET /api/v2/forum/threads/{id}/revisions — auth required (author + mod)
pub async fn list_thread_revisions(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<RevisionDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let thread = repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if thread.author_id != auth.id && auth.role != "mod" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let revs = repo::list_revisions(&state.db, "thread", id).await?;
    let dtos: Vec<RevisionDto> = revs
        .into_iter()
        .map(|r| RevisionDto {
            id: r.id.to_string(),
            seq: r.seq,
            editor_id: r.editor_id.to_string(),
            old_title: r.old_title,
            old_body: r.old_body,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(dtos))
}

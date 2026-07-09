use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{
    PollDto, PollOptionDto, RevisionDto, ThreadDetailDto, ThreadDto, ThreadInput, ThreadUpdateInput,
};
use crate::repo;
use crate::repo::base64_encode_i64;

use super::{default_limit, default_sort, thread_to_detail_dto, thread_to_dto};

/// Load poll data for a thread and attach it to a `ThreadDetailDto`.
async fn attach_poll_to_detail(pool: &sqlx::PgPool, thread_id: i64, dto: &mut ThreadDetailDto) {
    let poll_result = repo::get_poll(pool, thread_id).await;
    if let Ok(Some(poll_with_opts)) = poll_result {
        let option_dtos: Vec<PollOptionDto> = poll_with_opts
            .options
            .into_iter()
            .map(|o| PollOptionDto {
                id: o.id.to_string(),
                label: o.label,
                vote_count: o.vote_count,
                position: o.position,
            })
            .collect();
        dto.poll = Some(PollDto {
            id: poll_with_opts.poll.id.to_string(),
            question: poll_with_opts.poll.question,
            multi_select: poll_with_opts.poll.multi_select,
            closes_at: poll_with_opts.poll.closes_at.map(|v| v.timestamp()),
            options: option_dtos,
            my_votes: vec![],
        });
    }
}

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
    headers: HeaderMap,
    Path(board_id_str): Path<String>,
    Query(q): Query<ThreadListQuery>,
) -> AppResult<Json<Page<ThreadDto>>> {
    let board_id: i64 = board_id_str.parse().map_err(|_| AppError::NotFound)?;

    // Try auth — if the user is logged in, filter out ignored authors.
    let current_user_id: Option<i64> = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .ok()
    .map(|a| a.id);

    let c = q.cursor.as_deref().unwrap_or("");
    // Only use the cache for unauthenticated requests.
    if current_user_id.is_some() {
        let (rows, next_cursor) = repo::list_threads(
            &state.db,
            board_id,
            &q.sort,
            q.cursor.as_deref(),
            q.limit,
            current_user_id,
        )
        .await?;
        let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        Ok(Json(Page::new(items, next_cursor)))
    } else {
        let cache_id = format!("{board_id}:{}:{}", q.sort, c);
        let page =
            shared::cache::cached_json(state.redis.as_ref(), "board", &cache_id, 60, async {
                let (rows, next_cursor) = repo::list_threads(
                    &state.db,
                    board_id,
                    &q.sort,
                    q.cursor.as_deref(),
                    q.limit,
                    None,
                )
                .await?;
                let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
                Ok::<_, AppError>(Page::new(items, next_cursor))
            })
            .await?;
        Ok(Json(page))
    }
}

/// GET /api/v2/forum/threads — global feed (optional board filter, sort=following)
pub async fn list_threads_feed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ThreadFeedQuery>,
) -> AppResult<Json<Page<ThreadDto>>> {
    if q.sort == "unread" {
        let auth = identity::auth_middleware::authenticate(
            &headers,
            &state.db,
            &state.jwt_secret,
            state.redis.as_ref(),
        )
        .await
        .map_err(|_r| AppError::Unauthorized)?;

        let cursor: Option<i64> = q.cursor.and_then(|c| c.parse().ok());
        let (_rows, next_cursor) =
            repo::get_unread_thread_ids(&state.db, auth.id, q.limit, cursor).await?;

        let mut items: Vec<ThreadDto> = Vec::new();
        for (thread_id, unread_count) in &_rows {
            if let Ok(Some(row)) = repo::find_thread(&state.db, *thread_id).await {
                items.push(ThreadDto {
                    id: row.id.to_string(),
                    board_id: row.board_id.to_string(),
                    author_handle: row.author_handle,
                    title: row.title,
                    reply_count: row.reply_count,
                    vote_count: row.vote_count,
                    hot_score: row.hot_score,
                    tags: vec![],
                    created_at: row.created_at.timestamp(),
                    last_activity_at: row.last_activity_at.timestamp(),
                    unread_count: Some(*unread_count),
                });
            }
        }
        let next_str = next_cursor.map(|c| c.to_string());
        return Ok(Json(Page::new(items, next_str)));
    }

    if q.sort == "hot" && q.board.is_none() && q.cursor.is_none() {
        // G6: Try Redis ZSET first for global hot feed (no board filter, no cursor).
        if let Some(ref redis_pool) = state.redis {
            if let Ok(mut conn) = redis_pool.get().await {
                let ids: Vec<i64> = redis::cmd("ZREVRANGE")
                    .arg("hot:threads")
                    .arg(0i64)
                    .arg(q.limit - 1)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or_default();

                if !ids.is_empty() {
                    let current_user_id: Option<i64> = identity::auth_middleware::authenticate(
                        &headers,
                        &state.db,
                        &state.jwt_secret,
                        state.redis.as_ref(),
                    )
                    .await
                    .ok()
                    .map(|a| a.id);

                    let rows = repo::fetch_threads_by_ids(&state.db, &ids, current_user_id).await?;
                    let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
                    let next = if ids.len() as i64 >= q.limit {
                        Some(base64_encode_i64(q.limit))
                    } else {
                        None
                    };
                    return Ok(Json(Page::new(items, next)));
                }
            }
        }
        // Fall through to DB if Redis is unavailable or ZSET is empty
    }

    if q.sort == "following" {
        let auth = identity::auth_middleware::authenticate(
            &headers,
            &state.db,
            &state.jwt_secret,
            state.redis.as_ref(),
        )
        .await
        .map_err(|_r| AppError::Unauthorized)?;

        let cursor: Option<i64> = q.cursor.and_then(|c| c.parse().ok());
        let (rows, next_cursor) =
            repo::list_threads_feed_following(&state.db, auth.id, cursor, q.limit).await?;
        let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        let next_str = next_cursor.map(|c| c.to_string());
        return Ok(Json(Page::new(items, next_str)));
    }

    let board_id: Option<i64> = q.board.as_deref().and_then(|b| b.parse::<i64>().ok());

    // Try auth — if the user is logged in, filter out ignored authors.
    let current_user_id: Option<i64> = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .ok()
    .map(|a| a.id);

    // Only use the cache for unauthenticated requests.
    if current_user_id.is_some() {
        let (rows, next_cursor) = repo::list_threads_feed(
            &state.db,
            board_id,
            &q.sort,
            q.cursor.as_deref(),
            q.limit,
            current_user_id,
        )
        .await?;
        let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        Ok(Json(Page::new(items, next_cursor)))
    } else {
        let cache_id =
            format!("feed:{}:{:?}:{}", q.sort, board_id, q.cursor.as_deref().unwrap_or(""));
        let page =
            shared::cache::cached_json(state.redis.as_ref(), "board", &cache_id, 60, async {
                let (rows, next_cursor) = repo::list_threads_feed(
                    &state.db,
                    board_id,
                    &q.sort,
                    q.cursor.as_deref(),
                    q.limit,
                    None,
                )
                .await?;
                let items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
                Ok::<_, AppError>(Page::new(items, next_cursor))
            })
            .await?;
        Ok(Json(page))
    }
}

/// GET /api/v2/forum/threads/{id} — public
pub async fn get_thread(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
) -> AppResult<Json<ThreadDetailDto>> {
    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Try soft auth — if the user is logged in, show their read position.
    let current_user_id: Option<i64> = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .ok()
    .map(|a| a.id);

    if let Some(user_id) = current_user_id {
        // Authenticated — skip the cache since the response is user-specific.
        let row = repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;
        let mut dto = thread_to_detail_dto(&row);
        attach_poll_to_detail(&state.db, id, &mut dto).await;
        dto.my_last_read_comment_id = repo::get_last_read_comment_id(&state.db, user_id, id)
            .await
            .ok()
            .flatten()
            .map(|v| v.to_string());
        Ok(Json(dto))
    } else {
        let detail = shared::cache::cached_json(
            state.redis.as_ref(),
            "thread",
            &id.to_string(),
            120,
            async {
                let row = repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;
                let mut dto = thread_to_detail_dto(&row);
                // Load poll data (best-effort — not all threads have polls).
                attach_poll_to_detail(&state.db, id, &mut dto).await;
                Ok::<_, AppError>(dto)
            },
        )
        .await?;
        Ok(Json(detail))
    }
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

    // Check that the account is not silenced
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

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

    // Check watched words on the body text (if provided).
    let watched_word_action: Option<String> = body.body.as_ref().and_then(|body_text| {
        crate::watched_words::check_watched_words(body_text).map(|(_matched, action)| action)
    });

    // Block action: reject before insert.
    if watched_word_action.as_deref() == Some("block") {
        return Err(AppError::BadRequest("content contains blocked words".into()));
    }

    let board_id: i64 = body.board_id.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::create_thread(&state.db, board_id, auth.id, &body).await?;

    // Store thread tags (request-scope, capped at 3).
    if let Some(ref tag_slugs) = body.tags {
        if !tag_slugs.is_empty() {
            let capped: Vec<String> = tag_slugs.iter().take(3).cloned().collect();
            let resolved = repo::resolve_tag_slugs(&state.db, &capped).await?;
            let tag_ids: Vec<i64> = resolved.into_iter().map(|(id, _)| id).collect();
            repo::set_thread_tags(&state.db, row.id, &tag_ids).await?;
        }
    }

    // Update user_stats: threads_created +1 (best-effort).
    let _ = sqlx::query(
        "INSERT INTO forum.user_stats (account_id, threads_created, last_posted_at) \
         VALUES ($1, 1, now()) \
         ON CONFLICT (account_id) \
         DO UPDATE SET threads_created = forum.user_stats.threads_created + 1, \
                       last_posted_at = now()",
    )
    .bind(auth.id)
    .execute(&state.db)
    .await;

    // Auto-track: creator automatically subscribes to their thread.
    let _ = crate::repo::set_subscription(&state.db, auth.id, "thread", row.id, "tracking").await;

    // Auto-award first-thread badge (fire-and-forget).
    let pool = state.db.clone();
    let author_id = auth.id;
    tokio::spawn(async move {
        match crate::badges::award_first_thread_badge(&pool, author_id).await {
            Ok(true) => tracing::info!(author_id, "first-thread badge awarded"),
            Ok(false) => {} // already had it or not first thread
            Err(e) => tracing::warn!(error = %e, author_id, "failed to award first-thread badge"),
        }
    });

    // Look up own handle for self-mention filtering.
    let my_handle: String =
        sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
            .bind(auth.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or_default();

    // Parse @mentions from body and notify mentioned users (fire-and-forget).
    if let Some(ref body_text) = body.body {
        let mention_re =
            regex::Regex::new(r"@([\p{L}\p{N}_-]+)").expect("mention regex is statically valid");
        let handles: Vec<String> = mention_re
            .captures_iter(body_text)
            .map(|c| c[1].to_string())
            .filter(|h| h != &my_handle) // skip self-mentions
            .take(10)
            .collect();

        let thread_actor_id = auth.id;
        if !handles.is_empty() {
            let pool = state.db.clone();
            let thread_author = row.author_handle.clone();
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
                            None,
                            Some(thread_actor_id),
                        )
                        .await;
                    }
                }
            });
        }
    }

    // Look up board slug for Meilisearch.
    let board_slug: String = sqlx::query_scalar("SELECT slug FROM forum.boards WHERE id = $1")
        .bind(board_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or_default();

    // Look up tag slugs for Meilisearch.
    let tag_slugs = crate::repo::get_thread_tag_slugs(&state.db, row.id).await.unwrap_or_default();

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
                board: board_slug,
                tags: tag_slugs,
                author_handle: thread_author,
                reply_count: thread_reply,
                vote_count: thread_vote,
                created_at: thread_created,
                status: thread_status,
            },
        )
        .await;
    });

    // Handle queue action: auto-hide the thread.
    if watched_word_action.as_deref() == Some("queue") {
        let _ = sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
            .bind(row.id)
            .execute(&state.db)
            .await;
    }

    // Create poll if provided.
    if let Some(ref poll_input) = body.poll {
        if poll_input.options.len() < 2 {
            return Err(AppError::BadRequest("poll requires at least 2 options".into()));
        }
        if poll_input.options.len() > 20 {
            return Err(AppError::BadRequest("poll cannot have more than 20 options".into()));
        }
        let closes_at = poll_input
            .closes_at
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or(chrono::Utc::now()));
        let _poll_id = repo::create_poll(
            &state.db,
            row.id,
            &poll_input.question,
            poll_input.multi_select,
            closes_at,
            &poll_input.options,
        )
        .await?;
    }

    // Build response DTO, applying censorship for censor action.
    let mut dto = thread_to_detail_dto(&row);

    // Load poll data into the response if a poll was created.
    if body.poll.is_some() {
        attach_poll_to_detail(&state.db, row.id, &mut dto).await;
    }

    if watched_word_action.as_deref() == Some("censor") {
        if let Some(ref body_text) = dto.body {
            dto.body = Some(crate::watched_words::censor_text(body_text));
        }
    }

    // Bump board cache version.
    shared::cache::bump_version_opt(state.redis.as_ref(), "board", &board_id.to_string())
        .await
        .ok();

    Ok(Json(dto))
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
    if !within_grace && (body.title.is_some() || body.body.is_some()) {
        let old_title = Some(thread.title.as_str());
        let old_body = thread.body.as_deref().unwrap_or("");
        repo::create_revision(&state.db, "thread", id, auth.id, old_title, old_body).await?;
    }

    // Update the thread
    let row =
        repo::update_thread(&state.db, id, body.title.as_deref(), body.body.as_deref()).await?;

    // Look up board slug for Meilisearch.
    let board_slug: String = sqlx::query_scalar("SELECT slug FROM forum.boards WHERE id = $1")
        .bind(row.board_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or_default();

    // Look up tag slugs for Meilisearch.
    let tag_slugs = crate::repo::get_thread_tag_slugs(&state.db, row.id).await.unwrap_or_default();

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
                board: board_slug,
                tags: tag_slugs,
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

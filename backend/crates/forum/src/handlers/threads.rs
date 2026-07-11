use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
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

pub(crate) async fn hydrate_thread_summaries(
    pool: &sqlx::PgPool,
    viewer_id: Option<i64>,
    threads: &mut [ThreadDto],
) -> AppResult<()> {
    let thread_ids =
        threads.iter().filter_map(|thread| thread.id.parse::<i64>().ok()).collect::<Vec<_>>();
    let (mut tags, mut excerpts, mut viewer_states) = tokio::try_join!(
        repo::get_thread_tag_slugs_batch(pool, &thread_ids),
        repo::get_thread_body_excerpts(pool, &thread_ids),
        async {
            match viewer_id {
                Some(account_id) => {
                    repo::get_post_viewer_states(pool, account_id, "thread", &thread_ids).await
                }
                None => Ok(std::collections::HashMap::new()),
            }
        },
    )?;
    for thread in threads {
        if let Ok(thread_id) = thread.id.parse::<i64>() {
            thread.tags = tags.remove(&thread_id).unwrap_or_default();
            thread.body_excerpt = excerpts
                .remove(&thread_id)
                .and_then(|excerpt| (!excerpt.is_empty()).then_some(excerpt));
            if let Some(state) = viewer_states.remove(&thread_id) {
                thread.viewer_vote = state.vote;
                thread.is_bookmarked = state.is_bookmarked;
            }
        }
    }
    Ok(())
}

/// Load poll data for a thread and attach it to a `ThreadDetailDto`.
pub(crate) async fn attach_poll_to_detail(
    pool: &sqlx::PgPool,
    thread_id: i64,
    account_id: Option<i64>,
    dto: &mut ThreadDetailDto,
) -> AppResult<()> {
    if let Some(poll_with_opts) = repo::get_poll(pool, thread_id).await? {
        let my_votes = match account_id {
            Some(account_id) => {
                repo::get_voted_option_ids(pool, poll_with_opts.poll.id, account_id)
                    .await?
                    .into_iter()
                    .map(|option_id| option_id.to_string())
                    .collect()
            }
            None => Vec::new(),
        };
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
            my_votes,
        });
    }
    Ok(())
}

/// Attach canonical tags and optional viewer-specific state to a thread detail.
pub(crate) async fn hydrate_thread_detail(
    pool: &sqlx::PgPool,
    thread_id: i64,
    account_id: Option<i64>,
    dto: &mut ThreadDetailDto,
) -> AppResult<()> {
    dto.tags = repo::get_thread_tag_slugs(pool, thread_id).await?;
    attach_poll_to_detail(pool, thread_id, account_id, dto).await?;
    if let Some(account_id) = account_id {
        let mut states =
            repo::get_post_viewer_states(pool, account_id, "thread", &[thread_id]).await?;
        if let Some(state) = states.remove(&thread_id) {
            dto.viewer_vote = state.vote;
            dto.is_bookmarked = state.is_bookmarked;
        }
        dto.my_last_read_comment_id = repo::get_last_read_comment_id(pool, account_id, thread_id)
            .await?
            .map(|comment_id| comment_id.to_string());
        dto.my_subscription_level =
            repo::get_thread_subscription(pool, account_id, thread_id).await?;
    }
    Ok(())
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
    tag: Option<String>,
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
    pub tag: Option<String>,
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

    if !matches!(q.sort.as_str(), "hot" | "new") {
        return Err(AppError::BadRequest("sort must be hot/new".into()));
    }
    let tag = q.tag.as_deref().map(str::trim).filter(|tag| !tag.is_empty());
    if tag.is_some_and(|tag| tag.chars().count() > 64) {
        return Err(AppError::BadRequest("tag must be 1–64 characters".into()));
    }
    if let Some(tag) = tag {
        let (rows, next_cursor) = repo::list_threads_by_tag(
            &state.db,
            Some(board_id),
            tag,
            &q.sort,
            q.cursor.as_deref(),
            q.limit,
            current_user_id,
            None,
        )
        .await?;
        let mut items = rows.iter().map(thread_to_dto).collect::<Vec<_>>();
        hydrate_thread_summaries(&state.db, current_user_id, &mut items).await?;
        return Ok(Json(Page::new(items, next_cursor)));
    }

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
        let mut items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        hydrate_thread_summaries(&state.db, current_user_id, &mut items).await?;
        Ok(Json(Page::new(items, next_cursor)))
    } else {
        let generation = crate::cache::board_generation(state.redis.as_ref(), board_id).await;
        let cache_id = format!("{board_id}:v{generation}:{}:{}", q.sort, c);
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
                let mut items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
                hydrate_thread_summaries(&state.db, None, &mut items).await?;
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
    if !matches!(q.sort.as_str(), "hot" | "new" | "subscriptions" | "following" | "unread") {
        return Err(AppError::BadRequest(
            "sort must be hot/new/subscriptions/following/unread".into(),
        ));
    }
    if !(1..=100).contains(&q.limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let board_id = q
        .board
        .as_deref()
        .map(|board| board.parse::<i64>().map_err(|_| AppError::BadRequest("invalid board".into())))
        .transpose()?;
    let tag = q.tag.as_deref().map(str::trim).filter(|tag| !tag.is_empty());
    if tag.is_some_and(|tag| tag.chars().count() > 64) {
        return Err(AppError::BadRequest("tag must be 1–64 characters".into()));
    }

    if q.sort == "unread" {
        let auth = identity::auth_middleware::authenticate(
            &headers,
            &state.db,
            &state.jwt_secret,
            state.redis.as_ref(),
        )
        .await
        .map_err(|_r| AppError::Unauthorized)?;

        let cursor = q
            .cursor
            .as_deref()
            .map(|cursor| {
                cursor.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into()))
            })
            .transpose()?;
        let (_rows, next_cursor) =
            repo::get_unread_thread_ids(&state.db, auth.id, board_id, tag, q.limit, cursor).await?;

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
            .collect::<Vec<_>>();
        hydrate_thread_summaries(&state.db, Some(auth.id), &mut items).await?;
        let next_str = next_cursor.map(|c| c.to_string());
        return Ok(Json(Page::new(items, next_str)));
    }

    if q.sort == "hot" && board_id.is_none() && tag.is_none() && q.cursor.is_none() {
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
                    let mut items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
                    hydrate_thread_summaries(&state.db, current_user_id, &mut items).await?;
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

    if q.sort == "subscriptions" {
        let auth = identity::auth_middleware::authenticate(
            &headers,
            &state.db,
            &state.jwt_secret,
            state.redis.as_ref(),
        )
        .await
        .map_err(|_r| AppError::Unauthorized)?;

        if let Some(tag) = tag {
            let (rows, next_cursor) = repo::list_threads_by_tag(
                &state.db,
                board_id,
                tag,
                "new",
                q.cursor.as_deref(),
                q.limit,
                Some(auth.id),
                Some(auth.id),
            )
            .await?;
            let mut items = rows.iter().map(thread_to_dto).collect::<Vec<_>>();
            hydrate_thread_summaries(&state.db, Some(auth.id), &mut items).await?;
            return Ok(Json(Page::new(items, next_cursor)));
        }
        let (rows, next_cursor) = repo::list_threads_feed_subscriptions(
            &state.db,
            auth.id,
            board_id,
            q.cursor.as_deref(),
            q.limit,
        )
        .await?;
        let mut items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        hydrate_thread_summaries(&state.db, Some(auth.id), &mut items).await?;
        return Ok(Json(Page::new(items, next_cursor)));
    }

    if q.sort == "following" {
        let auth = identity::auth_middleware::authenticate(
            &headers,
            &state.db,
            &state.jwt_secret,
            state.redis.as_ref(),
        )
        .await
        .map_err(|_| AppError::Unauthorized)?;
        let (rows, next_cursor) = repo::list_threads_feed_following(
            &state.db,
            auth.id,
            board_id,
            tag,
            q.cursor.as_deref(),
            q.limit,
        )
        .await?;
        let mut items = rows.iter().map(thread_to_dto).collect::<Vec<_>>();
        hydrate_thread_summaries(&state.db, Some(auth.id), &mut items).await?;
        return Ok(Json(Page::new(items, next_cursor)));
    }

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

    if let Some(tag) = tag {
        let (rows, next_cursor) = repo::list_threads_by_tag(
            &state.db,
            board_id,
            tag,
            &q.sort,
            q.cursor.as_deref(),
            q.limit,
            current_user_id,
            None,
        )
        .await?;
        let mut items = rows.iter().map(thread_to_dto).collect::<Vec<_>>();
        hydrate_thread_summaries(&state.db, current_user_id, &mut items).await?;
        return Ok(Json(Page::new(items, next_cursor)));
    }

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
        let mut items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
        hydrate_thread_summaries(&state.db, current_user_id, &mut items).await?;
        Ok(Json(Page::new(items, next_cursor)))
    } else {
        let generation = crate::cache::global_feed_generation(state.redis.as_ref()).await;
        let cache_id = format!(
            "v{generation}:{}:{:?}:{}",
            q.sort,
            board_id,
            q.cursor.as_deref().unwrap_or("")
        );
        let page =
            shared::cache::cached_json(state.redis.as_ref(), "forum-feed", &cache_id, 60, async {
                let (rows, next_cursor) = repo::list_threads_feed(
                    &state.db,
                    board_id,
                    &q.sort,
                    q.cursor.as_deref(),
                    q.limit,
                    None,
                )
                .await?;
                let mut items: Vec<ThreadDto> = rows.iter().map(thread_to_dto).collect();
                hydrate_thread_summaries(&state.db, None, &mut items).await?;
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
        hydrate_thread_detail(&state.db, id, Some(user_id), &mut dto).await?;
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
                hydrate_thread_detail(&state.db, id, None, &mut dto).await?;
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
) -> AppResult<(StatusCode, Json<ThreadDetailDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    let prepared = crate::content_policy::prepare_thread_create(body)?;
    let body = prepared.input;
    let is_queued = prepared.is_queued;
    // Check that the account is not silenced
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let board_id: i64 = body.board_id.parse().map_err(|_| AppError::NotFound)?;
    let tl = crate::trust_levels::get_trust_level(&state.db, auth.id).await?;
    let posting_actor = crate::repo::boards::BoardPostingActor {
        trust_level: tl,
        can_bypass_board_gates: auth.has_capability(shared::auth::Capability::ModerateContent),
    };
    crate::repo::boards::authorize_board_posting(&state.db, board_id, posting_actor).await?;
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

    let row =
        repo::create_thread(&state.db, board_id, auth.id, &body, is_queued, posting_actor).await?;

    if !is_queued {
        let pool = state.db.clone();
        let author_id = auth.id;
        tokio::spawn(async move {
            match crate::badges::award_first_thread_badge(&pool, author_id).await {
                Ok(true) => tracing::info!(author_id, "first-thread badge awarded"),
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(%error, author_id, "failed to award first-thread badge")
                }
            }
        });
    }

    if !is_queued {
        let my_handle: String =
            sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
                .bind(auth.id)
                .fetch_one(&state.db)
                .await
                .unwrap_or_default();
        if let Some(ref body_text) = body.body {
            let handles =
                crate::content_policy::mention_handles(body_text, body.content_format, &my_handle);

            let thread_actor_id = auth.id;
            if !handles.is_empty() {
                let pool = state.db.clone();
                let thread_author = row.author_handle.clone();
                let thread_body = row.body.clone().unwrap_or_default();
                let thread_body_excerpt = crate::content_policy::plain_text_projection(
                    &thread_body,
                    crate::dto::ContentFormat::from_db(&row.content_format),
                    100,
                );
                let thread_id_val = row.id;
                tokio::spawn(async move {
                    for handle in handles {
                        let account_id: Option<i64> = sqlx::query_scalar(
                            "SELECT id FROM identity.accounts WHERE handle = $1",
                        )
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
    }

    if !is_queued {
        let pool = state.db.clone();
        let meili_url = state.meili_url.clone();
        let meili_key = state.meili_master_key.clone();
        let thread_id = row.id;
        tokio::spawn(async move {
            if let Err(error) =
                crate::meili::reconcile_thread_in_meili(&pool, &meili_url, &meili_key, thread_id)
                    .await
            {
                tracing::warn!(%error, thread_id, "failed to reconcile created thread in search");
            }
        });
    }

    let mut dto = thread_to_detail_dto(&row);
    hydrate_thread_detail(&state.db, row.id, Some(auth.id), &mut dto).await?;

    crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), row.id, board_id).await;

    Ok((StatusCode::CREATED, Json(dto)))
}

/// PATCH /api/v2/forum/threads/{id} — auth required (author only)
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
    let prepared = crate::content_policy::prepare_thread_update(body)?;
    let body = prepared.input;
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "edit",
        &auth.id.to_string(),
        10,
        60,
    )
    .await?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    let row = repo::update_thread(&state.db, id, auth.id, &body, prepared.is_queued).await?;

    // Re-read canonical visibility and content inside the background index sync.
    let pool = state.db.clone();
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let thread_id = row.id;
    tokio::spawn(async move {
        if let Err(error) =
            crate::meili::reconcile_thread_in_meili(&pool, &meili_url, &meili_key, thread_id).await
        {
            tracing::warn!(%error, thread_id, "failed to reconcile edited thread in search");
        }
    });

    crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), id, row.board_id).await;

    let mut dto = thread_to_detail_dto(&row);
    hydrate_thread_detail(&state.db, row.id, Some(auth.id), &mut dto).await?;
    Ok(Json(dto))
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
    let mut tx = state.db.begin().await?;
    let (author_id, board_id): (Option<i64>, i64) = sqlx::query_as(
        "SELECT author_id, board_id FROM forum.threads \
         WHERE id = $1 AND deleted_at IS NULL AND hidden_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if author_id != Some(auth.id) {
        return Err(AppError::Forbidden);
    }
    let affected_board_ids =
        crate::repo::boards::lock_boards_for_thread_count(&mut tx, &[board_id]).await?;
    sqlx::query(
        "UPDATE forum.threads SET deleted_at = now(), deleted_by = $1 \
         WHERE id = $2",
    )
    .bind(auth.id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    activity::contributions::deactivate_contribution(
        &mut tx,
        &format!("forum_thread:{id}"),
        chrono::Utc::now(),
    )
    .await?;
    crate::repo::deactivate_target_vote_contributions(&mut tx, "thread", id, chrono::Utc::now())
        .await?;
    crate::repo::boards::refresh_board_thread_counts(&mut tx, &affected_board_ids).await?;
    tx.commit().await?;

    // Delete from Meilisearch (fire-and-forget).
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    let thread_id = id;
    tokio::spawn(async move {
        if let Err(error) =
            crate::meili::delete_thread_from_meili(&meili_url, &meili_key, thread_id).await
        {
            tracing::warn!(%error, thread_id, "failed to remove deleted thread from search");
        }
    });

    crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), id, board_id).await;

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
            old_content_format: crate::dto::ContentFormat::from_db(&r.old_content_format),
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(dtos))
}

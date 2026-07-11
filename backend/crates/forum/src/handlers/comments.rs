use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{CommentDto, CommentInput, CommentUpdateInput, RevisionDto};
use crate::repo;

use super::{comment_to_dto, default_limit};

// ---------------------------------------------------------------------------
// query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/forum/threads/{thread_id}/comments — public
pub async fn list_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(thread_id_str): Path<String>,
    Query(q): Query<CommentListQuery>,
) -> AppResult<Json<Page<CommentDto>>> {
    let thread_id: i64 = thread_id_str.parse().map_err(|_| AppError::NotFound)?;

    // Try auth — if the user is logged in, filter out ignored authors.
    let current_account = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .ok();
    let current_user_id = current_account.as_ref().map(|account| account.id);
    let can_view_moderated_parent = current_account
        .as_ref()
        .is_some_and(|account| account.has_capability(shared::auth::Capability::ModerateContent));

    let (rows, next_cursor) = repo::list_comments(
        &state.db,
        thread_id,
        q.cursor.as_deref(),
        q.limit,
        current_user_id,
        can_view_moderated_parent,
    )
    .await?;

    let solved_comment_id: Option<i64> =
        sqlx::query_scalar("SELECT solved_answer_id FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_optional(&state.db)
            .await?
            .flatten();

    let items: Vec<CommentDto> =
        rows.iter().map(|r| comment_to_dto(r, solved_comment_id)).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/forum/threads/{thread_id}/comments — auth required
pub async fn create_comment(
    State(state): State<AppState>,
    Path(thread_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CommentInput>,
) -> AppResult<(StatusCode, Json<CommentDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    let body_length = body.body.chars().count();
    if body.body.trim().is_empty() || body_length > 16_000 {
        return Err(AppError::BadRequest("body must be 1–16000 characters".into()));
    }
    // Check that the account is not silenced
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let tl = crate::trust_levels::get_trust_level(state.redis.as_ref(), &state.db, auth.id).await?;
    if tl == 0 {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "comment_create_tl0",
            &auth.id.to_string(),
            5,
            86400,
        )
        .await?;
    } else {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "comment_create",
            &auth.id.to_string(),
            20,
            60,
        )
        .await?;
    }

    // Check watched words on the body text.
    let watched_word_action: Option<String> =
        crate::watched_words::check_watched_words(&body.body).map(|(_matched, action)| action);
    let is_queued = watched_word_action.as_deref() == Some("queue");

    // Block action: reject before insert.
    if watched_word_action.as_deref() == Some("block") {
        return Err(AppError::BadRequest("content contains blocked words".into()));
    }

    let thread_id: i64 = thread_id_str.parse().map_err(|_| AppError::NotFound)?;

    let parent_id: Option<i64> = body
        .parent_id
        .as_deref()
        .map(|s| s.parse::<i64>().map_err(|_| AppError::BadRequest("invalid parentId".into())))
        .transpose()?;

    let quoted_comment_id: Option<i64> = body
        .quoted_comment_id
        .as_deref()
        .map(|s| {
            s.parse::<i64>().map_err(|_| AppError::BadRequest("invalid quotedCommentId".into()))
        })
        .transpose()?;

    let row = repo::create_comment(
        &state.db,
        thread_id,
        auth.id,
        &body.body,
        parent_id,
        quoted_comment_id,
        is_queued,
    )
    .await?;

    // Update user_stats: comments_created +1 (best-effort).
    let _ = sqlx::query(
        "INSERT INTO forum.user_stats (account_id, comments_created, last_posted_at) \
         VALUES ($1, 1, now()) \
         ON CONFLICT (account_id) \
         DO UPDATE SET comments_created = forum.user_stats.comments_created + 1, \
                       last_posted_at = now()",
    )
    .bind(auth.id)
    .execute(&state.db)
    .await;

    // Auto-award first-comment badge (fire-and-forget).
    let pool = state.db.clone();
    let comment_author_id = auth.id;
    tokio::spawn(async move {
        match crate::badges::award_first_comment_badge(&pool, comment_author_id).await {
            Ok(true) => {
                tracing::info!(author_id = comment_author_id, "first-comment badge awarded")
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(error = %e, author_id = comment_author_id, "failed to award first-comment badge")
            }
        }
    });

    // Auto-track: commenter automatically follows the thread.
    let _ =
        crate::repo::set_subscription(&state.db, auth.id, "thread", thread_id, "tracking").await;

    // Look up thread author for notification.
    let thread_author_id: i64 =
        sqlx::query_scalar("SELECT author_id FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&state.db)
            .await?;

    // Notify thread author of reply (fire-and-forget).
    if row.author_id != thread_author_id {
        let pool = state.db.clone();
        let actor_id = auth.id;
        let payload = serde_json::json!({
            "threadId": thread_id.to_string(),
            "commentId": row.id.to_string(),
            "authorHandle": row.author_handle,
            "bodyExcerpt": row.body.chars().take(100).collect::<String>(),
        });
        tokio::spawn(async move {
            crate::notification_hooks::create_notification(
                &pool,
                thread_author_id,
                "reply",
                payload,
                None,
                Some(actor_id),
            )
            .await;
        });
    }

    // Notify watching subscribers (fire-and-forget), excluding thread author and commenter.
    {
        let pool = state.db.clone();
        let watcher_exclude = vec![thread_author_id, auth.id];
        let watching_payload = serde_json::json!({
            "threadId": thread_id.to_string(),
            "commentId": row.id.to_string(),
            "authorHandle": row.author_handle,
            "bodyExcerpt": row.body.chars().take(100).collect::<String>(),
        });
        let comment_actor_id = auth.id;
        tokio::spawn(async move {
            let watcher_ids =
                match crate::repo::get_watching_subscriber_ids(&pool, thread_id, &watcher_exclude)
                    .await
                {
                    Ok(ids) => ids,
                    Err(e) => {
                        tracing::warn!(error = %e, thread_id, "failed to get watching subscribers");
                        return;
                    }
                };
            for watcher_id in watcher_ids {
                if !crate::notification_hooks::is_notification_enabled(
                    &pool, watcher_id, "watching",
                )
                .await
                {
                    continue;
                }
                crate::notification_hooks::create_notification(
                    &pool,
                    watcher_id,
                    "watching",
                    watching_payload.clone(),
                    None,
                    Some(comment_actor_id),
                )
                .await;
            }
        });
    }

    // Notify quoted comment author (fire-and-forget).
    if let Some(qcid) = quoted_comment_id {
        let quoted_author_id: Option<i64> =
            sqlx::query_scalar("SELECT author_id FROM forum.comments WHERE id = $1")
                .bind(qcid)
                .fetch_optional(&state.db)
                .await?;
        if let Some(qaid) = quoted_author_id {
            if qaid != auth.id {
                let pool = state.db.clone();
                let qcid_str = qcid.to_string();
                let thread_id_str = thread_id.to_string();
                let author_handle = row.author_handle.clone();
                let new_comment_id = row.id;
                let actor_id = auth.id;
                tokio::spawn(async move {
                    if !crate::notification_hooks::is_notification_enabled(&pool, qaid, "quote")
                        .await
                    {
                        return;
                    }
                    crate::notification_hooks::create_notification(
                        &pool,
                        qaid,
                        "quote",
                        serde_json::json!({
                            "threadId": thread_id_str,
                            "commentId": new_comment_id.to_string(),
                            "quotedCommentId": qcid_str,
                            "authorHandle": author_handle,
                        }),
                        None,
                        Some(actor_id),
                    )
                    .await;
                });
            }
        }
    }

    // Look up own handle for self-mention filtering.
    let my_handle: String =
        sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
            .bind(auth.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or_default();

    // Parse @mentions from body and notify mentioned users (fire-and-forget).
    let mention_re =
        regex::Regex::new(r"@([\p{L}\p{N}_-]+)").expect("mention regex is statically valid");
    let handles: Vec<String> = mention_re
        .captures_iter(&body.body)
        .map(|c| c[1].to_string())
        .filter(|h| h != &my_handle) // skip self-mentions
        .take(10)
        .collect();

    let comment_actor_id = auth.id;
    if !handles.is_empty() {
        let pool = state.db.clone();
        let comment_author = row.author_handle.clone();
        let comment_body = row.body.chars().take(100).collect::<String>();
        let thread_id_val = thread_id;
        let comment_id_val = row.id;
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
                            "commentId": comment_id_val.to_string(),
                            "authorHandle": comment_author,
                            "handle": handle,
                            "bodyExcerpt": comment_body,
                        }),
                        None,
                        Some(comment_actor_id),
                    )
                    .await;
                }
            }
        });
    }

    // Build response DTO, applying censorship for censor action.
    let mut dto = comment_to_dto(&row, None);
    if watched_word_action.as_deref() == Some("censor") {
        dto.body = crate::watched_words::censor_text(&dto.body);
    }

    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, thread_id).await;

    Ok((StatusCode::CREATED, Json(dto)))
}

/// PATCH /api/v2/forum/comments/{id} — auth required (author only)
pub async fn update_comment(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CommentUpdateInput>,
) -> AppResult<Json<CommentDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Staff moderate through reasoned admin actions; they never overwrite user speech.
    let comment = repo::find_comment(&state.db, id).await?.ok_or(AppError::NotFound)?;
    if comment.author_id != auth.id {
        return Err(AppError::Forbidden);
    }

    // Check grace period: edits within 5 min don't create a revision
    let now = chrono::Utc::now();
    let within_grace = comment.created_at > now - chrono::Duration::minutes(5);

    // Save revision if outside grace period
    if !within_grace {
        let old_body = comment.body.clone();
        repo::create_revision(&state.db, "comment", id, auth.id, None, &old_body).await?;
    }

    // Update the comment
    let row = repo::update_comment(&state.db, id, &body.body).await?;

    // Fetch solved_answer_id for Q&A solved-answer marking
    let solved_comment_id: Option<i64> =
        sqlx::query_scalar("SELECT solved_answer_id FROM forum.threads WHERE id = $1")
            .bind(row.thread_id)
            .fetch_optional(&state.db)
            .await?
            .flatten();

    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, row.thread_id).await;

    Ok(Json(comment_to_dto(&row, solved_comment_id)))
}

/// DELETE /api/v2/forum/comments/{id} — auth required (author only)
pub async fn delete_comment(
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
    let comment = repo::find_comment(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if comment.author_id != auth.id {
        return Err(AppError::Forbidden);
    }

    let mut tx = state.db.begin().await?;
    let deleted_thread_id: Option<i64> = sqlx::query_scalar(
        "UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 \
         WHERE id = $2 AND author_id = $1 AND deleted_at IS NULL RETURNING thread_id",
    )
    .bind(auth.id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let deleted_thread_id =
        deleted_thread_id.ok_or_else(|| AppError::Conflict("comment is already deleted".into()))?;
    sqlx::query(
        "UPDATE forum.threads SET reply_count = GREATEST(reply_count - 1, 0) WHERE id = $1",
    )
    .bind(deleted_thread_id)
    .execute(&mut *tx)
    .await?;
    activity::contributions::deactivate_contribution(
        &mut tx,
        &format!("forum_comment:{id}"),
        chrono::Utc::now(),
    )
    .await?;
    crate::repo::deactivate_target_vote_contributions(&mut tx, "comment", id, chrono::Utc::now())
        .await?;
    tx.commit().await?;

    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, comment.thread_id).await;

    Ok(Json(json!({"ok": true})))
}

/// GET /api/v2/forum/comments/{id}/revisions — auth required (author + mod)
pub async fn list_comment_revisions(
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
    let comment = repo::find_comment(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if comment.author_id != auth.id && auth.role != "mod" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let revs = repo::list_revisions(&state.db, "comment", id).await?;
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

/// POST /api/v2/forum/comments/{id}/solve — auth required (thread author or mod)
///
/// Mark a comment as the solved answer on a Q&A board. Cannot solve own comment.
pub async fn mark_solved_handler(
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

    let comment_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Find the comment and its thread
    let comment = repo::find_comment(&state.db, comment_id).await?.ok_or(AppError::NotFound)?;

    let thread =
        repo::find_thread(&state.db, comment.thread_id).await?.ok_or(AppError::NotFound)?;

    // Find the board to check is_qa
    let board =
        crate::repo::find_board(&state.db, thread.board_id).await?.ok_or(AppError::NotFound)?;

    if !board.is_qa {
        return Err(AppError::BadRequest("board is not a Q&A board".into()));
    }

    // Verify permission: must be thread author or mod/admin
    let is_mod = auth.role == "mod" || auth.role == "admin";
    if thread.author_id != auth.id && !is_mod {
        return Err(AppError::Forbidden);
    }

    // Cannot solve own comment
    if comment.author_id == auth.id {
        return Err(AppError::BadRequest("cannot mark own comment as the solved answer".into()));
    }

    repo::set_solved_answer(&state.db, comment.thread_id, comment_id).await?;

    // Log mod action if a mod/admin did this
    if is_mod {
        repo::insert_mod_action(
            &state.db,
            auth.id,
            "solve",
            "comment",
            comment_id,
            None,
            Some(&serde_json::json!({
                "threadId": comment.thread_id.to_string(),
            })),
        )
        .await?;
    }

    shared::cache::bump_version_silent(
        state.redis.as_ref(),
        "thread",
        &comment.thread_id.to_string(),
    )
    .await;

    Ok(Json(json!({"ok": true})))
}

/// DELETE /api/v2/forum/comments/{id}/solve — auth required (thread author or mod)
///
/// Unmark the solved answer on a Q&A board.
pub async fn unmark_solved_handler(
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

    let comment_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Find the comment and its thread
    let comment = repo::find_comment(&state.db, comment_id).await?.ok_or(AppError::NotFound)?;

    let thread =
        repo::find_thread(&state.db, comment.thread_id).await?.ok_or(AppError::NotFound)?;

    // Verify permission: must be thread author or mod/admin
    let is_mod = auth.role == "mod" || auth.role == "admin";
    if thread.author_id != auth.id && !is_mod {
        return Err(AppError::Forbidden);
    }

    repo::clear_solved_answer(&state.db, comment.thread_id).await?;

    // Log mod action if a mod/admin did this
    if is_mod {
        repo::insert_mod_action(
            &state.db,
            auth.id,
            "unsolve",
            "comment",
            comment_id,
            None,
            Some(&serde_json::json!({
                "threadId": comment.thread_id.to_string(),
            })),
        )
        .await?;
    }

    shared::cache::bump_version_silent(
        state.redis.as_ref(),
        "thread",
        &comment.thread_id.to_string(),
    )
    .await;

    Ok(Json(json!({"ok": true})))
}

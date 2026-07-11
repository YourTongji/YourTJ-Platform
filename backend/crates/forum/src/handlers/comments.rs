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

async fn hydrate_comment_viewer_states(
    pool: &sqlx::PgPool,
    account_id: i64,
    comments: &mut [CommentDto],
) -> AppResult<()> {
    let comment_ids =
        comments.iter().filter_map(|comment| comment.id.parse::<i64>().ok()).collect::<Vec<_>>();
    let states = repo::get_post_viewer_states(pool, account_id, "comment", &comment_ids).await?;
    for comment in comments {
        let Some(comment_id) = comment.id.parse::<i64>().ok() else {
            continue;
        };
        if let Some(state) = states.get(&comment_id) {
            comment.viewer_vote.clone_from(&state.vote);
            comment.is_bookmarked = state.is_bookmarked;
        }
    }
    Ok(())
}

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

    let mut items: Vec<CommentDto> =
        rows.iter().map(|r| comment_to_dto(r, solved_comment_id)).collect();
    if let Some(account_id) = current_user_id {
        hydrate_comment_viewer_states(&state.db, account_id, &mut items).await?;
    }
    let parent_allows_edit = repo::thread_allows_comment_edits(&state.db, thread_id).await?;
    crate::content_permissions::hydrate_comments(
        &state.db,
        current_account.as_ref(),
        &rows,
        parent_allows_edit,
        &mut items,
    )
    .await?;
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
    let prepared = crate::content_policy::prepare_comment_create(body)?;
    let body = prepared.input;
    let is_queued = prepared.is_queued;
    // Check that the account is not silenced
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let tl = crate::trust_levels::get_trust_level(&state.db, auth.id).await?;
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

    let outcome = repo::create_comment(
        &state.db,
        thread_id,
        auth.id,
        repo::comments::CommentSource {
            body: &body.body,
            content_format: body.content_format.as_str(),
        },
        parent_id,
        quoted_comment_id,
        is_queued,
    )
    .await?;
    let row = outcome.row;
    let thread_author_id = outcome.thread_author_id;
    let quoted_author_id = outcome.quoted_author_id;

    if !is_queued {
        let pool = state.db.clone();
        let comment_author_id = auth.id;
        tokio::spawn(async move {
            match crate::badges::award_first_comment_badge(&pool, comment_author_id).await {
                Ok(true) => {
                    tracing::info!(author_id = comment_author_id, "first-comment badge awarded")
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(%error, author_id = comment_author_id, "failed to award first-comment badge")
                }
            }
        });
    }

    // Notify thread author of reply (fire-and-forget).
    if let Some(thread_author_id) =
        thread_author_id.filter(|thread_author_id| !is_queued && row.author_id != *thread_author_id)
    {
        let pool = state.db.clone();
        let actor_id = auth.id;
        let payload = serde_json::json!({
            "threadId": thread_id.to_string(),
            "commentId": row.id.to_string(),
            "authorHandle": row.author_handle,
            "bodyExcerpt": crate::content_policy::plain_text_projection(
                &row.body,
                crate::dto::ContentFormat::from_db(&row.content_format),
                100,
            ),
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
    if !is_queued {
        let pool = state.db.clone();
        let mut watcher_exclude = vec![auth.id];
        if let Some(thread_author_id) = thread_author_id {
            watcher_exclude.push(thread_author_id);
        }
        let watching_payload = serde_json::json!({
            "threadId": thread_id.to_string(),
            "commentId": row.id.to_string(),
            "authorHandle": row.author_handle,
            "bodyExcerpt": crate::content_policy::plain_text_projection(
                &row.body,
                crate::dto::ContentFormat::from_db(&row.content_format),
                100,
            ),
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
    if !is_queued {
        if let (Some(qcid), Some(qaid)) = (quoted_comment_id, quoted_author_id) {
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

    if !is_queued {
        let handles = crate::content_policy::mention_handles(
            &body.body,
            body.content_format,
            &row.author_handle,
        );

        let comment_actor_id = auth.id;
        if !handles.is_empty() {
            let pool = state.db.clone();
            let context = crate::mentions::MentionContext {
                thread_id,
                comment_id: Some(row.id),
                author_handle: row.author_handle.clone(),
                body_excerpt: crate::content_policy::plain_text_projection(
                    &row.body,
                    crate::dto::ContentFormat::from_db(&row.content_format),
                    100,
                ),
            };
            tokio::spawn(async move {
                if let Err(error) = crate::mentions::create_mention_notifications(
                    &pool,
                    comment_actor_id,
                    &handles,
                    context,
                )
                .await
                {
                    tracing::warn!(%error, comment_actor_id, "failed to create mention notifications");
                }
            });
        }
    }

    if !is_queued {
        crate::meili::reconcile_thread_in_background(&state, thread_id);
    }
    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, thread_id).await;

    let mut dto = comment_to_dto(&row, None);
    crate::content_permissions::hydrate_comments(
        &state.db,
        Some(&auth),
        std::slice::from_ref(&row),
        true,
        std::slice::from_mut(&mut dto),
    )
    .await?;
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
    if body.expected_version < 1 {
        return Err(AppError::BadRequest("expectedVersion must be a positive integer".into()));
    }
    let prepared = crate::content_policy::prepare_comment_update(body)?;
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    let row = repo::update_comment(
        &state.db,
        id,
        auth.id,
        &prepared.input.body,
        prepared.input.content_format.as_str(),
        prepared.input.expected_version,
        prepared.is_queued,
    )
    .await?;

    // Fetch solved_answer_id for Q&A solved-answer marking
    let solved_comment_id: Option<i64> =
        sqlx::query_scalar("SELECT solved_answer_id FROM forum.threads WHERE id = $1")
            .bind(row.thread_id)
            .fetch_optional(&state.db)
            .await?
            .flatten();

    crate::meili::reconcile_thread_in_background(&state, row.thread_id);
    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, row.thread_id).await;

    let mut dto = comment_to_dto(&row, solved_comment_id);
    hydrate_comment_viewer_states(&state.db, auth.id, std::slice::from_mut(&mut dto)).await?;
    crate::content_permissions::hydrate_comments(
        &state.db,
        Some(&auth),
        std::slice::from_ref(&row),
        true,
        std::slice::from_mut(&mut dto),
    )
    .await?;
    Ok(Json(dto))
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
            old_content_format: crate::dto::ContentFormat::from_db(&r.old_content_format),
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

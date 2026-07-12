use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{CommentDto, CommentInput, CommentUpdateInput, RevisionDto, RevisionListQuery};
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

pub(crate) async fn hydrate_comment_attachments(
    pool: &sqlx::PgPool,
    comments: &mut [CommentDto],
) -> AppResult<()> {
    let comment_ids =
        comments.iter().filter_map(|comment| comment.id.parse::<i64>().ok()).collect::<Vec<_>>();
    let mut attachments = media::attachments::resolve_forum_attachments_batch(
        pool,
        media::attachments::ForumTargetType::Comment,
        &comment_ids,
    )
    .await?;
    for comment in comments {
        if let Ok(comment_id) = comment.id.parse::<i64>() {
            let projected = attachments.remove(&comment_id).unwrap_or_default();
            let references = crate::content_policy::image_references_for_stored_content(
                Some(&comment.body),
                comment.content_format,
                media::attachments::ForumTargetType::Comment,
            )
            .unwrap_or_else(|error| {
                tracing::warn!(%error, comment_id, "stored comment image references are invalid");
                Vec::new()
            });
            if crate::content_policy::attachment_projection_matches(&references, &projected) {
                comment.attachments = projected;
            } else {
                tracing::warn!(comment_id, "comment attachment projection mismatch");
            }
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
    hydrate_comment_attachments(&state.db, &mut items).await?;
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
            image_references: &prepared.image_references,
        },
        parent_id,
        quoted_comment_id,
        is_queued,
    )
    .await?;
    let row = outcome.row;

    if !is_queued {
        crate::meili::reconcile_thread_in_background(&state, thread_id);
    }
    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, thread_id).await;

    let mut dto = comment_to_dto(&row, None);
    hydrate_comment_attachments(&state.db, std::slice::from_mut(&mut dto)).await?;
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
        repo::comments::CommentUpdateSource {
            body: &prepared.input.body,
            content_format: prepared.input.content_format.as_str(),
            expected_version: prepared.input.expected_version,
            is_queued: prepared.is_queued,
            image_references: &prepared.image_references,
        },
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
    hydrate_comment_attachments(&state.db, std::slice::from_mut(&mut dto)).await?;
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
    let mut tx = state.db.begin().await?;
    let thread_id: i64 = sqlx::query_scalar("SELECT thread_id FROM forum.comments WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    sqlx::query("SELECT id FROM forum.threads WHERE id = $1 FOR UPDATE")
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;
    let (author_id, deleted_at): (i64, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT author_id, deleted_at FROM forum.comments \
         WHERE id = $1 AND thread_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(thread_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if author_id != auth.id {
        return Err(AppError::Forbidden);
    }
    if deleted_at.is_some() {
        return Err(AppError::Conflict("comment is already deleted".into()));
    }
    sqlx::query("UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 WHERE id = $2")
        .bind(auth.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE forum.threads SET reply_count = GREATEST(reply_count - 1, 0) WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&mut *tx)
    .await?;
    crate::repo::activity_projection::synchronize_comment_activity(&mut tx, id, chrono::Utc::now())
        .await?;
    media::attachments::detach_forum_asset_bindings(
        &mut tx,
        media::attachments::ForumTargetType::Comment,
        id,
    )
    .await?;
    tx.commit().await?;

    crate::cache::invalidate_thread_by_id(state.redis.as_ref(), &state.db, thread_id).await;

    Ok(Json(json!({"ok": true})))
}

/// GET /api/v2/forum/comments/{id}/revisions — auth required (author + mod)
pub async fn list_comment_revisions(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Query(query): Query<RevisionListQuery>,
) -> AppResult<Json<Page<RevisionDto>>> {
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

    if !crate::content_permissions::can_read_revisions(&state.db, &auth, comment.author_id).await? {
        return Err(AppError::Forbidden);
    }

    let (revs, next_cursor) =
        repo::list_revisions(&state.db, "comment", id, query.cursor.as_deref(), query.limit)
            .await?;
    let content_versions =
        revs.iter().map(|revision| revision.old_content_version).collect::<Vec<_>>();
    let mut projections = media::attachments::resolve_forum_attachments_at_versions(
        &state.db,
        media::attachments::ForumTargetType::Comment,
        id,
        &content_versions,
    )
    .await?;
    let mut dtos = Vec::with_capacity(revs.len());
    for revision in revs {
        let projected = projections.remove(&revision.old_content_version).unwrap_or_default();
        let references = crate::content_policy::image_references_for_stored_content(
            Some(&revision.old_body),
            crate::dto::ContentFormat::from_db(&revision.old_content_format),
            media::attachments::ForumTargetType::Comment,
        )?;
        let attachments =
            if crate::content_policy::attachment_projection_matches(&references, &projected) {
                projected
            } else {
                tracing::warn!(
                    comment_id = id,
                    revision_id = revision.id,
                    "comment revision attachment projection mismatch"
                );
                Vec::new()
            };
        dtos.push(RevisionDto {
            id: revision.id.to_string(),
            seq: revision.seq,
            editor_id: revision.editor_id.to_string(),
            old_title: revision.old_title,
            old_body: revision.old_body,
            old_content_format: crate::dto::ContentFormat::from_db(&revision.old_content_format),
            old_content_version: revision.old_content_version,
            attachments,
            created_at: revision.created_at.timestamp(),
        });
    }

    Ok(Json(Page::new(dtos, next_cursor)))
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

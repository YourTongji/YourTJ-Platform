//! Admin forum comment action endpoints: soft-delete, restore, hide, unhide.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

use crate::dto::CommentDto;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCommentActionInput {
    reason: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AdminCommentRow {
    author_id: Option<i64>,
    thread_id: i64,
    board_id: i64,
    thread_status: String,
    thread_deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    thread_hidden_at: Option<chrono::DateTime<chrono::Utc>>,
    thread_archived_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    hidden_at: Option<chrono::DateTime<chrono::Utc>>,
    body: String,
    content_format: String,
    content_version: i64,
}

/// GET /api/v2/admin/forum/comments/{id} — staff recovery detail.
pub async fn get_comment_for_moderation(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<CommentDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let comment =
        crate::repo::find_comment_for_moderation(&state.db, id).await?.ok_or(AppError::NotFound)?;
    let solved_comment_id: Option<i64> =
        sqlx::query_scalar("SELECT solved_answer_id FROM forum.threads WHERE id = $1")
            .bind(comment.thread_id)
            .fetch_optional(&state.db)
            .await?
            .flatten();
    let parent_allows_edit =
        crate::repo::thread_allows_comment_edits(&state.db, comment.thread_id).await?;
    let mut dto = crate::handlers::comment_to_dto(&comment, solved_comment_id);
    crate::handlers::hydrate_comment_attachments(&state.db, std::slice::from_mut(&mut dto)).await?;
    crate::content_permissions::hydrate_comments(
        &state.db,
        Some(&auth),
        std::slice::from_ref(&comment),
        parent_allows_edit,
        std::slice::from_mut(&mut dto),
    )
    .await?;
    Ok(Json(dto))
}

/// POST /api/v2/admin/forum/comments/{id}/{action}
///
/// Actions: `delete` (mod soft-delete), `restore`, `hide`, `unhide`
pub async fn admin_comment_action(
    State(state): State<AppState>,
    Path((id_str, action)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<AdminCommentActionInput>,
) -> AppResult<Json<Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let reason = body.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
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
    let comment = sqlx::query_as::<_, AdminCommentRow>(
        "SELECT c.author_id, c.thread_id, t.board_id, t.status AS thread_status, \
                t.deleted_at AS thread_deleted_at, t.hidden_at AS thread_hidden_at, \
                t.archived_at AS thread_archived_at, c.created_at, \
                c.deleted_at, c.hidden_at, c.body, c.content_format, c.content_version \
         FROM forum.comments c \
         JOIN forum.threads t ON t.id = c.thread_id \
         WHERE c.id = $1 AND c.thread_id = $2 FOR UPDATE OF c",
    )
    .bind(id)
    .bind(thread_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    super::require_lower_author_role(&mut tx, &auth, comment.author_id).await?;
    let parent_is_visible = comment.thread_status == "visible"
        && comment.thread_deleted_at.is_none()
        && comment.thread_hidden_at.is_none()
        && comment.thread_archived_at.is_none();
    let was_visible =
        parent_is_visible && comment.deleted_at.is_none() && comment.hidden_at.is_none();

    match action.as_str() {
        "delete" => {
            if comment.deleted_at.is_some() {
                return Err(AppError::Conflict("comment is already deleted".into()));
            }
            sqlx::query(
                "UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 WHERE id = $2",
            )
            .bind(auth.id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            if comment.deleted_at.is_none() {
                sqlx::query(
                    "UPDATE forum.threads SET reply_count = GREATEST(reply_count - 1, 0) \
                     WHERE id = $1",
                )
                .bind(comment.thread_id)
                .execute(&mut *tx)
                .await?;
            }
            media::attachments::detach_forum_asset_bindings(
                &mut tx,
                media::attachments::ForumTargetType::Comment,
                id,
            )
            .await?;
        }
        "restore" => {
            if comment.deleted_at.is_none() {
                return Err(AppError::Conflict("comment is not deleted".into()));
            }
            sqlx::query(
                "UPDATE forum.comments SET deleted_at = NULL, deleted_by = NULL WHERE id = $1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
            if comment.deleted_at.is_some() {
                sqlx::query("UPDATE forum.threads SET reply_count = reply_count + 1 WHERE id = $1")
                    .bind(comment.thread_id)
                    .execute(&mut *tx)
                    .await?;
            }
            let image_references = crate::content_policy::image_references_for_stored_content(
                Some(&comment.body),
                crate::dto::ContentFormat::from_db(&comment.content_format),
                media::attachments::ForumTargetType::Comment,
            )?;
            if !image_references.is_empty() {
                let author_id = comment.author_id.ok_or_else(|| {
                    AppError::Conflict(
                        "comment without an author cannot restore attachments".into(),
                    )
                })?;
                media::attachments::sync_forum_asset_bindings(
                    &mut tx,
                    author_id,
                    media::attachments::ForumTargetType::Comment,
                    id,
                    comment.content_version,
                    &image_references,
                )
                .await?;
            }
        }
        "hide" => {
            if comment.hidden_at.is_some() || comment.deleted_at.is_some() {
                return Err(AppError::Conflict(
                    "comment cannot be hidden from its current state".into(),
                ));
            }
            sqlx::query("UPDATE forum.comments SET hidden_at = now() WHERE id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        "unhide" => {
            sqlx::query("UPDATE forum.comments SET hidden_at = NULL WHERE id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        _ => return Err(AppError::BadRequest(format!("unknown action: {action}"))),
    }

    let is_visible = match action.as_str() {
        "restore" => parent_is_visible && comment.hidden_at.is_none(),
        "unhide" => parent_is_visible && comment.deleted_at.is_none(),
        _ => false,
    };
    if was_visible && matches!(action.as_str(), "delete" | "hide") {
        activity::contributions::deactivate_contribution(
            &mut tx,
            &format!("forum_comment:{id}"),
            chrono::Utc::now(),
        )
        .await?;
    } else if is_visible {
        if let Some(author_id) = comment.author_id {
            activity::contributions::activate_contribution(
                &mut tx,
                author_id,
                activity::contributions::ActivityKind::Comment,
                &format!("forum_comment:{id}"),
                comment.created_at,
            )
            .await?;
        }
    }
    if matches!(action.as_str(), "delete" | "hide") {
        crate::repo::deactivate_target_vote_contributions(
            &mut tx,
            "comment",
            id,
            chrono::Utc::now(),
        )
        .await?;
    } else if matches!(action.as_str(), "restore" | "unhide") {
        crate::repo::reactivate_target_vote_contributions(&mut tx, "comment", id).await?;
    }
    crate::repo::insert_mod_action(&mut *tx, auth.id, &action, "comment", id, Some(reason), None)
        .await?;
    let governance_event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        &format!("forum.comment.{action}"),
        "comment",
        &id.to_string(),
        reason,
        None,
    )
    .await?;
    if matches!(action.as_str(), "hide" | "delete") {
        if let Some(author_id) = comment.author_id {
            governance::notices::create_notice_tx(
                &mut tx,
                author_id,
                "content_restricted",
                &format!("audit:{governance_event_id}:forum-comment"),
                Some(governance_event_id),
                None,
                "forum_comment",
                &id.to_string(),
                &format!(
                    "你的评论已被{}，可在申诉中心查看并申请复核。",
                    if action == "hide" { "隐藏" } else { "软移除" }
                ),
            )
            .await?;
        }
    }
    tx.commit().await?;

    crate::cache::invalidate_thread_surfaces(
        state.redis.as_ref(),
        comment.thread_id,
        comment.board_id,
    )
    .await;
    crate::meili::reconcile_thread_in_background(&state, comment.thread_id);

    Ok(Json(json!({"ok": true})))
}

//! Admin forum thread action endpoints: state transitions and organization.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

use crate::dto::ThreadDetailDto;

#[derive(Debug, sqlx::FromRow)]
struct AdminThreadRow {
    author_id: Option<i64>,
    board_id: i64,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    hidden_at: Option<chrono::DateTime<chrono::Utc>>,
    archived_at: Option<chrono::DateTime<chrono::Utc>>,
    body: Option<String>,
    content_format: String,
    content_version: i64,
}

/// GET /api/v2/admin/forum/threads/{id} — staff recovery detail.
pub async fn get_thread_for_moderation(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<ThreadDetailDto>> {
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
    let row =
        crate::repo::find_thread_for_moderation(&state.db, id).await?.ok_or(AppError::NotFound)?;
    let mut dto = crate::handlers::thread_to_detail_dto(&row);
    crate::handlers::hydrate_thread_detail(&state.db, id, Some(&auth), &mut dto).await?;
    Ok(Json(dto))
}

/// POST /api/v2/admin/forum/threads/{id}/{action}
///
/// Actions: `pin`, `unpin`, `close`, `reopen`, `archive`, `unarchive`, `delete`, `restore`,
/// `hide`, `unhide`, `move`
/// - `pin`   body: `{ globally: bool }`
/// - `move`  body: `{ boardId: string }`
/// - every action requires a bounded `reason`
pub async fn admin_thread_action(
    State(state): State<AppState>,
    Path((id_str, action)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
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
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|reason| (3..=500).contains(&reason.chars().count()))
        .ok_or_else(|| AppError::BadRequest("reason must be 3–500 characters".into()))?;
    let mut tx = state.db.begin().await?;
    let thread = sqlx::query_as::<_, AdminThreadRow>(
        "SELECT author_id, board_id, status, created_at, deleted_at, hidden_at, archived_at, \
                body, content_format, content_version \
         FROM forum.threads WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    super::require_lower_author_role(&mut tx, &auth, thread.author_id).await?;
    let requested_board_id = if action == "move" {
        Some(
            body.get("boardId")
                .and_then(Value::as_str)
                .and_then(|board_id| board_id.parse().ok())
                .ok_or(AppError::BadRequest("boardId required".into()))?,
        )
    } else {
        None
    };
    let mut affected_board_ids = vec![thread.board_id];
    affected_board_ids.extend(requested_board_id);
    let affected_board_ids =
        crate::repo::boards::lock_boards_for_thread_count(&mut tx, &affected_board_ids).await?;
    let was_visible = thread.status == "visible"
        && thread.deleted_at.is_none()
        && thread.hidden_at.is_none()
        && thread.archived_at.is_none();
    let mut metadata = None;
    let mut moved_to_board_id = None;

    match action.as_str() {
        "pin" => {
            let globally = body.get("globally").and_then(Value::as_bool).unwrap_or(false);
            crate::repo::pin_thread(&mut *tx, id, globally).await?;
            metadata = Some(json!({ "globally": globally }));
        }
        "unpin" => {
            crate::repo::unpin_thread(&mut *tx, id).await?;
        }
        "close" => {
            crate::repo::close_thread(&mut *tx, id).await?;
        }
        "reopen" => {
            crate::repo::reopen_thread(&mut *tx, id).await?;
        }
        "archive" => {
            crate::repo::archive_thread(&mut *tx, id).await?;
            if was_visible {
                activity::contributions::deactivate_contribution(
                    &mut tx,
                    &format!("forum_thread:{id}"),
                    chrono::Utc::now(),
                )
                .await?;
            }
            media::attachments::detach_forum_asset_bindings(
                &mut tx,
                media::attachments::ForumTargetType::Thread,
                id,
            )
            .await?;
        }
        "unarchive" => {
            if thread.archived_at.is_none() {
                return Err(AppError::Conflict("thread is not archived".into()));
            }
            crate::repo::unarchive_thread(&mut *tx, id).await?;
            if let (true, true, Some(author_id)) =
                (thread.deleted_at.is_none(), thread.hidden_at.is_none(), thread.author_id)
            {
                activity::contributions::activate_contribution(
                    &mut tx,
                    author_id,
                    activity::contributions::ActivityKind::Thread,
                    &format!("forum_thread:{id}"),
                    thread.created_at,
                )
                .await?;
            }
        }
        "delete" => {
            if thread.deleted_at.is_some() {
                return Err(AppError::Conflict("thread is already deleted".into()));
            }
            sqlx::query(
                "UPDATE forum.threads SET deleted_at = now(), deleted_by = $1 WHERE id = $2",
            )
            .bind(auth.id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            if was_visible {
                activity::contributions::deactivate_contribution(
                    &mut tx,
                    &format!("forum_thread:{id}"),
                    chrono::Utc::now(),
                )
                .await?;
            }
        }
        "restore" => {
            if thread.deleted_at.is_none() {
                return Err(AppError::Conflict("thread is not deleted".into()));
            }
            crate::repo::restore_thread(&mut *tx, id).await?;
            let image_references = crate::content_policy::image_references_for_stored_content(
                thread.body.as_deref(),
                crate::dto::ContentFormat::from_db(&thread.content_format),
                media::attachments::ForumTargetType::Thread,
            )?;
            if !image_references.is_empty() {
                let author_id = thread.author_id.ok_or_else(|| {
                    AppError::Conflict("thread without an author cannot restore attachments".into())
                })?;
                media::attachments::sync_forum_asset_bindings(
                    &mut tx,
                    author_id,
                    media::attachments::ForumTargetType::Thread,
                    id,
                    thread.content_version,
                    &image_references,
                )
                .await?;
            }
            if let (true, true, Some(author_id)) =
                (thread.hidden_at.is_none(), thread.archived_at.is_none(), thread.author_id)
            {
                activity::contributions::activate_contribution(
                    &mut tx,
                    author_id,
                    activity::contributions::ActivityKind::Thread,
                    &format!("forum_thread:{id}"),
                    thread.created_at,
                )
                .await?;
            }
        }
        "hide" => {
            if thread.hidden_at.is_some() || thread.deleted_at.is_some() {
                return Err(AppError::Conflict(
                    "thread cannot be hidden from its current state".into(),
                ));
            }
            crate::repo::hide_thread(&mut *tx, id).await?;
            if was_visible {
                activity::contributions::deactivate_contribution(
                    &mut tx,
                    &format!("forum_thread:{id}"),
                    chrono::Utc::now(),
                )
                .await?;
            }
        }
        "unhide" => {
            crate::repo::unhide_thread(&mut *tx, id).await?;
            if let (true, true, Some(author_id)) = (
                thread.deleted_at.is_none() && thread.archived_at.is_none(),
                thread.hidden_at.is_some(),
                thread.author_id,
            ) {
                activity::contributions::activate_contribution(
                    &mut tx,
                    author_id,
                    activity::contributions::ActivityKind::Thread,
                    &format!("forum_thread:{id}"),
                    thread.created_at,
                )
                .await?;
            }
        }
        "move" => {
            let board_id = requested_board_id.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("validated move board is missing"))
            })?;
            crate::repo::move_thread(&mut *tx, id, board_id).await?;
            moved_to_board_id = Some(board_id);
            metadata = Some(json!({ "boardId": board_id.to_string() }));
        }
        _ => return Err(AppError::BadRequest(format!("unknown action: {action}"))),
    }

    crate::repo::boards::refresh_board_thread_counts(&mut tx, &affected_board_ids).await?;

    if matches!(action.as_str(), "archive" | "delete" | "hide") {
        crate::repo::deactivate_target_vote_contributions(
            &mut tx,
            "thread",
            id,
            chrono::Utc::now(),
        )
        .await?;
    } else if matches!(action.as_str(), "unarchive" | "restore" | "unhide") {
        crate::repo::reactivate_target_vote_contributions(&mut tx, "thread", id).await?;
    }

    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        &action,
        "thread",
        id,
        Some(reason),
        metadata.as_ref(),
    )
    .await?;
    let governance_event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        &format!("forum.thread.{action}"),
        "thread",
        &id.to_string(),
        reason,
        metadata.as_ref(),
    )
    .await?;
    if matches!(action.as_str(), "hide" | "delete") {
        if let Some(author_id) = thread.author_id {
            governance::notices::create_notice_tx(
                &mut tx,
                author_id,
                "content_restricted",
                &format!("audit:{governance_event_id}:forum-thread"),
                Some(governance_event_id),
                None,
                "forum_thread",
                &id.to_string(),
                &format!(
                    "你的主题已被{}，可在申诉中心查看并申请复核。",
                    if action == "hide" { "隐藏" } else { "软移除" }
                ),
            )
            .await?;
        }
    }
    tx.commit().await?;

    crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), id, thread.board_id).await;
    if let Some(board_id) = moved_to_board_id.filter(|board_id| *board_id != thread.board_id) {
        crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), id, board_id).await;
    }
    if matches!(
        action.as_str(),
        "archive" | "unarchive" | "delete" | "restore" | "hide" | "unhide" | "move"
    ) {
        crate::meili::reconcile_thread_in_background(&state, id);
    }

    Ok(Json(json!({"ok": true})))
}

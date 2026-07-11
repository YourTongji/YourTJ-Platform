//! Owner-domain validation and reversal for forum content appeals.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection};

#[derive(Debug, Clone)]
pub struct ForumAppealTarget {
    pub target_kind: String,
    pub target_id: i64,
    pub disposition_kind: String,
    pub author_role: String,
    pub thread_id: i64,
    pub board_id: i64,
}

#[derive(Debug, Clone)]
pub struct ForumAppealMutation {
    pub thread_id: i64,
    pub board_id: i64,
}

#[derive(Debug, FromRow)]
struct ForumContentState {
    author_id: Option<i64>,
    thread_id: i64,
    board_id: i64,
    deleted_at: Option<DateTime<Utc>>,
    hidden_at: Option<DateTime<Utc>>,
    content_body: Option<String>,
    content_format: String,
    content_version: i64,
}

fn classify_event(
    action: &str,
    target_type: &str,
    target_id: &str,
) -> AppResult<(String, i64, String)> {
    let direct = match (action, target_type) {
        ("forum.thread.hide", "thread") => Some(("thread", target_id, "hide")),
        ("forum.thread.delete", "thread") => Some(("thread", target_id, "delete")),
        ("forum.comment.hide", "comment") => Some(("comment", target_id, "hide")),
        ("forum.comment.delete", "comment") => Some(("comment", target_id, "delete")),
        _ => None,
    };
    let (kind, numeric_id, disposition) = if let Some(target) = direct {
        target
    } else if matches!(action, "forum.flag.uphold" | "forum.content.auto_hidden")
        && target_type == "forum_content"
    {
        let (kind, numeric_id) = target_id.split_once(':').ok_or(AppError::NotFound)?;
        let disposition = if action == "forum.flag.uphold" { "delete" } else { "hide" };
        (kind, numeric_id, disposition)
    } else {
        return Err(AppError::NotFound);
    };
    if !matches!(kind, "thread" | "comment") {
        return Err(AppError::NotFound);
    }
    let numeric_id = numeric_id.parse().map_err(|_| AppError::NotFound)?;
    Ok((kind.to_owned(), numeric_id, disposition.to_owned()))
}

fn require_restriction_changed(metadata: Option<&serde_json::Value>) -> AppResult<()> {
    let metadata = metadata.ok_or(AppError::NotFound)?;
    if metadata.get("contentChanged").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

async fn content_state(
    connection: &mut PgConnection,
    target_kind: &str,
    target_id: i64,
    lock: bool,
) -> AppResult<ForumContentState> {
    if target_kind == "comment" && lock {
        return locked_comment_state(connection, target_id).await;
    }
    let lock_clause = if lock { " FOR UPDATE" } else { " FOR SHARE" };
    let query = match target_kind {
        "thread" => format!(
            "SELECT author_id, id AS thread_id, board_id, \
                    deleted_at, hidden_at, body AS content_body, content_format, \
                    content_version \
             FROM forum.threads WHERE id = $1{lock_clause}"
        ),
        "comment" => format!(
            "SELECT comment.author_id, comment.thread_id, thread.board_id, \
                    comment.deleted_at, comment.hidden_at, \
                    comment.body AS content_body, comment.content_format, comment.content_version \
             FROM forum.comments comment \
             JOIN forum.threads thread ON thread.id = comment.thread_id \
             WHERE comment.id = $1{lock_clause} OF comment"
        ),
        _ => return Err(AppError::NotFound),
    };
    sqlx::query_as(&query)
        .bind(target_id)
        .fetch_optional(connection)
        .await?
        .ok_or(AppError::NotFound)
}

async fn locked_comment_state(
    connection: &mut PgConnection,
    comment_id: i64,
) -> AppResult<ForumContentState> {
    let thread_id: i64 = sqlx::query_scalar("SELECT thread_id FROM forum.comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(AppError::NotFound)?;
    sqlx::query("SELECT id FROM forum.threads WHERE id = $1 FOR UPDATE")
        .bind(thread_id)
        .execute(&mut *connection)
        .await?;
    sqlx::query_as(
        "SELECT comment.author_id, comment.thread_id, thread.board_id, \
                comment.deleted_at, comment.hidden_at, \
                comment.body AS content_body, comment.content_format, comment.content_version \
         FROM forum.comments comment \
         JOIN forum.threads thread ON thread.id = comment.thread_id \
         WHERE comment.id = $1 AND comment.thread_id = $2 FOR UPDATE OF comment",
    )
    .bind(comment_id)
    .bind(thread_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn rebind_restored_attachments(
    connection: &mut PgConnection,
    state: &ForumContentState,
    target_kind: &str,
    target_id: i64,
) -> AppResult<()> {
    let target_type = match target_kind {
        "thread" => media::attachments::ForumTargetType::Thread,
        "comment" => media::attachments::ForumTargetType::Comment,
        _ => return Err(AppError::NotFound),
    };
    let references = crate::content_policy::image_references_for_stored_content(
        state.content_body.as_deref(),
        crate::dto::ContentFormat::from_db(&state.content_format),
        target_type,
    )?;
    if references.is_empty() {
        return Ok(());
    }
    let author_id = state
        .author_id
        .ok_or_else(|| AppError::Conflict("restored content has no attachment owner".into()))?;
    media::attachments::sync_forum_asset_bindings(
        connection,
        author_id,
        target_type,
        target_id,
        state.content_version,
        &references,
    )
    .await
}

/// Validate that the original governance event still describes an active restriction on
/// content authored by the appellant. No content body is returned.
pub async fn inspect_appealable_content_tx(
    connection: &mut PgConnection,
    action: &str,
    target_type: &str,
    target_id: &str,
    metadata: Option<&serde_json::Value>,
    appellant_account_id: i64,
) -> AppResult<ForumAppealTarget> {
    let (target_kind, target_id, disposition_kind) =
        classify_event(action, target_type, target_id)?;
    if action == "forum.flag.uphold" {
        require_restriction_changed(metadata)?;
    }
    let state = content_state(connection, &target_kind, target_id, false).await?;
    if state.author_id != Some(appellant_account_id) {
        return Err(AppError::NotFound);
    }
    let restriction_is_active = match disposition_kind.as_str() {
        "hide" => state.hidden_at.is_some() && state.deleted_at.is_none(),
        "delete" => state.deleted_at.is_some(),
        _ => false,
    };
    if !restriction_is_active {
        return Err(AppError::Conflict("the forum disposition is no longer active".into()));
    }
    let author_role =
        identity::public_accounts::find_account_role_by_id(connection, appellant_account_id)
            .await?
            .ok_or(AppError::NotFound)?;
    Ok(ForumAppealTarget {
        target_kind: format!("forum_{target_kind}"),
        target_id,
        disposition_kind,
        author_role,
        thread_id: state.thread_id,
        board_id: state.board_id,
    })
}

/// Reverse the exact forum restriction under appeal while preserving the original event.
#[allow(clippy::too_many_arguments)] // reason: original target keys are required for the fail-closed later-event guard
pub async fn overturn_content_for_appeal_tx(
    connection: &mut PgConnection,
    original_event_id: i64,
    original_created_at: DateTime<Utc>,
    original_action: &str,
    original_target_type: &str,
    original_target_id: &str,
    original_metadata: Option<&serde_json::Value>,
    target_kind: &str,
    target_id: i64,
    disposition_kind: &str,
    appellant_account_id: i64,
) -> AppResult<ForumAppealMutation> {
    let canonical_kind = target_kind.strip_prefix("forum_").ok_or(AppError::NotFound)?;
    let alternate_target_id = format!("{canonical_kind}:{target_id}");
    let state = content_state(connection, canonical_kind, target_id, true).await?;
    if governance::has_later_target_event_tx(
        connection,
        original_event_id,
        original_target_type,
        original_target_id,
        Some(canonical_kind),
        Some(&target_id.to_string()),
    )
    .await?
        || governance::has_later_target_event_tx(
            connection,
            original_event_id,
            original_target_type,
            original_target_id,
            Some("forum_content"),
            Some(&alternate_target_id),
        )
        .await?
    {
        return Err(AppError::Conflict(
            "forum content changed after the appealed disposition".into(),
        ));
    }
    if state.author_id != Some(appellant_account_id) {
        return Err(AppError::NotFound);
    }
    let affected_board_ids = if canonical_kind == "thread" {
        crate::repo::boards::lock_boards_for_thread_count(connection, &[state.board_id]).await?
    } else {
        Vec::new()
    };
    match (canonical_kind, disposition_kind) {
        ("thread", "hide") if state.hidden_at.is_some() && state.deleted_at.is_none() => {
            crate::repo::unhide_thread(&mut *connection, target_id).await?;
        }
        ("thread", "delete") if state.deleted_at.is_some() => {
            crate::repo::restore_thread(&mut *connection, target_id).await?;
        }
        ("comment", "hide") if state.hidden_at.is_some() && state.deleted_at.is_none() => {
            sqlx::query("UPDATE forum.comments SET hidden_at = NULL WHERE id = $1")
                .bind(target_id)
                .execute(&mut *connection)
                .await?;
        }
        ("comment", "delete") if state.deleted_at.is_some() => {
            sqlx::query(
                "UPDATE forum.comments SET deleted_at = NULL, deleted_by = NULL WHERE id = $1",
            )
            .bind(target_id)
            .execute(&mut *connection)
            .await?;
            sqlx::query("UPDATE forum.threads SET reply_count = reply_count + 1 WHERE id = $1")
                .bind(state.thread_id)
                .execute(&mut *connection)
                .await?;
        }
        _ => {
            return Err(AppError::Conflict(
                "forum content state no longer matches the appealed disposition".into(),
            ))
        }
    }
    if disposition_kind == "delete" {
        rebind_restored_attachments(connection, &state, canonical_kind, target_id).await?;
    }
    match canonical_kind {
        "thread" => {
            crate::repo::activity_projection::synchronize_thread_activity_subtree(
                connection,
                target_id,
                Utc::now(),
            )
            .await?;
        }
        "comment" => {
            crate::repo::activity_projection::synchronize_comment_activity(
                connection,
                target_id,
                Utc::now(),
            )
            .await?;
        }
        _ => return Err(AppError::NotFound),
    }
    if original_action == "forum.flag.uphold" {
        require_restriction_changed(original_metadata)?;
        let adjustments: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT reporter_id, COUNT(*) FROM forum.flags \
             WHERE target_type = $1 AND target_id = $2 AND status = 'upheld' \
               AND handled_at = $3 GROUP BY reporter_id ORDER BY reporter_id",
        )
        .bind(canonical_kind)
        .bind(target_id)
        .bind(original_created_at)
        .fetch_all(&mut *connection)
        .await?;
        if adjustments.is_empty() {
            return Err(AppError::Conflict("forum report projection is incomplete".into()));
        }
        let author_stats = sqlx::query(
            "UPDATE forum.user_stats \
             SET flagged_upheld = GREATEST(flagged_upheld - 1, 0) WHERE account_id = $1",
        )
        .bind(appellant_account_id)
        .execute(&mut *connection)
        .await?;
        if author_stats.rows_affected() != 1 {
            return Err(AppError::Conflict("forum trust projection is incomplete".into()));
        }
        for (reporter_id, count) in adjustments {
            let reporter_stats = sqlx::query(
                "UPDATE forum.user_stats \
                 SET flags_upheld = GREATEST(flags_upheld - $2::int, 0) WHERE account_id = $1",
            )
            .bind(reporter_id)
            .bind(count)
            .execute(&mut *connection)
            .await?;
            if reporter_stats.rows_affected() != 1 {
                return Err(AppError::Conflict("forum trust projection is incomplete".into()));
            }
        }
    }
    crate::repo::boards::refresh_board_thread_counts(connection, &affected_board_ids).await?;
    Ok(ForumAppealMutation { thread_id: state.thread_id, board_id: state.board_id })
}

#[cfg(test)]
mod tests {
    use super::{classify_event, require_restriction_changed};

    #[test]
    fn only_reversible_forum_restrictions_are_appealable() {
        assert_eq!(
            classify_event("forum.thread.hide", "thread", "12").expect("thread hide"),
            ("thread".into(), 12, "hide".into())
        );
        assert_eq!(
            classify_event("forum.flag.uphold", "forum_content", "comment:7")
                .expect("flag decision"),
            ("comment".into(), 7, "delete".into())
        );
        assert!(classify_event("forum.thread.pin", "thread", "12").is_err());
    }

    #[test]
    fn report_decision_requires_provenance_that_it_changed_content() {
        assert!(require_restriction_changed(Some(&serde_json::json!({
            "contentChanged": true
        })))
        .is_ok());
        assert!(require_restriction_changed(Some(&serde_json::json!({
            "contentChanged": false
        })))
        .is_err());
        assert!(require_restriction_changed(None).is_err());
    }
}

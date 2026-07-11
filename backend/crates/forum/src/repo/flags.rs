//! Flag (report) CRUD, threshold checks, and admin resolution.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection, PgPool};

use crate::models::{FlagQueueRow, FlagRow};

#[derive(Debug)]
pub struct FlagInsertOutcome {
    pub auto_hidden: bool,
    pub author_id: Option<i64>,
    pub thread_id: i64,
    pub board_id: i64,
}

#[derive(Debug)]
pub struct FlagResolutionOutcome {
    pub flag: FlagRow,
    pub thread_id: i64,
    pub board_id: i64,
    pub content_changed: bool,
}

#[derive(Debug, FromRow)]
struct FlagTargetRow {
    author_id: Option<i64>,
    thread_id: i64,
    board_id: i64,
    deleted_at: Option<DateTime<Utc>>,
    hidden_at: Option<DateTime<Utc>>,
}

async fn lock_target(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
) -> AppResult<FlagTargetRow> {
    if target_type == "thread" {
        return sqlx::query_as(
            "SELECT author_id, id AS thread_id, board_id, deleted_at, hidden_at \
             FROM forum.threads WHERE id = $1 FOR UPDATE",
        )
        .bind(target_id)
        .fetch_optional(connection)
        .await?
        .ok_or(AppError::NotFound);
    }
    if target_type != "comment" {
        return Err(AppError::BadRequest("postType must be thread/comment".into()));
    }

    let thread_id: i64 = sqlx::query_scalar("SELECT thread_id FROM forum.comments WHERE id = $1")
        .bind(target_id)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(AppError::NotFound)?;
    sqlx::query("SELECT id FROM forum.threads WHERE id = $1 FOR UPDATE")
        .bind(thread_id)
        .execute(&mut *connection)
        .await?;
    sqlx::query_as(
        "SELECT comment.author_id, comment.thread_id, thread.board_id, \
                comment.deleted_at, comment.hidden_at \
         FROM forum.comments comment \
         JOIN forum.threads thread ON thread.id = comment.thread_id \
         WHERE comment.id = $1 AND comment.thread_id = $2 FOR UPDATE OF comment",
    )
    .bind(target_id)
    .bind(thread_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn synchronize_target_activity(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
    transition_at: DateTime<Utc>,
) -> AppResult<()> {
    match target_type {
        "thread" => {
            super::activity_projection::synchronize_thread_activity_subtree(
                connection,
                target_id,
                transition_at,
            )
            .await
        }
        "comment" => {
            super::activity_projection::synchronize_comment_activity(
                connection,
                target_id,
                transition_at,
            )
            .await
        }
        _ => Err(AppError::BadRequest("postType must be thread/comment".into())),
    }
}

/// Lock an open report and its target, returning the target author's account id.
pub async fn lock_flag_target_author(
    connection: &mut PgConnection,
    flag_id: i64,
) -> AppResult<Option<i64>> {
    let flag: (String, i64, String) = sqlx::query_as(
        "SELECT target_type, target_id, status FROM forum.flags WHERE id = $1 FOR UPDATE",
    )
    .bind(flag_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)?;
    if flag.2 != "open" {
        return Err(AppError::Conflict("flag is already resolved".into()));
    }
    Ok(lock_target(connection, &flag.0, flag.1).await?.author_id)
}

/// Insert or replace one reporter's open flag and apply the auto-hide threshold atomically.
#[allow(clippy::too_many_arguments)] // reason: each report field is bound explicitly and belongs to one mutation
pub async fn insert_flag(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
    reporter_id: i64,
    reason: &str,
    note: Option<&str>,
    weight: f32,
) -> AppResult<FlagInsertOutcome> {
    let target = lock_target(connection, target_type, target_id).await?;
    if target.author_id == Some(reporter_id) {
        return Err(AppError::BadRequest("users cannot flag their own content".into()));
    }

    let flag_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.flags (target_type, target_id, reporter_id, reason, note, weight) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (target_type, target_id, reporter_id) WHERE status = 'open' \
         DO UPDATE SET reason = EXCLUDED.reason, note = EXCLUDED.note, \
                       weight = EXCLUDED.weight \
         RETURNING id",
    )
    .bind(target_type)
    .bind(target_id)
    .bind(reporter_id)
    .bind(reason)
    .bind(note)
    .bind(weight)
    .fetch_one(&mut *connection)
    .await?;

    let score: f32 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(weight), 0.0)::real FROM forum.flags \
         WHERE target_type = $1 AND target_id = $2 AND status = 'open'",
    )
    .bind(target_type)
    .bind(target_id)
    .fetch_one(&mut *connection)
    .await?;

    let should_hide = score >= 3.0 && target.hidden_at.is_none() && target.deleted_at.is_none();
    let affected_board_ids = if should_hide && target_type == "thread" {
        super::boards::lock_boards_for_thread_count(connection, &[target.board_id]).await?
    } else {
        Vec::new()
    };
    let hidden_at = if should_hide {
        let statement = match target_type {
            "thread" => {
                "UPDATE forum.threads SET hidden_at = now() \
                 WHERE id = $1 AND hidden_at IS NULL AND deleted_at IS NULL RETURNING hidden_at"
            }
            "comment" => {
                "UPDATE forum.comments SET hidden_at = now() \
                 WHERE id = $1 AND hidden_at IS NULL AND deleted_at IS NULL RETURNING hidden_at"
            }
            _ => return Err(AppError::Internal(anyhow::anyhow!("invalid flag target type"))),
        };
        sqlx::query_scalar(statement).bind(target_id).fetch_optional(&mut *connection).await?
    } else {
        None
    };

    if let Some(hidden_at) = hidden_at {
        sqlx::query("UPDATE forum.flags SET auto_hidden_at = $1 WHERE id = $2")
            .bind(hidden_at)
            .bind(flag_id)
            .execute(&mut *connection)
            .await?;
        synchronize_target_activity(connection, target_type, target_id, hidden_at).await?;
        super::boards::refresh_board_thread_counts(connection, &affected_board_ids).await?;
        if let Some(author_id) = target.author_id {
            let governance_event_id = governance::record_system_event_with_id_tx(
                connection,
                "forum.content.auto_hidden",
                "forum_content",
                &format!("{target_type}:{target_id}"),
                "community report threshold reached",
                Some(&serde_json::json!({ "threshold": 3.0 })),
            )
            .await?;
            governance::notices::create_notice_tx(
                connection,
                author_id,
                "content_restricted",
                &format!("audit:{governance_event_id}:forum-auto-hide"),
                Some(governance_event_id),
                None,
                if target_type == "thread" { "forum_thread" } else { "forum_comment" },
                &target_id.to_string(),
                "你的社区内容因举报阈值被自动隐藏，可在申诉中心查看并申请复核。",
            )
            .await?;
        }
        return Ok(FlagInsertOutcome {
            auto_hidden: true,
            author_id: target.author_id,
            thread_id: target.thread_id,
            board_id: target.board_id,
        });
    }

    super::boards::refresh_board_thread_counts(connection, &affected_board_ids).await?;

    Ok(FlagInsertOutcome {
        auto_hidden: false,
        author_id: target.author_id,
        thread_id: target.thread_id,
        board_id: target.board_id,
    })
}

/// List individual reports, newest first, with bounded cursor pagination.
pub async fn list_flag_queue(
    pool: &PgPool,
    status: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<FlagQueueRow>, Option<i64>)> {
    let before_id = cursor.unwrap_or(i64::MAX);
    let status_filter = status.unwrap_or("open");
    let page_size = limit.clamp(1, 100);

    let rows: Vec<FlagQueueRow> = sqlx::query_as(
        "SELECT flag.id, flag.target_type, flag.target_id, flag.reporter_id, \
                flag.reason, flag.note, flag.weight, flag.status, flag.handled_by, \
                flag.handled_at, flag.auto_hidden_at, flag.resolution_note, flag.created_at, \
                target_author.handle AS author_handle, \
                CASE WHEN flag.target_type = 'thread' \
                     THEN target_thread.title ELSE comment_thread.title END AS target_title, \
                LEFT(CASE WHEN flag.target_type = 'thread' \
                          THEN target_thread.body ELSE target_comment.body END, 500) \
                    AS content_excerpt \
         FROM forum.flags flag \
         LEFT JOIN forum.threads target_thread \
           ON flag.target_type = 'thread' AND target_thread.id = flag.target_id \
         LEFT JOIN forum.comments target_comment \
           ON flag.target_type = 'comment' AND target_comment.id = flag.target_id \
         LEFT JOIN forum.threads comment_thread ON comment_thread.id = target_comment.thread_id \
         LEFT JOIN identity.accounts target_author \
           ON target_author.id = COALESCE(target_thread.author_id, target_comment.author_id) \
         WHERE ($1 = 'all' OR flag.status = $1) AND flag.id < $2 \
         ORDER BY flag.id DESC LIMIT $3",
    )
    .bind(status_filter)
    .bind(before_id)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    let items = if has_more { rows[..page_size as usize].to_vec() } else { rows };
    let next_cursor = has_more.then(|| items.last().map(|row| row.id)).flatten();
    Ok((items, next_cursor))
}

/// Resolve every open report for the selected target as one moderation case.
pub async fn resolve_flag(
    connection: &mut PgConnection,
    flag_id: i64,
    action: &str,
    handled_by: i64,
    note: &str,
) -> AppResult<FlagResolutionOutcome> {
    if !matches!(action, "uphold" | "reject" | "ignore") {
        return Err(AppError::BadRequest("action must be uphold/reject/ignore".into()));
    }
    let terminal_status = match action {
        "uphold" => "upheld",
        "reject" => "rejected",
        "ignore" => "ignored",
        _ => return Err(AppError::BadRequest("action must be uphold/reject/ignore".into())),
    };
    let flag: FlagRow = sqlx::query_as(
        "SELECT id, target_type, target_id, reporter_id, reason, note, weight, status, \
                handled_by, handled_at, auto_hidden_at, resolution_note, created_at \
         FROM forum.flags WHERE id = $1 FOR UPDATE",
    )
    .bind(flag_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(AppError::NotFound)?;
    if flag.status != "open" {
        return Err(AppError::Conflict("flag is already resolved".into()));
    }

    let target = lock_target(connection, &flag.target_type, flag.target_id).await?;
    let affected_board_ids = if flag.target_type == "thread" {
        super::boards::lock_boards_for_thread_count(connection, &[target.board_id]).await?
    } else {
        Vec::new()
    };
    let auto_hidden_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MAX(auto_hidden_at) FROM forum.flags \
         WHERE target_type = $1 AND target_id = $2 AND status = 'open'",
    )
    .bind(&flag.target_type)
    .bind(flag.target_id)
    .fetch_one(&mut *connection)
    .await?;
    let mut content_changed = false;

    if action == "uphold" {
        let statement = match flag.target_type.as_str() {
            "thread" => {
                "UPDATE forum.threads SET deleted_at = now(), deleted_by = $1 \
                 WHERE id = $2 AND deleted_at IS NULL RETURNING id"
            }
            "comment" => {
                "UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 \
                 WHERE id = $2 AND deleted_at IS NULL RETURNING id"
            }
            _ => return Err(AppError::Internal(anyhow::anyhow!("invalid flag target type"))),
        };
        let newly_deleted: Option<i64> = sqlx::query_scalar(statement)
            .bind(handled_by)
            .bind(flag.target_id)
            .fetch_optional(&mut *connection)
            .await?;
        content_changed = newly_deleted.is_some();
        synchronize_target_activity(connection, &flag.target_type, flag.target_id, Utc::now())
            .await?;

        if newly_deleted.is_some() {
            let target_type = match flag.target_type.as_str() {
                "thread" => media::attachments::ForumTargetType::Thread,
                "comment" => media::attachments::ForumTargetType::Comment,
                _ => return Err(AppError::Internal(anyhow::anyhow!("invalid flag target type"))),
            };
            media::attachments::detach_forum_asset_bindings(
                connection,
                target_type,
                flag.target_id,
            )
            .await?;
            if flag.target_type == "comment" {
                sqlx::query(
                    "UPDATE forum.threads SET reply_count = GREATEST(reply_count - 1, 0) \
                     WHERE id = $1",
                )
                .bind(target.thread_id)
                .execute(&mut *connection)
                .await?;
            }
            if let Some(author_id) = target.author_id {
                sqlx::query(
                    "INSERT INTO forum.user_stats (account_id, flagged_upheld) VALUES ($1, 1) \
                     ON CONFLICT (account_id) DO UPDATE \
                     SET flagged_upheld = forum.user_stats.flagged_upheld + 1",
                )
                .bind(author_id)
                .execute(&mut *connection)
                .await?;
            }
        }
        sqlx::query(
            "INSERT INTO forum.user_stats (account_id, flags_upheld) \
             SELECT reporter_id, COUNT(*)::int FROM forum.flags \
             WHERE target_type = $1 AND target_id = $2 AND status = 'open' GROUP BY reporter_id \
             ON CONFLICT (account_id) DO UPDATE \
             SET flags_upheld = forum.user_stats.flags_upheld + EXCLUDED.flags_upheld",
        )
        .bind(&flag.target_type)
        .bind(flag.target_id)
        .execute(&mut *connection)
        .await?;
    } else if let Some(auto_hidden_at) = auto_hidden_at {
        let statement = match flag.target_type.as_str() {
            "thread" => {
                "UPDATE forum.threads SET hidden_at = NULL \
                 WHERE id = $1 AND hidden_at = $2 RETURNING id"
            }
            "comment" => {
                "UPDATE forum.comments SET hidden_at = NULL \
                 WHERE id = $1 AND hidden_at = $2 RETURNING id"
            }
            _ => return Err(AppError::Internal(anyhow::anyhow!("invalid flag target type"))),
        };
        let unhidden: Option<i64> = sqlx::query_scalar(statement)
            .bind(flag.target_id)
            .bind(auto_hidden_at)
            .fetch_optional(&mut *connection)
            .await?;
        if unhidden.is_some() {
            synchronize_target_activity(connection, &flag.target_type, flag.target_id, Utc::now())
                .await?;
        }
    }

    super::boards::refresh_board_thread_counts(connection, &affected_board_ids).await?;

    sqlx::query(
        "UPDATE forum.flags SET status = $1, handled_by = $2, handled_at = now(), \
                resolution_note = $3 \
         WHERE target_type = $4 AND target_id = $5 AND status = 'open'",
    )
    .bind(terminal_status)
    .bind(handled_by)
    .bind(note)
    .bind(&flag.target_type)
    .bind(flag.target_id)
    .execute(&mut *connection)
    .await?;

    let resolved_flag = sqlx::query_as(
        "SELECT id, target_type, target_id, reporter_id, reason, note, weight, status, \
                handled_by, handled_at, auto_hidden_at, resolution_note, created_at \
         FROM forum.flags WHERE id = $1",
    )
    .bind(flag_id)
    .fetch_one(connection)
    .await?;
    Ok(FlagResolutionOutcome {
        flag: resolved_flag,
        thread_id: target.thread_id,
        board_id: target.board_id,
        content_changed,
    })
}

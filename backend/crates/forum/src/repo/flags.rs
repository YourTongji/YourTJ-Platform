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
}

#[derive(Debug, FromRow)]
struct FlagTargetRow {
    author_id: Option<i64>,
    thread_id: i64,
    board_id: i64,
    can_reactivate: bool,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    hidden_at: Option<DateTime<Utc>>,
}

async fn lock_target(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
) -> AppResult<FlagTargetRow> {
    let query = match target_type {
        "thread" => {
            "SELECT author_id, id AS thread_id, board_id, \
                    status = 'visible' AND archived_at IS NULL AS can_reactivate, \
                    created_at, deleted_at, hidden_at \
             FROM forum.threads WHERE id = $1 FOR UPDATE"
        }
        "comment" => {
            "SELECT c.author_id, c.thread_id, t.board_id, \
                    t.status = 'visible' AND t.deleted_at IS NULL \
                      AND t.hidden_at IS NULL AND t.archived_at IS NULL AS can_reactivate, \
                    c.created_at, \
                    c.deleted_at, c.hidden_at \
             FROM forum.comments c \
             JOIN forum.threads t ON t.id = c.thread_id \
             WHERE c.id = $1 FOR UPDATE OF c"
        }
        _ => return Err(AppError::BadRequest("postType must be thread/comment".into())),
    };
    sqlx::query_as(query)
        .bind(target_id)
        .fetch_optional(connection)
        .await?
        .ok_or(AppError::NotFound)
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
        activity::contributions::deactivate_contribution(
            connection,
            &format!("forum_{target_type}:{target_id}"),
            hidden_at,
        )
        .await?;
        super::votes::deactivate_target_vote_contributions(
            connection,
            target_type,
            target_id,
            hidden_at,
        )
        .await?;
        super::boards::refresh_board_thread_counts(connection, &affected_board_ids).await?;
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
        activity::contributions::deactivate_contribution(
            connection,
            &format!("forum_{}:{}", flag.target_type, flag.target_id),
            Utc::now(),
        )
        .await?;
        super::votes::deactivate_target_vote_contributions(
            connection,
            &flag.target_type,
            flag.target_id,
            Utc::now(),
        )
        .await?;

        if newly_deleted.is_some() {
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
        if unhidden.is_some() && target.deleted_at.is_none() && target.can_reactivate {
            if let Some(author_id) = target.author_id {
                activity::contributions::activate_contribution(
                    connection,
                    author_id,
                    match flag.target_type.as_str() {
                        "thread" => activity::contributions::ActivityKind::Thread,
                        "comment" => activity::contributions::ActivityKind::Comment,
                        _ => {
                            return Err(AppError::Internal(anyhow::anyhow!(
                                "invalid flag target type"
                            )))
                        }
                    },
                    &format!("forum_{}:{}", flag.target_type, flag.target_id),
                    target.created_at,
                )
                .await?;
            }
            super::votes::reactivate_target_vote_contributions(
                connection,
                &flag.target_type,
                flag.target_id,
            )
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
    })
}

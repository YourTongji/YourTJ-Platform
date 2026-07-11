//! Read tracking: upsert read position, compute unread counts.

use shared::AppResult;
use sqlx::PgPool;

/// Upsert read position (only forward).
/// Returns true if position was updated (moved forward).
pub async fn upsert_read_position(
    pool: &PgPool,
    account_id: i64,
    thread_id: i64,
    last_read_comment_id: Option<i64>,
) -> AppResult<bool> {
    // Get current position
    let current: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT last_read_comment_id FROM forum.thread_reads \
         WHERE account_id = $1 AND thread_id = $2",
    )
    .bind(account_id)
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;

    // Only forward movement allowed
    if let Some(Some(cur)) = current {
        if let Some(new) = last_read_comment_id {
            if new <= cur {
                return Ok(false); // Backward or same — reject
            }
        }
    }

    sqlx::query(
        "INSERT INTO forum.thread_reads (account_id, thread_id, last_read_comment_id, updated_at) \
         VALUES ($1, $2, $3, now()) \
         ON CONFLICT (account_id, thread_id) \
         DO UPDATE SET last_read_comment_id = COALESCE($3, forum.thread_reads.last_read_comment_id), \
                       updated_at = now()",
    )
    .bind(account_id)
    .bind(thread_id)
    .bind(last_read_comment_id)
    .execute(pool)
    .await?;

    Ok(true)
}

/// Get last read comment ID for a thread.
pub async fn get_last_read_comment_id(
    pool: &PgPool,
    account_id: i64,
    thread_id: i64,
) -> AppResult<Option<i64>> {
    let id: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT last_read_comment_id FROM forum.thread_reads \
         WHERE account_id = $1 AND thread_id = $2",
    )
    .bind(account_id)
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;
    Ok(id.flatten())
}

/// Get unread threads for a user (threads with comments after last_read_comment_id).
pub async fn get_unread_thread_ids(
    pool: &PgPool,
    account_id: i64,
    limit: i64,
    cursor: Option<i64>,
) -> AppResult<(Vec<(i64, i32)>, Option<i64>)> {
    let since_id = cursor.unwrap_or(0);

    let rows: Vec<(i64, i32)> = sqlx::query_as(
        "SELECT t.id, \
                t.reply_count - COALESCE(\
                    (SELECT COUNT(*)::int FROM forum.comments c WHERE c.thread_id = t.id \
                     AND c.id <= COALESCE(tr.last_read_comment_id, 0)), 0\
                ) AS unread_count \
         FROM forum.threads t \
         JOIN forum.thread_reads tr ON tr.thread_id = t.id AND tr.account_id = $1 \
         WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL AND t.archived_at IS NULL \
           AND t.id > $2 \
           AND (t.reply_count > 0) \
           AND (tr.last_read_comment_id IS NULL \
                OR t.last_activity_at > tr.updated_at) \
         ORDER BY t.last_activity_at DESC \
         LIMIT $3",
    )
    .bind(account_id)
    .bind(since_id)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = items.last().map(|r| r.0);

    Ok((items, next_cursor))
}

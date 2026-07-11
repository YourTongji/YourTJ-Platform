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
    let mut tx = pool.begin().await?;
    let thread_exists = sqlx::query_scalar::<_, i64>(
        "SELECT thread.id FROM forum.threads thread \
         WHERE thread.id = $1 AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL FOR KEY SHARE",
    )
    .bind(thread_id)
    .fetch_optional(&mut *tx)
    .await?;
    thread_exists.ok_or(shared::AppError::NotFound)?;

    let resolved_comment_id = if let Some(comment_id) = last_read_comment_id {
        Some(
            sqlx::query_scalar::<_, i64>(
                "SELECT id FROM forum.comments \
             WHERE id = $1 AND thread_id = $2 \
               AND deleted_at IS NULL AND hidden_at IS NULL FOR KEY SHARE",
            )
            .bind(comment_id)
            .bind(thread_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                shared::AppError::BadRequest(
                    "lastReadCommentId must be a visible comment in this thread".into(),
                )
            })?,
        )
    } else {
        sqlx::query_scalar(
            "SELECT MAX(id) FROM forum.comments \
             WHERE thread_id = $1 AND deleted_at IS NULL AND hidden_at IS NULL",
        )
        .bind(thread_id)
        .fetch_one(&mut *tx)
        .await?
    };

    let result = sqlx::query(
        "INSERT INTO forum.thread_reads (account_id, thread_id, last_read_comment_id, updated_at) \
         VALUES ($1, $2, $3, now()) \
         ON CONFLICT (account_id, thread_id) \
         DO UPDATE SET last_read_comment_id = EXCLUDED.last_read_comment_id, updated_at = now() \
         WHERE (forum.thread_reads.last_read_comment_id IS NULL \
                AND EXCLUDED.last_read_comment_id IS NOT NULL) \
            OR EXCLUDED.last_read_comment_id > forum.thread_reads.last_read_comment_id",
    )
    .bind(account_id)
    .bind(thread_id)
    .bind(resolved_comment_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
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
    board_id: Option<i64>,
    tag_slug: Option<&str>,
    limit: i64,
    cursor: Option<i64>,
) -> AppResult<(Vec<(i64, i32)>, Option<i64>)> {
    let before_id = cursor.unwrap_or(i64::MAX);
    let page_size = limit.clamp(1, 100);

    let rows: Vec<(i64, i32)> = sqlx::query_as(
        "SELECT thread.id, \
                (SELECT COUNT(*)::int FROM forum.comments comment \
                 WHERE comment.thread_id = thread.id \
                   AND comment.deleted_at IS NULL AND comment.hidden_at IS NULL \
                   AND comment.id > COALESCE(thread_read.last_read_comment_id, 0)) \
                  AS unread_count \
         FROM forum.threads thread \
         JOIN forum.thread_reads thread_read \
           ON thread_read.thread_id = thread.id AND thread_read.account_id = $1 \
         WHERE thread.status = 'visible' AND thread.deleted_at IS NULL \
           AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
           AND thread.id < $2 \
           AND ($3::bigint IS NULL OR thread.board_id = $3) \
           AND ($4::text IS NULL OR EXISTS ( \
             SELECT 1 FROM forum.thread_tags thread_tag \
             JOIN forum.tags tag ON tag.id = thread_tag.tag_id \
             WHERE thread_tag.thread_id = thread.id AND tag.slug = $4 \
           )) \
           AND NOT EXISTS (SELECT 1 FROM forum.user_ignores ignored \
             WHERE ignored.account_id = $1 \
               AND ignored.ignored_account_id = thread.author_id) \
           AND EXISTS (SELECT 1 FROM forum.comments unread_comment \
             WHERE unread_comment.thread_id = thread.id \
               AND unread_comment.deleted_at IS NULL AND unread_comment.hidden_at IS NULL \
               AND unread_comment.id > COALESCE(thread_read.last_read_comment_id, 0)) \
         ORDER BY thread.id DESC \
         LIMIT $5",
    )
    .bind(account_id)
    .bind(before_id)
    .bind(board_id)
    .bind(tag_slug)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    let items = if has_more { rows[..page_size as usize].to_vec() } else { rows };
    let next_cursor = has_more.then(|| items.last().map(|row| row.0)).flatten();

    Ok((items, next_cursor))
}

use shared::AppResult;
use sqlx::PgPool;

use crate::models::CommentRowJoined;

use super::{base64_decode_str, base64_encode_str};

/// List comments for a thread with cursor pagination.
/// Ordered by `path` ASC for correct nested (楼中楼) display.
/// When `current_user_id` is `Some`, comments by users the current user has
/// ignored are excluded.
pub async fn list_comments(
    pool: &PgPool,
    thread_id: i64,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
) -> AppResult<(Vec<CommentRowJoined>, Option<String>)> {
    let cursor_path: Option<String> = cursor
        .map(base64_decode_str)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let Some(ref cp) = cursor_path {
        sqlx::query_as::<_, CommentRowJoined>(
            "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                    c.body, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                    c.quoted_comment_id, \
                    a.handle AS author_handle \
             FROM forum.comments c \
             JOIN identity.accounts a ON a.id = c.author_id \
             WHERE c.thread_id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL \
               AND c.path > $3 \
               AND ($4::bigint IS NULL OR c.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $4 \
               )) \
             ORDER BY c.path ASC \
             LIMIT $2",
        )
        .bind(thread_id)
        .bind(limit + 1)
        .bind(cp)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, CommentRowJoined>(
            "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                    c.body, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                    c.quoted_comment_id, \
                    a.handle AS author_handle \
             FROM forum.comments c \
             JOIN identity.accounts a ON a.id = c.author_id \
             WHERE c.thread_id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL \
               AND ($3::bigint IS NULL OR c.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $3 \
               )) \
             ORDER BY c.path ASC \
             LIMIT $2",
        )
        .bind(thread_id)
        .bind(limit + 1)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more {
        items.last().and_then(|r| r.path.as_ref()).map(|p| base64_encode_str(p))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// Create a comment with materialized path for 楼中楼 ordering.
///
/// If `parent_id` is provided, the path is computed as `{parent_path}.{next_sibling}`.
/// Otherwise the path is the next zero-padded top-level index.
/// Uses a transaction with row-level locks for race-free path generation.
pub async fn create_comment(
    pool: &PgPool,
    thread_id: i64,
    author_id: i64,
    body: &str,
    parent_id: Option<i64>,
    quoted_comment_id: Option<i64>,
) -> AppResult<CommentRowJoined> {
    let mut tx = pool.begin().await?;

    if let Some(pid) = parent_id {
        // Lock the parent comment row to prevent concurrent sibling inserts.
        // Fetch parent path inside the same transaction with FOR UPDATE.
        let parent_path: Option<String> = sqlx::query_scalar(
            "SELECT path FROM forum.comments WHERE id = $1 AND thread_id = $2 FOR UPDATE",
        )
        .bind(pid)
        .bind(thread_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        let parent_path = parent_path.ok_or(crate::error::ForumError::CommentMissing)?;

        // Find max child path under this parent inside the locked transaction.
        let max_child: String = sqlx::query_scalar(
            "SELECT COALESCE(MAX(path), '') FROM forum.comments \
             WHERE thread_id = $1 AND parent_id = $2 AND path IS NOT NULL",
        )
        .bind(thread_id)
        .bind(pid)
        .fetch_one(&mut *tx)
        .await?;

        let next_index = next_sibling_index(&max_child, &parent_path);
        let path = format!("{parent_path}.{next_index:04x}");

        let row = insert_comment_tx(
            &mut tx,
            thread_id,
            parent_id,
            &path,
            author_id,
            body,
            quoted_comment_id,
        )
        .await?;
        tx.commit().await?;
        Ok(row)
    } else {
        // Lock the thread row to prevent concurrent top-level comment creation.
        let _: (i64,) = sqlx::query_as("SELECT id FROM forum.threads WHERE id = $1 FOR UPDATE")
            .bind(thread_id)
            .fetch_one(&mut *tx)
            .await?;

        // Top-level comment: find next top-level index.
        let max_path: String = sqlx::query_scalar(
            "SELECT COALESCE(MAX(path), '') FROM forum.comments \
             WHERE thread_id = $1 AND parent_id IS NULL AND path IS NOT NULL",
        )
        .bind(thread_id)
        .fetch_one(&mut *tx)
        .await?;

        let top_level = next_sibling_index(&max_path, "");
        let path = format!("{top_level:04x}");

        let row =
            insert_comment_tx(&mut tx, thread_id, None, &path, author_id, body, quoted_comment_id)
                .await?;
        tx.commit().await?;
        Ok(row)
    }
}

/// Insert the comment row and update thread reply_count in the active transaction.
async fn insert_comment_tx(
    tx: &mut sqlx::PgConnection,
    thread_id: i64,
    parent_id: Option<i64>,
    path: &str,
    author_id: i64,
    body: &str,
    quoted_comment_id: Option<i64>,
) -> AppResult<CommentRowJoined> {
    let row = sqlx::query_as::<_, CommentRowJoined>(
        "WITH inserted AS ( \
            INSERT INTO forum.comments (thread_id, parent_id, path, author_id, body, quoted_comment_id) \
            VALUES ($1, $2, $3, $4, $5, $6) \
            RETURNING id, thread_id, parent_id, path, author_id, body, vote_count, created_at, quoted_comment_id \
         ) \
         SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                c.quoted_comment_id, \
                a.handle AS author_handle \
         FROM inserted c \
         JOIN identity.accounts a ON a.id = c.author_id",
    )
    .bind(thread_id)
    .bind(parent_id)
    .bind(path)
    .bind(author_id)
    .bind(body)
    .bind(quoted_comment_id)
    .fetch_one(&mut *tx)
    .await?;

    // Bump thread reply_count and last_activity_at.
    sqlx::query(
        "UPDATE forum.threads \
         SET reply_count = reply_count + 1, last_activity_at = now() \
         WHERE id = $1",
    )
    .bind(thread_id)
    .execute(&mut *tx)
    .await?;

    Ok(row)
}

/// Compute the next sibling index from the max child path.
///
/// Given a max child path like "0003.0007" and parent path "0003", returns 8.
pub fn next_sibling_index(max_child_path: &str, parent_path: &str) -> u32 {
    if max_child_path.is_empty() || max_child_path == parent_path {
        1
    } else {
        let parent_prefix =
            if parent_path.is_empty() { String::new() } else { format!("{parent_path}.") };
        let suffix = max_child_path.strip_prefix(&parent_prefix).unwrap_or(max_child_path);
        let last = suffix.split('.').next().unwrap_or("0");
        u32::from_str_radix(last, 16).unwrap_or(0).saturating_add(1)
    }
}

/// Find a single comment by id, joined with author handle.
pub async fn find_comment(pool: &PgPool, id: i64) -> AppResult<Option<CommentRowJoined>> {
    let row = sqlx::query_as::<_, CommentRowJoined>(
        "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                c.quoted_comment_id, \
                a.handle AS author_handle \
         FROM forum.comments c \
         JOIN identity.accounts a ON a.id = c.author_id \
         WHERE c.id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Update a comment's body. Returns the updated row joined with author handle.
pub async fn update_comment(pool: &PgPool, id: i64, body: &str) -> AppResult<CommentRowJoined> {
    let row = sqlx::query_as::<_, CommentRowJoined>(
        "WITH updated AS ( \
         UPDATE forum.comments SET body = $1, edited_at = now() WHERE id = $2 \
         RETURNING id, thread_id, parent_id, path, author_id, body, vote_count, created_at, quoted_comment_id \
         ) \
         SELECT u.id, u.thread_id, u.parent_id, u.path, u.author_id, \
                u.body, u.vote_count, u.deleted_at, u.hidden_at, u.edited_at, u.created_at, \
                u.quoted_comment_id, \
                a.handle AS author_handle \
         FROM updated u \
         JOIN identity.accounts a ON a.id = u.author_id",
    )
    .bind(body)
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

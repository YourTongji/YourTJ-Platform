//! Bookmark CRUD for threads and comments.

use shared::AppResult;
use sqlx::PgPool;

use crate::models::BookmarkRow;

/// Upsert a bookmark (create or update note).
pub async fn upsert_bookmark(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
    note: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO forum.bookmarks (account_id, target_type, target_id, note) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (account_id, target_type, target_id) \
         DO UPDATE SET note = EXCLUDED.note",
    )
    .bind(account_id)
    .bind(target_type)
    .bind(target_id)
    .bind(note)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a bookmark.
pub async fn delete_bookmark(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "DELETE FROM forum.bookmarks \
         WHERE account_id = $1 AND target_type = $2 AND target_id = $3",
    )
    .bind(account_id)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// List bookmarks for an account, cursor-paginated by `created_at DESC`.
///
/// Cursor is an epoch-seconds timestamp. Returns at most `limit` rows plus
/// an optional opaque cursor for the next page.
pub async fn list_bookmarks(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<BookmarkRow>, Option<i64>)> {
    let cursor_ts = cursor.unwrap_or(i64::MAX);

    let rows: Vec<BookmarkRow> = sqlx::query_as(
        "SELECT account_id, target_type, target_id, note, created_at \
         FROM forum.bookmarks \
         WHERE account_id = $1 AND EXTRACT(EPOCH FROM created_at)::bigint < $2 \
         ORDER BY created_at DESC \
         LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor_ts)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|r| r.created_at.timestamp()) } else { None };

    Ok((items, next_cursor))
}

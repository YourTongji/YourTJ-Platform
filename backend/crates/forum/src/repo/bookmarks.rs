//! Bookmark CRUD for threads and comments.

use shared::{AppError, AppResult};
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
    if !matches!(target_type, "thread" | "comment") {
        return Err(AppError::BadRequest("postType must be thread/comment".into()));
    }
    if note.is_some_and(|note| note.chars().count() > 500) {
        return Err(AppError::BadRequest("note must be at most 500 characters".into()));
    }
    let mut tx = pool.begin().await?;
    let target_exists = if target_type == "thread" {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM forum.threads \
             WHERE id = $1 AND status = 'visible' AND deleted_at IS NULL \
               AND hidden_at IS NULL AND archived_at IS NULL \
             FOR KEY SHARE",
        )
        .bind(target_id)
        .fetch_optional(&mut *tx)
        .await?
        .is_some()
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT comment.id FROM forum.comments comment \
             JOIN forum.threads thread ON thread.id = comment.thread_id \
             WHERE comment.id = $1 AND comment.deleted_at IS NULL \
               AND comment.hidden_at IS NULL AND thread.status = 'visible' \
               AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
               AND thread.archived_at IS NULL \
             FOR KEY SHARE OF comment, thread",
        )
        .bind(target_id)
        .fetch_optional(&mut *tx)
        .await?
        .is_some()
    };
    if !target_exists {
        return Err(AppError::NotFound);
    }
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
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Delete a bookmark.
pub async fn delete_bookmark(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
) -> AppResult<()> {
    if !matches!(target_type, "thread" | "comment") {
        return Err(AppError::BadRequest("postType must be thread/comment".into()));
    }
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

fn encode_bookmark_cursor(row: &BookmarkRow) -> String {
    super::base64_encode_str(&format!(
        "{}|{}|{}",
        row.created_at.timestamp_micros(),
        row.target_type,
        row.target_id
    ))
}

fn decode_bookmark_cursor(cursor: &str) -> AppResult<(chrono::DateTime<chrono::Utc>, String, i64)> {
    let decoded = super::base64_decode_str(cursor)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let mut parts = decoded.split('|');
    let micros = parts
        .next()
        .and_then(|part| part.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    let target_type = parts
        .next()
        .filter(|target_type| matches!(*target_type, "thread" | "comment"))
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?
        .to_owned();
    let target_id = parts
        .next()
        .and_then(|part| part.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    if parts.next().is_some() {
        return Err(AppError::BadRequest("invalid cursor".into()));
    }
    let created_at = chrono::DateTime::from_timestamp_micros(micros)
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    Ok((created_at, target_type, target_id))
}

/// List current bookmarks with a stable, bounded cursor.
pub async fn list_bookmarks(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<BookmarkRow>, Option<String>)> {
    let (cursor_at, cursor_type, cursor_id) = match cursor {
        Some(cursor) => {
            let (created_at, target_type, target_id) = decode_bookmark_cursor(cursor)?;
            (Some(created_at), Some(target_type), Some(target_id))
        }
        None => (None, None, None),
    };
    let page_size = limit.clamp(1, 100);

    let mut rows: Vec<BookmarkRow> = sqlx::query_as(
        "SELECT bookmark.account_id, bookmark.target_type, bookmark.target_id, \
                bookmark.note, bookmark.created_at \
         FROM forum.bookmarks bookmark \
         WHERE bookmark.account_id = $1 \
           AND ($2::timestamptz IS NULL OR \
                (bookmark.created_at, bookmark.target_type, bookmark.target_id) < ($2, $3, $4)) \
           AND ( \
             (bookmark.target_type = 'thread' AND EXISTS ( \
               SELECT 1 FROM forum.threads thread WHERE thread.id = bookmark.target_id \
                 AND thread.status = 'visible' AND thread.deleted_at IS NULL \
                 AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
             )) OR \
             (bookmark.target_type = 'comment' AND EXISTS ( \
               SELECT 1 FROM forum.comments comment \
               JOIN forum.threads thread ON thread.id = comment.thread_id \
               WHERE comment.id = bookmark.target_id AND comment.deleted_at IS NULL \
                 AND comment.hidden_at IS NULL AND thread.status = 'visible' \
                 AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
                 AND thread.archived_at IS NULL \
             )) \
           ) \
         ORDER BY bookmark.created_at DESC, bookmark.target_type DESC, bookmark.target_id DESC \
         LIMIT $5",
    )
    .bind(account_id)
    .bind(cursor_at)
    .bind(cursor_type)
    .bind(cursor_id)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(encode_bookmark_cursor)).flatten();

    Ok((rows, next_cursor))
}

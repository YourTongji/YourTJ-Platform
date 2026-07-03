use shared::AppResult;
use sqlx::PgPool;

use crate::dto::ThreadInput;
use crate::models::ThreadRowJoined;

use super::{base64_decode_i64, base64_encode_i64, decode_hot_cursor, encode_hot_cursor};

/// List threads for a board with cursor pagination.
///
/// `sort` is "hot" (hot_score desc, last_activity_at desc) or "new" (created_at desc).
/// `cursor` is an opaque base64-encoded value from the previous page.
/// Returns `(rows, next_cursor)`.
pub async fn list_threads(
    pool: &PgPool,
    board_id: i64,
    sort: &str,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    match sort {
        "hot" => list_threads_hot(pool, board_id, cursor, limit).await,
        _ => list_threads_new(pool, board_id, cursor, limit).await,
    }
}

async fn list_threads_new(
    pool: &PgPool,
    board_id: i64,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let cursor_id: Option<i64> = cursor
        .map(base64_decode_i64)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let Some(cid) = cursor_id {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.created_at < (SELECT created_at FROM forum.threads WHERE id = $3) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .bind(cid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|r| base64_encode_i64(r.id)) } else { None };

    Ok((items, next_cursor))
}

async fn list_threads_hot(
    pool: &PgPool,
    board_id: i64,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let (cursor_hot, cursor_id): (Option<f64>, Option<i64>) = if let Some(c) = cursor {
        let decoded = decode_hot_cursor(c)
            .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
        (Some(decoded.0), Some(decoded.1))
    } else {
        (None, None)
    };

    let rows = if let (Some(ch), Some(ci)) = (cursor_hot, cursor_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 \
               AND (COALESCE(t.hot_score, 0) < $3 \
                    OR (COALESCE(t.hot_score, 0) = $3 AND t.id < $4)) \
             ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .bind(ch)
        .bind(ci)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 \
             ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more {
        items.last().map(|r| encode_hot_cursor(r.hot_score.unwrap_or(0.0), r.id))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// List threads across all boards (optional board filter) with cursor pagination.
///
/// `board_id` is optional — when `None`, returns threads from all boards.
pub async fn list_threads_feed(
    pool: &PgPool,
    board_id: Option<i64>,
    sort: &str,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    match sort {
        "hot" => list_threads_feed_hot(pool, board_id, cursor, limit).await,
        _ => list_threads_feed_new(pool, board_id, cursor, limit).await,
    }
}

async fn list_threads_feed_new(
    pool: &PgPool,
    board_id: Option<i64>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let cursor_id: Option<i64> = cursor
        .map(base64_decode_i64)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let (Some(cid), Some(bid)) = (cursor_id, board_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.created_at < (SELECT created_at FROM forum.threads WHERE id = $3) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(bid)
        .bind(limit + 1)
        .bind(cid)
        .fetch_all(pool)
        .await?
    } else if let (Some(cid), None) = (cursor_id, board_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.created_at < (SELECT created_at FROM forum.threads WHERE id = $2) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $1",
        )
        .bind(limit + 1)
        .bind(cid)
        .fetch_all(pool)
        .await?
    } else if let (None, Some(bid)) = (cursor_id, board_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(bid)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $1",
        )
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|r| base64_encode_i64(r.id)) } else { None };

    Ok((items, next_cursor))
}

async fn list_threads_feed_hot(
    pool: &PgPool,
    board_id: Option<i64>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let (cursor_hot, cursor_id): (Option<f64>, Option<i64>) = if let Some(c) = cursor {
        let decoded = decode_hot_cursor(c)
            .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
        (Some(decoded.0), Some(decoded.1))
    } else {
        (None, None)
    };

    let (ch, ci) = (cursor_hot, cursor_id);
    let rows = match (board_id, ch, ci) {
        (Some(bid), Some(ch), Some(ci)) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.board_id = $1 \
                   AND (COALESCE(t.hot_score, 0) < $3 \
                        OR (COALESCE(t.hot_score, 0) = $3 AND t.id < $4)) \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $2",
            )
            .bind(bid)
            .bind(limit + 1)
            .bind(ch)
            .bind(ci)
            .fetch_all(pool)
            .await?
        }
        (None, Some(ch), Some(ci)) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE (COALESCE(t.hot_score, 0) < $2 \
                        OR (COALESCE(t.hot_score, 0) = $2 AND t.id < $3)) \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $1",
            )
            .bind(limit + 1)
            .bind(ch)
            .bind(ci)
            .fetch_all(pool)
            .await?
        }
        (Some(bid), _, _) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.board_id = $1 \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $2",
            )
            .bind(bid)
            .bind(limit + 1)
            .fetch_all(pool)
            .await?
        }
        (None, _, _) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $1",
            )
            .bind(limit + 1)
            .fetch_all(pool)
            .await?
        }
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more {
        items.last().map(|r| encode_hot_cursor(r.hot_score.unwrap_or(0.0), r.id))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// Find a single thread by id, joined with author handle.
pub async fn find_thread(pool: &PgPool, id: i64) -> AppResult<Option<ThreadRowJoined>> {
    let row = sqlx::query_as::<_, ThreadRowJoined>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         WHERE t.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Insert a new thread. Returns the created thread joined with author handle.
pub async fn create_thread(
    pool: &PgPool,
    board_id: i64,
    author_id: i64,
    input: &crate::dto::ThreadInput,
) -> AppResult<ThreadRowJoined> {
    let row = sqlx::query_as::<_, ThreadRowJoined>(
        "WITH inserted AS ( \
            INSERT INTO forum.threads (board_id, author_id, title, body) \
            VALUES ($1, $2, $3, $4) \
            RETURNING id, board_id, author_id, title, body, reply_count, vote_count, \
                      hot_score, status, created_at, last_activity_at \
         ) \
         SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM inserted t \
         JOIN identity.accounts a ON a.id = t.author_id",
    )
    .bind(board_id)
    .bind(author_id)
    .bind(&input.title)
    .bind(&input.body)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Update a thread's title and/or body. Returns the updated row joined with author handle.
pub async fn update_thread(
    pool: &PgPool,
    id: i64,
    title: Option<&str>,
    body: Option<&str>,
) -> AppResult<ThreadRowJoined> {
    let row = sqlx::query_as::<_, ThreadRowJoined>(
        "WITH updated AS ( \
         UPDATE forum.threads SET \
         title = COALESCE($1, title), \
         body = COALESCE($2, body), \
         edited_at = now() \
         WHERE id = $3 \
         RETURNING id, board_id, author_id, title, body, reply_count, vote_count, \
                   hot_score, status, created_at, last_activity_at \
         ) \
         SELECT u.id, u.board_id, u.author_id, u.title, u.body, \
                u.reply_count, u.vote_count, u.hot_score, u.status, \
                u.created_at, u.last_activity_at, \
                a.handle AS author_handle \
         FROM updated u \
         JOIN identity.accounts a ON a.id = u.author_id",
    )
    .bind(title)
    .bind(body)
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

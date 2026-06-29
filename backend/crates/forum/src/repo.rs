//! Database access layer for the forum domain.
//!
//! Every function takes `&PgPool` and returns `Result` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use shared::AppResult;
use sqlx::PgPool;

use crate::dto::ThreadInput;
use crate::models::{BoardRow, CommentRowJoined, ThreadRowJoined};

// ---------------------------------------------------------------------------
// boards
// ---------------------------------------------------------------------------

/// List all boards.
pub async fn list_boards(pool: &PgPool) -> AppResult<Vec<BoardRow>> {
    let rows = sqlx::query_as::<_, BoardRow>("SELECT id, slug, name FROM forum.boards ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// threads
// ---------------------------------------------------------------------------

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
    input: &ThreadInput,
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

// ---------------------------------------------------------------------------
// comments
// ---------------------------------------------------------------------------

/// List comments for a thread with cursor pagination.
/// Ordered by `path` ASC for correct nested (楼中楼) display.
pub async fn list_comments(
    pool: &PgPool,
    thread_id: i64,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<CommentRowJoined>, Option<String>)> {
    let cursor_path: Option<String> = cursor
        .map(base64_decode_str)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let Some(ref cp) = cursor_path {
        sqlx::query_as::<_, CommentRowJoined>(
            "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                    c.body, c.vote_count, c.created_at, \
                    a.handle AS author_handle \
             FROM forum.comments c \
             JOIN identity.accounts a ON a.id = c.author_id \
             WHERE c.thread_id = $1 AND c.path > $3 \
             ORDER BY c.path ASC \
             LIMIT $2",
        )
        .bind(thread_id)
        .bind(limit + 1)
        .bind(cp)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, CommentRowJoined>(
            "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                    c.body, c.vote_count, c.created_at, \
                    a.handle AS author_handle \
             FROM forum.comments c \
             JOIN identity.accounts a ON a.id = c.author_id \
             WHERE c.thread_id = $1 \
             ORDER BY c.path ASC \
             LIMIT $2",
        )
        .bind(thread_id)
        .bind(limit + 1)
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
pub async fn create_comment(
    pool: &PgPool,
    thread_id: i64,
    author_id: i64,
    body: &str,
    parent_id: Option<i64>,
) -> AppResult<CommentRowJoined> {
    let path = if let Some(pid) = parent_id {
        // Find parent's path.
        let parent_path: Option<String> =
            sqlx::query_scalar("SELECT path FROM forum.comments WHERE id = $1 AND thread_id = $2")
                .bind(pid)
                .bind(thread_id)
                .fetch_optional(pool)
                .await?
                .flatten();

        let parent_path = parent_path.ok_or(crate::error::ForumError::CommentMissing)?;

        // Find max child path under this parent.
        let max_child: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE(MAX(path), '') FROM forum.comments \
             WHERE thread_id = $1 AND parent_id = $2 AND path IS NOT NULL",
        )
        .bind(thread_id)
        .bind(pid)
        .fetch_one(pool)
        .await?;

        let max_child_str = max_child.unwrap_or_default();
        let next_index = next_sibling_index(&max_child_str, &parent_path);
        format!("{parent_path}.{next_index:04x}")
    } else {
        // Top-level comment: find next top-level index.
        let max_path: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE(MAX(path), '') FROM forum.comments \
             WHERE thread_id = $1 AND parent_id IS NULL AND path IS NOT NULL",
        )
        .bind(thread_id)
        .fetch_one(pool)
        .await?;

        let max_path_str = max_path.unwrap_or_default();
        let top_level = next_sibling_index(&max_path_str, "");
        format!("{top_level:04x}")
    };

    let row = sqlx::query_as::<_, CommentRowJoined>(
        "WITH inserted AS ( \
            INSERT INTO forum.comments (thread_id, parent_id, path, author_id, body) \
            VALUES ($1, $2, $3, $4, $5) \
            RETURNING id, thread_id, parent_id, path, author_id, body, vote_count, created_at \
         ) \
         SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.vote_count, c.created_at, \
                a.handle AS author_handle \
         FROM inserted c \
         JOIN identity.accounts a ON a.id = c.author_id",
    )
    .bind(thread_id)
    .bind(parent_id)
    .bind(&path)
    .bind(author_id)
    .bind(body)
    .fetch_one(pool)
    .await?;

    // Bump thread reply_count and last_activity_at.
    sqlx::query(
        "UPDATE forum.threads \
         SET reply_count = reply_count + 1, last_activity_at = now() \
         WHERE id = $1",
    )
    .bind(thread_id)
    .execute(pool)
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

// ---------------------------------------------------------------------------
// votes
// ---------------------------------------------------------------------------

/// Vote on a thread or comment. `value` is "up" (+1) or "down" (-1).
///
/// Returns `true` if a row was affected, `false` if the post was not found.
pub async fn vote_post(pool: &PgPool, post_id: i64, value: &str) -> AppResult<bool> {
    let delta: i32 = match value {
        "up" => 1,
        "down" => -1,
        _ => return Err(shared::AppError::BadRequest("vote value must be 'up' or 'down'".into())),
    };

    // Try threads first.
    let affected =
        sqlx::query("UPDATE forum.threads SET vote_count = vote_count + $1 WHERE id = $2")
            .bind(delta)
            .bind(post_id)
            .execute(pool)
            .await?
            .rows_affected();

    if affected > 0 {
        return Ok(true);
    }

    // Try comments.
    let affected =
        sqlx::query("UPDATE forum.comments SET vote_count = vote_count + $1 WHERE id = $2")
            .bind(delta)
            .bind(post_id)
            .execute(pool)
            .await?
            .rows_affected();

    Ok(affected > 0)
}

// ---------------------------------------------------------------------------
// cursor helpers
// ---------------------------------------------------------------------------

fn base64_encode_i64(val: i64) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(val.to_string())
}

fn base64_decode_i64(s: &str) -> Result<i64, String> {
    use base64::Engine;
    let bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).map_err(|e| e.to_string())?;
    let s = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    s.parse::<i64>().map_err(|e| e.to_string())
}

fn base64_encode_str(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
}

fn base64_decode_str(s: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn encode_hot_cursor(hot_score: f64, id: i64) -> String {
    base64_encode_str(&format!("{hot_score}:{id}"))
}

fn decode_hot_cursor(cursor: &str) -> Result<(f64, i64), String> {
    let s = base64_decode_str(cursor)?;
    let (hot_str, id_str) = s.rsplit_once(':').ok_or("invalid hot cursor")?;
    let hot_score = hot_str.parse::<f64>().map_err(|e| e.to_string())?;
    let id = id_str.parse::<i64>().map_err(|e| e.to_string())?;
    Ok((hot_score, id))
}

/// Compute hot rank scores and store in Redis ZSET.
pub async fn refresh_hot_rank(pool: &deadpool_redis::Pool, db: &PgPool) -> anyhow::Result<()> {
    let threads = sqlx::query_as::<_, (i64, i32, i32)>(
        "SELECT id, vote_count, reply_count FROM forum.threads WHERE status = 'normal'",
    )
    .fetch_all(db)
    .await?;
    let mut conn = pool.get().await?;
    for (id, vote_count, reply_count) in threads {
        let score = (vote_count as f64) * 0.7 + (reply_count as f64) * 0.3;
        redis::cmd("ZADD")
            .arg("hot:threads")
            .arg(score)
            .arg(id)
            .query_async::<()>(&mut conn)
            .await?;
    }
    Ok(())
}

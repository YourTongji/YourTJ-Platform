use shared::AppResult;
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::dto::ContentFormat;
use crate::models::{ThreadRowJoined, ThreadRowJoinedFull};

use super::{base64_decode_i64, base64_encode_i64, decode_hot_cursor, encode_hot_cursor};

/// Build canonical bounded body excerpts for a batch of thread summaries.
pub async fn get_thread_body_excerpts(
    pool: &PgPool,
    thread_ids: &[i64],
) -> AppResult<std::collections::HashMap<i64, String>> {
    if thread_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let rows: Vec<(i64, Option<String>, String)> =
        sqlx::query_as("SELECT id, body, content_format FROM forum.threads WHERE id = ANY($1)")
            .bind(thread_ids)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(thread_id, body, format)| {
            let excerpt = crate::content_policy::plain_text_projection(
                body.as_deref().unwrap_or_default(),
                ContentFormat::from_db(&format),
                280,
            );
            (thread_id, excerpt)
        })
        .collect())
}

/// List threads for a board with cursor pagination.
///
/// `sort` is "hot" (hot_score desc, last_activity_at desc) or "new" (created_at desc).
/// `cursor` is an opaque base64-encoded value from the previous page.
/// When `current_user_id` is `Some`, threads authored by users the
/// current user has ignored are excluded.
/// Returns `(rows, next_cursor)`.
pub async fn list_threads(
    pool: &PgPool,
    board_id: i64,
    sort: &str,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    match sort {
        "hot" => list_threads_hot(pool, board_id, cursor, limit, current_user_id).await,
        _ => list_threads_new(pool, board_id, cursor, limit, current_user_id).await,
    }
}

async fn list_threads_new(
    pool: &PgPool,
    board_id: i64,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let cursor_id: Option<i64> = cursor
        .map(base64_decode_i64)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let Some(cid) = cursor_id {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND (t.created_at, t.id) < (SELECT created_at, id FROM forum.threads WHERE id = $3) \
               AND ($4::bigint IS NULL OR NOT forum.user_content_hidden($4, t.author_id)) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .bind(cid)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($3::bigint IS NULL OR NOT forum.user_content_hidden($3, t.author_id)) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .bind(current_user_id)
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
    current_user_id: Option<i64>,
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 \
               AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND (COALESCE(t.hot_score, 0) < $3 \
                    OR (COALESCE(t.hot_score, 0) = $3 AND t.id < $4)) \
               AND ($5::bigint IS NULL OR NOT forum.user_content_hidden($5, t.author_id)) \
             ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .bind(ch)
        .bind(ci)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($3::bigint IS NULL OR NOT forum.user_content_hidden($3, t.author_id)) \
             ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(board_id)
        .bind(limit + 1)
        .bind(current_user_id)
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
/// When `current_user_id` is `Some`, threads authored by users the
/// current user has ignored are excluded.
pub async fn list_threads_feed(
    pool: &PgPool,
    board_id: Option<i64>,
    sort: &str,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    match sort {
        "hot" => list_threads_feed_hot(pool, board_id, cursor, limit, current_user_id).await,
        _ => list_threads_feed_new(pool, board_id, cursor, limit, current_user_id).await,
    }
}

/// List visible threads matching an exact tag slug, optionally limited to subscriptions.
#[allow(clippy::too_many_arguments)] // reason: feed filters and viewer scope are independent bound inputs
pub async fn list_threads_by_tag(
    pool: &PgPool,
    board_id: Option<i64>,
    tag_slug: &str,
    sort: &str,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
    subscription_account_id: Option<i64>,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let page_size = limit.clamp(1, 100);
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT thread.id, thread.board_id, thread.author_id, thread.title, thread.body, thread.content_version, \
                thread.reply_count, thread.vote_count, thread.hot_score, thread.status, \
                thread.created_at, thread.last_activity_at, account.handle AS author_handle \
         FROM forum.threads thread \
         JOIN identity.accounts account ON account.id = thread.author_id \
         WHERE thread.status = 'visible' AND thread.deleted_at IS NULL \
           AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
           AND EXISTS (SELECT 1 FROM forum.thread_tags thread_tag \
                       JOIN forum.tags tag ON tag.id = thread_tag.tag_id \
                       WHERE thread_tag.thread_id = thread.id AND tag.slug = ",
    );
    query.push_bind(tag_slug).push(")");
    if let Some(board_id) = board_id {
        query.push(" AND thread.board_id = ").push_bind(board_id);
    }
    if let Some(account_id) = current_user_id {
        query
            .push(" AND NOT forum.user_content_hidden(")
            .push_bind(account_id)
            .push(", thread.author_id)");
    }
    if let Some(account_id) = subscription_account_id {
        query
            .push(
                " AND COALESCE((SELECT direct.level FROM forum.subscriptions direct \
                   WHERE direct.account_id = ",
            )
            .push_bind(account_id)
            .push(
                " AND direct.target_type = 'thread' AND direct.target_id = thread.id), \
                   (SELECT board.level FROM forum.subscriptions board \
                    WHERE board.account_id = ",
            )
            .push_bind(account_id)
            .push(
                " AND board.target_type = 'board' \
                    AND board.target_id = thread.board_id)) IN ('watching', 'tracking')",
            );
    }

    let next_cursor = if sort == "hot" {
        if let Some(cursor) = cursor {
            let (hot_score, thread_id) = decode_hot_cursor(cursor)
                .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
            query
                .push(" AND (COALESCE(thread.hot_score, 0), thread.id) < (")
                .push_bind(hot_score)
                .push(", ")
                .push_bind(thread_id)
                .push(")");
        }
        query.push(" ORDER BY COALESCE(thread.hot_score, 0) DESC, thread.id DESC LIMIT ");
        query.push_bind(page_size + 1);
        let mut rows = query.build_query_as::<ThreadRowJoined>().fetch_all(pool).await?;
        let has_more = rows.len() > page_size as usize;
        if has_more {
            rows.truncate(page_size as usize);
        }
        let next = has_more.then(|| {
            rows.last().map(|row| encode_hot_cursor(row.hot_score.unwrap_or(0.0), row.id))
        });
        return Ok((rows, next.flatten()));
    } else {
        if let Some(cursor) = cursor {
            let thread_id = base64_decode_i64(cursor)
                .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
            query
                .push(
                    " AND (thread.created_at, thread.id) < (SELECT created_at, id \
                       FROM forum.threads WHERE id = ",
                )
                .push_bind(thread_id)
                .push(")");
        }
        query.push(" ORDER BY thread.created_at DESC, thread.id DESC LIMIT ");
        query.push_bind(page_size + 1);
        None
    };

    let mut rows = query.build_query_as::<ThreadRowJoined>().fetch_all(pool).await?;
    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = next_cursor
        .or_else(|| has_more.then(|| rows.last().map(|row| base64_encode_i64(row.id))).flatten());
    Ok((rows, next_cursor))
}

async fn list_threads_feed_new(
    pool: &PgPool,
    board_id: Option<i64>,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let cursor_id: Option<i64> = cursor
        .map(base64_decode_i64)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let (Some(cid), Some(bid)) = (cursor_id, board_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND (t.created_at, t.id) < (SELECT created_at, id FROM forum.threads WHERE id = $3) \
               AND ($4::bigint IS NULL OR NOT forum.user_content_hidden($4, t.author_id)) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(bid)
        .bind(limit + 1)
        .bind(cid)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else if let (Some(cid), None) = (cursor_id, board_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND (t.created_at, t.id) < (SELECT created_at, id FROM forum.threads WHERE id = $2) \
               AND ($3::bigint IS NULL OR NOT forum.user_content_hidden($3, t.author_id)) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $1",
        )
        .bind(limit + 1)
        .bind(cid)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else if let (None, Some(bid)) = (cursor_id, board_id) {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($3::bigint IS NULL OR NOT forum.user_content_hidden($3, t.author_id)) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $2",
        )
        .bind(bid)
        .bind(limit + 1)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ThreadRowJoined>(
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($2::bigint IS NULL OR NOT forum.user_content_hidden($2, t.author_id)) \
             ORDER BY t.created_at DESC, t.id DESC \
             LIMIT $1",
        )
        .bind(limit + 1)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|r| base64_encode_i64(r.id)) } else { None };

    Ok((items, next_cursor))
}

/// Fetch full thread rows in order by a given list of IDs (preserving order).
pub async fn fetch_threads_by_ids(
    pool: &PgPool,
    ids: &[i64],
    current_user_id: Option<i64>,
) -> AppResult<Vec<ThreadRowJoined>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let rows: Vec<ThreadRowJoined> = sqlx::query_as(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         WHERE t.id = ANY($1) AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
           AND t.archived_at IS NULL \
           AND ($2::bigint IS NULL OR NOT forum.user_content_hidden($2, t.author_id))",
    )
    .bind(ids)
    .bind(current_user_id)
    .fetch_all(pool)
    .await?;

    // Preserve ZSET order by sorting in-memory
    let mut ordered: Vec<ThreadRowJoined> = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(pos) = rows.iter().position(|r| r.id == *id) {
            ordered.push(rows[pos].clone());
        }
    }
    Ok(ordered)
}

async fn list_threads_feed_hot(
    pool: &PgPool,
    board_id: Option<i64>,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
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
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.board_id = $1 \
                   AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND (COALESCE(t.hot_score, 0) < $3 \
                        OR (COALESCE(t.hot_score, 0) = $3 AND t.id < $4)) \
                   AND ($5::bigint IS NULL OR NOT forum.user_content_hidden($5, t.author_id)) \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $2",
            )
            .bind(bid)
            .bind(limit + 1)
            .bind(ch)
            .bind(ci)
            .bind(current_user_id)
            .fetch_all(pool)
            .await?
        }
        (None, Some(ch), Some(ci)) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND (COALESCE(t.hot_score, 0) < $2 \
                        OR (COALESCE(t.hot_score, 0) = $2 AND t.id < $3)) \
                   AND ($4::bigint IS NULL OR NOT forum.user_content_hidden($4, t.author_id)) \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $1",
            )
            .bind(limit + 1)
            .bind(ch)
            .bind(ci)
            .bind(current_user_id)
            .fetch_all(pool)
            .await?
        }
        (Some(bid), _, _) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND ($3::bigint IS NULL OR NOT forum.user_content_hidden($3, t.author_id)) \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $2",
            )
            .bind(bid)
            .bind(limit + 1)
            .bind(current_user_id)
            .fetch_all(pool)
            .await?
        }
        (None, _, _) => {
            sqlx::query_as::<_, ThreadRowJoined>(
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND ($2::bigint IS NULL OR NOT forum.user_content_hidden($2, t.author_id)) \
                 ORDER BY COALESCE(t.hot_score, 0) DESC, t.id DESC \
                 LIMIT $1",
            )
            .bind(limit + 1)
            .bind(current_user_id)
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

fn encode_subscription_feed_cursor(row: &ThreadRowJoined) -> String {
    super::base64_encode_str(&format!("{}|{}", row.last_activity_at.timestamp_micros(), row.id))
}

fn decode_subscription_feed_cursor(
    cursor: &str,
) -> AppResult<(chrono::DateTime<chrono::Utc>, i64)> {
    let decoded = super::base64_decode_str(cursor)
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
    let (micros, id) = decoded
        .split_once('|')
        .ok_or_else(|| shared::AppError::BadRequest("invalid cursor".into()))?;
    let last_activity_at = micros
        .parse::<i64>()
        .ok()
        .and_then(chrono::DateTime::from_timestamp_micros)
        .ok_or_else(|| shared::AppError::BadRequest("invalid cursor".into()))?;
    let thread_id =
        id.parse::<i64>().map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
    Ok((last_activity_at, thread_id))
}

/// List threads covered by an effective watching/tracking subscription.
///
/// A thread-level subscription overrides its board-level fallback.
pub async fn list_threads_feed_subscriptions(
    pool: &PgPool,
    account_id: i64,
    board_id: Option<i64>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let (cursor_at, cursor_id) = match cursor {
        Some(cursor) => {
            let (last_activity_at, thread_id) = decode_subscription_feed_cursor(cursor)?;
            (Some(last_activity_at), Some(thread_id))
        }
        None => (None, None),
    };
    let page_size = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, ThreadRowJoined>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_version, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         WHERE COALESCE( \
           (SELECT direct.level FROM forum.subscriptions direct \
            WHERE direct.account_id = $1 AND direct.target_type = 'thread' \
              AND direct.target_id = t.id), \
           (SELECT board.level FROM forum.subscriptions board \
            WHERE board.account_id = $1 AND board.target_type = 'board' \
              AND board.target_id = t.board_id) \
         ) IN ('watching', 'tracking') \
           AND t.deleted_at IS NULL AND t.hidden_at IS NULL AND t.archived_at IS NULL \
           AND ($2::bigint IS NULL OR t.board_id = $2) \
           AND NOT forum.user_content_hidden($1, t.author_id) \
           AND ($3::timestamptz IS NULL OR (t.last_activity_at, t.id) < ($3, $4)) \
         ORDER BY t.last_activity_at DESC, t.id DESC \
         LIMIT $5",
    )
    .bind(account_id)
    .bind(board_id)
    .bind(cursor_at)
    .bind(cursor_id)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    let items = if has_more { rows[..page_size as usize].to_vec() } else { rows };
    let next_cursor = has_more.then(|| items.last().map(encode_subscription_feed_cursor)).flatten();

    Ok((items, next_cursor))
}

fn encode_following_feed_cursor(row: &ThreadRowJoined) -> String {
    encode_following_feed_cursor_parts(row.created_at, row.id)
}

fn encode_following_feed_cursor_parts(
    created_at: chrono::DateTime<chrono::Utc>,
    thread_id: i64,
) -> String {
    super::base64_encode_str(&format!("{}|{thread_id}", created_at.timestamp_micros()))
}

fn decode_following_feed_cursor(cursor: &str) -> AppResult<(chrono::DateTime<chrono::Utc>, i64)> {
    let decoded = super::base64_decode_str(cursor)
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
    let (micros, thread_id) = decoded
        .split_once('|')
        .ok_or_else(|| shared::AppError::BadRequest("invalid cursor".into()))?;
    let created_at = micros
        .parse::<i64>()
        .ok()
        .and_then(chrono::DateTime::from_timestamp_micros)
        .ok_or_else(|| shared::AppError::BadRequest("invalid cursor".into()))?;
    let thread_id = thread_id
        .parse::<i64>()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;
    Ok((created_at, thread_id))
}

#[derive(Debug, sqlx::FromRow)]
struct FollowingThreadCandidate {
    id: i64,
    board_id: i64,
    author_id: i64,
    title: String,
    body: Option<String>,
    content_version: i64,
    reply_count: i32,
    vote_count: i32,
    hot_score: Option<f64>,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity_at: chrono::DateTime<chrono::Utc>,
}

impl FollowingThreadCandidate {
    fn into_joined(self, author_handle: String) -> ThreadRowJoined {
        ThreadRowJoined {
            id: self.id,
            board_id: self.board_id,
            author_id: self.author_id,
            title: self.title,
            body: self.body,
            content_version: self.content_version,
            reply_count: self.reply_count,
            vote_count: self.vote_count,
            hot_score: self.hot_score,
            status: self.status,
            created_at: self.created_at,
            last_activity_at: self.last_activity_at,
            author_handle,
        }
    }
}

/// List public threads authored by accounts the viewer currently follows.
///
/// PostgreSQL follow facts are authoritative. The read applies account lifecycle,
/// active suspension, block, mute, and content visibility before pagination.
pub async fn list_threads_feed_following(
    pool: &PgPool,
    account_id: i64,
    board_id: Option<i64>,
    tag_slug: Option<&str>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<String>)> {
    let page_size = limit.clamp(1, 100);
    let mut scan_cursor = cursor.map(decode_following_feed_cursor).transpose()?;
    let mut rows = Vec::with_capacity((page_size + 1) as usize);
    let mut scanned = 0usize;
    const MAX_CANDIDATES_PER_PAGE: usize = 1_000;

    while rows.len() <= page_size as usize && scanned < MAX_CANDIDATES_PER_PAGE {
        let remaining = page_size as usize + 1 - rows.len();
        let candidate_limit = (remaining.saturating_mul(4)).clamp(20, 100);
        let candidate_limit = candidate_limit.min(MAX_CANDIDATES_PER_PAGE - scanned);
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT thread.id, thread.board_id, thread.author_id, thread.title, thread.body, \
                    thread.content_version, \
                    thread.reply_count, thread.vote_count, thread.hot_score, thread.status, \
                    thread.created_at, thread.last_activity_at \
             FROM forum.threads thread \
             WHERE EXISTS (SELECT 1 FROM forum.user_follows follow \
                           WHERE follow.follower_id = ",
        );
        query
            .push_bind(account_id)
            .push(
                " AND follow.followed_id = thread.author_id) \
                   AND thread.status = 'visible' AND thread.deleted_at IS NULL \
                   AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
                   AND NOT forum.user_content_hidden(",
            )
            .push_bind(account_id)
            .push(", thread.author_id)");
        if let Some(board_id) = board_id {
            query.push(" AND thread.board_id = ").push_bind(board_id);
        }
        if let Some(tag_slug) = tag_slug {
            query
                .push(
                    " AND EXISTS (SELECT 1 FROM forum.thread_tags thread_tag \
                                  JOIN forum.tags tag ON tag.id = thread_tag.tag_id \
                                  WHERE thread_tag.thread_id = thread.id AND tag.slug = ",
                )
                .push_bind(tag_slug)
                .push(")");
        }
        if let Some((created_at, thread_id)) = scan_cursor {
            query
                .push(" AND (thread.created_at, thread.id) < (")
                .push_bind(created_at)
                .push(", ")
                .push_bind(thread_id)
                .push(")");
        }
        query.push(" ORDER BY thread.created_at DESC, thread.id DESC LIMIT ");
        query.push_bind(candidate_limit as i64);
        let candidates = query.build_query_as::<FollowingThreadCandidate>().fetch_all(pool).await?;
        if candidates.is_empty() {
            break;
        }
        scanned += candidates.len();
        scan_cursor = candidates.last().map(|candidate| (candidate.created_at, candidate.id));
        let author_ids = candidates.iter().map(|candidate| candidate.author_id).collect::<Vec<_>>();
        let handles = identity::public_accounts::find_public_accounts_by_ids(pool, &author_ids)
            .await?
            .into_iter()
            .map(|account| (account.id, account.handle))
            .collect::<std::collections::HashMap<_, _>>();
        let candidate_count = candidates.len();
        rows.extend(candidates.into_iter().filter_map(|candidate| {
            handles.get(&candidate.author_id).cloned().map(|handle| candidate.into_joined(handle))
        }));
        if candidate_count < candidate_limit {
            break;
        }
    }

    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = if has_more {
        rows.last().map(encode_following_feed_cursor)
    } else if scanned >= MAX_CANDIDATES_PER_PAGE {
        scan_cursor.map(|(created_at, thread_id)| {
            encode_following_feed_cursor_parts(created_at, thread_id)
        })
    } else {
        None
    };
    Ok((rows, next_cursor))
}

/// Find a single thread by id, joined with author handle (full columns).
pub async fn find_thread(pool: &PgPool, id: i64) -> AppResult<Option<ThreadRowJoinedFull>> {
    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_format, t.content_version, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.pinned_at, t.pinned_globally, t.featured_at, t.closed_at, t.archived_at, \
                t.deleted_at, t.deleted_by, t.edited_at, t.hidden_at, \
                t.solved_answer_id, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         WHERE t.id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Find a thread for staff recovery, including hidden and soft-deleted rows.
pub async fn find_thread_for_moderation(
    pool: &PgPool,
    id: i64,
) -> AppResult<Option<ThreadRowJoinedFull>> {
    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_format, t.content_version, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.pinned_at, t.pinned_globally, t.featured_at, t.closed_at, t.archived_at, \
                t.deleted_at, t.deleted_by, t.edited_at, t.hidden_at, \
                t.solved_answer_id, t.created_at, t.last_activity_at, \
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

/// Insert a new thread. Returns the created thread joined with author handle (full columns).
pub async fn create_thread(
    pool: &PgPool,
    board_id: i64,
    author_id: i64,
    input: &crate::dto::ThreadInput,
    is_hidden: bool,
    posting_actor: super::boards::BoardPostingActor,
) -> AppResult<ThreadRowJoinedFull> {
    let mut tx = pool.begin().await?;
    let board_ids = super::boards::lock_board_for_posting(&mut tx, board_id, posting_actor).await?;
    let tag_ids = match input.tags.as_ref() {
        Some(tag_slugs) => super::tags::resolve_tag_slugs_tx(&mut tx, tag_slugs).await?,
        None => Vec::new(),
    };
    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "WITH inserted AS ( \
            INSERT INTO forum.threads (board_id, author_id, title, body, content_format, hidden_at) \
            VALUES ($1, $2, $3, $4, $5, CASE WHEN $6 THEN now() ELSE NULL END) \
            RETURNING id, board_id, author_id, title, body, content_format, content_version, reply_count, vote_count, \
                      hot_score, status, pinned_at, pinned_globally, featured_at, closed_at, archived_at, \
                      deleted_at, deleted_by, edited_at, hidden_at, solved_answer_id, created_at, last_activity_at \
         ) \
         SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_format, t.content_version, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.pinned_at, t.pinned_globally, t.featured_at, t.closed_at, t.archived_at, \
                t.deleted_at, t.deleted_by, t.edited_at, t.hidden_at, \
                t.solved_answer_id, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM inserted t \
         JOIN identity.accounts a ON a.id = t.author_id",
    )
    .bind(board_id)
    .bind(author_id)
    .bind(&input.title)
    .bind(&input.body)
    .bind(input.content_format.as_str())
    .bind(is_hidden)
    .fetch_one(&mut *tx)
    .await?;

    if input.tags.is_some() {
        super::tags::set_thread_tags_tx(&mut tx, row.id, &tag_ids).await?;
    }
    if let Some(poll) = input.poll.as_ref() {
        let closes_at =
            poll.closes_at.and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0));
        super::polls::create_poll_tx(
            &mut tx,
            row.id,
            &poll.question,
            poll.multi_select,
            closes_at,
            &poll.options,
        )
        .await?;
    }
    if !is_hidden {
        activity::contributions::activate_contribution(
            &mut tx,
            author_id,
            activity::contributions::ActivityKind::Thread,
            &format!("forum_thread:{}", row.id),
            row.created_at,
        )
        .await?;
        sqlx::query(
            "INSERT INTO forum.user_stats (account_id, threads_created, last_posted_at) \
             VALUES ($1, 1, now()) \
             ON CONFLICT (account_id) DO UPDATE \
             SET threads_created = forum.user_stats.threads_created + 1, \
                 last_posted_at = now(), updated_at = now()",
        )
        .bind(author_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO forum.subscriptions (account_id, target_type, target_id, level) \
             VALUES ($1, 'thread', $2, 'tracking') \
             ON CONFLICT (account_id, target_type, target_id) \
             DO UPDATE SET level = EXCLUDED.level",
        )
        .bind(author_id)
        .bind(row.id)
        .execute(&mut *tx)
        .await?;
    }
    super::boards::refresh_board_thread_counts(&mut tx, &board_ids).await?;
    tx.commit().await?;
    Ok(row)
}

/// Update a thread's title and/or body. Returns the updated row joined with author handle (full columns).
pub async fn update_thread(
    pool: &PgPool,
    id: i64,
    author_id: i64,
    input: &crate::dto::ThreadUpdateInput,
    is_queued: bool,
) -> AppResult<ThreadRowJoinedFull> {
    let mut tx = pool.begin().await?;
    let existing = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, t.content_format, t.content_version, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.pinned_at, t.pinned_globally, t.featured_at, t.closed_at, t.archived_at, \
                t.deleted_at, t.deleted_by, t.edited_at, t.hidden_at, t.solved_answer_id, \
                t.created_at, t.last_activity_at, a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         WHERE t.id = $1 FOR UPDATE OF t",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    if existing.author_id != author_id {
        return Err(shared::AppError::Forbidden);
    }
    if existing.deleted_at.is_some() || existing.hidden_at.is_some() {
        return Err(shared::AppError::NotFound);
    }
    if existing.archived_at.is_some() {
        return Err(shared::AppError::Conflict("thread is archived".into()));
    }
    if input.expected_version != existing.content_version {
        return Err(shared::AppError::OptimisticLockConflict {
            current_version: existing.content_version,
        });
    }
    let locked_board_ids =
        super::boards::lock_boards_for_thread_count(&mut tx, &[existing.board_id]).await?;

    let content_changed = input.title.as_ref().is_some_and(|title| title != &existing.title)
        || input.body.as_ref().is_some_and(|body| Some(body.as_str()) != existing.body.as_deref())
        || input.content_format.is_some_and(|format| format.as_str() != existing.content_format);
    let tag_ids = match input.tags.as_ref() {
        Some(tag_slugs) => Some(super::tags::resolve_tag_slugs_tx(&mut tx, tag_slugs).await?),
        None => None,
    };
    let within_grace = existing.created_at > chrono::Utc::now() - chrono::Duration::minutes(5);
    if content_changed && !within_grace {
        super::revisions::create_revision_tx(
            &mut tx,
            "thread",
            id,
            author_id,
            Some(&existing.title),
            existing.body.as_deref().unwrap_or(""),
            &existing.content_format,
        )
        .await?;
    }

    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "WITH updated AS ( \
         UPDATE forum.threads SET \
         title = COALESCE($1, title), \
         body = COALESCE($2, body), \
         content_format = COALESCE($3, content_format), \
         content_version = content_version + 1, \
         edited_at = CASE WHEN $4 THEN now() ELSE edited_at END, \
         hidden_at = CASE WHEN $5 THEN now() ELSE hidden_at END \
         WHERE id = $6 AND content_version = $7 \
         RETURNING id, board_id, author_id, title, body, content_format, content_version, reply_count, vote_count, \
                   hot_score, status, pinned_at, pinned_globally, featured_at, closed_at, archived_at, \
                   deleted_at, deleted_by, edited_at, hidden_at, solved_answer_id, created_at, last_activity_at \
         ) \
         SELECT u.id, u.board_id, u.author_id, u.title, u.body, u.content_format, u.content_version, \
                u.reply_count, u.vote_count, u.hot_score, u.status, \
                u.pinned_at, u.pinned_globally, u.featured_at, u.closed_at, u.archived_at, \
                u.deleted_at, u.deleted_by, u.edited_at, u.hidden_at, \
                u.solved_answer_id, \
                u.created_at, u.last_activity_at, \
                a.handle AS author_handle \
         FROM updated u \
         JOIN identity.accounts a ON a.id = u.author_id",
    )
    .bind(input.title.as_deref())
    .bind(input.body.as_deref())
    .bind(input.content_format.map(|format| format.as_str()))
    .bind(content_changed)
    .bind(is_queued)
    .bind(id)
    .bind(input.expected_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(shared::AppError::OptimisticLockConflict {
        current_version: existing.content_version,
    })?;

    if let Some(tag_ids) = tag_ids.as_ref() {
        super::tags::set_thread_tags_tx(&mut tx, id, tag_ids).await?;
    } else if is_queued {
        let existing_tag_ids: Vec<i64> =
            sqlx::query_scalar("SELECT tag_id FROM forum.thread_tags WHERE thread_id = $1")
                .bind(id)
                .fetch_all(&mut *tx)
                .await?;
        super::tags::set_thread_tags_tx(&mut tx, id, &existing_tag_ids).await?;
    }

    if is_queued {
        activity::contributions::deactivate_contribution(
            &mut tx,
            &format!("forum_thread:{id}"),
            chrono::Utc::now(),
        )
        .await?;
        super::votes::deactivate_target_vote_contributions(
            &mut tx,
            "thread",
            id,
            chrono::Utc::now(),
        )
        .await?;
        super::boards::refresh_board_thread_counts(&mut tx, &locked_board_ids).await?;
    }

    tx.commit().await?;
    Ok(row)
}

use shared::AppResult;
use sqlx::PgPool;

use crate::models::{ThreadRowJoined, ThreadRowJoinedFull};

use super::{base64_decode_i64, base64_encode_i64, decode_hot_cursor, encode_hot_cursor};

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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND t.created_at < (SELECT created_at FROM forum.threads WHERE id = $3) \
               AND ($4::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $4 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($3::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $3 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
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
               AND ($5::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $5 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($3::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $3 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND t.created_at < (SELECT created_at FROM forum.threads WHERE id = $3) \
               AND ($4::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $4 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND t.created_at < (SELECT created_at FROM forum.threads WHERE id = $2) \
               AND ($3::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $3 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($3::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $3 \
               )) \
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
            "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                    t.reply_count, t.vote_count, t.hot_score, t.status, \
                    t.created_at, t.last_activity_at, \
                    a.handle AS author_handle \
             FROM forum.threads t \
             JOIN identity.accounts a ON a.id = t.author_id \
             WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
               AND t.archived_at IS NULL \
               AND ($2::bigint IS NULL OR t.author_id <> ALL( \
                    SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $2 \
               )) \
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
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         WHERE t.id = ANY($1) AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
           AND t.archived_at IS NULL \
           AND ($2::bigint IS NULL OR t.author_id <> ALL( \
                SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $2 \
           ))",
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
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
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
                   AND ($5::bigint IS NULL OR t.author_id <> ALL( \
                        SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $5 \
                   )) \
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
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND (COALESCE(t.hot_score, 0) < $2 \
                        OR (COALESCE(t.hot_score, 0) = $2 AND t.id < $3)) \
                   AND ($4::bigint IS NULL OR t.author_id <> ALL( \
                        SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $4 \
                   )) \
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
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.board_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND ($3::bigint IS NULL OR t.author_id <> ALL( \
                        SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $3 \
                   )) \
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
                "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                        t.reply_count, t.vote_count, t.hot_score, t.status, \
                        t.created_at, t.last_activity_at, \
                        a.handle AS author_handle \
                 FROM forum.threads t \
                 JOIN identity.accounts a ON a.id = t.author_id \
                 WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
                   AND t.archived_at IS NULL \
                   AND ($2::bigint IS NULL OR t.author_id <> ALL( \
                        SELECT ignored_account_id FROM forum.user_ignores WHERE account_id = $2 \
                   )) \
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

/// List threads that the given account is following (watching/tracking).
///
/// Returns full `ThreadRowJoined` rows ordered by `last_activity_at DESC`.
pub async fn list_threads_feed_following(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<ThreadRowJoined>, Option<i64>)> {
    let since_id = cursor.unwrap_or(0);
    let rows = sqlx::query_as::<_, ThreadRowJoined>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
                t.reply_count, t.vote_count, t.hot_score, t.status, \
                t.created_at, t.last_activity_at, \
                a.handle AS author_handle \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         JOIN forum.subscriptions s ON s.target_type = 'thread' AND s.target_id = t.id \
         WHERE s.account_id = $1 AND s.level IN ('watching', 'tracking') \
           AND t.deleted_at IS NULL AND t.hidden_at IS NULL AND t.archived_at IS NULL \
           AND t.id > $2 \
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
    let next_cursor = items.last().map(|r| r.id);

    Ok((items, next_cursor))
}

/// Find a single thread by id, joined with author handle (full columns).
pub async fn find_thread(pool: &PgPool, id: i64) -> AppResult<Option<ThreadRowJoinedFull>> {
    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
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
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
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
) -> AppResult<ThreadRowJoinedFull> {
    let mut tx = pool.begin().await?;
    let board_ids = super::boards::lock_boards_for_thread_count(&mut tx, &[board_id]).await?;
    let tag_ids = match input.tags.as_ref() {
        Some(tag_slugs) => super::tags::resolve_tag_slugs_tx(&mut tx, tag_slugs).await?,
        None => Vec::new(),
    };
    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "WITH inserted AS ( \
            INSERT INTO forum.threads (board_id, author_id, title, body, hidden_at) \
            VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() ELSE NULL END) \
            RETURNING id, board_id, author_id, title, body, reply_count, vote_count, \
                      hot_score, status, pinned_at, pinned_globally, featured_at, closed_at, archived_at, \
                      deleted_at, deleted_by, edited_at, hidden_at, solved_answer_id, created_at, last_activity_at \
         ) \
         SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
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
        "SELECT t.id, t.board_id, t.author_id, t.title, t.body, \
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
    let locked_board_ids =
        super::boards::lock_boards_for_thread_count(&mut tx, &[existing.board_id]).await?;

    let content_changed = input.title.as_ref().is_some_and(|title| title != &existing.title)
        || input.body.as_ref().is_some_and(|body| Some(body.as_str()) != existing.body.as_deref());
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
        )
        .await?;
    }

    let row = sqlx::query_as::<_, ThreadRowJoinedFull>(
        "WITH updated AS ( \
         UPDATE forum.threads SET \
         title = COALESCE($1, title), \
         body = COALESCE($2, body), \
         edited_at = CASE WHEN $3 THEN now() ELSE edited_at END, \
         hidden_at = CASE WHEN $4 THEN now() ELSE hidden_at END \
         WHERE id = $5 \
         RETURNING id, board_id, author_id, title, body, reply_count, vote_count, \
                   hot_score, status, pinned_at, pinned_globally, featured_at, closed_at, archived_at, \
                   deleted_at, deleted_by, edited_at, hidden_at, solved_answer_id, created_at, last_activity_at \
         ) \
         SELECT u.id, u.board_id, u.author_id, u.title, u.body, \
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
    .bind(content_changed)
    .bind(is_queued)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

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

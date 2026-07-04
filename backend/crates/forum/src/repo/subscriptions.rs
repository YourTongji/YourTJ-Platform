//! Subscription CRUD — watching, tracking, muted per board or thread.

use crate::models::SubscriptionRow;
use shared::AppResult;
use sqlx::PgPool;

/// Set subscription level (UPSERT).
pub async fn set_subscription(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
    level: &str,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO forum.subscriptions (account_id, target_type, target_id, level) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (account_id, target_type, target_id) \
         DO UPDATE SET level = EXCLUDED.level, created_at = now()",
    )
    .bind(account_id)
    .bind(target_type)
    .bind(target_id)
    .bind(level)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a subscription (revert to default).
pub async fn delete_subscription(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "DELETE FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = $2 AND target_id = $3",
    )
    .bind(account_id)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// List subscriptions for an account, optionally filtered by type.
pub async fn list_subscriptions(
    pool: &PgPool,
    account_id: i64,
    target_type: Option<&str>,
) -> AppResult<Vec<SubscriptionRow>> {
    if let Some(tt) = target_type {
        let rows = sqlx::query_as::<_, SubscriptionRow>(
            "SELECT account_id, target_type, target_id, level, created_at \
             FROM forum.subscriptions \
             WHERE account_id = $1 AND target_type = $2 \
             ORDER BY created_at DESC",
        )
        .bind(account_id)
        .bind(tt)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    } else {
        let rows = sqlx::query_as::<_, SubscriptionRow>(
            "SELECT account_id, target_type, target_id, level, created_at \
             FROM forum.subscriptions \
             WHERE account_id = $1 \
             ORDER BY created_at DESC",
        )
        .bind(account_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}

/// Get thread-level subscription for an account.
pub async fn get_thread_subscription(
    pool: &PgPool,
    account_id: i64,
    thread_id: i64,
) -> AppResult<Option<String>> {
    let level: Option<String> = sqlx::query_scalar(
        "SELECT level FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = 'thread' AND target_id = $2",
    )
    .bind(account_id)
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;
    Ok(level)
}

/// Get muted thread and board IDs for an account.
pub async fn get_muted_ids(pool: &PgPool, account_id: i64) -> AppResult<(Vec<i64>, Vec<i64>)> {
    let muted_threads: Vec<i64> = sqlx::query_scalar(
        "SELECT target_id FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = 'thread' AND level = 'muted'",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    let muted_boards: Vec<i64> = sqlx::query_scalar(
        "SELECT target_id FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = 'board' AND level = 'muted'",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    Ok((muted_threads, muted_boards))
}

/// Get subscriber IDs who have a `watching` subscription on a thread (or its parent board).
///
/// Excludes any IDs in `exclude_ids` (e.g. the commenter and thread author).
/// Capped at 200 to prevent fan-out storms (matching Discourse behavior).
pub async fn get_watching_subscriber_ids(
    pool: &PgPool,
    thread_id: i64,
    exclude_ids: &[i64],
) -> AppResult<Vec<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT s.account_id FROM forum.subscriptions s \
         WHERE s.level = 'watching' \
           AND ( \
             (s.target_type = 'thread' AND s.target_id = $1) \
             OR (s.target_type = 'board' AND s.target_id = ( \
               SELECT board_id FROM forum.threads WHERE id = $1 \
             )) \
           ) \
           AND s.account_id != ALL($2) \
         LIMIT 200",
    )
    .bind(thread_id)
    .bind(exclude_ids)
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

/// Get watching + tracking thread IDs for an account (for following feed).
pub async fn get_following_thread_ids(
    pool: &PgPool,
    account_id: i64,
    limit: i64,
    cursor: Option<i64>,
) -> AppResult<(Vec<(i64, String)>, Option<i64>)> {
    let since_id = cursor.unwrap_or(0);

    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT t.id, t.title \
         FROM forum.threads t \
         JOIN forum.subscriptions s ON s.target_type = 'thread' AND s.target_id = t.id \
         WHERE s.account_id = $1 AND s.level IN ('watching', 'tracking') \
           AND t.deleted_at IS NULL \
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
    let next_cursor = items.last().map(|r| r.0);

    Ok((items, next_cursor))
}

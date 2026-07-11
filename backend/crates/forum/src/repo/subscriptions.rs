//! Subscription CRUD — watching, tracking, muted per board or thread.

use crate::models::SubscriptionRow;
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

/// Serialize effective thread/board subscription reads and writes for one account.
pub async fn lock_account_subscriptions(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("forum-subscriptions:{account_id}"))
        .execute(connection)
        .await?;
    Ok(())
}

fn encode_subscription_cursor(row: &SubscriptionRow) -> String {
    super::base64_encode_str(&format!(
        "{}|{}|{}",
        row.created_at.timestamp_micros(),
        row.target_type,
        row.target_id
    ))
}

fn decode_subscription_cursor(
    cursor: &str,
) -> AppResult<(chrono::DateTime<chrono::Utc>, String, i64)> {
    let decoded = super::base64_decode_str(cursor)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let mut parts = decoded.split('|');
    let micros = parts
        .next()
        .and_then(|part| part.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    let target_type = parts
        .next()
        .filter(|target_type| matches!(*target_type, "board" | "thread"))
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

/// Set subscription level (UPSERT).
pub async fn set_subscription(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
    level: &str,
) -> AppResult<()> {
    if !matches!(target_type, "board" | "thread") {
        return Err(AppError::BadRequest("targetType must be board/thread".into()));
    }
    if !matches!(level, "watching" | "tracking" | "muted") {
        return Err(AppError::BadRequest("level must be watching/tracking/muted".into()));
    }
    let target_exists: bool = if target_type == "board" {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.boards WHERE id = $1)")
            .bind(target_id)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM forum.threads \
             WHERE id = $1 AND status = 'visible' AND deleted_at IS NULL \
               AND hidden_at IS NULL AND archived_at IS NULL)",
        )
        .bind(target_id)
        .fetch_one(pool)
        .await?
    };
    if !target_exists {
        return Err(AppError::NotFound);
    }
    let mut transaction = pool.begin().await?;
    lock_account_subscriptions(&mut transaction, account_id).await?;
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
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// Remove a subscription (revert to default).
pub async fn delete_subscription(
    pool: &PgPool,
    account_id: i64,
    target_type: &str,
    target_id: i64,
) -> AppResult<()> {
    if !matches!(target_type, "board" | "thread") {
        return Err(AppError::BadRequest("targetType must be board/thread".into()));
    }
    let mut transaction = pool.begin().await?;
    lock_account_subscriptions(&mut transaction, account_id).await?;
    sqlx::query(
        "DELETE FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = $2 AND target_id = $3",
    )
    .bind(account_id)
    .bind(target_type)
    .bind(target_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// List current subscriptions with a stable, bounded cursor.
pub async fn list_subscriptions_page(
    pool: &PgPool,
    account_id: i64,
    target_type: Option<&str>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<SubscriptionRow>, Option<String>)> {
    if target_type.is_some_and(|target_type| !matches!(target_type, "board" | "thread")) {
        return Err(AppError::BadRequest("type must be board/thread".into()));
    }
    let (cursor_at, cursor_type, cursor_id) = match cursor {
        Some(cursor) => {
            let (created_at, target_type, target_id) = decode_subscription_cursor(cursor)?;
            (Some(created_at), Some(target_type), Some(target_id))
        }
        None => (None, None, None),
    };
    let page_size = limit.clamp(1, 100);
    let mut rows = sqlx::query_as::<_, SubscriptionRow>(
        "SELECT subscription.account_id, subscription.target_type, \
                subscription.target_id, subscription.level, subscription.created_at \
         FROM forum.subscriptions subscription \
         WHERE subscription.account_id = $1 \
           AND ($2::text IS NULL OR subscription.target_type = $2) \
           AND ($3::timestamptz IS NULL OR \
                (subscription.created_at, subscription.target_type, subscription.target_id) \
                  < ($3, $4, $5)) \
           AND ( \
             (subscription.target_type = 'board' AND EXISTS ( \
               SELECT 1 FROM forum.boards board WHERE board.id = subscription.target_id \
             )) OR \
             (subscription.target_type = 'thread' AND EXISTS ( \
               SELECT 1 FROM forum.threads thread WHERE thread.id = subscription.target_id \
                 AND thread.status = 'visible' AND thread.deleted_at IS NULL \
                 AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
             )) \
           ) \
         ORDER BY subscription.created_at DESC, subscription.target_type DESC, \
                  subscription.target_id DESC \
         LIMIT $6",
    )
    .bind(account_id)
    .bind(target_type)
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
    let next_cursor = has_more.then(|| rows.last().map(encode_subscription_cursor)).flatten();
    Ok((rows, next_cursor))
}

/// Get the effective thread subscription, preferring a direct override over its board fallback.
pub async fn get_thread_subscription(
    pool: &PgPool,
    account_id: i64,
    thread_id: i64,
) -> AppResult<Option<String>> {
    let level: Option<Option<String>> = sqlx::query_scalar(
        "SELECT COALESCE( \
           (SELECT direct.level FROM forum.subscriptions direct \
            WHERE direct.account_id = $1 AND direct.target_type = 'thread' \
              AND direct.target_id = thread.id), \
           (SELECT board.level FROM forum.subscriptions board \
            WHERE board.account_id = $1 AND board.target_type = 'board' \
              AND board.target_id = thread.board_id) \
         ) \
         FROM forum.threads thread WHERE thread.id = $2",
    )
    .bind(account_id)
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;
    Ok(level.flatten())
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
    let mut connection = pool.acquire().await?;
    get_watching_subscriber_ids_tx(&mut connection, thread_id, exclude_ids).await
}

/// Resolve watching recipients inside the business transaction that appends their outbox events.
pub async fn get_watching_subscriber_ids_tx(
    connection: &mut PgConnection,
    thread_id: i64,
    exclude_ids: &[i64],
) -> AppResult<Vec<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT candidate.account_id FROM forum.subscriptions candidate \
         WHERE ((candidate.target_type = 'thread' AND candidate.target_id = $1) \
            OR (candidate.target_type = 'board' AND candidate.target_id = ( \
              SELECT board_id FROM forum.threads WHERE id = $1 \
            ))) \
           AND candidate.account_id != ALL($2) \
           AND COALESCE( \
             (SELECT direct.level FROM forum.subscriptions direct \
              WHERE direct.account_id = candidate.account_id \
                AND direct.target_type = 'thread' AND direct.target_id = $1), \
             (SELECT board.level FROM forum.subscriptions board \
              WHERE board.account_id = candidate.account_id \
                AND board.target_type = 'board' AND board.target_id = ( \
                  SELECT board_id FROM forum.threads WHERE id = $1 \
                )) \
           ) = 'watching' \
         LIMIT 200",
    )
    .bind(thread_id)
    .bind(exclude_ids)
    .fetch_all(connection)
    .await?;
    Ok(ids)
}

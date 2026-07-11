use chrono::{DateTime, Utc};
use serde_json::Value;
use shared::AppResult;
use sqlx::FromRow;
use sqlx::PgPool;

#[derive(Debug, Clone, FromRow)]
pub struct NotificationRow {
    pub id: i64,
    pub r#type: String,
    pub payload: Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Lists an account's notifications newest-id first with an exclusive id cursor.
pub async fn list_notifications(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    unread_only: bool,
    limit: i64,
) -> AppResult<(Vec<NotificationRow>, Option<i64>)> {
    let mut rows = sqlx::query_as::<_, NotificationRow>(
        "SELECT id, type, payload, read_at, created_at \
         FROM forum.notifications \
         WHERE account_id = $1 \
           AND ($2::bigint IS NULL OR id < $2) \
           AND (NOT $3 OR read_at IS NULL) \
         ORDER BY id DESC \
         LIMIT $4",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(unread_only)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = if has_more { rows.last().map(|row| row.id) } else { None };

    Ok((rows, next_cursor))
}

/// Mark notifications as read. Only touches notifications belonging to the
/// given account, silently skipping any `ids` that belong to another account.
pub async fn mark_read(pool: &PgPool, account_id: i64, notification_ids: &[i64]) -> AppResult<()> {
    if notification_ids.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "UPDATE forum.notifications SET read_at = now() \
         WHERE account_id = $1 AND id = ANY($2) AND read_at IS NULL",
    )
    .bind(account_id)
    .bind(notification_ids)
    .execute(pool)
    .await?;

    Ok(())
}

/// Marks every unread notification belonging to an account as read.
pub async fn mark_all_read(pool: &PgPool, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE forum.notifications SET read_at = now() \
         WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(account_id)
    .execute(pool)
    .await?;

    Ok(())
}

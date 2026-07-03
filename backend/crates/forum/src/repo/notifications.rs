use chrono::{DateTime, Utc};
use serde_json::Value;
use shared::AppResult;
use sqlx::FromRow;
use sqlx::PgPool;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct NotificationRow {
    pub id: i64,
    pub account_id: i64,
    pub r#type: String,
    pub payload: Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// List notifications for an account, cursor-paginated by created_at DESC.
pub async fn list_notifications(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<NotificationRow>, Option<i64>)> {
    let rows = if let Some(cursor_id) = cursor {
        sqlx::query_as::<_, NotificationRow>(
            "SELECT id, account_id, type, payload, read_at, created_at \
             FROM forum.notifications \
             WHERE account_id = $1 AND id < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(account_id)
        .bind(cursor_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, NotificationRow>(
            "SELECT id, account_id, type, payload, read_at, created_at \
             FROM forum.notifications \
             WHERE account_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(account_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let next_cursor = if has_more { rows.get(limit as usize).map(|r| r.id) } else { None };

    let truncated: Vec<NotificationRow> =
        if has_more { rows.into_iter().take(limit as usize).collect() } else { rows };

    Ok((truncated, next_cursor))
}

/// Mark notifications as read. Only touches notifications belonging to the
/// given account, silently skipping any `ids` that belong to another account.
pub async fn mark_read(pool: &PgPool, account_id: i64, notification_ids: &[i64]) -> AppResult<()> {
    if notification_ids.is_empty() {
        return Ok(());
    }

    // sqlx does not support array binding natively, so we build IN ($1, $2, ...).
    let placeholders: Vec<String> =
        notification_ids.iter().enumerate().map(|(i, _)| format!("${}", i + 2)).collect();

    let sql = format!(
        "UPDATE forum.notifications SET read_at = now() \
         WHERE account_id = $1 AND id IN ({}) AND read_at IS NULL",
        placeholders.join(", ")
    );

    let mut q = sqlx::query(&sql).bind(account_id);
    for id in notification_ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;

    Ok(())
}

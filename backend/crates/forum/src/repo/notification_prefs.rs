//! Notification preferences CRUD.

use crate::models::NotificationPrefsRow;
use shared::AppResult;
use sqlx::PgPool;

pub async fn get_notification_prefs(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<NotificationPrefsRow> {
    let row = sqlx::query_as::<_, NotificationPrefsRow>(
        "SELECT account_id, prefs, updated_at FROM forum.notification_prefs WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(NotificationPrefsRow {
        account_id,
        prefs: serde_json::json!({}),
        updated_at: chrono::Utc::now(),
    });
    Ok(row)
}

pub async fn set_notification_prefs(
    pool: &PgPool,
    account_id: i64,
    prefs: &serde_json::Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO forum.notification_prefs (account_id, prefs, updated_at) \
         VALUES ($1, $2, now()) \
         ON CONFLICT (account_id) \
         DO UPDATE SET prefs = EXCLUDED.prefs, updated_at = now()",
    )
    .bind(account_id)
    .bind(prefs)
    .execute(pool)
    .await?;
    Ok(())
}

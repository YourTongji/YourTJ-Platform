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
    input: crate::dto::NotificationPreferencesInput,
) -> AppResult<crate::dto::NotificationPreferences> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("notification-prefs:{account_id}"))
        .execute(&mut *transaction)
        .await?;
    let current: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT prefs FROM forum.notification_prefs WHERE account_id = $1 FOR SHARE",
    )
    .bind(account_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let current: crate::dto::NotificationPreferences =
        current.and_then(|value| serde_json::from_value(value).ok()).unwrap_or_default();
    let prefs = crate::dto::NotificationPreferences {
        in_app: crate::dto::InAppNotificationPreferences {
            replies: input.in_app.replies,
            mentions: input.in_app.mentions,
            quotes: input.in_app.quotes,
            votes: input.in_app.votes,
            badges: input.in_app.badges,
            subscriptions: input.in_app.subscriptions,
            follows: input.in_app.follows.unwrap_or(current.in_app.follows),
            direct_messages: input.in_app.direct_messages,
        },
        email: input.email,
    };
    let stored = serde_json::to_value(&prefs)
        .map_err(|error| shared::AppError::Internal(anyhow::Error::new(error)))?;
    sqlx::query(
        "INSERT INTO forum.notification_prefs (account_id, prefs, updated_at) \
         VALUES ($1, $2, now()) \
         ON CONFLICT (account_id) \
         DO UPDATE SET prefs = EXCLUDED.prefs, updated_at = now()",
    )
    .bind(account_id)
    .bind(stored)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(prefs)
}

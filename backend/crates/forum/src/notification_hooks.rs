//! Fire-and-forget notification creation helpers.
//!
//! These are called from domain handlers (reviews, forum, etc.) as a
//! `tokio::spawn` so they never block the HTTP response.

use serde_json::Value;
use sqlx::PgPool;

/// Insert a notification row. Call via `tokio::spawn` so the caller does not
/// wait for the INSERT.
pub async fn create_notification(pool: &PgPool, account_id: i64, r#type: &str, payload: Value) {
    let result = sqlx::query(
        "INSERT INTO forum.notifications (account_id, type, payload) VALUES ($1, $2, $3)",
    )
    .bind(account_id)
    .bind(r#type)
    .bind(&payload)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(error = %e, account_id, notification_type = %r#type, "failed to create notification");
    }
}

/// Check whether a notification type is enabled for an account.
///
/// Looks at `forum.notification_prefs.prefs` — a JSON object keyed by
/// notification type. Returns `true` if the type is absent (default-enabled).
pub async fn is_notification_enabled(pool: &PgPool, account_id: i64, r#type: &str) -> bool {
    let prefs: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT prefs FROM forum.notification_prefs WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    match prefs {
        Some(ref v) => v.get(r#type).and_then(|x| x.as_bool()).unwrap_or(true),
        None => true,
    }
}

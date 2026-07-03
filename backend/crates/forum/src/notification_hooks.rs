//! Fire-and-forget notification creation helpers.
//!
//! These are called from domain handlers (reviews, forum, etc.) as a
//! `tokio::spawn` so they never block the HTTP response.

use serde_json::Value;
use sqlx::PgPool;

/// Insert a notification row. Call via `tokio::spawn` so the caller does not
/// wait for the INSERT.
pub async fn create_notification(
    pool: &PgPool,
    account_id: i64,
    r#type: &str,
    payload: Value,
    aggregation_key: Option<&str>,
) {
    let result = sqlx::query(
        "INSERT INTO forum.notifications (account_id, type, payload, aggregation_key) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(account_id)
    .bind(r#type)
    .bind(&payload)
    .bind(aggregation_key)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(error = %e, account_id, notification_type = %r#type, "failed to create notification");
    }
}

/// Create a notification with aggregation support.
///
/// If an unread notification with the same `(account_id, type, aggregation_key)`
/// exists within the last 10 minutes, its payload count is incremented instead
/// of inserting a new row. Falls back to `create_notification` when no match is
/// found.
pub async fn create_notification_aggregated(
    pool: &PgPool,
    account_id: i64,
    r#type: &str,
    aggregation_key: &str,
    payload: Value,
) {
    // Try to find an existing unread notification to aggregate into.
    let existing: Option<(i64, Value)> = sqlx::query_as(
        "SELECT id, payload FROM forum.notifications \
         WHERE account_id = $1 AND type = $2 AND aggregation_key = $3 \
           AND read_at IS NULL \
           AND created_at > now() - interval '10 minutes' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(account_id)
    .bind(r#type)
    .bind(aggregation_key)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    if let Some((existing_id, existing_payload)) = existing {
        // Bump the count in the existing payload.
        let count = existing_payload.get("count").and_then(|c| c.as_i64()).unwrap_or(1) + 1;

        if let Some(obj) = existing_payload.as_object() {
            let mut new_payload = serde_json::Map::new();
            for (k, v) in obj {
                new_payload.insert(k.clone(), v.clone());
            }
            let mut payload_obj = Value::Object(new_payload);
            if let Some(obj) = payload_obj.as_object_mut() {
                obj.insert("count".into(), Value::from(count));
            }
            let _ = sqlx::query("UPDATE forum.notifications SET payload = $1 WHERE id = $2")
                .bind(&payload_obj)
                .bind(existing_id)
                .execute(pool)
                .await;
        }
    } else {
        // No existing aggregation — insert fresh with the aggregation key.
        create_notification(pool, account_id, r#type, payload, Some(aggregation_key)).await;
    }
}

/// Check whether a notification type is enabled for an account.
///
/// Looks at `forum.notification_prefs.prefs` — a JSON object keyed by
/// notification type. Returns `true` if the type is absent (default-enabled).
pub async fn is_notification_enabled(pool: &PgPool, account_id: i64, r#type: &str) -> bool {
    let prefs: Option<Value> =
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

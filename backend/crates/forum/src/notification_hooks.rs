//! Fire-and-forget notification creation helpers.
//!
//! These are called from domain handlers (reviews, forum, etc.) as a
//! `tokio::spawn` so they never block the HTTP response.

use serde_json::Value;
use sqlx::PgPool;

/// Insert a notification row. Call via `tokio::spawn` so the caller does not
/// wait for the INSERT.
///
/// When `actor_id` is `Some`, the notification is silently skipped if the
/// recipient (`account_id`) has ignored the actor (user blocking).
pub async fn create_notification(
    pool: &PgPool,
    account_id: i64,
    r#type: &str,
    payload: Value,
    aggregation_key: Option<&str>,
    actor_id: Option<i64>,
) {
    if !is_notification_enabled(pool, account_id, r#type).await {
        return;
    }

    // Check ignore: if the recipient has ignored the actor, drop the notification.
    if let Some(aid) = actor_id {
        let ignored: bool = sqlx::query_scalar("SELECT forum.user_content_hidden($1, $2)")
            .bind(account_id)
            .bind(aid)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

        if ignored {
            return;
        }
    }

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
        return;
    }

    // Publish SSE event for real-time delivery.
    crate::sse::publish_event(account_id, r#type, payload);
}

/// Create a notification with aggregation support.
///
/// If an unread notification with the same `(account_id, type, aggregation_key)`
/// exists within the last 10 minutes, its payload count is incremented instead
/// of inserting a new row. Falls back to `create_notification` when no match is
/// found.
///
/// When `actor_id` is `Some`, the notification is silently skipped if the
/// recipient (`account_id`) has ignored the actor (user blocking).
pub async fn create_notification_aggregated(
    pool: &PgPool,
    account_id: i64,
    r#type: &str,
    aggregation_key: &str,
    payload: Value,
    actor_id: Option<i64>,
) {
    if !is_notification_enabled(pool, account_id, r#type).await {
        return;
    }

    // Check ignore: if the recipient has ignored the actor, drop the notification.
    if let Some(aid) = actor_id {
        let ignored: bool = sqlx::query_scalar("SELECT forum.user_content_hidden($1, $2)")
            .bind(account_id)
            .bind(aid)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

        if ignored {
            return;
        }
    }

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

        // Publish SSE event for the aggregated (bumped) notification.
        crate::sse::publish_event(account_id, r#type, payload);
    } else {
        // No existing aggregation — insert fresh with the aggregation key.
        // Re-check ignore inside create_notification via actor_id.
        create_notification(pool, account_id, r#type, payload, Some(aggregation_key), actor_id)
            .await;
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

    let Some(prefs) = prefs else {
        return true;
    };
    let Some(category) = in_app_category(r#type) else {
        return true;
    };
    prefs
        .get("inApp")
        .and_then(|value| value.get(category))
        .and_then(Value::as_bool)
        .or_else(|| prefs.get(r#type).and_then(Value::as_bool))
        .unwrap_or(true)
}

fn in_app_category(event_type: &str) -> Option<&'static str> {
    match event_type {
        "reply" => Some("replies"),
        "mention" => Some("mentions"),
        "quote" => Some("quotes"),
        "vote" => Some("votes"),
        "badge" => Some("badges"),
        "watching" => Some("subscriptions"),
        "dm" | "dm_request" | "dm_request_accepted" => Some("directMessages"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::in_app_category;

    #[test]
    fn maps_only_optional_interaction_events_to_user_preferences() {
        assert_eq!(in_app_category("reply"), Some("replies"));
        assert_eq!(in_app_category("dm"), Some("directMessages"));
        assert_eq!(in_app_category("dm_request"), Some("directMessages"));
        assert_eq!(in_app_category("content_moderated"), None);
        assert_eq!(in_app_category("security"), None);
    }
}

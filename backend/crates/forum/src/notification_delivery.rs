//! Idempotent delivery of platform outbox events into the Forum notification store.

use chrono::{DateTime, Utc};
use serde_json::Value;
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use platform::outbox::OutboxEvent;

/// Post-commit refresh hint for one newly delivered durable notification.
#[derive(Debug, Clone)]
pub struct DeliveryHint {
    pub account_id: i64,
    pub event_type: String,
}

#[derive(sqlx::FromRow)]
struct DmSourceState {
    request_status: String,
    request_sender_id: Option<i64>,
    request_recipient_id: Option<i64>,
    requested_at: Option<DateTime<Utc>>,
}

fn in_app_category(event_type: &str) -> Option<&'static str> {
    match event_type {
        "reply" => Some("replies"),
        "mention" => Some("mentions"),
        "quote" => Some("quotes"),
        "vote" => Some("votes"),
        "badge"
        | "achievement_awarded"
        | "achievement_revoked"
        | "verification_granted"
        | "verification_revoked"
        | "verification_expired" => Some("badges"),
        "watching" => Some("subscriptions"),
        "follow" => Some("follows"),
        "dm" | "dm_request" | "dm_request_accepted" => Some("directMessages"),
        _ => None,
    }
}

fn preference_is_enabled(prefs: Option<&Value>, event_type: &str) -> bool {
    let Some(category) = in_app_category(event_type) else {
        return true;
    };
    prefs
        .and_then(|value| value.get("inApp"))
        .and_then(|value| value.get(category))
        .and_then(Value::as_bool)
        .or_else(|| prefs.and_then(|value| value.get(event_type)).and_then(Value::as_bool))
        .unwrap_or(true)
}

fn numeric_payload_id(payload: &Value, field: &str) -> Option<i64> {
    payload.get(field)?.as_str()?.parse().ok().filter(|value| *value > 0)
}

fn event_requires_actor(event_type: &str) -> bool {
    matches!(
        event_type,
        "reply"
            | "mention"
            | "quote"
            | "vote"
            | "watching"
            | "follow"
            | "dm"
            | "dm_request"
            | "dm_request_accepted"
    )
}

async fn delivery_outcome(
    connection: &mut PgConnection,
    event: &OutboxEvent,
) -> AppResult<(&'static str, Option<String>)> {
    let relationship_hidden = if let Some(actor_id) = event.actor_account_id {
        crate::repo::relationships::lock_notification_pair(
            connection,
            event.recipient_account_id,
            actor_id,
        )
        .await?
    } else {
        false
    };
    let (recipient, actor_available) = match event.actor_account_id {
        Some(actor_id) if actor_id != event.recipient_account_id => {
            if event.recipient_account_id < actor_id {
                let recipient = identity::public_accounts::lock_notification_recipient(
                    connection,
                    event.recipient_account_id,
                )
                .await?;
                let actor =
                    identity::public_accounts::lock_notification_recipient(connection, actor_id)
                        .await?;
                (recipient, actor.is_some())
            } else {
                let actor =
                    identity::public_accounts::lock_notification_recipient(connection, actor_id)
                        .await?;
                let recipient = identity::public_accounts::lock_notification_recipient(
                    connection,
                    event.recipient_account_id,
                )
                .await?;
                (recipient, actor.is_some())
            }
        }
        Some(_) => {
            let recipient = identity::public_accounts::lock_notification_recipient(
                connection,
                event.recipient_account_id,
            )
            .await?;
            let actor_available = recipient.is_some();
            (recipient, actor_available)
        }
        None => {
            let recipient = identity::public_accounts::lock_notification_recipient(
                connection,
                event.recipient_account_id,
            )
            .await?;
            (recipient, !event_requires_actor(&event.event_type))
        }
    };
    let Some(recipient) = recipient else {
        return Ok(("recipient_unavailable", None));
    };

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("notification-prefs:{}", event.recipient_account_id))
        .execute(&mut *connection)
        .await?;
    let prefs: Option<Value> = sqlx::query_scalar(
        "SELECT prefs FROM forum.notification_prefs WHERE account_id = $1 FOR SHARE",
    )
    .bind(event.recipient_account_id)
    .fetch_optional(&mut *connection)
    .await?;
    if !preference_is_enabled(prefs.as_ref(), &event.event_type) {
        return Ok(("preference_disabled", None));
    }

    if let Some(actor_id) = event.actor_account_id {
        if !actor_available {
            return Ok(("actor_unavailable", None));
        }
        if relationship_hidden {
            return Ok(("relationship_hidden", None));
        }
        if event.event_type == "mention" {
            let allowed = match recipient.mention_policy.as_str() {
                "everyone" => true,
                "following" => {
                    sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM forum.user_follows \
                         WHERE follower_id = $1 AND followed_id = $2)",
                    )
                    .bind(event.recipient_account_id)
                    .bind(actor_id)
                    .fetch_one(&mut *connection)
                    .await?
                }
                _ => false,
            };
            if !allowed {
                return Ok(("mention_disallowed", None));
            }
        }
    } else if !actor_available {
        return Ok(("actor_unavailable", None));
    }

    if matches!(event.event_type.as_str(), "dm" | "dm_request" | "dm_request_accepted") {
        if let Some(conversation_id) = numeric_payload_id(&event.payload, "conversationId") {
            let muted = sqlx::query_scalar::<_, bool>(
                "SELECT muted_at IS NOT NULL FROM forum.dm_participants \
                 WHERE conversation_id = $1 AND account_id = $2 FOR SHARE",
            )
            .bind(conversation_id)
            .bind(event.recipient_account_id)
            .fetch_optional(&mut *connection)
            .await?
            .unwrap_or(false);
            if muted {
                return Ok(("conversation_muted", None));
            }
        }
    }

    Ok(("delivered", Some(recipient.handle)))
}

async fn lock_content_if_available(
    connection: &mut PgConnection,
    event: &OutboxEvent,
) -> AppResult<bool> {
    let Some(thread_id) = numeric_payload_id(&event.payload, "threadId") else {
        return Ok(true);
    };
    let thread_available: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM forum.threads \
         WHERE id = $1 AND status = 'visible' AND deleted_at IS NULL AND hidden_at IS NULL \
         FOR SHARE",
    )
    .bind(thread_id)
    .fetch_optional(&mut *connection)
    .await?;
    if thread_available.is_none() {
        return Ok(false);
    }
    let comment_id = numeric_payload_id(&event.payload, "commentId").or_else(|| {
        (event.event_type == "vote"
            && event.payload.get("postType").and_then(Value::as_str) == Some("comment"))
        .then(|| numeric_payload_id(&event.payload, "postId"))
        .flatten()
    });
    let Some(comment_id) = comment_id else {
        return Ok(true);
    };
    let comment_available: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM forum.comments \
         WHERE id = $1 AND thread_id = $2 AND deleted_at IS NULL AND hidden_at IS NULL \
         FOR SHARE",
    )
    .bind(comment_id)
    .bind(thread_id)
    .fetch_optional(connection)
    .await?;
    Ok(comment_available.is_some())
}

async fn lock_source_lifecycle_if_available(
    connection: &mut PgConnection,
    event: &OutboxEvent,
) -> AppResult<bool> {
    match event.event_type.as_str() {
        "verification_expired" => {
            let Some(grant_id) = numeric_payload_id(&event.payload, "verificationGrantId") else {
                return Ok(false);
            };
            platform::verifications::lock_current_expiry_notification(connection, grant_id).await
        }
        "follow" => lock_follow_source(connection, event).await,
        "vote" => lock_vote_source(connection, event).await,
        "watching" => lock_watching_source(connection, event).await,
        "dm_request" | "dm_request_accepted" | "dm" => lock_dm_source(connection, event).await,
        _ => Ok(true),
    }
}

async fn lock_follow_source(connection: &mut PgConnection, event: &OutboxEvent) -> AppResult<bool> {
    let (Some(follower_id), Some(followed_at_micros)) =
        (event.actor_account_id, numeric_payload_id(&event.payload, "followedAtMicros"))
    else {
        return Ok(false);
    };
    let followed_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT created_at FROM forum.user_follows \
         WHERE follower_id = $1 AND followed_id = $2 FOR SHARE",
    )
    .bind(follower_id)
    .bind(event.recipient_account_id)
    .fetch_optional(connection)
    .await?;
    Ok(followed_at.is_some_and(|value| value.timestamp_micros() == followed_at_micros))
}

async fn lock_vote_source(connection: &mut PgConnection, event: &OutboxEvent) -> AppResult<bool> {
    let Some(voter_id) = event.actor_account_id else {
        return Ok(false);
    };
    if numeric_payload_id(&event.payload, "voterId") != Some(voter_id) {
        return Ok(false);
    }
    let Some(post_type) = event.payload.get("postType").and_then(Value::as_str) else {
        return Ok(false);
    };
    if !matches!(post_type, "thread" | "comment") {
        return Ok(false);
    }
    let (Some(post_id), Some(updated_at_micros)) = (
        numeric_payload_id(&event.payload, "postId"),
        numeric_payload_id(&event.payload, "voteUpdatedAtMicros"),
    ) else {
        return Ok(false);
    };
    let vote: Option<(i16, DateTime<Utc>)> = sqlx::query_as(
        "SELECT value, updated_at FROM forum.votes \
         WHERE post_type = $1 AND post_id = $2 AND account_id = $3 FOR SHARE",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(voter_id)
    .fetch_optional(connection)
    .await?;
    Ok(vote.is_some_and(|(value, updated_at)| {
        value == 1 && updated_at.timestamp_micros() == updated_at_micros
    }))
}

async fn lock_watching_source(
    connection: &mut PgConnection,
    event: &OutboxEvent,
) -> AppResult<bool> {
    let Some(thread_id) = numeric_payload_id(&event.payload, "threadId") else {
        return Ok(false);
    };
    let board_id: Option<i64> =
        sqlx::query_scalar("SELECT board_id FROM forum.threads WHERE id = $1 FOR SHARE")
            .bind(thread_id)
            .fetch_optional(&mut *connection)
            .await?;
    let Some(board_id) = board_id else {
        return Ok(false);
    };
    crate::repo::subscriptions::lock_account_subscriptions(connection, event.recipient_account_id)
        .await?;
    let direct: Option<String> = sqlx::query_scalar(
        "SELECT level FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = 'thread' AND target_id = $2 FOR SHARE",
    )
    .bind(event.recipient_account_id)
    .bind(thread_id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(level) = direct {
        return Ok(level == "watching");
    }
    let board: Option<String> = sqlx::query_scalar(
        "SELECT level FROM forum.subscriptions \
         WHERE account_id = $1 AND target_type = 'board' AND target_id = $2 FOR SHARE",
    )
    .bind(event.recipient_account_id)
    .bind(board_id)
    .fetch_optional(connection)
    .await?;
    Ok(board.as_deref() == Some("watching"))
}

async fn lock_dm_participants_available(
    connection: &mut PgConnection,
    conversation_id: i64,
    account_a: i64,
    account_b: i64,
) -> AppResult<bool> {
    if account_a == account_b {
        return Ok(false);
    }
    let account_ids = [account_a.min(account_b), account_a.max(account_b)];
    let participants: Vec<(i64, bool)> = sqlx::query_as(
        "SELECT account_id, deleted_at IS NULL FROM forum.dm_participants \
         WHERE conversation_id = $1 AND account_id = ANY($2) \
         ORDER BY account_id FOR SHARE",
    )
    .bind(conversation_id)
    .bind(&account_ids[..])
    .fetch_all(connection)
    .await?;
    Ok(participants.len() == 2 && participants.iter().all(|(_, is_available)| *is_available))
}

async fn lock_dm_source(connection: &mut PgConnection, event: &OutboxEvent) -> AppResult<bool> {
    let (Some(conversation_id), Some(actor_id)) =
        (numeric_payload_id(&event.payload, "conversationId"), event.actor_account_id)
    else {
        return Ok(false);
    };
    let conversation = sqlx::query_as::<_, DmSourceState>(
        "SELECT request_status, request_sender_id, request_recipient_id, requested_at \
         FROM forum.dm_conversations WHERE id = $1 FOR SHARE",
    )
    .bind(conversation_id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(conversation) = conversation else {
        return Ok(false);
    };
    match event.event_type.as_str() {
        "dm_request" => {
            let Some(requested_at_micros) = numeric_payload_id(&event.payload, "requestedAtMicros")
            else {
                return Ok(false);
            };
            Ok(conversation.request_status == "pending"
                && conversation.request_sender_id == Some(actor_id)
                && conversation.request_recipient_id == Some(event.recipient_account_id)
                && conversation
                    .requested_at
                    .is_some_and(|value| value.timestamp_micros() == requested_at_micros))
        }
        "dm_request_accepted" => {
            let Some(requested_at_micros) = numeric_payload_id(&event.payload, "requestedAtMicros")
            else {
                return Ok(false);
            };
            if conversation.request_status != "accepted"
                || conversation.request_sender_id != Some(event.recipient_account_id)
                || conversation.request_recipient_id != Some(actor_id)
                || conversation
                    .requested_at
                    .is_none_or(|value| value.timestamp_micros() != requested_at_micros)
            {
                return Ok(false);
            }
            lock_dm_participants_available(
                connection,
                conversation_id,
                actor_id,
                event.recipient_account_id,
            )
            .await
        }
        "dm" => {
            let Some(message_id) = numeric_payload_id(&event.payload, "messageId") else {
                return Ok(false);
            };
            if conversation.request_status != "accepted" {
                return Ok(false);
            }
            let message_exists = sqlx::query_scalar::<_, i64>(
                "SELECT id FROM forum.dm_messages \
                 WHERE id = $1 AND conversation_id = $2 AND sender_id = $3 FOR SHARE",
            )
            .bind(message_id)
            .bind(conversation_id)
            .bind(actor_id)
            .fetch_optional(&mut *connection)
            .await?
            .is_some();
            if !message_exists {
                return Ok(false);
            }
            lock_dm_participants_available(
                connection,
                conversation_id,
                actor_id,
                event.recipient_account_id,
            )
            .await
        }
        _ => Ok(false),
    }
}

fn with_profile_target(event: &OutboxEvent, recipient_handle: Option<&str>) -> OutboxEvent {
    let mut event = event.clone();
    if matches!(
        event.event_type.as_str(),
        "badge"
            | "achievement_awarded"
            | "achievement_revoked"
            | "verification_granted"
            | "verification_revoked"
            | "verification_expired"
    ) {
        if let (Some(handle), Some(payload)) = (recipient_handle, event.payload.as_object_mut()) {
            payload
                .entry("targetUrl")
                .or_insert_with(|| Value::String(format!("/profile/{handle}")));
        }
    }
    event
}

async fn insert_or_aggregate_notification(
    connection: &mut PgConnection,
    event: &OutboxEvent,
) -> AppResult<i64> {
    if let Some(aggregation_key) = event.aggregation_key.as_deref() {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!(
                "notification:{}:{}:{aggregation_key}",
                event.recipient_account_id, event.event_type
            ))
            .execute(&mut *connection)
            .await?;
        let existing: Option<(i64, Value)> = sqlx::query_as(
            "SELECT id, payload FROM forum.notifications \
             WHERE account_id = $1 AND type = $2 AND aggregation_key = $3 \
               AND read_at IS NULL AND created_at > now() - interval '10 minutes' \
             ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
        )
        .bind(event.recipient_account_id)
        .bind(&event.event_type)
        .bind(aggregation_key)
        .fetch_optional(&mut *connection)
        .await?;
        if let Some((notification_id, existing_payload)) = existing {
            let count = existing_payload.get("count").and_then(Value::as_i64).unwrap_or(1) + 1;
            let mut payload = event.payload.clone();
            payload
                .as_object_mut()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("validated payload changed")))?
                .insert("count".into(), Value::from(count));
            sqlx::query("UPDATE forum.notifications SET payload = $1 WHERE id = $2")
                .bind(payload)
                .bind(notification_id)
                .execute(&mut *connection)
                .await?;
            return Ok(notification_id);
        }
    }

    let notification_id = sqlx::query_scalar(
        "INSERT INTO forum.notifications (account_id, type, payload, aggregation_key) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(event.recipient_account_id)
    .bind(&event.event_type)
    .bind(&event.payload)
    .bind(&event.aggregation_key)
    .fetch_one(connection)
    .await?;
    Ok(notification_id)
}

/// Deliver one claimed notification exactly once and complete its outbox event atomically.
pub async fn deliver_event(pool: &PgPool, event: &OutboxEvent) -> AppResult<Option<DeliveryHint>> {
    if event.topic != "notification" {
        return Err(AppError::Internal(anyhow::anyhow!(
            "notification consumer received a different outbox topic"
        )));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("notification-outbox:{}", event.id))
        .execute(&mut *tx)
        .await?;
    if !platform::outbox::lock_claim_tx(&mut tx, event.id, event.claimed_by).await? {
        tx.rollback().await?;
        return Ok(None);
    }
    let already_delivered: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.notification_delivery_receipts \
         WHERE outbox_event_id = $1)",
    )
    .bind(event.id)
    .fetch_one(&mut *tx)
    .await?;
    if already_delivered {
        if !platform::outbox::mark_succeeded_tx(&mut tx, event.id, event.claimed_by).await? {
            return Err(AppError::Internal(anyhow::anyhow!("locked outbox claim changed")));
        }
        tx.commit().await?;
        return Ok(None);
    }

    let (mut outcome, recipient_handle) = delivery_outcome(&mut tx, event).await?;
    if outcome == "delivered" && !lock_source_lifecycle_if_available(&mut tx, event).await? {
        outcome = "content_unavailable";
    }
    if outcome == "delivered" && !lock_content_if_available(&mut tx, event).await? {
        outcome = "content_unavailable";
    }
    let delivery_event = with_profile_target(event, recipient_handle.as_deref());
    let notification_id = if outcome == "delivered" {
        Some(insert_or_aggregate_notification(&mut tx, &delivery_event).await?)
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO forum.notification_delivery_receipts \
         (outbox_event_id, notification_id, outcome) VALUES ($1, $2, $3)",
    )
    .bind(event.id)
    .bind(notification_id)
    .bind(outcome)
    .execute(&mut *tx)
    .await?;
    if !platform::outbox::mark_succeeded_tx(&mut tx, event.id, event.claimed_by).await? {
        return Err(AppError::Internal(anyhow::anyhow!("locked outbox claim changed")));
    }
    tx.commit().await?;

    Ok(notification_id.map(|_| DeliveryHint {
        account_id: event.recipient_account_id,
        event_type: event.event_type.clone(),
    }))
}

/// Retain delivery receipts for 90 days, covering every outbox retry/dead-letter window.
pub async fn purge_delivery_receipts(pool: &PgPool) -> AppResult<u64> {
    let removed = sqlx::query(
        "DELETE FROM forum.notification_delivery_receipts \
         WHERE created_at < now() - interval '90 days'",
    )
    .execute(pool)
    .await?;
    Ok(removed.rows_affected())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{event_requires_actor, in_app_category, preference_is_enabled};

    #[test]
    fn maps_every_optional_interaction_to_one_preference() {
        assert_eq!(in_app_category("follow"), Some("follows"));
        assert_eq!(in_app_category("verification_expired"), Some("badges"));
        assert_eq!(in_app_category("dm_request"), Some("directMessages"));
        assert_eq!(in_app_category("security"), None);
        assert!(event_requires_actor("follow"));
        assert!(!event_requires_actor("verification_granted"));
    }

    #[test]
    fn legacy_missing_follow_preference_remains_enabled() {
        let prefs = json!({ "inApp": { "replies": false } });
        assert!(!preference_is_enabled(Some(&prefs), "reply"));
        assert!(preference_is_enabled(Some(&prefs), "follow"));
    }
}

//! Runtime composition for durable outbox consumers and cross-instance refresh hints.

use std::time::Duration;

use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use shared::AppState;
use uuid::Uuid;

const REDIS_HINT_CHANNEL: &str = "yourtj:notification-hints:v1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationHint {
    instance_id: String,
    account_id: i64,
    event_type: String,
}

async fn publish_hint(
    redis_pool: Option<&deadpool_redis::Pool>,
    instance_id: Uuid,
    hint: forum::notification_delivery::DeliveryHint,
) {
    forum::sse::publish_event(hint.account_id, &hint.event_type, serde_json::json!({}));
    let Some(redis_pool) = redis_pool else {
        return;
    };
    let payload = match serde_json::to_string(&NotificationHint {
        instance_id: instance_id.to_string(),
        account_id: hint.account_id,
        event_type: hint.event_type,
    }) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::warn!(?error, "failed to encode notification refresh hint");
            return;
        }
    };
    let Ok(mut connection) = redis_pool.get().await else {
        tracing::warn!("failed to acquire Redis connection for notification refresh hint");
        return;
    };
    let result: redis::RedisResult<i64> = redis::cmd("PUBLISH")
        .arg(REDIS_HINT_CHANNEL)
        .arg(payload)
        .query_async(&mut connection)
        .await;
    if let Err(error) = result {
        tracing::warn!(?error, "failed to publish notification refresh hint");
    }
}

async fn process_event(
    state: &AppState,
    instance_id: Uuid,
    event: &platform::outbox::OutboxEvent,
) -> shared::AppResult<()> {
    match event.topic.as_str() {
        "notification" => {
            if let Some(hint) =
                forum::notification_delivery::deliver_event(&state.db, event).await?
            {
                publish_hint(state.redis.as_ref(), instance_id, hint).await;
            }
        }
        "achievement_award" => {
            platform::achievements::deliver_automatic_award(&state.db, event).await?;
        }
        _ => {
            return Err(shared::AppError::Internal(
                std::io::Error::other("claimed outbox event has an unsupported topic").into(),
            ));
        }
    }
    Ok(())
}

async fn outbox_loop(state: AppState, instance_id: Uuid) {
    loop {
        let events = match platform::outbox::claim_events(&state.db, instance_id, 50).await {
            Ok(events) => events,
            Err(error) => {
                tracing::warn!(?error, "failed to claim durable outbox events");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        if events.is_empty() {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }
        for event in events {
            if let Err(error) = process_event(&state, instance_id, &event).await {
                tracing::warn!(?error, outbox_event_id = event.id, topic = %event.topic, "durable outbox event failed");
                let error_code = match event.topic.as_str() {
                    "notification" => "notification_delivery_failed",
                    "achievement_award" => "achievement_award_failed",
                    _ => "unsupported_topic",
                };
                match platform::outbox::record_failure(&state.db, &event, error_code).await {
                    Ok(Some(status)) if status == "dead" => {
                        tracing::error!(outbox_event_id = event.id, topic = %event.topic, "durable outbox event moved to dead letter")
                    }
                    Ok(_) => {}
                    Err(record_error) => tracing::warn!(
                        ?record_error,
                        outbox_event_id = event.id,
                        "failed to record outbox retry state"
                    ),
                }
            }
        }
    }
}

async fn redis_hint_loop(redis_url: String, instance_id: Uuid) {
    let mut retry_delay = Duration::from_secs(1);
    loop {
        let result = async {
            let client = redis::Client::open(redis_url.as_str())?;
            let mut pubsub = client.get_async_pubsub().await?;
            pubsub.subscribe(REDIS_HINT_CHANNEL).await?;
            tracing::info!("Redis notification hint bridge connected");
            retry_delay = Duration::from_secs(1);
            let mut messages = pubsub.on_message();
            while let Some(message) = messages.next().await {
                let raw: String = message.get_payload()?;
                let Ok(hint) = serde_json::from_str::<NotificationHint>(&raw) else {
                    tracing::warn!("discarded malformed Redis notification hint");
                    continue;
                };
                if hint.instance_id == instance_id.to_string()
                    || hint.account_id <= 0
                    || hint.event_type.is_empty()
                    || hint.event_type.len() > 80
                {
                    continue;
                }
                forum::sse::publish_event(hint.account_id, &hint.event_type, serde_json::json!({}));
            }
            Ok::<(), redis::RedisError>(())
        }
        .await;
        if let Err(error) = result {
            tracing::warn!(?error, "Redis notification hint bridge disconnected");
        }
        tokio::time::sleep(retry_delay).await;
        retry_delay = (retry_delay * 2).min(Duration::from_secs(30));
    }
}

async fn retention_loop(pool: sqlx::PgPool) {
    loop {
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
        if let Err(error) = forum::notification_delivery::purge_delivery_receipts(&pool).await {
            tracing::warn!(?error, "notification delivery receipt retention failed");
        }
        if let Err(error) = platform::outbox::purge_terminal_events(&pool).await {
            tracing::warn!(?error, "durable outbox retention failed");
        }
    }
}

pub fn start(state: &AppState) {
    let instance_id = Uuid::new_v4();
    tokio::spawn(outbox_loop(state.clone(), instance_id));
    tokio::spawn(retention_loop(state.db.clone()));
    if state.redis.is_some() && !state.config.redis_url.is_empty() {
        tokio::spawn(redis_hint_loop(state.config.redis_url.clone(), instance_id));
    } else {
        tracing::warn!("Redis unavailable; durable notifications remain correct without cross-instance realtime hints");
    }
    tracing::info!(%instance_id, "durable notification worker started");
}

#[cfg(test)]
mod tests {
    use super::NotificationHint;

    #[test]
    fn realtime_hint_contains_no_notification_payload() {
        let encoded = serde_json::to_value(NotificationHint {
            instance_id: uuid::Uuid::nil().to_string(),
            account_id: 42,
            event_type: "reply".into(),
        })
        .expect("encode hint");
        assert_eq!(encoded.as_object().map(|object| object.len()), Some(3));
        assert!(encoded.get("payload").is_none());
    }
}

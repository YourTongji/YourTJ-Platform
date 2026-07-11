//! SSE (Server-Sent Events) notification stream for the forum domain.
//!
//! Infrastructure:
//! - A `tokio::sync::broadcast` channel carries [`SsePayload`] events to all
//!   connected SSE clients.
//! - The global sender is stored as a `OnceLock` so that
//!   [`crate::notification_hooks`] can publish events without plumbing the
//!   sender through every call site.
//! - On multi-instance deployments an external Redis pub/sub layer would
//!   bridge instances; for the single-instance case the broadcast channel
//!   suffices.
//!
//! The SSE endpoint (`GET /api/v2/notifications/stream`) authenticates,
//! subscribes to the broadcast channel, filters by `account_id`, and streams
//! events with a 30‑second heartbeat via `KeepAlive`.

use std::sync::OnceLock;
use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use shared::{AppState, SsePayload};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

/// Convenience alias for the broadcast sender.
pub type SseTx = broadcast::Sender<SsePayload>;

// ---------------------------------------------------------------------------
// Global sender (set once at startup)
// ---------------------------------------------------------------------------

static GLOBAL_SSE_TX: OnceLock<SseTx> = OnceLock::new();

/// Initialise the global SSE sender. Called once during bootstrap.
pub fn init_global(tx: SseTx) {
    let _ = GLOBAL_SSE_TX.set(tx);
}

/// Publish an event for a specific account through the global sender.
///
/// This is a no-op when the global sender has not been initialised (SSE
/// disabled).
pub fn publish_event(account_id: i64, event_type: &str, payload: serde_json::Value) {
    if let Some(tx) = GLOBAL_SSE_TX.get() {
        let _ = tx.send(SsePayload { account_id, event_type: event_type.to_string(), payload });
    }
}

/// Publish a generic "notification" event (convenience wrapper).
pub fn publish_notification(account_id: i64) {
    publish_event(account_id, "notification", serde_json::json!({}));
}

// ---------------------------------------------------------------------------
// SSE stream handler
// ---------------------------------------------------------------------------

/// `GET /api/v2/notifications/stream` — SSE notification stream.
///
/// Authenticates the request, subscribes to the broadcast channel, and returns
/// an infinite SSE stream of events scoped to the authenticated account.
///
/// Heartbeat is sent every 30 seconds (comment‑only keep‑alive). The stream
/// ends when the server shuts down; the client should reconnect.
pub async fn handle_sse_stream(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let auth = match identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    {
        Ok(auth) => auth,
        Err(resp) => return resp,
    };

    let rx = match &state.sse_tx {
        Some(tx) => tx.subscribe(),
        None => {
            return (axum::http::StatusCode::SERVICE_UNAVAILABLE, "SSE not available")
                .into_response()
        }
    };

    let account_id = auth.id;

    // Wrap the broadcast receiver in a stream, filter by account, and map to
    // SSE Event structs.
    let stream = BroadcastStream::new(rx)
        .filter_map(move |result| match result {
            Ok(payload) if payload.account_id == account_id => Some(payload),
            _ => None,
        })
        .map(|payload| {
            let data = serde_json::to_string(&payload.payload).unwrap_or_default();
            Ok::<_, std::convert::Infallible>(Event::default().event(payload.event_type).data(data))
        });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)).text("heartbeat"))
        .into_response()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_channel_basic_roundtrip() {
        let (tx, mut rx) = broadcast::channel(8);

        let payload = SsePayload {
            account_id: 42,
            event_type: "test".into(),
            payload: serde_json::json!({"key": "value"}),
        };

        assert!(tx.send(payload.clone()).is_ok());
        let received = rx.blocking_recv().unwrap();
        assert_eq!(received.account_id, 42);
        assert_eq!(received.event_type, "test");
        assert_eq!(received.payload, serde_json::json!({"key": "value"}));
    }

    #[test]
    fn sse_publish_event_global() {
        let (tx, _rx) = broadcast::channel(8);
        init_global(tx);

        publish_event(42, "test", serde_json::json!({"ok": true}));
        // No panic = success (fire-and-forget on the global).
    }
}

//! Lightweight SSE payload type shared by all domain crates.
//!
//! `SsePayload` flows through a `tokio::sync::broadcast` channel. The forum
//! crate owns the SSE endpoint, but any crate can publish events through
//! `AppState::sse_tx`.

use serde::{Deserialize, Serialize};

/// An event published over the SSE broadcast channel.
///
/// `account_id` determines which connected client receives the event.
/// The SSE handler filters by `account_id` so each client only gets its own
/// events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsePayload {
    pub account_id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
}

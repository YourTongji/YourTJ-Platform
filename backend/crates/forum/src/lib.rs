//! Forum domain (Phase B): boards, threads, nested comments (楼中楼), votes,
//! follows, notifications, hot ranking, and forum search.
//!
//! At current scale, timelines are read-aggregated and cached — do NOT build
//! fan-out-on-write. Hot ranking is a periodic job writing a Redis ZSET.

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::AppState;

/// All routes owned by the forum domain.
pub fn routes(_state: AppState) -> Router {
    Router::new().route("/api/v2/forum/boards", get(boards))
}

async fn boards() -> Json<Value> {
    Json(json!({ "todo": "forum.boards" }))
}

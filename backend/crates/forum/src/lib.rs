//! Forum domain (Phase B): boards, threads, nested comments (楼中楼), votes,
//! follows, notifications, hot ranking, and forum search.
//!
//! At current scale, timelines are read-aggregated and cached — do NOT build
//! fan-out-on-write. Hot ranking is a periodic job writing a Redis ZSET.
mod dto;
mod error;
mod models;
pub mod repo;

use axum::Router;
use shared::AppState;

/// All routes owned by the forum domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/forum/boards",
            axum::routing::get(|| async {
                axum::Json(serde_json::json!({"todo": "forum.boards"}))
            }),
        )
        .with_state(state)
}

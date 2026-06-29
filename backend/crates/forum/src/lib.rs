//! Forum domain (Phase B): boards, threads, nested comments (楼中楼), votes,
//! follows, notifications, hot ranking, and forum search.
//!
//! At current scale, timelines are read-aggregated and cached — do NOT build
//! fan-out-on-write. Hot ranking is a periodic job writing a Redis ZSET.
mod dto;
mod error;
mod handlers;
mod models;
pub mod notification_hooks;
mod notifications;
pub mod repo;

use axum::routing::{get, post};
use axum::Router;
use shared::AppState;

/// All routes owned by the forum domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/forum/boards", get(handlers::list_boards))
        .route("/api/v2/forum/boards/{board_id}/threads", get(handlers::list_threads))
        .route(
            "/api/v2/forum/threads",
            get(handlers::list_threads_feed).post(handlers::create_thread),
        )
        .route("/api/v2/forum/threads/{id}", get(handlers::get_thread))
        .route(
            "/api/v2/forum/threads/{thread_id}/comments",
            get(handlers::list_comments).post(handlers::create_comment),
        )
        .route("/api/v2/forum/posts/{post_id}/vote", post(handlers::vote_post))
        // Notifications
        .route("/api/v2/notifications", get(notifications::list_notifications_handler))
        .route("/api/v2/notifications/read", post(notifications::mark_read_handler))
        .with_state(state)
}

//! Forum domain (Phase B): boards, threads, nested comments (楼中楼), votes,
//! follows, notifications, hot ranking, and forum search.
//!
//! At current scale, timelines are read-aggregated and cached — do NOT build
//! fan-out-on-write. Hot ranking is a periodic job writing a Redis ZSET.
mod admin;
mod dto;
mod error;
mod handlers;
pub mod meili;
mod models;
pub mod notification_hooks;
mod notifications;
pub mod repo;
pub mod trust_levels;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use shared::AppState;

/// All routes owned by the forum domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/forum/tags", get(handlers::list_tags_handler))
        .route("/api/v2/forum/boards", get(handlers::list_boards))
        .route("/api/v2/forum/boards/{board_id}/threads", get(handlers::list_threads))
        .route(
            "/api/v2/forum/threads",
            get(handlers::list_threads_feed).post(handlers::create_thread),
        )
        .route(
            "/api/v2/forum/threads/{id}",
            get(handlers::get_thread)
                .patch(handlers::update_thread)
                .delete(handlers::delete_thread),
        )
        .route("/api/v2/forum/threads/{id}/revisions", get(handlers::list_thread_revisions))
        .route(
            "/api/v2/forum/threads/{thread_id}/comments",
            get(handlers::list_comments).post(handlers::create_comment),
        )
        .route("/api/v2/forum/comments/{id}", patch(handlers::update_comment))
        .route("/api/v2/forum/comments/{id}", delete(handlers::delete_comment))
        .route("/api/v2/forum/comments/{id}/revisions", get(handlers::list_comment_revisions))
        .route("/api/v2/forum/threads/unread", get(handlers::list_unread_threads))
        .route("/api/v2/forum/threads/{id}/read", post(handlers::report_read))
        .route("/api/v2/forum/posts/{post_id}/vote", post(handlers::vote_post))
        .route("/api/v2/forum/posts/{id}/flag", post(handlers::flag_post))
        .route(
            "/api/v2/forum/posts/{id}/bookmark",
            put(handlers::set_bookmark).delete(handlers::remove_bookmark),
        )
        .route("/api/v2/forum/bookmarks", get(handlers::list_bookmarks_handler))
        // Subscriptions
        .route(
            "/api/v2/forum/subscriptions",
            get(handlers::list_subscriptions_handler)
                .put(handlers::set_subscription_handler)
                .delete(handlers::delete_subscription_handler),
        )
        // Notifications
        .route("/api/v2/notifications", get(notifications::list_notifications_handler))
        .route("/api/v2/notifications/unread-count", get(notifications::unread_count_handler))
        .route("/api/v2/notifications/read", post(notifications::mark_read_handler))
        // Notification prefs (user level)
        .route(
            "/api/v2/me/notification-prefs",
            get(handlers::get_my_notification_prefs).put(handlers::set_my_notification_prefs),
        )
        .merge(admin::routes())
        .with_state(state)
}

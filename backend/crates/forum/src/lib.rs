//! Forum domain (Phase B): boards, threads, nested comments (楼中楼), votes,
//! follows, notifications, hot ranking, and forum search.
//!
//! At current scale, timelines are read-aggregated and cached — do NOT build
//! fan-out-on-write. Hot ranking is a periodic job writing a Redis ZSET.
mod admin;
pub mod badges;
mod cache;
mod content_policy;
pub mod digest;
mod dto;
mod error;
mod handlers;
pub mod meili;
mod models;
pub mod notification_hooks;
mod notifications;
pub mod repo;
mod sanctions;
pub mod sse;
pub mod tip_targets;
pub mod trust_levels;
pub mod watched_words;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use shared::AppState;

/// All routes owned by the forum domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/users/{handle}", get(handlers::get_user_profile))
        .route("/api/v2/users/{handle}/threads", get(handlers::list_user_threads))
        .route("/api/v2/users/{handle}/comments", get(handlers::list_user_comments))
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
        .route(
            "/api/v2/forum/comments/{id}/solve",
            post(handlers::mark_solved_handler).delete(handlers::unmark_solved_handler),
        )
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
        .route("/api/v2/notifications/stream", get(sse::handle_sse_stream))
        // Drafts
        .route(
            "/api/v2/me/drafts",
            get(handlers::list_drafts_handler).put(handlers::save_draft_handler),
        )
        .route(
            "/api/v2/me/drafts/{draft_key}",
            get(handlers::get_draft_handler).delete(handlers::delete_draft_handler),
        )
        // Notification prefs (user level)
        .route(
            "/api/v2/me/notification-prefs",
            get(handlers::get_my_notification_prefs).put(handlers::set_my_notification_prefs),
        )
        // User ignores (blocking)
        .route(
            "/api/v2/me/ignores/{account_id}",
            put(handlers::ignore_user_handler).delete(handlers::unignore_user_handler),
        )
        .route("/api/v2/me/ignores", get(handlers::list_ignores_handler))
        // Polls
        .route("/api/v2/forum/polls/{id}/vote", post(handlers::vote_poll_handler))
        .route("/api/v2/forum/polls/{id}/results", get(handlers::poll_results_handler))
        // DMs (1:1 private messages)
        .route(
            "/api/v2/forum/dm/conversations",
            get(handlers::list_conversations_handler)
                .post(handlers::create_or_get_conversation_handler),
        )
        .route("/api/v2/forum/dm/unread-count", get(handlers::unread_dm_count_handler))
        .route("/api/v2/forum/dm/conversations/{id}", delete(handlers::delete_conversation_handler))
        .route(
            "/api/v2/forum/dm/conversations/{id}/recover",
            post(handlers::recover_conversation_handler),
        )
        .route(
            "/api/v2/forum/dm/conversations/{id}/archive",
            put(handlers::archive_conversation_handler)
                .delete(handlers::unarchive_conversation_handler),
        )
        .route(
            "/api/v2/forum/dm/conversations/{id}/mute",
            put(handlers::mute_conversation_handler).delete(handlers::unmute_conversation_handler),
        )
        .route(
            "/api/v2/forum/dm/conversations/{id}/messages",
            get(handlers::list_messages_handler).post(handlers::send_message_handler),
        )
        .route(
            "/api/v2/forum/dm/conversations/{id}/read",
            post(handlers::read_conversation_handler),
        )
        .route("/api/v2/forum/dm/messages/{id}/report", post(handlers::report_message_handler))
        .route("/api/v2/admin/dm/reports", get(handlers::list_dm_reports_handler))
        .route("/api/v2/admin/dm/reports/{id}/resolve", post(handlers::resolve_dm_report_handler))
        .merge(admin::routes())
        .with_state(state)
}

//! Forum admin endpoints: board/tag/thread/comment CRUD, flag queue,
//! watched words, and moderator action log.
//!
//! Every handler checks for mod or admin role via `identity::auth_middleware`.

pub mod boards;
pub mod comments;
pub mod flags;
pub mod mod_actions;
pub mod tags;
pub mod threads;
pub mod watched_words;

use axum::routing::{delete, get, patch, post};
use axum::Router;
use shared::AppState;

/// All `/api/v2/admin/forum/...` routes, assembled from the admin submodules.
pub fn routes() -> Router<AppState> {
    Router::<AppState>::new()
        // Boards
        .route("/api/v2/admin/forum/boards", post(boards::create_board))
        .route(
            "/api/v2/admin/forum/boards/{id}",
            patch(boards::update_board).delete(boards::delete_board),
        )
        // Tags
        .route("/api/v2/admin/forum/tags", get(tags::list_tags_admin).post(tags::create_tag))
        .route("/api/v2/admin/forum/tags/{id}", patch(tags::update_tag).delete(tags::delete_tag))
        // Thread actions
        .route("/api/v2/admin/forum/threads/{id}/{action}", post(threads::admin_thread_action))
        // Comment actions
        .route("/api/v2/admin/forum/comments/{id}/{action}", post(comments::admin_comment_action))
        // Flags
        .route("/api/v2/admin/forum/flags", get(flags::list_flags_queue))
        .route("/api/v2/admin/forum/flags/{id}/resolve", post(flags::resolve_flag))
        // Watched words
        .route(
            "/api/v2/admin/forum/watched-words",
            get(watched_words::list_watched_words).post(watched_words::create_watched_word),
        )
        .route("/api/v2/admin/forum/watched-words/{id}", delete(watched_words::delete_watched_word))
        // Mod action log
        .route("/api/v2/admin/forum/mod-actions", get(mod_actions::list_mod_actions))
}

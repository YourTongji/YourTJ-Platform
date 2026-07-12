//! Forum admin endpoints: board/tag/thread/comment CRUD, flag queue,
//! watched words, and moderator action log.
//!
//! Every handler checks for mod or admin role via `identity::auth_middleware`.

pub mod badges;
pub mod boards;
pub mod comments;
pub mod flags;
pub mod mod_actions;
pub mod tags;
pub mod threads;
pub mod watched_words;

use axum::routing::{delete, get, patch, post};
use axum::Router;
use shared::auth::AuthAccount;
use shared::{AppError, AppResult, AppState};
use sqlx::PgConnection;

fn role_rank(role: &str) -> Option<u8> {
    match role {
        "user" => Some(0),
        "mod" => Some(1),
        "admin" => Some(2),
        _ => None,
    }
}

/// Reject moderation when the content author has the actor's role or a higher role.
pub(crate) async fn require_lower_author_role(
    connection: &mut PgConnection,
    actor: &AuthAccount,
    author_id: Option<i64>,
) -> AppResult<()> {
    let Some(author_id) = author_id else {
        return Ok(());
    };
    let author_role = identity::public_accounts::find_account_role_by_id(connection, author_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("forum author account is missing")))?;
    let actor_rank = role_rank(&actor.role).ok_or(AppError::Forbidden)?;
    let author_rank = role_rank(&author_role).ok_or(AppError::Forbidden)?;
    if actor.id == author_id || actor_rank <= author_rank {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

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
        .route("/api/v2/admin/forum/threads/{id}", get(threads::get_thread_for_moderation))
        .route("/api/v2/admin/forum/threads/{id}/{action}", post(threads::admin_thread_action))
        // Comment actions
        .route("/api/v2/admin/forum/comments/{id}", get(comments::get_comment_for_moderation))
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
        // Thread feature
        .route("/api/v2/admin/forum/threads/{id}/feature", post(badges::feature_thread))
}

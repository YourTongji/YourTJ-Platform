//! Axum request handlers for the forum domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

mod boards;
mod bookmarks;
mod comments;
mod flags;
mod read_tracking;
mod subscriptions;
mod tags;
mod threads;
mod user;
mod votes;

use crate::dto::{BoardDto, CommentDto, ThreadDetailDto, ThreadDto};

pub use boards::*;
pub use bookmarks::*;
pub use comments::*;
pub use flags::*;
pub use read_tracking::*;
pub use subscriptions::*;
pub use tags::*;
pub use threads::*;
pub use user::{get_my_notification_prefs, set_my_notification_prefs};
pub use votes::*;

// ---------------------------------------------------------------------------
// row → dto helpers
// ---------------------------------------------------------------------------

pub(crate) fn thread_to_dto(row: &crate::models::ThreadRowJoined) -> ThreadDto {
    ThreadDto {
        id: row.id.to_string(),
        board_id: row.board_id.to_string(),
        author_handle: row.author_handle.clone(),
        title: row.title.clone(),
        reply_count: row.reply_count,
        vote_count: row.vote_count,
        hot_score: row.hot_score,
        created_at: row.created_at.timestamp(),
        last_activity_at: row.last_activity_at.timestamp(),
    }
}

pub(crate) fn thread_to_detail_dto(row: &crate::models::ThreadRowJoined) -> ThreadDetailDto {
    ThreadDetailDto { base: thread_to_dto(row), body: row.body.clone() }
}

pub(crate) fn comment_to_dto(row: &crate::models::CommentRowJoined) -> CommentDto {
    CommentDto {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        parent_id: row.parent_id.map(|v| v.to_string()),
        path: row.path.clone().unwrap_or_default(),
        author_handle: row.author_handle.clone(),
        body: row.body.clone(),
        vote_count: row.vote_count,
        created_at: row.created_at.timestamp(),
    }
}

pub(crate) fn board_to_dto(row: &crate::models::BoardRow) -> BoardDto {
    BoardDto { id: row.id.to_string(), slug: row.slug.clone(), name: row.name.clone() }
}

pub(crate) fn default_sort() -> String {
    "new".into()
}

pub(crate) fn default_limit() -> i64 {
    20
}

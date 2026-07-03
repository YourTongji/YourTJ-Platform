//! Axum request handlers for the forum domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

mod boards;
mod bookmarks;
mod comments;
mod dms;
mod drafts;
mod flags;
mod ignores;
mod polls;
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
pub use dms::*;
pub use drafts::*;
pub use flags::*;
pub use ignores::*;
pub use polls::*;
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
        tags: vec![],
        created_at: row.created_at.timestamp(),
        last_activity_at: row.last_activity_at.timestamp(),
    }
}

pub(crate) fn thread_to_detail_dto(row: &crate::models::ThreadRowJoinedFull) -> ThreadDetailDto {
    ThreadDetailDto {
        id: row.id.to_string(),
        board_id: row.board_id.to_string(),
        author_handle: row.author_handle.clone(),
        author_id: row.author_id.to_string(),
        title: row.title.clone(),
        body: row.body.clone(),
        reply_count: row.reply_count,
        vote_count: row.vote_count,
        hot_score: row.hot_score,
        tags: vec![],
        status: row.status.clone(),
        pinned_at: row.pinned_at.map(|v| v.timestamp()),
        pinned_globally: row.pinned_globally,
        featured_at: row.featured_at.map(|v| v.timestamp()),
        closed_at: row.closed_at.map(|v| v.timestamp()),
        archived_at: row.archived_at.map(|v| v.timestamp()),
        deleted_at: row.deleted_at.map(|v| v.timestamp()),
        edited_at: row.edited_at.map(|v| v.timestamp()),
        hidden_at: row.hidden_at.map(|v| v.timestamp()),
        created_at: row.created_at.timestamp(),
        last_activity_at: row.last_activity_at.timestamp(),
        solved_answer_id: row.solved_answer_id.map(|v| v.to_string()),
        my_last_read_comment_id: None,
        my_subscription_level: None,
        poll: None,
    }
}

pub(crate) fn comment_to_dto(
    row: &crate::models::CommentRowJoined,
    solved_comment_id: Option<i64>,
) -> CommentDto {
    CommentDto {
        id: row.id.to_string(),
        thread_id: row.thread_id.to_string(),
        parent_id: row.parent_id.map(|v| v.to_string()),
        path: row.path.clone().unwrap_or_default(),
        author_handle: row.author_handle.clone(),
        author_id: row.author_id.to_string(),
        body: row.body.clone(),
        vote_count: row.vote_count,
        is_deleted: row.deleted_at.is_some(),
        is_hidden: row.hidden_at.is_some(),
        edited_at: row.edited_at.map(|v| v.timestamp()),
        created_at: row.created_at.timestamp(),
        quoted_comment_id: row.quoted_comment_id.map(|v| v.to_string()),
        is_solved: Some(row.id) == solved_comment_id,
    }
}

pub(crate) fn board_to_dto(row: &crate::models::BoardRow) -> BoardDto {
    BoardDto {
        id: row.id.to_string(),
        slug: row.slug.clone(),
        name: row.name.clone(),
        parent_id: row.parent_id.map(|v| v.to_string()),
        description: row.description.clone(),
        position: row.position,
        is_locked: row.is_locked,
        min_trust_to_post: row.min_trust_to_post,
        thread_count: row.thread_count,
    }
}

pub(crate) fn default_sort() -> String {
    "new".into()
}

pub(crate) fn default_limit() -> i64 {
    20
}

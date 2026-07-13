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
mod profiles;
mod read_tracking;
mod relationships;
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
pub use profiles::*;
pub use read_tracking::*;
pub use relationships::*;
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
        author_display_name: row.author_display_name.clone(),
        author_avatar: None,
        title: row.title.clone(),
        body_excerpt: None,
        content_version: row.content_version,
        reply_count: row.reply_count,
        vote_count: row.vote_count,
        hot_score: row.hot_score,
        status: row.status.clone(),
        tags: vec![],
        attachments: vec![],
        created_at: row.created_at.timestamp(),
        last_activity_at: row.last_activity_at.timestamp(),
        viewer_vote: None,
        is_bookmarked: false,
        can_edit: false,
        can_delete: false,
        can_moderate: false,
        unread_count: None,
    }
}

pub(crate) fn thread_to_detail_dto(row: &crate::models::ThreadRowJoinedFull) -> ThreadDetailDto {
    ThreadDetailDto {
        id: row.id.to_string(),
        board_id: row.board_id.to_string(),
        author_handle: row.author_handle.clone(),
        author_display_name: row.author_display_name.clone(),
        author_avatar: None,
        author_id: row.author_id.to_string(),
        title: row.title.clone(),
        body: row.body.clone(),
        content_format: crate::dto::ContentFormat::from_db(&row.content_format),
        content_version: row.content_version,
        reply_count: row.reply_count,
        vote_count: row.vote_count,
        hot_score: row.hot_score,
        tags: vec![],
        attachments: vec![],
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
        viewer_vote: None,
        is_bookmarked: false,
        my_last_read_comment_id: None,
        my_subscription_level: None,
        poll: None,
        can_edit: false,
        can_delete: false,
        can_moderate: false,
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
        author_display_name: row.author_display_name.clone(),
        author_avatar: None,
        author_id: row.author_id.to_string(),
        body: row.body.clone(),
        content_format: crate::dto::ContentFormat::from_db(&row.content_format),
        content_version: row.content_version,
        attachments: vec![],
        vote_count: row.vote_count,
        viewer_vote: None,
        is_bookmarked: false,
        is_deleted: row.deleted_at.is_some(),
        is_hidden: row.hidden_at.is_some(),
        edited_at: row.edited_at.map(|v| v.timestamp()),
        created_at: row.created_at.timestamp(),
        quoted_comment_id: row.quoted_comment_id.map(|v| v.to_string()),
        is_solved: Some(row.id) == solved_comment_id,
        can_edit: false,
        can_delete: false,
        can_moderate: false,
    }
}

pub(crate) fn board_to_dto(
    row: &crate::models::BoardRow,
    actor: Option<crate::repo::boards::BoardPostingActor>,
) -> BoardDto {
    let posting_restriction = crate::repo::boards::posting_restriction(row, actor);
    BoardDto {
        id: row.id.to_string(),
        slug: row.slug.clone(),
        name: row.name.clone(),
        parent_id: row.parent_id.map(|v| v.to_string()),
        description: row.description.clone(),
        position: row.position,
        is_locked: row.is_locked,
        min_trust_to_post: row.min_trust_to_post,
        is_qa: row.is_qa,
        thread_count: row.thread_count,
        can_post: posting_restriction.is_none(),
        posting_restriction: posting_restriction.map(|restriction| restriction.as_str().to_owned()),
    }
}

pub(crate) fn default_sort() -> String {
    "new".into()
}

pub(crate) fn default_limit() -> i64 {
    20
}

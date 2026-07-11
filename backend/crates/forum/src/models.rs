//! Database row types mapped from the forum schema via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// A row from `forum.boards`.
#[derive(Debug, Clone, FromRow)]
pub struct BoardRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub position: i32,
    pub is_locked: bool,
    pub is_qa: bool,
    pub min_trust_to_post: i16,
    pub thread_count: i32,
}

/// A row from `forum.threads`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct ThreadRow {
    pub id: i64,
    pub board_id: i64,
    pub author_id: i64,
    pub title: String,
    pub body: Option<String>,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

/// A row from `forum.comments`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct CommentRow {
    pub id: i64,
    pub thread_id: i64,
    pub parent_id: Option<i64>,
    pub path: Option<String>,
    pub author_id: i64,
    pub body: String,
    pub vote_count: i32,
    pub created_at: DateTime<Utc>,
}

/// A joined row from `forum.threads` + `identity.accounts` (via author_id).
#[derive(Debug, Clone, FromRow)]
pub struct ThreadRowJoined {
    pub id: i64,
    pub board_id: i64,
    pub author_id: i64,
    pub title: String,
    pub body: Option<String>,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub author_handle: String,
}

/// A thread row joined with author handle, including all F1 state-machine columns.
#[derive(Debug, Clone, FromRow)]
pub struct ThreadRowJoinedFull {
    pub id: i64,
    pub board_id: i64,
    pub author_id: i64,
    pub title: String,
    pub body: Option<String>,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub status: String,
    pub pinned_at: Option<DateTime<Utc>>,
    pub pinned_globally: bool,
    pub featured_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
    pub edited_at: Option<DateTime<Utc>>,
    pub hidden_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub solved_answer_id: Option<i64>,
    pub author_handle: String,
}

/// A joined row from `forum.comments` + `identity.accounts` (via author_id).
#[derive(Debug, Clone, FromRow)]
pub struct CommentRowJoined {
    pub id: i64,
    pub thread_id: i64,
    pub parent_id: Option<i64>,
    pub path: Option<String>,
    pub author_id: i64,
    pub body: String,
    pub vote_count: i32,
    pub deleted_at: Option<DateTime<Utc>>,
    pub hidden_at: Option<DateTime<Utc>>,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub author_handle: String,
    pub quoted_comment_id: Option<i64>,
}

/// A row from `forum.flags`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct FlagRow {
    pub id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub reporter_id: i64,
    pub reason: String,
    pub note: Option<String>,
    pub weight: f32,
    pub status: String,
    pub handled_by: Option<i64>,
    pub handled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub auto_hidden_at: Option<chrono::DateTime<chrono::Utc>>,
    pub resolution_note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A moderation-queue report joined with bounded target evidence.
#[derive(Debug, Clone, FromRow)]
pub struct FlagQueueRow {
    pub id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub reporter_id: i64,
    pub reason: String,
    pub note: Option<String>,
    pub weight: f32,
    pub status: String,
    pub handled_by: Option<i64>,
    pub handled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub auto_hidden_at: Option<chrono::DateTime<chrono::Utc>>,
    pub resolution_note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub author_handle: Option<String>,
    pub target_title: Option<String>,
    pub content_excerpt: Option<String>,
}

/// A row from `forum.tags`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct TagRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub thread_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.thread_reads`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct ThreadReadRow {
    pub account_id: i64,
    pub thread_id: i64,
    pub last_read_comment_id: Option<i64>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.bookmarks`.
#[derive(Debug, Clone, FromRow)]
pub struct BookmarkRow {
    pub account_id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.subscriptions`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct SubscriptionRow {
    pub account_id: i64,
    pub target_type: String,
    pub target_id: i64,
    pub level: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.mod_actions`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct ModActionRow {
    pub id: i64,
    pub actor_id: i64,
    pub action: String,
    pub target_type: String,
    pub target_id: i64,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.post_revisions`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct PostRevisionRow {
    pub id: i64,
    pub post_type: String,
    pub post_id: i64,
    pub seq: i32,
    pub editor_id: i64,
    pub old_title: Option<String>,
    pub old_body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.dm_conversations`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmConversationRow {
    pub id: i64,
    pub account_low_id: i64,
    pub account_high_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.dm_participants`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DmParticipantRow {
    pub conversation_id: i64,
    pub account_id: i64,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub last_read_message_id: Option<i64>,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A row from `forum.dm_messages`, joined with sender handle.
#[derive(Debug, Clone, FromRow)]
pub struct DmMessageRow {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub sender_handle: String,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A conversation with the other participant's handle and last message time.
#[derive(Debug, Clone, FromRow)]
pub struct DmConversationListRow {
    pub id: i64,
    pub other_account_id: i64,
    pub other_handle: String,
    pub other_avatar_url: Option<String>,
    pub last_message_excerpt: Option<String>,
    pub last_message_at: chrono::DateTime<chrono::Utc>,
    pub unread_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A DM report joined with the reported message and sender identity.
#[derive(Debug, Clone, FromRow)]
pub struct DmMessageReportRow {
    pub id: i64,
    pub message_id: i64,
    pub conversation_id: i64,
    pub reported_by: i64,
    pub reporter_handle: String,
    pub sender_id: i64,
    pub sender_handle: String,
    pub message_excerpt: String,
    pub reason: String,
    pub note: Option<String>,
    pub status: String,
    pub handled_by: Option<i64>,
    pub handled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.notification_prefs`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct NotificationPrefsRow {
    pub account_id: i64,
    pub prefs: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.polls`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct PollRow {
    pub id: i64,
    pub thread_id: i64,
    pub question: String,
    pub multi_select: bool,
    pub closes_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A row from `forum.poll_options`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct PollOptionRow {
    pub id: i64,
    pub poll_id: i64,
    pub position: i32,
    pub label: String,
    pub vote_count: i32,
}

/// A row from `forum.poll_votes`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct PollVoteRow {
    pub poll_option_id: i64,
    pub account_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

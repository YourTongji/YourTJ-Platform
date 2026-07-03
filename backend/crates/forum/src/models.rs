//! Database row types mapped from the forum schema via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// A row from `forum.boards`.
#[derive(Debug, Clone, FromRow)]
pub struct BoardRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
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
    pub created_at: DateTime<Utc>,
    pub author_handle: String,
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
    pub created_at: chrono::DateTime<chrono::Utc>,
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

/// A row from `forum.notification_prefs`.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct NotificationPrefsRow {
    pub account_id: i64,
    pub prefs: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

//! Request and response types for the forum domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// Public-facing board DTO.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardDto {
    pub id: String,
    pub slug: String,
    pub name: String,
}

/// Summary view of a thread (list responses).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDto {
    pub id: String,
    pub board_id: String,
    pub author_handle: String,
    pub title: String,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub created_at: i64,
    pub last_activity_at: i64,
}

/// Detail view of a thread — ThreadDto + optional body.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDetailDto {
    #[serde(flatten)]
    pub base: ThreadDto,
    pub body: Option<String>,
}

/// POST /forum/threads
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInput {
    pub board_id: String,
    pub title: String,
    pub body: Option<String>,
}

/// Public-facing comment DTO.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: String,
    pub thread_id: String,
    pub parent_id: Option<String>,
    pub path: String,
    pub author_handle: String,
    pub body: String,
    pub vote_count: i32,
    pub created_at: i64,
}

/// POST /forum/threads/{thread_id}/comments
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
    pub parent_id: Option<String>,
    pub body: String,
}

/// POST /forum/posts/{post_id}/vote
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteInput {
    pub value: String,     // "up" or "down"
    pub post_type: String, // "thread" or "comment"
}

/// Tag DTO.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub thread_count: i32,
    pub created_at: i64,
}

/// POST /forum/threads/{id}/read — report read position
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTrackingInput {
    pub last_read_comment_id: Option<String>,
}

/// Feed DTO for unread threads (includes unread count).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadFeedDto {
    pub id: String,
    pub board_id: String,
    pub author_handle: String,
    pub title: String,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub created_at: i64,
    pub last_activity_at: i64,
    pub unread_count: i32,
}

/// Bookmark input — used when (un)setting a bookmark.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkInput {
    pub note: Option<String>,
}

/// Bookmark DTO for list responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkDto {
    pub target_type: String,
    pub target_id: String,
    pub note: Option<String>,
    pub created_at: i64,
}

/// POST /forum/posts/{id}/flag
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagInput {
    pub reason: String,
    pub note: Option<String>,
    #[serde(default = "default_flag_post_type")]
    pub post_type: String,
}

fn default_flag_post_type() -> String {
    "thread".into()
}

/// PUT /api/v2/forum/subscriptions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionInput {
    pub target_type: String,
    pub target_id: String,
    pub level: String,
}

/// Subscription DTO for list responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDto {
    pub target_type: String,
    pub target_id: String,
    pub level: String,
    pub created_at: i64,
}

/// Mod action DTO for the admin log list.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModActionDto {
    pub id: String,
    pub actor_id: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

/// GET/PUT /api/v2/me/notification-prefs — request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPrefsInput {
    pub prefs: serde_json::Value,
}

/// GET/PUT /api/v2/me/notification-prefs — response body.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPrefsDto {
    pub prefs: serde_json::Value,
}

/// PATCH /forum/threads/{id}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUpdateInput {
    pub title: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// PATCH /forum/comments/{id}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentUpdateInput {
    pub body: String,
}

/// Revision history entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDto {
    pub id: String,
    pub seq: i32,
    pub editor_id: String,
    pub old_title: Option<String>,
    pub old_body: String,
    pub created_at: i64,
}

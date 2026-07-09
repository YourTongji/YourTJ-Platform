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
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub position: i32,
    pub is_locked: bool,
    pub min_trust_to_post: i16,
    pub is_qa: bool,
    pub thread_count: i32,
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
    pub tags: Vec<String>,
    pub created_at: i64,
    pub last_activity_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread_count: Option<i32>,
}

/// Full thread detail matching OpenAPI `ThreadDetail`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDetailDto {
    pub id: String,
    pub board_id: String,
    pub author_handle: String,
    pub author_id: String,
    pub title: String,
    pub body: Option<String>,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub tags: Vec<String>,
    pub status: String,
    pub pinned_at: Option<i64>,
    pub pinned_globally: bool,
    pub featured_at: Option<i64>,
    pub closed_at: Option<i64>,
    pub archived_at: Option<i64>,
    pub deleted_at: Option<i64>,
    pub edited_at: Option<i64>,
    pub hidden_at: Option<i64>,
    pub created_at: i64,
    pub last_activity_at: i64,
    pub solved_answer_id: Option<String>,
    pub my_last_read_comment_id: Option<String>,
    pub my_subscription_level: Option<String>,
    pub poll: Option<PollDto>,
}

/// POST /forum/threads
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInput {
    pub board_id: String,
    pub title: String,
    pub body: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub poll: Option<PollInput>,
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
    pub author_id: String,
    pub body: String,
    pub vote_count: i32,
    pub is_deleted: bool,
    pub is_hidden: bool,
    pub edited_at: Option<i64>,
    pub created_at: i64,
    pub quoted_comment_id: Option<String>,
    pub is_solved: bool,
}

/// POST /forum/threads/{thread_id}/comments
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
    pub parent_id: Option<String>,
    pub body: String,
    pub quoted_comment_id: Option<String>,
}

/// POST /forum/posts/{post_id}/vote
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteInput {
    pub value: String, // "up" or "down"
    /// Optional per the API contract (which only requires `value`). When
    /// omitted the handler infers whether the id is a thread or a comment.
    #[serde(default)]
    pub post_type: Option<String>, // "thread" or "comment"
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
#[allow(dead_code)]
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

/// PUT /api/v2/me/drafts — request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftInput {
    pub draft_key: String,
    pub payload: serde_json::Value,
}

/// Draft DTO for list responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftDto {
    pub draft_key: String,
    pub payload: serde_json::Value,
    pub updated_at: i64,
}

/// Draft DTO for single-get responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPayloadDto {
    pub payload: serde_json::Value,
}

/// PATCH /forum/threads/{id}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUpdateInput {
    pub title: Option<String>,
    pub body: Option<String>,
    #[allow(dead_code)]
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

// ---------------------------------------------------------------------------
// Polls
// ---------------------------------------------------------------------------

/// A poll option in responses.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollOptionDto {
    pub id: String,
    pub label: String,
    pub vote_count: i32,
    pub position: i32,
}

/// Poll DTO returned with thread detail or results.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollDto {
    pub id: String,
    pub question: String,
    pub multi_select: bool,
    pub closes_at: Option<i64>,
    pub options: Vec<PollOptionDto>,
    pub my_votes: Vec<String>,
}

/// POST /api/v2/forum/polls/{id}/vote
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollVoteInput {
    pub option_id: String,
}

/// Optional poll data included in thread creation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollInput {
    pub question: String,
    #[serde(default)]
    pub multi_select: bool,
    pub closes_at: Option<i64>,
    pub options: Vec<String>,
}

// ---------------------------------------------------------------------------
// DMs (1:1 private messages)
// ---------------------------------------------------------------------------

/// POST /api/v2/forum/dm/conversations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmConversationInput {
    pub recipient_id: String,
}

/// Response from creating/getting a DM conversation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmConversationCreatedDto {
    pub id: String,
}

/// A DM conversation in the list response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmConversationDto {
    pub id: String,
    pub participant_handle: String,
    pub participant_id: String,
    pub last_message_at: i64,
}

/// POST /api/v2/forum/dm/conversations/{id}/messages
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageInput {
    pub body: String,
}

/// A single DM message.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageDto {
    pub id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_handle: String,
    pub body: String,
    pub created_at: i64,
}

//! Request and response types for the forum domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// Canonical source format for persisted community content.
#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentFormat {
    #[default]
    PlainV1,
    MarkdownV1,
}

impl ContentFormat {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PlainV1 => "plain_v1",
            Self::MarkdownV1 => "markdown_v1",
        }
    }

    pub(crate) fn from_db(value: &str) -> Self {
        match value {
            "markdown_v1" => Self::MarkdownV1,
            _ => Self::PlainV1,
        }
    }
}

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
    pub can_post: bool,
    pub posting_restriction: Option<String>,
}

/// Summary view of a thread (list responses).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDto {
    pub id: String,
    pub board_id: String,
    pub author_handle: String,
    pub title: String,
    pub body_excerpt: Option<String>,
    #[serde(default = "legacy_content_version")]
    pub content_version: i64,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub status: String,
    pub tags: Vec<String>,
    pub attachments: Vec<media::attachments::ForumAttachment>,
    pub created_at: i64,
    pub last_activity_at: i64,
    pub viewer_vote: Option<String>,
    pub is_bookmarked: bool,
    #[serde(default)]
    pub can_edit: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub can_moderate: bool,
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
    pub content_format: ContentFormat,
    #[serde(default = "legacy_content_version")]
    pub content_version: i64,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub tags: Vec<String>,
    pub attachments: Vec<media::attachments::ForumAttachment>,
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
    pub viewer_vote: Option<String>,
    pub is_bookmarked: bool,
    pub my_last_read_comment_id: Option<String>,
    pub my_subscription_level: Option<String>,
    pub poll: Option<PollDto>,
    #[serde(default)]
    pub can_edit: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub can_moderate: bool,
}

/// POST /forum/threads
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInput {
    pub board_id: String,
    pub title: String,
    pub body: Option<String>,
    #[serde(default)]
    pub content_format: ContentFormat,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub attachment_asset_ids: Vec<String>,
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
    pub content_format: ContentFormat,
    pub content_version: i64,
    pub attachments: Vec<media::attachments::ForumAttachment>,
    pub vote_count: i32,
    pub viewer_vote: Option<String>,
    pub is_bookmarked: bool,
    pub is_deleted: bool,
    pub is_hidden: bool,
    pub edited_at: Option<i64>,
    pub created_at: i64,
    pub quoted_comment_id: Option<String>,
    pub is_solved: bool,
    pub can_edit: bool,
    pub can_delete: bool,
    pub can_moderate: bool,
}

/// POST /forum/threads/{thread_id}/comments
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
    pub parent_id: Option<String>,
    pub body: String,
    #[serde(default)]
    pub content_format: ContentFormat,
    #[serde(default)]
    pub attachment_asset_ids: Vec<String>,
    pub quoted_comment_id: Option<String>,
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

/// Bookmark input — used when (un)setting a bookmark.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkInput {
    pub post_type: String,
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
    pub post_type: String,
}

/// PUT /api/v2/forum/subscriptions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionInput {
    pub target_type: String,
    pub target_id: String,
    pub level: String,
}

/// DELETE /api/v2/forum/subscriptions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeInput {
    pub target_type: String,
    pub target_id: String,
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

/// User-controlled in-app interaction categories.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InAppNotificationPreferences {
    pub replies: bool,
    pub mentions: bool,
    pub quotes: bool,
    pub votes: bool,
    pub badges: bool,
    pub subscriptions: bool,
    pub direct_messages: bool,
}

impl Default for InAppNotificationPreferences {
    fn default() -> Self {
        Self {
            replies: true,
            mentions: true,
            quotes: true,
            votes: true,
            badges: true,
            subscriptions: true,
            direct_messages: true,
        }
    }
}

/// User-controlled email notification channels; security mail is not optional here.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmailNotificationPreferences {
    pub weekly_digest: bool,
}

/// Stable event-by-channel notification preference contract.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotificationPreferences {
    pub in_app: InAppNotificationPreferences,
    pub email: EmailNotificationPreferences,
}

/// GET/PUT /api/v2/me/notification-prefs — request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotificationPrefsInput {
    pub prefs: NotificationPreferences,
}

/// GET/PUT /api/v2/me/notification-prefs — response body.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPrefsDto {
    pub prefs: NotificationPreferences,
}

/// PUT /api/v2/me/drafts — request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftInput {
    pub draft_key: String,
    #[serde(default)]
    pub expected_version: i64,
    pub payload: DraftPayload,
}

/// A bounded, typed forum draft payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DraftPayload {
    /// An unpublished forum thread.
    Thread {
        board_id: Option<String>,
        title: String,
        body: String,
        #[serde(default)]
        content_format: ContentFormat,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        poll_question: String,
        #[serde(default)]
        poll_options: Vec<String>,
        #[serde(default)]
        attachment_asset_ids: Vec<String>,
    },
    /// An unpublished reply to one thread.
    Comment {
        thread_id: String,
        body: String,
        #[serde(default)]
        content_format: ContentFormat,
        parent_id: Option<String>,
        #[serde(default)]
        attachment_asset_ids: Vec<String>,
    },
}

/// Draft DTO for list responses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftDto {
    pub draft_key: String,
    pub payload: DraftPayload,
    pub version: i64,
    pub updated_at: i64,
}

/// PATCH /forum/threads/{id}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUpdateInput {
    #[serde(default = "legacy_content_version")]
    pub expected_version: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub content_format: Option<ContentFormat>,
    #[serde(default)]
    pub attachment_asset_ids: Vec<String>,
    #[allow(dead_code)]
    pub tags: Option<Vec<String>>,
}

/// PATCH /forum/comments/{id}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentUpdateInput {
    #[serde(default = "legacy_content_version")]
    pub expected_version: i64,
    pub body: String,
    #[serde(default)]
    pub content_format: ContentFormat,
    #[serde(default)]
    pub attachment_asset_ids: Vec<String>,
}

const fn legacy_content_version() -> i64 {
    1
}

fn default_revision_limit() -> i64 {
    20
}

/// Bounded cursor page requested from a revision history endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionListQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_revision_limit")]
    pub limit: i64,
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
    pub old_content_format: ContentFormat,
    pub old_content_version: i64,
    pub attachments: Vec<media::attachments::ForumAttachment>,
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
    pub recipient_handle: String,
    pub request_message: Option<String>,
}

/// A DM conversation in the list response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmConversationDto {
    pub id: String,
    pub participant_id: String,
    pub participant_handle: String,
    pub participant_avatar_url: Option<String>,
    pub last_message_excerpt: Option<String>,
    pub last_message_at: i64,
    pub unread_count: i64,
    pub is_archived: bool,
    pub is_muted: bool,
    pub is_deleted: bool,
    pub request_status: String,
    pub request_direction: Option<String>,
    pub can_send: bool,
    pub created_at: i64,
}

/// Accepted-message unread and incoming-request counts for global navigation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmCountsDto {
    pub count: i64,
    pub unread_count: i64,
    pub request_count: i64,
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

/// POST /api/v2/forum/dm/conversations/{id}/read
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmReadInput {
    pub last_read_message_id: Option<String>,
}

/// POST /api/v2/forum/dm/messages/{id}/report
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageReportInput {
    pub reason: String,
    pub note: Option<String>,
}

/// POST /api/v2/admin/forum/dm/reports/{id}/resolve
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmReportResolveInput {
    pub action: String,
    pub note: Option<String>,
}

/// A reported DM message exposed only through the scoped moderation queue.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageReportDto {
    pub id: String,
    pub message_id: String,
    pub conversation_id: String,
    pub reporter_id: String,
    pub reporter_handle: String,
    pub sender_id: String,
    pub sender_handle: String,
    pub message_excerpt: String,
    pub reason: String,
    pub note: Option<String>,
    pub status: String,
    pub handled_by: Option<String>,
    pub handled_at: Option<i64>,
    pub created_at: i64,
}

/// GET /api/v2/users/{handle} — public community profile.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileDto {
    pub id: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub role: String,
    pub trust_level: i16,
    pub badges: Vec<UserBadgeDto>,
    pub verifications: Vec<platform::verifications::PublicVerificationDto>,
    pub thread_count: i32,
    pub comment_count: i32,
    pub votes_received: i32,
    pub follower_count: i32,
    pub following_count: i32,
    pub can_view_activity: bool,
    pub created_at: i64,
}

/// One active account shown in a followers or following page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummaryDto {
    pub id: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub followed_at: i64,
}

/// Current account's complete first-phase relationship with one target.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRelationshipDto {
    pub is_self: bool,
    pub following: bool,
    pub followed_by: bool,
    pub muted: bool,
    pub blocked_by_me: bool,
    pub blocked_me: bool,
    pub can_follow: bool,
    pub can_start_conversation: bool,
    pub can_mention: bool,
}

/// A badge displayed on a public community profile.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeDto {
    pub slug: String,
    pub name: String,
}

/// GET /api/v2/users/{handle}/threads item.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserThreadDto {
    pub id: String,
    pub title: String,
    pub board_slug: String,
    pub reply_count: i32,
    pub vote_count: i32,
    pub created_at: i64,
}

/// GET /api/v2/users/{handle}/comments item.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCommentDto {
    pub id: String,
    pub thread_id: String,
    pub thread_title: String,
    pub body: String,
    pub content_format: ContentFormat,
    pub created_at: i64,
}

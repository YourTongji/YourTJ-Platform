//! Request and response types for the media domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// A resumable intended profile slot for an upload.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaUsage {
    ProfileAvatar,
    ProfileBanner,
    ForumThread,
    ForumComment,
}

impl MediaUsage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProfileAvatar => "profile_avatar",
            Self::ProfileBanner => "profile_banner",
            Self::ForumThread => "forum_thread",
            Self::ForumComment => "forum_comment",
        }
    }
}

/// Upload intent request for direct OSS upload.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadIntentInput {
    pub kind: String,
    pub content_type: String,
    pub usage: Option<MediaUsage>,
}

/// STS credentials and callback fields returned for direct OSS upload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadCredentialsDto {
    pub upload_intent_id: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub security_token: String,
    pub region: String,
    pub bucket: String,
    pub prefix: String,
    pub oss_key: String,
    pub callback_url: String,
    pub callback_body: String,
    pub expiration: i64,
}

/// Body received from the OSS callback after a successful upload.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadCallbackInput {
    pub upload_intent_id: String,
    pub callback_token: String,
    pub oss_key: String,
    pub bytes: i64,
    pub mime: String,
    pub sha256: String,
}

/// Public upload DTO returned to clients.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadDto {
    pub id: String,
    pub account_id: String,
    pub kind: String,
    pub bytes: i64,
    pub mime: String,
    pub status: String,
    pub usage: Option<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub approval_requirement: String,
    pub deletion_state: Option<String>,
    pub created_at: i64,
}

/// Owner-safe upload state without storage identifiers or object URLs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyUploadDto {
    pub id: String,
    pub kind: String,
    pub usage: Option<String>,
    pub bytes: i64,
    pub mime: String,
    pub status: String,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub created_at: i64,
}

/// Signed / CDN URL for an upload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadUrlDto {
    pub url: String,
}

/// Short-lived one-time credential for a same-origin moderation preview stream.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerationPreviewGrantDto {
    pub token: String,
    pub expires_at: i64,
}

/// Owned clean image to bind to one controlled profile slot.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAssetInput {
    pub asset_id: String,
}

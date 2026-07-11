//! Request and response types for the media domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Deserializer, Serialize};

/// Explicit compare-and-swap expectation for a retention hold mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedHoldId {
    None,
    Exact(String),
}

impl ExpectedHoldId {
    pub fn as_deref(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Exact(id) => Some(id),
        }
    }
}

fn deserialize_expected_hold_id<'de, D>(deserializer: D) -> Result<ExpectedHoldId, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<String>::deserialize(deserializer)? {
        Some(id) => ExpectedHoldId::Exact(id),
        None => ExpectedHoldId::None,
    })
}

/// Purpose category for a time-bounded asset retention hold.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionHoldKind {
    Moderation,
    Security,
}

impl RetentionHoldKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Moderation => "moderation",
            Self::Security => "security",
        }
    }
}

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
    pub retention_held: bool,
    pub retention_state: String,
    pub retention_expires_at: Option<i64>,
    pub created_at: i64,
}

/// Place one purpose-bound, time-bounded hold on a media object.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetentionHoldInput {
    pub hold_kind: RetentionHoldKind,
    pub expires_at: i64,
    pub reason: String,
    #[serde(deserialize_with = "deserialize_expected_hold_id")]
    pub expected_hold_id: ExpectedHoldId,
}

/// Release exactly the operations hold the caller reviewed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseRetentionHoldInput {
    pub expected_hold_id: String,
    pub reason: String,
}

/// Operations-only retention record without provider object identifiers.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionHoldDto {
    pub id: String,
    pub upload_id: String,
    pub account_id: String,
    pub upload_status: String,
    pub hold_kind: String,
    pub reason: String,
    pub placed_by: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub is_expired: bool,
}

/// Operations-only durable object-deletion job without provider object identifiers.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionJobDto {
    pub id: String,
    pub upload_id: String,
    pub account_id: String,
    pub upload_status: String,
    pub request_source: String,
    pub reason: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_error_code: Option<String>,
    pub available_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ExpectedHoldId, RetentionHoldInput};

    #[test]
    fn retention_hold_input_requires_an_explicit_cas_expectation() {
        let missing = serde_json::from_value::<RetentionHoldInput>(json!({
            "holdKind": "security",
            "expiresAt": 1,
            "reason": "bounded review",
        }));
        assert!(missing.is_err());

        let create = serde_json::from_value::<RetentionHoldInput>(json!({
            "holdKind": "security",
            "expiresAt": 1,
            "reason": "bounded review",
            "expectedHoldId": null,
        }))
        .expect("explicit create-if-none expectation");
        assert_eq!(create.expected_hold_id, ExpectedHoldId::None);

        let update = serde_json::from_value::<RetentionHoldInput>(json!({
            "holdKind": "security",
            "expiresAt": 1,
            "reason": "bounded review",
            "expectedHoldId": "42",
        }))
        .expect("explicit exact-hold expectation");
        assert_eq!(update.expected_hold_id, ExpectedHoldId::Exact("42".into()));

        let unknown = serde_json::from_value::<RetentionHoldInput>(json!({
            "holdKind": "security",
            "expiresAt": 1,
            "reason": "bounded review",
            "expectedHoldId": null,
            "unexpected": true,
        }));
        assert!(unknown.is_err());
    }
}

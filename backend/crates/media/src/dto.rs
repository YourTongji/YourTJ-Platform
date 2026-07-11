//! Request and response types for the media domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// Upload intent request for direct OSS upload.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadIntentInput {
    pub kind: String,
    pub content_type: String,
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
    pub oss_key: String,
    pub url: String,
    pub bytes: i64,
    pub mime: String,
    pub sha256: String,
    pub status: String,
    pub created_at: i64,
}

/// Signed / CDN URL for an upload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadUrlDto {
    pub url: String,
}

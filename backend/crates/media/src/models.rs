//! Database row types mapped from the media schema via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// A row from `media.uploads`.
#[derive(Debug, Clone, FromRow)]
pub struct UploadRow {
    pub id: i64,
    pub account_id: i64,
    pub kind: String,
    pub oss_key: String,
    pub bytes: i64,
    pub mime: String,
    pub status: String,
    pub usage: Option<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// An upload projected for one authorized moderator without storage identifiers.
#[derive(Debug, Clone, FromRow)]
pub struct ModerationUploadRow {
    pub id: i64,
    pub account_id: i64,
    pub kind: String,
    pub bytes: i64,
    pub mime: String,
    pub status: String,
    pub usage: Option<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub has_reviewer_evidence: bool,
    pub deletion_state: Option<String>,
    pub retention_held: bool,
    pub retention_state: String,
    pub retention_expires_at: Option<DateTime<Utc>>,
}

/// A derived asset variant row from `media.asset_variants`.
#[allow(dead_code)] // reason: phase 2 adds the model before any caller wires it into the public API.
#[derive(Debug, Clone, FromRow)]
pub struct AssetVariantRow {
    pub id: i64,
    pub asset_id: i64,
    pub variant: String,
    pub object_key: String,
    pub content_hash: String,
    pub mime: String,
    pub bytes: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub status: String,
    pub processing_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub quarantined_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

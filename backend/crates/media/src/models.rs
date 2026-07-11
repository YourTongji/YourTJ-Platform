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

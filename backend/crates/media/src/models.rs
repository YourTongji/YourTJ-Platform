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
    pub url: String,
    pub bytes: i64,
    pub mime: String,
    pub sha256: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

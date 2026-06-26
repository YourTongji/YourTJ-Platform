//! Database row types mapped from the reviews schema via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// A row from `reviews.reviews`.
#[derive(Debug, Clone, FromRow)]
pub struct ReviewRow {
    pub id: i64,
    pub course_id: i64,
    pub account_id: i64,
    pub rating: i32,
    pub comment: Option<String>,
    pub score: Option<String>,
    pub semester: Option<String>,
    pub approve_count: i32,
    pub disapprove_count: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A row from `reviews.review_likes`.
#[derive(Debug, Clone, FromRow)]
pub struct ReviewLikeRow {
    pub review_id: i64,
    pub account_id: i64,
}

/// A row from `reviews.review_reports`.
#[derive(Debug, Clone, FromRow)]
pub struct ReviewReportRow {
    pub id: i64,
    pub review_id: i64,
    pub reporter_account_id: i64,
    pub reason: String,
    pub status: String,
    pub admin_note: Option<String>,
    pub created_at: DateTime<Utc>,
}

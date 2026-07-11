//! Request and response types for the reviews domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// A review returned in list / detail endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDto {
    pub id: String,
    pub course_id: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub score: Option<String>,
    pub semester: Option<String>,
    pub author_handle: String,
    pub author_avatar: Option<String>,
    pub approve_count: i32,
    pub status: String,
    pub created_at: i64,
}

/// POST /courses/{id}/reviews and PATCH /reviews/{id}.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    /// Must be between 0 and 5 inclusive.
    pub rating: i32,
    pub comment: Option<String>,
    pub semester: Option<String>,
    pub score: Option<String>,
    pub captcha_token: Option<String>,
}

/// A review report returned in admin endpoints.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDto {
    pub id: String,
    pub review_id: String,
    pub reason: String,
    pub status: String,
    pub course_id: Option<String>,
    pub review_author_handle: Option<String>,
    pub review_rating: Option<i32>,
    pub review_status: Option<String>,
    pub review_excerpt: Option<String>,
    pub created_at: i64,
}

/// POST /reviews/{id}/report.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportInput {
    pub reason: String,
    pub captcha_token: String,
}

/// Query params for GET /courses/{id}/reviews.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReviewsQuery {
    pub sort: Option<String>,
    pub cursor: Option<i64>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

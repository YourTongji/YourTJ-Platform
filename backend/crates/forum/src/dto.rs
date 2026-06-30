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
    pub created_at: i64,
    pub last_activity_at: i64,
}

/// Detail view of a thread — ThreadDto + optional body.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDetailDto {
    #[serde(flatten)]
    pub base: ThreadDto,
    pub body: Option<String>,
}

/// POST /forum/threads
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInput {
    pub board_id: String,
    pub title: String,
    pub body: Option<String>,
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
    pub body: String,
    pub vote_count: i32,
    pub created_at: i64,
}

/// POST /forum/threads/{thread_id}/comments
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentInput {
    pub parent_id: Option<String>,
    pub body: String,
}

/// POST /forum/posts/{post_id}/vote
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteInput {
    pub value: String,     // "up" or "down"
    pub post_type: String, // "thread" or "comment"
}

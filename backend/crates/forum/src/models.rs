//! Database row types mapped from the forum schema via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// A row from `forum.boards`.
#[derive(Debug, Clone, FromRow)]
pub struct BoardRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
}

/// A row from `forum.threads`.
#[derive(Debug, Clone, FromRow)]
pub struct ThreadRow {
    pub id: i64,
    pub board_id: i64,
    pub author_id: i64,
    pub title: String,
    pub body: Option<String>,
    pub reply_count: i32,
    pub vote_count: i32,
    pub hot_score: Option<f64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

/// A row from `forum.comments`.
#[derive(Debug, Clone, FromRow)]
pub struct CommentRow {
    pub id: i64,
    pub thread_id: i64,
    pub parent_id: Option<i64>,
    pub path: Option<String>,
    pub author_id: i64,
    pub body: String,
    pub vote_count: i32,
    pub created_at: DateTime<Utc>,
}

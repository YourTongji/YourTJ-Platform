use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ActiveEventRow {
    pub id: i64,
    pub account_id: i64,
    pub kind: String,
    pub generation: i32,
    pub activity_date: NaiveDate,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ActivityDayRow {
    pub activity_date: NaiveDate,
    pub threads_created: i32,
    pub comments_created: i32,
    pub likes_given: i32,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ScorePolicyRow {
    pub version: i64,
    pub thread_weight: i32,
    pub comment_weight: i32,
    pub like_weight: i32,
    pub reason: String,
    pub changed_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

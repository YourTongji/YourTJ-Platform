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

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct TrustLevelPolicyRow {
    pub version: i64,
    pub score_policy_version: i64,
    pub threshold_level_2: i32,
    pub threshold_level_3: i32,
    pub threshold_level_4: i32,
    pub threshold_level_5: i32,
    pub threshold_level_6: i32,
    pub like_daily_cap: i32,
    pub reason: String,
    pub changed_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)] // reason: sqlx maps the full progress row; only a subset is read in evaluation paths
pub(crate) struct TrustProgressRow {
    pub account_id: i64,
    pub trust_level: i16,
    pub qualifying_score: i64,
    pub policy_version: i64,
    pub override_level: Option<i16>,
    pub override_reason: Option<String>,
    pub override_by: Option<i64>,
    pub override_at: Option<DateTime<Utc>>,
    pub last_evaluated_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct TrustLevelEventRow {
    pub id: i64,
    pub account_id: i64,
    pub event_kind: String,
    pub from_level: i16,
    pub to_level: i16,
    pub qualifying_score: i64,
    pub policy_version: i64,
    pub actor_kind: String,
    pub actor_account_id: Option<i64>,
    pub reason: Option<String>,
    pub governance_event_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

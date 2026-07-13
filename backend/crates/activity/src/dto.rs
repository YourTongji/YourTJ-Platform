use serde::{Deserialize, Serialize};

/// Weights applied to the public activity dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityWeightsDto {
    pub thread: i32,
    pub comment: i32,
    pub like: i32,
    pub check_in: i32,
}

/// One Asia/Shanghai calendar day in an activity heatmap.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDayDto {
    pub date: String,
    pub threads: i32,
    pub comments: i32,
    pub likes: i32,
    pub check_ins: i32,
    pub score: i64,
}

/// Daily check-in state for the authenticated account.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckInStatusDto {
    pub timezone: &'static str,
    pub date: String,
    pub checked_in: bool,
    pub newly_checked_in: bool,
    pub checked_in_at: Option<i64>,
    pub current_streak: i64,
    pub total_days: i64,
    pub next_reset_at: i64,
}

/// Continuous activity calendar returned to the authenticated user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCalendarDto {
    pub timezone: &'static str,
    pub from: String,
    pub to: String,
    pub policy_version: i64,
    pub trust_policy_version: i64,
    pub weights: ActivityWeightsDto,
    pub like_daily_cap: i32,
    pub days: Vec<ActivityDayDto>,
}

/// A versioned activity scoring policy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPolicyDto {
    pub version: i64,
    pub timezone: &'static str,
    pub weights: ActivityWeightsDto,
    pub reason: String,
    pub changed_by: String,
    pub created_at: i64,
}

/// Publish a new policy after checking the caller's observed version.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPolicyUpdateInput {
    pub expected_version: i64,
    pub weights: ActivityWeightsDto,
    pub reason: String,
}

/// Authenticated trust progress for the growth card and admin surfaces.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustProgressDto {
    pub trust_level: i16,
    pub tea_name: String,
    pub qualifying_score: i64,
    pub next_level: Option<i16>,
    pub next_threshold: Option<i32>,
    pub remaining_score: Option<i64>,
    pub progress_percent: i32,
    pub policy_version: i64,
    pub is_max_level: bool,
    pub override_active: bool,
    pub promotion_blocked_until: Option<i64>,
    pub promotion_requires_new_activity: bool,
}

/// Versioned trust threshold policy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustLevelPolicyDto {
    pub version: i64,
    pub score_policy_version: i64,
    pub threshold_level_2: i32,
    pub threshold_level_3: i32,
    pub threshold_level_4: i32,
    pub threshold_level_5: i32,
    pub threshold_level_6: i32,
    pub like_daily_cap: i32,
    pub demotion_cooldown_days: i32,
    pub reason: String,
    pub changed_by: String,
    pub created_at: i64,
}

/// Publish a new trust threshold policy revision.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustLevelPolicyUpdateInput {
    pub expected_version: i64,
    pub threshold_level_2: i32,
    pub threshold_level_3: i32,
    pub threshold_level_4: i32,
    pub threshold_level_5: i32,
    pub threshold_level_6: i32,
    pub like_daily_cap: i32,
    pub demotion_cooldown_days: i32,
    pub reason: String,
}

/// Staff manual trust adjustment.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustLevelAdjustInput {
    pub trust_level: Option<i16>,
    #[serde(default)]
    pub clear_override: bool,
    pub reason: String,
}

/// One append-only trust transition event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustLevelEventDto {
    pub id: String,
    pub account_id: String,
    pub event_kind: String,
    pub from_level: i16,
    pub to_level: i16,
    pub qualifying_score: i64,
    pub policy_version: i64,
    pub actor_kind: String,
    pub actor_account_id: Option<String>,
    pub reason: Option<String>,
    pub governance_event_id: Option<String>,
    pub created_at: i64,
}

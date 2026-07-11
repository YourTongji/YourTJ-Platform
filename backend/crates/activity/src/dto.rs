use serde::{Deserialize, Serialize};

/// Weights applied to the three public activity dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityWeightsDto {
    pub thread: i32,
    pub comment: i32,
    pub like: i32,
}

/// One Asia/Shanghai calendar day in an activity heatmap.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDayDto {
    pub date: String,
    pub threads: i32,
    pub comments: i32,
    pub likes: i32,
    pub score: i64,
}

/// Continuous activity calendar returned to the authenticated user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityCalendarDto {
    pub timezone: &'static str,
    pub from: String,
    pub to: String,
    pub policy_version: i64,
    pub weights: ActivityWeightsDto,
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

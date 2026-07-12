use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{Duration, NaiveDate};
use serde::Deserialize;
use shared::{AppError, AppResult, AppState, Page};

use crate::dto::{
    ActivityCalendarDto, ActivityPolicyDto, ActivityPolicyUpdateInput, TrustLevelAdjustInput,
    TrustLevelEventDto, TrustLevelPolicyDto, TrustLevelPolicyUpdateInput, TrustProgressDto,
};
use crate::{repo, trust};

const DEFAULT_ACTIVITY_DAYS: i64 = 365;
const MAX_ACTIVITY_DAYS: i64 = 371;

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PolicyHistoryQuery {
    cursor: Option<String>,
    #[serde(default = "default_history_limit")]
    limit: i64,
}

fn default_history_limit() -> i64 {
    20
}

pub(crate) async fn get_my_activity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ActivityQuery>,
) -> AppResult<Json<ActivityCalendarDto>> {
    let auth = authenticate(&state, &headers).await?;
    let today = repo::current_activity_date(&state.db).await?;
    let (from, to) = resolve_range(&query, today)?;
    let calendar = repo::activity_calendar(&state.db, auth.id, from, to).await?;
    Ok(Json(calendar))
}

pub(crate) async fn get_activity_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ActivityPolicyDto>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    Ok(Json(repo::current_policy(&state.db).await?))
}

pub(crate) async fn update_activity_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ActivityPolicyUpdateInput>,
) -> AppResult<Json<ActivityPolicyDto>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    validate_policy_input(&input)?;
    let reason = input.reason.trim();
    let policy = repo::append_policy(
        &state.db,
        input.expected_version,
        &input.weights,
        reason,
        auth.id,
        &auth.role,
    )
    .await?;
    Ok(Json(policy))
}

pub(crate) async fn get_activity_policy_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PolicyHistoryQuery>,
) -> AppResult<Json<Page<ActivityPolicyDto>>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| {
            value.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into()))
        })
        .transpose()?;
    Ok(Json(repo::policy_history(&state.db, cursor, query.limit).await?))
}

pub(crate) async fn get_my_trust_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<TrustProgressDto>> {
    let auth = authenticate(&state, &headers).await?;
    Ok(Json(trust::trust_progress(&state.db, auth.id).await?))
}

pub(crate) async fn get_trust_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<TrustLevelPolicyDto>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    Ok(Json(trust::current_policy(&state.db).await?))
}

pub(crate) async fn update_trust_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<TrustLevelPolicyUpdateInput>,
) -> AppResult<Json<TrustLevelPolicyDto>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    Ok(Json(trust::append_policy(&state.db, &input, auth.id, &auth.role).await?))
}

pub(crate) async fn get_trust_policy_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PolicyHistoryQuery>,
) -> AppResult<Json<Page<TrustLevelPolicyDto>>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| {
            value.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into()))
        })
        .transpose()?;
    Ok(Json(trust::policy_history(&state.db, cursor, query.limit).await?))
}

pub(crate) async fn adjust_user_trust_level(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<TrustLevelAdjustInput>,
) -> AppResult<Json<TrustProgressDto>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    let account_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    Ok(Json(trust::adjust_trust_level(&state.db, account_id, &input, auth.id, &auth.role).await?))
}

pub(crate) async fn get_user_trust_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<PolicyHistoryQuery>,
) -> AppResult<Json<Page<TrustLevelEventDto>>> {
    let auth = authenticate(&state, &headers).await?;
    auth.require_capability(shared::auth::Capability::ManageActivity)
        .map_err(|_| AppError::Forbidden)?;
    let account_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| {
            value.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into()))
        })
        .transpose()?;
    Ok(Json(trust::event_history(&state.db, account_id, cursor, query.limit).await?))
}

async fn authenticate(state: &AppState, headers: &HeaderMap) -> AppResult<shared::AuthAccount> {
    identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

fn resolve_range(query: &ActivityQuery, today: NaiveDate) -> AppResult<(NaiveDate, NaiveDate)> {
    let parsed_from = query.from.as_deref().map(parse_date).transpose()?;
    let parsed_to = query.to.as_deref().map(parse_date).transpose()?;
    let to = parsed_to.unwrap_or(today);
    let from = parsed_from.unwrap_or_else(|| to - Duration::days(DEFAULT_ACTIVITY_DAYS - 1));
    let inclusive_days = (to - from).num_days() + 1;
    if inclusive_days <= 0 {
        return Err(AppError::BadRequest("from must not be after to".into()));
    }
    if inclusive_days > MAX_ACTIVITY_DAYS {
        return Err(AppError::BadRequest(format!(
            "activity range must not exceed {MAX_ACTIVITY_DAYS} days"
        )));
    }
    Ok((from, to))
}

fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("dates must use YYYY-MM-DD".into()))
}

fn validate_policy_input(input: &ActivityPolicyUpdateInput) -> AppResult<()> {
    let weights = [&input.weights.thread, &input.weights.comment, &input.weights.like];
    if weights.into_iter().any(|weight| !(0..=1000).contains(weight)) {
        return Err(AppError::BadRequest("activity weights must be between 0 and 1000".into()));
    }
    let reason = input.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must contain 3 to 500 characters".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::dto::{ActivityPolicyUpdateInput, ActivityWeightsDto};

    use super::{resolve_range, validate_policy_input, ActivityQuery, MAX_ACTIVITY_DAYS};

    #[test]
    fn defaults_to_365_continuous_days() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 11).expect("valid date");
        let query = ActivityQuery { from: None, to: None };
        let (from, to) = resolve_range(&query, today).expect("valid default range");
        assert_eq!((to - from).num_days() + 1, 365);
    }

    #[test]
    fn rejects_ranges_longer_than_contract_limit() {
        let query =
            ActivityQuery { from: Some("2025-01-01".into()), to: Some("2026-01-07".into()) };
        let today = NaiveDate::from_ymd_opt(2026, 1, 7).expect("valid date");
        let error = resolve_range(&query, today).expect_err("range exceeds maximum");
        assert!(error.to_string().contains(&MAX_ACTIVITY_DAYS.to_string()));
    }

    #[test]
    fn rejects_policy_reason_shorter_than_contract_minimum() {
        let input = ActivityPolicyUpdateInput {
            expected_version: 1,
            weights: ActivityWeightsDto { thread: 10, comment: 3, like: 1 },
            reason: "x".into(),
        };
        assert!(validate_policy_input(&input).is_err());
    }
}

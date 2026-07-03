//! Subscription handlers.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsQuery {
    #[serde(rename = "type")]
    pub target_type: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// PUT /api/v2/forum/subscriptions
pub async fn set_subscription_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::SubscriptionInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "subscription_change",
        &auth.id.to_string(),
        30,
        60,
    )
    .await?;

    if !matches!(body.target_type.as_str(), "board" | "thread") {
        return Err(AppError::BadRequest("targetType must be 'board' or 'thread'".into()));
    }
    if !matches!(body.level.as_str(), "watching" | "tracking" | "muted") {
        return Err(AppError::BadRequest(
            "level must be 'watching', 'tracking', or 'muted'".into(),
        ));
    }

    let target_id: i64 =
        body.target_id.parse().map_err(|_| AppError::BadRequest("invalid targetId".into()))?;

    crate::repo::set_subscription(&state.db, auth.id, &body.target_type, target_id, &body.level)
        .await?;

    // Bump following cache
    shared::cache::bump_version_opt(state.redis.as_ref(), "following", &auth.id.to_string())
        .await?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// DELETE /api/v2/forum/subscriptions (body-based)
pub async fn delete_subscription_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let target_type = body
        .get("targetType")
        .and_then(|v| v.as_str())
        .ok_or(AppError::BadRequest("targetType required".into()))?;
    let target_id: i64 = body
        .get("targetId")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or(AppError::BadRequest("targetId required".into()))?;

    crate::repo::delete_subscription(&state.db, auth.id, target_type, target_id).await?;

    shared::cache::bump_version_opt(state.redis.as_ref(), "following", &auth.id.to_string())
        .await?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// GET /api/v2/forum/subscriptions
pub async fn list_subscriptions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SubscriptionsQuery>,
) -> AppResult<Json<Vec<crate::dto::SubscriptionDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let rows =
        crate::repo::list_subscriptions(&state.db, auth.id, q.target_type.as_deref()).await?;
    let items: Vec<crate::dto::SubscriptionDto> = rows
        .into_iter()
        .map(|r| crate::dto::SubscriptionDto {
            target_type: r.target_type,
            target_id: r.target_id.to_string(),
            level: r.level,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(items))
}

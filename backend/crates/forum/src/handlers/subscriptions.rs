//! Subscription handlers.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

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
) -> AppResult<StatusCode> {
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

    shared::cache::bump_version_opt(state.redis.as_ref(), "following", &auth.id.to_string())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/forum/subscriptions
pub async fn delete_subscription_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::UnsubscribeInput>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let target_id: i64 =
        body.target_id.parse().map_err(|_| AppError::BadRequest("invalid targetId".into()))?;

    crate::repo::delete_subscription(&state.db, auth.id, &body.target_type, target_id).await?;

    shared::cache::bump_version_opt(state.redis.as_ref(), "following", &auth.id.to_string())
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v2/forum/subscriptions
pub async fn list_subscriptions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SubscriptionsQuery>,
) -> AppResult<Json<Page<crate::dto::SubscriptionDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let (rows, next_cursor) = crate::repo::list_subscriptions_page(
        &state.db,
        auth.id,
        q.target_type.as_deref(),
        q.cursor.as_deref(),
        q.limit,
    )
    .await?;
    let items: Vec<crate::dto::SubscriptionDto> = rows
        .into_iter()
        .map(|r| crate::dto::SubscriptionDto {
            target_type: r.target_type,
            target_id: r.target_id.to_string(),
            level: r.level,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(Page::new(items, next_cursor)))
}

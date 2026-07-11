//! User-level handlers: notification prefs.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppError, AppResult, AppState};

/// GET /api/v2/me/notification-prefs
pub async fn get_my_notification_prefs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<crate::dto::NotificationPrefsDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let row = crate::repo::get_notification_prefs(&state.db, auth.id).await?;
    let prefs = serde_json::from_value(row.prefs).unwrap_or_default();
    Ok(Json(crate::dto::NotificationPrefsDto { prefs }))
}

/// PUT /api/v2/me/notification-prefs
pub async fn set_my_notification_prefs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::NotificationPrefsInput>,
) -> AppResult<Json<crate::dto::NotificationPrefsDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let stored = serde_json::to_value(&body.prefs)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    crate::repo::set_notification_prefs(&state.db, auth.id, &stored).await?;
    Ok(Json(crate::dto::NotificationPrefsDto { prefs: body.prefs }))
}

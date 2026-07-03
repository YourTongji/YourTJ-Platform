//! Ignore / block user handlers.
//!
//! Routes:
//! - `PUT /api/v2/me/ignores/{account_id}`   — ignore a user
//! - `DELETE /api/v2/me/ignores/{account_id}` — unignore a user
//! - `GET /api/v2/me/ignores`                 — list ignored account ids

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppError, AppResult, AppState};

/// PUT /api/v2/me/ignores/{account_id}
///
/// Ignore a user. Self-ignore is rejected. The target account must exist.
pub async fn ignore_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ignored_account_id_str): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let ignored_account_id: i64 = ignored_account_id_str
        .parse()
        .map_err(|_| AppError::BadRequest("invalid accountId".into()))?;

    if auth.id == ignored_account_id {
        return Err(AppError::BadRequest("cannot ignore yourself".into()));
    }

    // Verify the target account exists.
    let exists: bool = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE id = $1)",
    )
    .bind(ignored_account_id)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(AppError::NotFound);
    }

    crate::repo::insert_ignore(&state.db, auth.id, ignored_account_id).await?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// DELETE /api/v2/me/ignores/{account_id}
///
/// Unignore a user. Succeeds even if the relationship did not exist.
pub async fn unignore_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ignored_account_id_str): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let ignored_account_id: i64 = ignored_account_id_str
        .parse()
        .map_err(|_| AppError::BadRequest("invalid accountId".into()))?;

    crate::repo::delete_ignore(&state.db, auth.id, ignored_account_id).await?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// GET /api/v2/me/ignores
///
/// List all account ids this user has ignored.
pub async fn list_ignores_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let ids = crate::repo::list_ignored_ids(&state.db, auth.id).await?;
    let items: Vec<serde_json::Value> =
        ids.into_iter().map(|id| serde_json::json!({"ignoredAccountId": id.to_string()})).collect();

    Ok(Json(items))
}

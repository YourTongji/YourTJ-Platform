//! Ignore / block user handlers.
//!
//! Routes:
//! - `PUT /api/v2/me/ignores/{account_id}`   — ignore a user
//! - `DELETE /api/v2/me/ignores/{account_id}` — unignore a user
//! - `GET /api/v2/me/ignores`                 — list ignored account ids

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreUserDto {
    account_id: String,
    handle: String,
    avatar_url: Option<String>,
    created_at: i64,
}

/// PUT /api/v2/me/ignores/{account_id}
///
/// Ignore a user. Self-ignore is rejected. The target account must exist.
pub async fn ignore_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ignored_account_id_str): Path<String>,
) -> AppResult<StatusCode> {
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

    identity::public_accounts::find_public_account_by_id(&state.db, ignored_account_id)
        .await?
        .ok_or(AppError::NotFound)?;

    crate::repo::insert_ignore(&state.db, auth.id, ignored_account_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/me/ignores/{account_id}
///
/// Unignore a user. Succeeds even if the relationship did not exist.
pub async fn unignore_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ignored_account_id_str): Path<String>,
) -> AppResult<StatusCode> {
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

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v2/me/ignores
///
/// List all account ids this user has ignored.
pub async fn list_ignores_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IgnoreListQuery>,
) -> AppResult<Json<shared::Page<IgnoreUserDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let mut rows = crate::repo::list_ignored_users(&state.db, auth.id, cursor, limit).await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.account_id.to_string())).flatten();
    let account_ids: Vec<i64> = rows.iter().map(|row| row.account_id).collect();
    let accounts =
        identity::public_accounts::find_public_accounts_by_ids(&state.db, &account_ids).await?;
    let accounts: HashMap<i64, _> = accounts.into_iter().map(|row| (row.id, row)).collect();
    let asset_ids: Vec<i64> = accounts.values().filter_map(|row| row.avatar_asset_id).collect();
    let avatar_urls = media::resolve_clean_profile_images(&state.db, &asset_ids).await?;
    let items = rows
        .into_iter()
        .filter_map(|row| {
            let account = accounts.get(&row.account_id)?;
            Some(IgnoreUserDto {
                account_id: row.account_id.to_string(),
                handle: account.handle.clone(),
                avatar_url: account.avatar_asset_id.and_then(|id| avatar_urls.get(&id).cloned()),
                created_at: row.created_at.timestamp(),
            })
        })
        .collect();
    Ok(Json(shared::Page::new(items, next_cursor)))
}

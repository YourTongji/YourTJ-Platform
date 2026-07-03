//! Admin forum mod-action log endpoint: list moderator actions.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModActionQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// GET /api/v2/admin/forum/mod-actions — list moderator action log
pub async fn list_mod_actions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ModActionQuery>,
) -> AppResult<Json<Page<crate::dto::ModActionDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let cursor: Option<i64> = q.cursor.and_then(|c| c.parse().ok());
    let (rows, next_cursor) = crate::repo::list_mod_actions(&state.db, cursor, q.limit).await?;

    let items: Vec<crate::dto::ModActionDto> = rows
        .into_iter()
        .map(|r| crate::dto::ModActionDto {
            id: r.id.to_string(),
            actor_id: r.actor_id.to_string(),
            action: r.action,
            target_type: r.target_type,
            target_id: r.target_id.to_string(),
            reason: r.reason,
            metadata: r.metadata,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    let next_str = next_cursor.map(|c| c.to_string());
    Ok(Json(Page::new(items, next_str)))
}

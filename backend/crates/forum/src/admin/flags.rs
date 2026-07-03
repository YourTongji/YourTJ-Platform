//! Admin forum flag queue endpoints: list flags and resolve individual flags.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

// ---------------------------------------------------------------------------
// Input DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagsQueueQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFlagInput {
    pub action: String,
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/admin/forum/flags — list the flag queue
pub async fn list_flags_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FlagsQueueQuery>,
) -> AppResult<Json<Page<Value>>> {
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
    let (rows, next_cursor) =
        crate::repo::list_flag_queue(&state.db, q.status.as_deref(), cursor, q.limit).await?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id.to_string(),
                "targetType": r.target_type,
                "targetId": r.target_id.to_string(),
                "reporterId": r.reporter_id.to_string(),
                "reason": r.reason,
                "note": r.note,
                "weight": r.weight,
                "status": r.status,
                "createdAt": r.created_at.timestamp(),
            })
        })
        .collect();

    let next_str = next_cursor.map(|c| c.to_string());
    Ok(Json(Page::new(items, next_str)))
}

/// POST /api/v2/admin/forum/flags/{id}/resolve — resolve a single flag
pub async fn resolve_flag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(flag_id_str): Path<String>,
    Json(body): Json<ResolveFlagInput>,
) -> AppResult<Json<Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let flag_id: i64 = flag_id_str.parse().map_err(|_| AppError::NotFound)?;

    crate::repo::resolve_flag(&state.db, flag_id, &body.action, auth.id, body.note.as_deref())
        .await?;

    // Write mod action
    if let Err(e) = crate::repo::insert_mod_action(
        &state.db,
        auth.id,
        &format!("resolve_flag_{}", body.action),
        "flag",
        flag_id,
        body.note.as_deref(),
        None,
    )
    .await
    {
        tracing::warn!(error = %e, "failed to record mod action");
    }

    Ok(Json(json!({"ok": true})))
}

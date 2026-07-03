//! Admin forum thread action endpoints: pin/unpin, close/reopen, archive/restore,
//! hide/unhide, and move.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

/// POST /api/v2/admin/forum/threads/{id}/{action}
///
/// Actions: `pin`, `unpin`, `close`, `reopen`, `archive`, `restore`, `hide`, `unhide`, `move`
/// - `pin`   body: `{ globally: bool }`
/// - `move`  body: `{ boardId: string }`
/// - others  body: `{}` (optional)
pub async fn admin_thread_action(
    State(state): State<AppState>,
    Path((id_str, action)): Path<(String, String)>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
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

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let body_inner = body.map(|j| j.0).unwrap_or(json!({}));

    match action.as_str() {
        "pin" => {
            let globally = body_inner.get("globally").and_then(|v| v.as_bool()).unwrap_or(false);
            crate::repo::pin_thread(&state.db, id, globally).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "pin", "thread", id, None, None)
                .await?;
        }
        "unpin" => {
            crate::repo::unpin_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "unpin", "thread", id, None, None)
                .await?;
        }
        "close" => {
            crate::repo::close_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "close", "thread", id, None, None)
                .await?;
        }
        "reopen" => {
            crate::repo::reopen_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "reopen", "thread", id, None, None)
                .await?;
        }
        "archive" => {
            crate::repo::archive_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "archive", "thread", id, None, None)
                .await?;
        }
        "restore" => {
            crate::repo::restore_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "restore", "thread", id, None, None)
                .await?;
        }
        "hide" => {
            crate::repo::hide_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "hide", "thread", id, None, None)
                .await?;
        }
        "unhide" => {
            crate::repo::unhide_thread(&state.db, id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "unhide", "thread", id, None, None)
                .await?;
        }
        "move" => {
            let board_id: i64 = body_inner
                .get("boardId")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .ok_or(AppError::BadRequest("boardId required".into()))?;
            crate::repo::move_thread(&state.db, id, board_id).await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "move", "thread", id, None, None)
                .await?;
        }
        _ => return Err(AppError::BadRequest(format!("unknown action: {action}"))),
    }

    // Bump cache
    shared::cache::bump_version_silent(state.redis.as_ref(), "board", &id.to_string()).await;

    Ok(Json(json!({"ok": true})))
}

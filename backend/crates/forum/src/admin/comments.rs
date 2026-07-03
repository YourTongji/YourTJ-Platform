//! Admin forum comment action endpoints: soft-delete, restore, hide, unhide.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

/// POST /api/v2/admin/forum/comments/{id}/{action}
///
/// Actions: `delete` (mod soft-delete), `restore`, `hide`, `unhide`
pub async fn admin_comment_action(
    State(state): State<AppState>,
    Path((id_str, action)): Path<(String, String)>,
    headers: HeaderMap,
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

    match action.as_str() {
        "delete" => {
            sqlx::query(
                "UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 WHERE id = $2",
            )
            .bind(auth.id)
            .bind(id)
            .execute(&state.db)
            .await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "delete", "comment", id, None, None)
                .await?;
        }
        "restore" => {
            sqlx::query(
                "UPDATE forum.comments SET deleted_at = NULL, deleted_by = NULL WHERE id = $1",
            )
            .bind(id)
            .execute(&state.db)
            .await?;
            crate::repo::insert_mod_action(
                &state.db, auth.id, "restore", "comment", id, None, None,
            )
            .await?;
        }
        "hide" => {
            sqlx::query("UPDATE forum.comments SET hidden_at = now() WHERE id = $1")
                .bind(id)
                .execute(&state.db)
                .await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "hide", "comment", id, None, None)
                .await?;
        }
        "unhide" => {
            sqlx::query("UPDATE forum.comments SET hidden_at = NULL WHERE id = $1")
                .bind(id)
                .execute(&state.db)
                .await?;
            crate::repo::insert_mod_action(&state.db, auth.id, "unhide", "comment", id, None, None)
                .await?;
        }
        _ => return Err(AppError::BadRequest(format!("unknown action: {action}"))),
    }

    // Bump cache
    shared::cache::bump_version_silent(state.redis.as_ref(), "comment", &id.to_string()).await;

    Ok(Json(json!({"ok": true})))
}

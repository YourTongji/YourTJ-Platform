//! Draft handlers under /api/v2/me/drafts.
//!
//! Each account may keep up to 50 drafts. When saving a draft with a **new**
//! key, the handler checks the current count and rejects with `BadRequest`
//! if the limit would be exceeded.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use shared::{AppError, AppResult, AppState};

use crate::dto::{DraftDto, DraftInput, DraftPayloadDto};

/// Maximum number of drafts per account.
const MAX_DRAFTS: usize = 50;

/// PUT /api/v2/me/drafts
///
/// Saves (upserts) a draft. If the draft key is new and the account already
/// has MAX_DRAFTS drafts, the request is rejected.
pub async fn save_draft_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DraftInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    if body.draft_key.is_empty() {
        return Err(AppError::BadRequest("draftKey must not be empty".into()));
    }

    // Enforce the 50-draft limit only for *new* keys.
    let exists = crate::repo::draft_exists(&state.db, auth.id, &body.draft_key).await?;
    if !exists {
        let count = crate::repo::count_drafts(&state.db, auth.id).await?;
        if count >= MAX_DRAFTS as i64 {
            return Err(AppError::BadRequest(format!("draft limit of {MAX_DRAFTS} reached")));
        }
    }

    crate::repo::upsert_draft(&state.db, auth.id, &body.draft_key, &body.payload).await?;

    Ok(Json(json!({"ok": true})))
}

/// GET /api/v2/me/drafts
///
/// Lists all drafts for the authenticated account, ordered by `updated_at`
/// descending.
pub async fn list_drafts_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<DraftDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let rows = crate::repo::list_drafts(&state.db, auth.id).await?;

    let items: Vec<DraftDto> = rows
        .into_iter()
        .map(|r| DraftDto {
            draft_key: r.draft_key,
            payload: r.payload,
            updated_at: r.updated_at.timestamp(),
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/v2/me/drafts/{draft_key}
///
/// Returns the payload for a single draft. Returns 404 if the draft does
/// not exist.
pub async fn get_draft_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(draft_key): Path<String>,
) -> AppResult<Json<DraftPayloadDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let payload =
        crate::repo::get_draft(&state.db, auth.id, &draft_key).await?.ok_or(AppError::NotFound)?;

    Ok(Json(DraftPayloadDto { payload }))
}

/// DELETE /api/v2/me/drafts/{draft_key}
///
/// Deletes a draft. Returns 204 No Content regardless of whether the draft
/// existed.
pub async fn delete_draft_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(draft_key): Path<String>,
) -> AppResult<impl IntoResponse> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    crate::repo::delete_draft(&state.db, auth.id, &draft_key).await?;

    Ok((StatusCode::NO_CONTENT, ()))
}

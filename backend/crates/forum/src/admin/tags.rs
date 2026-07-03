//! Admin forum tag endpoints: create, update, delete, and list tags.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState};

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub thread_count: i32,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagInput {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagInput {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/admin/forum/tags — list all tags (admin view)
pub async fn list_tags_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<TagDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    // Stub — real implementation queries the tags table
    let _ = auth;
    Err(AppError::NotFound)
}

/// POST /api/v2/admin/forum/tags — create a new tag
pub async fn create_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_body): Json<CreateTagInput>,
) -> AppResult<Json<TagDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    // Stub — real implementation creates the tag in DB
    Err(AppError::NotFound)
}

/// PATCH /api/v2/admin/forum/tags/{id} — update a tag
pub async fn update_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(_body): Json<UpdateTagInput>,
) -> AppResult<Json<TagDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let _tag_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    // Stub — real implementation updates the tag in DB
    Err(AppError::NotFound)
}

/// DELETE /api/v2/admin/forum/tags/{id} — delete a tag
pub async fn delete_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let _tag_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    // Stub — real implementation deletes the tag from DB
    Err(AppError::NotFound)
}

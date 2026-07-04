//! Admin forum tag endpoints: create, update, delete, and list tags.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagInput {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
}

#[allow(dead_code)]
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

    let rows = crate::repo::list_tags(&state.db).await?;
    let items: Vec<TagDto> = rows
        .into_iter()
        .map(|r| TagDto {
            id: r.id.to_string(),
            slug: r.slug,
            name: r.name,
            description: r.description,
            thread_count: r.thread_count,
            created_at: r.created_at.timestamp(),
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/v2/admin/forum/tags — create a new tag
pub async fn create_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateTagInput>,
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

    let row =
        crate::repo::create_tag(&state.db, &body.slug, &body.name, body.description.as_deref())
            .await?;

    crate::repo::insert_mod_action(&state.db, auth.id, "create_tag", "tag", row.id, None, None)
        .await?;

    Ok(Json(TagDto {
        id: row.id.to_string(),
        slug: row.slug,
        name: row.name,
        description: row.description,
        thread_count: row.thread_count,
        created_at: row.created_at.timestamp(),
    }))
}

/// PATCH /api/v2/admin/forum/tags/{id} — update a tag
pub async fn update_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateTagInput>,
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

    let tag_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;

    let row = crate::repo::update_tag(
        &state.db,
        tag_id,
        body.slug.as_deref(),
        body.name.as_deref(),
        body.description.as_deref().map(Some),
    )
    .await?;

    crate::repo::insert_mod_action(&state.db, auth.id, "update_tag", "tag", tag_id, None, None)
        .await?;

    Ok(Json(TagDto {
        id: row.id.to_string(),
        slug: row.slug,
        name: row.name,
        description: row.description,
        thread_count: row.thread_count,
        created_at: row.created_at.timestamp(),
    }))
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

    let tag_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;

    crate::repo::delete_tag(&state.db, tag_id).await?;

    crate::repo::insert_mod_action(&state.db, auth.id, "delete_tag", "tag", tag_id, None, None)
        .await?;

    Ok(Json(json!({"ok": true})))
}

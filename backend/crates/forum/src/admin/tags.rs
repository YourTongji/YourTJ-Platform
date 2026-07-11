//! Admin forum tag endpoints: create, update, delete, and list tags.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
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
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagInput {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTagInput {
    pub reason: String,
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
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
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

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
) -> AppResult<(StatusCode, Json<TagDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;
    let reason = validate_reason(&body.reason)?;
    let slug = body.slug.trim();
    let name = body.name.trim();
    if slug.is_empty() || slug.chars().count() > 64 || name.is_empty() || name.chars().count() > 100
    {
        return Err(AppError::BadRequest("invalid tag slug or name".into()));
    }
    let description = body.description.as_deref().map(str::trim);
    if description.is_some_and(|description| description.chars().count() > 500) {
        return Err(AppError::BadRequest("tag description is too long".into()));
    }
    let mut tx = state.db.begin().await?;
    let row = crate::repo::create_tag(&mut *tx, slug, name, description).await?;
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "create_tag",
        "tag",
        row.id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.tag.created",
        "tag",
        &row.id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(TagDto {
            id: row.id.to_string(),
            slug: row.slug,
            name: row.name,
            description: row.description,
            thread_count: row.thread_count,
            created_at: row.created_at.timestamp(),
        }),
    ))
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
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let tag_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_reason(&body.reason)?;
    let slug = body.slug.as_deref().map(str::trim).filter(|slug| !slug.is_empty());
    let name = body.name.as_deref().map(str::trim).filter(|name| !name.is_empty());
    let description = body.description.as_deref().map(str::trim);
    if slug.is_some_and(|slug| slug.chars().count() > 64)
        || name.is_some_and(|name| name.chars().count() > 100)
        || description.is_some_and(|description| description.chars().count() > 500)
        || (slug.is_none() && name.is_none() && description.is_none())
    {
        return Err(AppError::BadRequest("invalid tag update".into()));
    }
    let mut tx = state.db.begin().await?;
    let row = crate::repo::update_tag(&mut *tx, tag_id, slug, name, description.map(Some)).await?;
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "update_tag",
        "tag",
        tag_id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.tag.updated",
        "tag",
        &tag_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

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
    Json(body): Json<DeleteTagInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let tag_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_reason(&body.reason)?;
    let mut tx = state.db.begin().await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.tags WHERE id = $1)")
        .bind(tag_id)
        .fetch_one(&mut *tx)
        .await?;
    if !exists {
        return Err(AppError::NotFound);
    }
    crate::repo::delete_tag(&mut *tx, tag_id).await?;
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "delete_tag",
        "tag",
        tag_id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.tag.deleted",
        "tag",
        &tag_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(Json(json!({"ok": true})))
}

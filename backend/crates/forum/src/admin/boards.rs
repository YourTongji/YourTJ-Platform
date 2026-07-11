//! Admin forum board endpoints: create, update, delete boards.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

use crate::dto::BoardDto;
use crate::handlers::board_to_dto;
use crate::models::BoardRow;

// ---------------------------------------------------------------------------
// Input DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardInput {
    pub slug: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoardInput {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBoardInput {
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

/// POST /api/v2/admin/forum/boards — create a new board
pub async fn create_board(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBoardInput>,
) -> AppResult<(StatusCode, Json<BoardDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;
    let reason = validate_reason(&body.reason)?;
    let slug = body.slug.trim();
    let name = body.name.trim();
    if slug.is_empty() || slug.chars().count() > 64 || name.is_empty() || name.chars().count() > 100
    {
        return Err(AppError::BadRequest("invalid board slug or name".into()));
    }
    let mut tx = state.db.begin().await?;
    let row: BoardRow = sqlx::query_as(
        "INSERT INTO forum.boards (slug, name) \
         VALUES ($1, $2) \
         RETURNING id, slug, name, parent_id, description, position, \
                   is_locked, is_qa, min_trust_to_post, thread_count",
    )
    .bind(slug)
    .bind(name)
    .fetch_one(&mut *tx)
    .await?;
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "create_board",
        "board",
        row.id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.board.created",
        "board",
        &row.id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(board_to_dto(&row))))
}

/// PATCH /api/v2/admin/forum/boards/{id} — update a board
pub async fn update_board(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateBoardInput>,
) -> AppResult<Json<BoardDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let board_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_reason(&body.reason)?;
    let slug = body.slug.as_deref().map(str::trim).filter(|slug| !slug.is_empty());
    let name = body.name.as_deref().map(str::trim).filter(|name| !name.is_empty());
    if slug.is_some_and(|slug| slug.chars().count() > 64)
        || name.is_some_and(|name| name.chars().count() > 100)
        || (slug.is_none() && name.is_none())
    {
        return Err(AppError::BadRequest("invalid board update".into()));
    }
    let mut tx = state.db.begin().await?;
    let row: BoardRow = sqlx::query_as(
        "UPDATE forum.boards \
         SET slug = COALESCE($1, slug), name = COALESCE($2, name) \
         WHERE id = $3 \
         RETURNING id, slug, name, parent_id, description, position, \
                   is_locked, is_qa, min_trust_to_post, thread_count",
    )
    .bind(slug)
    .bind(name)
    .bind(board_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    // Write mod action
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "update_board",
        "board",
        board_id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.board.updated",
        "board",
        &board_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(Json(board_to_dto(&row)))
}

/// DELETE /api/v2/admin/forum/boards/{id} — delete a board
pub async fn delete_board(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<DeleteBoardInput>,
) -> AppResult<Json<Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let board_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_reason(&body.reason)?;
    let mut tx = state.db.begin().await?;
    let dependencies: Option<i64> = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM forum.threads WHERE board_id = board.id) \
              + (SELECT COUNT(*) FROM forum.boards child WHERE child.parent_id = board.id) \
         FROM forum.boards board WHERE board.id = $1 FOR UPDATE",
    )
    .bind(board_id)
    .fetch_optional(&mut *tx)
    .await?;
    let dependencies = dependencies.ok_or(AppError::NotFound)?;
    if dependencies > 0 {
        return Err(AppError::Conflict(
            "board with threads or child boards cannot be deleted".into(),
        ));
    }
    let result = sqlx::query("DELETE FROM forum.boards WHERE id = $1")
        .bind(board_id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    // Write mod action
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "delete_board",
        "board",
        board_id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.board.deleted",
        "board",
        &board_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(Json(json!({"ok": true})))
}

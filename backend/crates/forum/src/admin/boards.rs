//! Admin forum board endpoints: create, update, delete boards.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

use crate::dto::BoardDto;
use crate::models::BoardRow;

// ---------------------------------------------------------------------------
// Input DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardInput {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoardInput {
    pub slug: Option<String>,
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v2/admin/forum/boards — create a new board
pub async fn create_board(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBoardInput>,
) -> AppResult<Json<BoardDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let row: BoardRow = sqlx::query_as(
        "INSERT INTO forum.boards (slug, name) VALUES ($1, $2) RETURNING id, slug, name",
    )
    .bind(&body.slug)
    .bind(&body.name)
    .fetch_one(&state.db)
    .await?;

    // Write mod action
    crate::repo::insert_mod_action(&state.db, auth.id, "create_board", "board", row.id, None, None)
        .await?;

    Ok(Json(BoardDto { id: row.id.to_string(), slug: row.slug, name: row.name }))
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
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let board_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;

    let row: BoardRow = sqlx::query_as(
        "UPDATE forum.boards \
         SET slug = COALESCE($1, slug), name = COALESCE($2, name) \
         WHERE id = $3 \
         RETURNING id, slug, name",
    )
    .bind(&body.slug)
    .bind(&body.name)
    .bind(board_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Write mod action
    crate::repo::insert_mod_action(
        &state.db,
        auth.id,
        "update_board",
        "board",
        board_id,
        None,
        None,
    )
    .await?;

    Ok(Json(BoardDto { id: row.id.to_string(), slug: row.slug, name: row.name }))
}

/// DELETE /api/v2/admin/forum/boards/{id} — delete a board
pub async fn delete_board(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
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

    let board_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;

    let result = sqlx::query("DELETE FROM forum.boards WHERE id = $1")
        .bind(board_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    // Write mod action
    crate::repo::insert_mod_action(
        &state.db,
        auth.id,
        "delete_board",
        "board",
        board_id,
        None,
        None,
    )
    .await?;

    Ok(Json(json!({"ok": true})))
}

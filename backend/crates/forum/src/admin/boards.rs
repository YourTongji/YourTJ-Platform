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
    pub description: Option<String>,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub min_trust_to_post: i16,
    #[serde(default)]
    pub is_qa: bool,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoardInput {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub position: Option<i32>,
    pub is_locked: Option<bool>,
    pub min_trust_to_post: Option<i16>,
    pub is_qa: Option<bool>,
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
    let description = body.description.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if slug.is_empty()
        || slug.chars().count() > 64
        || name.is_empty()
        || name.chars().count() > 100
        || body.description.as_ref().is_some_and(|value| value.trim().chars().count() > 500)
        || body.position < 0
        || !(0..=3).contains(&body.min_trust_to_post)
    {
        return Err(AppError::BadRequest("invalid board settings".into()));
    }
    let mut tx = state.db.begin().await?;
    let row: BoardRow = sqlx::query_as(
        "INSERT INTO forum.boards (slug, name, description, position, is_locked, \
                                   min_trust_to_post, is_qa) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, slug, name, parent_id, description, position, \
                   is_locked, is_qa, min_trust_to_post, thread_count",
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(body.position)
    .bind(body.is_locked)
    .bind(body.min_trust_to_post)
    .bind(body.is_qa)
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

    Ok((
        StatusCode::CREATED,
        Json(board_to_dto(
            &row,
            Some(crate::repo::boards::BoardPostingActor {
                trust_level: 0,
                can_bypass_board_gates: true,
            }),
        )),
    ))
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
    let description = body.description.as_deref().map(str::trim);
    if slug.is_some_and(|slug| slug.chars().count() > 64)
        || name.is_some_and(|name| name.chars().count() > 100)
        || description.is_some_and(|value| value.chars().count() > 500)
        || body.position.is_some_and(|position| position < 0)
        || body.min_trust_to_post.is_some_and(|level| !(0..=3).contains(&level))
        || (slug.is_none()
            && name.is_none()
            && body.description.is_none()
            && body.position.is_none()
            && body.is_locked.is_none()
            && body.min_trust_to_post.is_none()
            && body.is_qa.is_none())
    {
        return Err(AppError::BadRequest("invalid board update".into()));
    }
    let mut tx = state.db.begin().await?;
    let row: BoardRow = sqlx::query_as(
        "UPDATE forum.boards \
         SET slug = COALESCE($1, slug), name = COALESCE($2, name), \
             description = CASE WHEN $3 THEN NULLIF($4, '') ELSE description END, \
             position = COALESCE($5, position), is_locked = COALESCE($6, is_locked), \
             min_trust_to_post = COALESCE($7, min_trust_to_post), \
             is_qa = COALESCE($8, is_qa) \
         WHERE id = $9 \
         RETURNING id, slug, name, parent_id, description, position, \
                   is_locked, is_qa, min_trust_to_post, thread_count",
    )
    .bind(slug)
    .bind(name)
    .bind(body.description.is_some())
    .bind(description)
    .bind(body.position)
    .bind(body.is_locked)
    .bind(body.min_trust_to_post)
    .bind(body.is_qa)
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

    Ok(Json(board_to_dto(
        &row,
        Some(crate::repo::boards::BoardPostingActor {
            trust_level: 0,
            can_bypass_board_gates: true,
        }),
    )))
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

//! Admin forum watched-words endpoints: list, create, and delete watched words.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedWordDto {
    pub id: String,
    pub word: String,
    pub action: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWatchedWordInput {
    pub word: String,
    pub action: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct WatchedWordRow {
    id: i64,
    word: String,
    action: String,
    created_by: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/admin/forum/watched-words — list all watched words
pub async fn list_watched_words(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<WatchedWordDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let rows: Vec<WatchedWordRow> = sqlx::query_as(
        "SELECT id, word, action, created_by, created_at FROM forum.watched_words ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| WatchedWordDto {
            id: r.id.to_string(),
            word: r.word,
            action: r.action,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(items))
}

/// POST /api/v2/admin/forum/watched-words — add a watched word
pub async fn create_watched_word(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateWatchedWordInput>,
) -> AppResult<Json<WatchedWordDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let row: WatchedWordRow = sqlx::query_as(
        "INSERT INTO forum.watched_words (word, action, created_by) \
         VALUES ($1, $2, $3) RETURNING id, word, action, created_by, created_at",
    )
    .bind(&body.word)
    .bind(&body.action)
    .bind(auth.id)
    .fetch_one(&state.db)
    .await?;

    // Write mod action
    crate::repo::insert_mod_action(
        &state.db,
        auth.id,
        "create_watched_word",
        "watched_word",
        row.id,
        None,
        None,
    )
    .await?;

    Ok(Json(WatchedWordDto {
        id: row.id.to_string(),
        word: row.word,
        action: row.action,
        created_at: row.created_at.timestamp(),
    }))
}

/// DELETE /api/v2/admin/forum/watched-words/{id} — remove a watched word
pub async fn delete_watched_word(
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

    let word_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;

    let result = sqlx::query("DELETE FROM forum.watched_words WHERE id = $1")
        .bind(word_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    // Write mod action
    crate::repo::insert_mod_action(
        &state.db,
        auth.id,
        "delete_watched_word",
        "watched_word",
        word_id,
        None,
        None,
    )
    .await?;

    Ok(Json(json!({"ok": true})))
}

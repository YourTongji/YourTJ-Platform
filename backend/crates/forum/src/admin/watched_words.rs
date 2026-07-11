//! Admin forum watched-words endpoints: list, create, and delete watched words.
//!
//! These handlers require mod/admin auth.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Reload the in-memory matcher *and* publish a cross-instance reload signal.
async fn reload_and_notify(state: &AppState) {
    if let Err(e) = crate::watched_words::reload_watched_words(&state.db).await {
        tracing::error!(error = %e, "failed to reload watched words");
    }
    if let Some(ref r) = state.redis {
        if let Ok(mut conn) = r.get().await {
            let _: () = redis::cmd("PUBLISH")
                .arg("forum:watched_words:reload")
                .arg("1")
                .query_async(&mut conn)
                .await
                .unwrap_or(());
        }
    }
}

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
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWatchedWordInput {
    pub reason: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct WatchedWordRow {
    id: i64,
    word: String,
    action: String,
    #[allow(dead_code)]
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
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

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
) -> AppResult<(StatusCode, Json<WatchedWordDto>)> {
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
    let reason = body.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    let word = body.word.trim();
    if word.is_empty() || word.chars().count() > 200 {
        return Err(AppError::BadRequest("word must be 1–200 characters".into()));
    }
    if !matches!(body.action.as_str(), "block" | "censor" | "queue") {
        return Err(AppError::BadRequest("invalid watched-word action".into()));
    }
    let mut tx = state.db.begin().await?;
    let row: WatchedWordRow = sqlx::query_as(
        "INSERT INTO forum.watched_words (word, action, created_by) \
         VALUES ($1, $2, $3) RETURNING id, word, action, created_by, created_at",
    )
    .bind(word)
    .bind(&body.action)
    .bind(auth.id)
    .fetch_one(&mut *tx)
    .await?;
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "create_watched_word",
        "watched_word",
        row.id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.watched_word.created",
        "watched_word",
        &row.id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    // Reload the matcher to pick up the new word and notify other instances
    reload_and_notify(&state).await;

    Ok((
        StatusCode::CREATED,
        Json(WatchedWordDto {
            id: row.id.to_string(),
            word: row.word,
            action: row.action,
            created_at: row.created_at.timestamp(),
        }),
    ))
}

/// DELETE /api/v2/admin/forum/watched-words/{id} — remove a watched word
pub async fn delete_watched_word(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<DeleteWatchedWordInput>,
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

    let word_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let reason = body.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    let mut tx = state.db.begin().await?;
    let result = sqlx::query("DELETE FROM forum.watched_words WHERE id = $1")
        .bind(word_id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        "delete_watched_word",
        "watched_word",
        word_id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.watched_word.deleted",
        "watched_word",
        &word_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    // Reload the matcher and notify other instances
    reload_and_notify(&state).await;

    Ok(Json(json!({"ok": true})))
}

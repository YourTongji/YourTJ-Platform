//! Platform module: announcements and settings exposed as public/admin
//! endpoints. Lives in the api crate because it has no domain dependency.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState};
use sqlx::FromRow;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Models (DB rows)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AnnouncementRow {
    pub id: i64,
    pub title: String,
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementDto {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingDto {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingInput {
    pub value: String,
}

// ---------------------------------------------------------------------------
// Repo
// ---------------------------------------------------------------------------

pub async fn list_announcements(pool: &PgPool) -> AppResult<Vec<AnnouncementRow>> {
    let rows = sqlx::query_as::<_, AnnouncementRow>(
        "SELECT id, title, body, created_at \
         FROM platform.announcements \
         ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_settings(pool: &PgPool) -> AppResult<Vec<SettingRow>> {
    let rows = sqlx::query_as::<_, SettingRow>(
        "SELECT key, value, updated_at FROM platform.settings ORDER BY key",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_setting(pool: &PgPool, key: &str) -> AppResult<Option<SettingRow>> {
    let row = sqlx::query_as::<_, SettingRow>(
        "SELECT key, value, updated_at FROM platform.settings WHERE key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update_setting(pool: &PgPool, key: &str, value: &str) -> AppResult<()> {
    let rows =
        sqlx::query("UPDATE platform.settings SET value = $1, updated_at = now() WHERE key = $2")
            .bind(value)
            .bind(key)
            .execute(pool)
            .await?
            .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /announcements — public
pub async fn list_announcements_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<AnnouncementDto>>> {
    let rows = list_announcements(&state.db).await?;
    let items: Vec<AnnouncementDto> = rows
        .into_iter()
        .map(|r| AnnouncementDto {
            id: r.id.to_string(),
            title: r.title,
            body: r.body,
            created_at: r.created_at.timestamp(),
        })
        .collect();
    Ok(Json(items))
}

/// GET /settings — public (returns safe subset of settings)
pub async fn list_settings_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<SettingDto>>> {
    let rows = list_settings(&state.db).await?;
    let public_keys = ["app_name", "version"];
    let items: Vec<SettingDto> = rows
        .into_iter()
        .filter(|r| public_keys.contains(&r.key.as_str()))
        .map(|r| SettingDto { key: r.key, value: r.value })
        .collect();
    Ok(Json(items))
}

/// POST /startup/verify — captcha stub (accept any token, return 200)
pub async fn startup_verify_handler() -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/v2/admin/settings — admin: list all settings
pub async fn admin_list_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SettingDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;
    let rows = list_settings(&state.db).await?;
    let items: Vec<SettingDto> =
        rows.into_iter().map(|r| SettingDto { key: r.key, value: r.value }).collect();
    Ok(Json(items))
}

/// GET /api/v2/admin/settings/{key} — admin: get a single setting
pub async fn admin_get_setting_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> AppResult<Json<SettingDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let row = get_setting(&state.db, &key).await?.ok_or(AppError::NotFound)?;
    Ok(Json(SettingDto { key: row.key, value: row.value }))
}

/// PUT /api/v2/admin/settings/{key} — admin: update a setting
pub async fn admin_update_setting_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(body): Json<UpdateSettingInput>,
) -> AppResult<Json<SettingDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    update_setting(&state.db, &key, &body.value).await?;
    Ok(Json(SettingDto { key, value: body.value }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// All platform-owned routes.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/announcements", get(list_announcements_handler))
        .route("/settings", get(list_settings_handler))
        .route("/startup/verify", post(startup_verify_handler))
        .route("/api/v2/admin/settings", get(admin_list_settings_handler))
        .route(
            "/api/v2/admin/settings/{key}",
            get(admin_get_setting_handler).put(admin_update_setting_handler),
        )
        .with_state(state)
}

/// Liveness probe used by SAE / load balancers.
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "yourtj-platform" }))
}

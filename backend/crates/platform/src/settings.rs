use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState};
use sqlx::FromRow;

use crate::auth::staff_account;
use crate::validation::reason;

#[derive(Debug, FromRow)]
struct SettingRow {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingDto {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSettingInput {
    value: String,
    reason: String,
}

async fn list_settings(state: &AppState) -> AppResult<Vec<SettingRow>> {
    Ok(sqlx::query_as::<_, SettingRow>("SELECT key, value FROM platform.settings ORDER BY key")
        .fetch_all(&state.db)
        .await?)
}

async fn list_public(State(state): State<AppState>) -> AppResult<Json<Vec<SettingDto>>> {
    let public_keys = ["app_name", "version"];
    let items = list_settings(&state)
        .await?
        .into_iter()
        .filter(|row| public_keys.contains(&row.key.as_str()))
        .map(|row| SettingDto { key: row.key, value: row.value })
        .collect();
    Ok(Json(items))
}

async fn startup_verify(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let token = body.get("token").and_then(serde_json::Value::as_str).unwrap_or("");
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "startup",
        token,
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn admin_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SettingDto>>> {
    staff_account(&headers, &state, Capability::ManagePlatform).await?;
    let items = list_settings(&state)
        .await?
        .into_iter()
        .map(|row| SettingDto { key: row.key, value: row.value })
        .collect();
    Ok(Json(items))
}

async fn admin_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> AppResult<Json<SettingDto>> {
    staff_account(&headers, &state, Capability::ManagePlatform).await?;
    let row =
        sqlx::query_as::<_, SettingRow>("SELECT key, value FROM platform.settings WHERE key = $1")
            .bind(&key)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    Ok(Json(SettingDto { key: row.key, value: row.value }))
}

async fn admin_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(body): Json<UpdateSettingInput>,
) -> AppResult<Json<SettingDto>> {
    let account = staff_account(&headers, &state, Capability::ManagePlatform).await?;
    let reason = reason(&body.reason)?;
    if body.value.chars().count() > 20_000 {
        return Err(AppError::BadRequest("setting value is too long".into()));
    }
    let mut tx = state.db.begin().await?;
    let affected =
        sqlx::query("UPDATE platform.settings SET value = $1, updated_at = now() WHERE key = $2")
            .bind(&body.value)
            .bind(&key)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    if affected != 1 {
        return Err(AppError::NotFound);
    }
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.setting.updated",
        "setting",
        &key,
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(SettingDto { key, value: body.value }))
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v2/settings", get(list_public))
        .route("/api/v2/startup/verify", post(startup_verify))
        .route("/api/v2/admin/settings", get(admin_list))
        .route("/api/v2/admin/settings/{key}", get(admin_get).put(admin_update))
}

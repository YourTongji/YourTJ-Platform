//! Platform module: announcements and settings exposed as public/admin
//! endpoints. Lives in the api crate because it has no domain dependency.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::Capability;
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
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementInput {
    pub title: String,
    pub body: Option<String>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReasonInput {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAnnouncementsQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
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

fn announcement_dto(row: AnnouncementRow) -> AnnouncementDto {
    AnnouncementDto {
        id: row.id.to_string(),
        title: row.title,
        body: row.body,
        created_at: row.created_at.timestamp(),
    }
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
}

fn validate_announcement(body: &AnnouncementInput) -> AppResult<(&str, Option<&str>, &str)> {
    let title = body.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(AppError::BadRequest("title must be 1–200 characters".into()));
    }
    let content = body.body.as_deref().map(str::trim).filter(|content| !content.is_empty());
    if content.is_some_and(|content| content.chars().count() > 20_000) {
        return Err(AppError::BadRequest("body must be at most 20000 characters".into()));
    }
    Ok((title, content, validate_reason(&body.reason)?))
}

async fn authenticate_staff(
    headers: &HeaderMap,
    state: &AppState,
    capability: Capability,
) -> AppResult<shared::AuthAccount> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(capability).map_err(|_| AppError::Forbidden)?;
    Ok(auth)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/announcements — public
pub async fn list_announcements_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<AnnouncementDto>>> {
    let rows = list_announcements(&state.db).await?;
    let items = rows.into_iter().map(announcement_dto).collect();
    Ok(Json(items))
}

/// GET /api/v2/settings — public (returns safe subset of settings)
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

/// POST /api/v2/startup/verify — captcha verification
pub async fn startup_verify_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let token = body.get("token").and_then(|v| v.as_str()).unwrap_or("");
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "startup",
        token,
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/v2/admin/settings — admin: list all settings
pub async fn admin_list_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SettingDto>>> {
    authenticate_staff(&headers, &state, Capability::ManagePlatform).await?;
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
    authenticate_staff(&headers, &state, Capability::ManagePlatform).await?;

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
    let auth = authenticate_staff(&headers, &state, Capability::ManagePlatform).await?;
    let reason = validate_reason(&body.reason)?;
    if body.value.chars().count() > 20_000 {
        return Err(AppError::BadRequest("setting value is too long".into()));
    }
    let mut tx = state.db.begin().await?;
    let rows =
        sqlx::query("UPDATE platform.settings SET value = $1, updated_at = now() WHERE key = $2")
            .bind(&body.value)
            .bind(&key)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound);
    }
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
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

/// GET /api/v2/admin/announcements — paginated announcement management list.
pub async fn admin_list_announcements_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminAnnouncementsQuery>,
) -> AppResult<Json<shared::Page<AnnouncementDto>>> {
    authenticate_staff(&headers, &state, Capability::ManageAnnouncements).await?;
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let mut rows = sqlx::query_as::<_, AnnouncementRow>(
        "SELECT id, title, body, created_at FROM platform.announcements \
         WHERE ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.id.to_string())).flatten();
    let items = rows.into_iter().map(announcement_dto).collect();
    Ok(Json(shared::Page::new(items, next_cursor)))
}

/// POST /api/v2/admin/announcements — publish a public announcement.
pub async fn admin_create_announcement_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AnnouncementInput>,
) -> AppResult<(StatusCode, Json<AnnouncementDto>)> {
    let auth = authenticate_staff(&headers, &state, Capability::ManageAnnouncements).await?;
    let (title, content, reason) = validate_announcement(&body)?;
    let mut tx = state.db.begin().await?;
    let row = sqlx::query_as::<_, AnnouncementRow>(
        "INSERT INTO platform.announcements (title, body) VALUES ($1, $2) \
         RETURNING id, title, body, created_at",
    )
    .bind(title)
    .bind(content)
    .fetch_one(&mut *tx)
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "platform.announcement.published",
        "announcement",
        &row.id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(announcement_dto(row))))
}

/// PATCH /api/v2/admin/announcements/{id} — update announcement copy.
pub async fn admin_update_announcement_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(announcement_id): Path<String>,
    Json(body): Json<AnnouncementInput>,
) -> AppResult<Json<AnnouncementDto>> {
    let auth = authenticate_staff(&headers, &state, Capability::ManageAnnouncements).await?;
    let announcement_id = announcement_id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid announcement id".into()))?;
    let (title, content, reason) = validate_announcement(&body)?;
    let mut tx = state.db.begin().await?;
    let row = sqlx::query_as::<_, AnnouncementRow>(
        "UPDATE platform.announcements SET title = $1, body = $2 WHERE id = $3 \
         RETURNING id, title, body, created_at",
    )
    .bind(title)
    .bind(content)
    .bind(announcement_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "platform.announcement.updated",
        "announcement",
        &announcement_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(announcement_dto(row)))
}

/// DELETE /api/v2/admin/announcements/{id} — remove an announcement.
pub async fn admin_delete_announcement_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(announcement_id): Path<String>,
    Json(body): Json<AdminReasonInput>,
) -> AppResult<StatusCode> {
    let auth = authenticate_staff(&headers, &state, Capability::ManageAnnouncements).await?;
    let reason = validate_reason(&body.reason)?;
    let announcement_id = announcement_id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid announcement id".into()))?;
    let mut tx = state.db.begin().await?;
    let rows = sqlx::query("DELETE FROM platform.announcements WHERE id = $1")
        .bind(announcement_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if rows == 0 {
        return Err(AppError::NotFound);
    }
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "platform.announcement.deleted",
        "announcement",
        &announcement_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// All platform-owned routes.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/announcements", get(list_announcements_handler))
        .route("/api/v2/settings", get(list_settings_handler))
        .route("/api/v2/startup/verify", post(startup_verify_handler))
        .route("/api/v2/admin/settings", get(admin_list_settings_handler))
        .route(
            "/api/v2/admin/settings/{key}",
            get(admin_get_setting_handler).put(admin_update_setting_handler),
        )
        .route(
            "/api/v2/admin/announcements",
            get(admin_list_announcements_handler).post(admin_create_announcement_handler),
        )
        .route(
            "/api/v2/admin/announcements/{id}",
            patch(admin_update_announcement_handler).delete(admin_delete_announcement_handler),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use shared::AppState;
    use tower::ServiceExt;

    use super::routes;

    #[tokio::test]
    async fn startup_verify_is_mounted_under_api_v2() {
        let state = AppState {
            db: sqlx::PgPool::connect_lazy("postgres://user:password@localhost/test")
                .expect("valid lazy postgres URL"),
            config: shared::Config::from_env().expect("test Config::from_env"),
            jwt_secret: "integration-test-secret-32bytes!".into(),
            jwt_ttl: 900,
            refresh_ttl: 604800,
            meili_url: String::new(),
            meili_master_key: String::new(),
            redis: None,
            system_private_key: vec![0u8; 32],
            system_public_key_b64: String::new(),
            email_encryption: None,
            captcha_verifier: Some(std::sync::Arc::new(shared::captcha::FakeCaptcha)),
            sse_tx: None,
        };

        let response = routes(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v2/startup/verify")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"token":"test-token"}"#))
                    .expect("request builds"),
            )
            .await
            .expect("request succeeds");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

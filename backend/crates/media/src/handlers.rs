//! HTTP handlers for the media domain.
//!
//! Routes are registered in [`crate::routes`]. Each handler follows the
//! established pattern: extract `State<AppState>`, authenticate, validate,
//! call repo/oss, build DTO, return `AppResult<Json<…>>`.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{UploadCredentialsDto, UploadDto, UploadUrlDto};
use crate::oss::{self, OssConfig};
use crate::repo;
use crate::{error::MediaError, models::UploadRow};

// ---------------------------------------------------------------------------
// constants
// ---------------------------------------------------------------------------

const DEFAULT_PAGE_LIMIT: i64 = 20;

// ---------------------------------------------------------------------------
// query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    DEFAULT_PAGE_LIMIT
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn upload_to_dto(row: &UploadRow) -> UploadDto {
    UploadDto {
        id: row.id.to_string(),
        account_id: row.account_id.to_string(),
        kind: row.kind.clone(),
        oss_key: row.oss_key.clone(),
        url: row.url.clone(),
        bytes: row.bytes,
        mime: row.mime.clone(),
        sha256: row.sha256.clone(),
        status: row.status.clone(),
        created_at: row.created_at.timestamp(),
    }
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// POST /api/v2/media/upload-credentials — auth required
///
/// Returns temporary STS credentials so the client can upload files directly
/// to OSS. Currently returns placeholder credentials.
pub async fn upload_credentials(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<UploadCredentialsDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    // Build placeholder OSS config (real integration would source from AppState).
    let oss_config = OssConfig {
        region: state.config.meili_url.clone(), // placeholder
        bucket: "yourtj-uploads".into(),
        access_key_id: "placeholder-key".into(),
        access_key_secret: "placeholder-secret".into(),
        role_arn: "placeholder-arn".into(),
    };

    let creds = oss::generate_sts_credentials(&oss_config, auth.id);
    Ok(Json(creds))
}

/// POST /api/v2/media/callback — OSS callback endpoint
///
/// Called by OSS after a successful upload. Creates a media.uploads row with
/// status `pending` and returns 200 OK.
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    body_bytes: axum::body::Bytes,
) -> AppResult<Json<serde_json::Value>> {
    // Verify callback signature (placeholder — always passes).
    let oss_config = OssConfig {
        region: String::new(),
        bucket: String::new(),
        access_key_id: String::new(),
        access_key_secret: String::new(),
        role_arn: String::new(),
    };
    if !oss::verify_callback_signature(&oss_config, &headers, &body_bytes) {
        return Err(AppError::BadRequest("invalid callback signature".into()));
    }

    // Parse the callback body.
    let input: crate::dto::UploadCallbackInput = serde_json::from_slice(&body_bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid callback body: {e}")))?;

    // The callback doesn't carry an account_id; we derive it from the prefix
    // format "uploads/{account_id}/...".
    let account_id =
        input.oss_key.split('/').nth(1).and_then(|s| s.parse::<i64>().ok()).ok_or_else(|| {
            AppError::BadRequest("invalid ossKey format — expected uploads/{accountId}/...".into())
        })?;

    let _row = repo::insert_upload(
        &state.db,
        account_id,
        "image", // default; client may override by prefix convention
        &input.oss_key,
        &input.url,
        input.bytes,
        &input.mime,
        &input.sha256,
    )
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/v2/media/{id}/url — get media URL
///
/// Returns the CDN / signed URL for a media upload. Requires authentication.
pub async fn get_url(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<UploadUrlDto>> {
    let _auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::find_upload(&state.db, id).await?.ok_or(MediaError::NotFound)?;

    let oss_config = OssConfig {
        region: String::new(),
        bucket: String::new(),
        access_key_id: String::new(),
        access_key_secret: String::new(),
        role_arn: String::new(),
    };
    let url = oss::generate_url(&oss_config, &row.oss_key);

    Ok(Json(UploadUrlDto { url }))
}

// ---------------------------------------------------------------------------
// admin handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/admin/media/uploads — list pending uploads (mod queue)
pub async fn list_uploads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<UploadListQuery>,
) -> AppResult<Json<Page<UploadDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let (rows, next_cursor) = repo::list_pending(&state.db, q.cursor.as_deref(), q.limit).await?;

    let items: Vec<UploadDto> = rows.iter().map(upload_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/admin/media/uploads/{id}/approve — approve a pending upload
pub async fn approve_upload(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
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

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::find_upload(&state.db, id).await?.ok_or(MediaError::NotFound)?;

    if row.status != "pending" {
        return Err(AppError::BadRequest(format!("upload is already {}", row.status)));
    }

    repo::update_status(&state.db, id, "clean").await?;
    tracing::info!(upload_id = id, moderator_id = auth.id, "upload approved");

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/v2/admin/media/uploads/{id}/block — block a pending upload
pub async fn block_upload(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
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

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::find_upload(&state.db, id).await?.ok_or(MediaError::NotFound)?;

    if row.status != "pending" {
        return Err(AppError::BadRequest(format!("upload is already {}", row.status)));
    }

    // Update status to blocked.
    repo::update_status(&state.db, id, "blocked").await?;

    // Deferred: Delete the object from OSS when real integration is wired (issue #oss-delete).
    // oss::delete_object(&oss_config, &row.oss_key).await?;

    tracing::info!(upload_id = id, moderator_id = auth.id, "upload blocked");
    Ok(Json(serde_json::json!({ "ok": true })))
}

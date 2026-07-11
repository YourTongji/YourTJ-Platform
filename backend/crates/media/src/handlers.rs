//! HTTP handlers for the media domain.
//!
//! Routes are registered in [`crate::routes`]. Each handler follows the
//! established pattern: extract `State<AppState>`, authenticate, validate,
//! call repo/oss, build DTO, return `AppResult<Json<…>>`.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::{HeaderMap, Uri};
use axum::{Extension, Json};
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{UploadCredentialsDto, UploadDto, UploadIntentInput, UploadUrlDto};
use crate::oss::{self, AliyunStsProvider, OssConfig};
use crate::quarantine::{quarantine_upload, require_independent_moderator, UploadObjectStore};
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerateUploadInput {
    reason: String,
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

fn require_oss_config(state: &AppState) -> AppResult<OssConfig> {
    OssConfig::from_config(&state.config)
        .ok_or_else(|| MediaError::Unavailable("oss is not configured".into()).into())
}

fn validate_upload_kind(kind: &str) -> AppResult<()> {
    if matches!(kind, "image" | "file") {
        Ok(())
    } else {
        Err(MediaError::BadRequest("invalid upload kind".into()).into())
    }
}

fn validate_moderation_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
}

async fn moderate_upload(
    state: &AppState,
    auth: &shared::AuthAccount,
    upload_id: i64,
    new_status: &str,
    reason: &str,
) -> AppResult<()> {
    let mut tx = state.db.begin().await?;
    let upload: Option<(String, i64)> =
        sqlx::query_as("SELECT status, account_id FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(upload_id)
            .fetch_optional(&mut *tx)
            .await?;
    let (current_status, owner_id) = upload.ok_or(MediaError::NotFound)?;
    require_independent_moderator(auth, owner_id)?;
    if current_status != "pending" {
        return Err(AppError::Conflict(format!("upload is already {current_status}")));
    }
    sqlx::query("UPDATE media.uploads SET status = $1 WHERE id = $2")
        .bind(new_status)
        .bind(upload_id)
        .execute(&mut *tx)
        .await?;
    let metadata = serde_json::json!({ "oldStatus": current_status, "newStatus": new_status });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        if new_status == "clean" { "media.upload.approved" } else { "media.upload.blocked" },
        "upload",
        &upload_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn can_read_upload_url(auth: &shared::AuthAccount, upload: &UploadRow) -> bool {
    match upload.status.as_str() {
        "clean" => true,
        "pending" => {
            upload.account_id == auth.id
                || auth.has_capability(shared::auth::Capability::ModerateContent)
        }
        "blocked" => false,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// POST /api/v2/media/upload-credentials — auth required
///
/// Issues a one-time upload intent and least-privilege STS credentials scoped to it.
pub async fn upload_credentials(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<UploadIntentInput>,
) -> AppResult<Json<UploadCredentialsDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    validate_upload_kind(&input.kind)?;
    let content_type = oss::validate_content_type(&input.kind, &input.content_type)?;
    let oss_config = require_oss_config(&state)?;
    let expires_at = oss::upload_intent_expires_at();
    let callback_token = oss::new_callback_token();
    let intent_id = uuid::Uuid::new_v4();
    let oss_key = oss::build_oss_key(auth.id, &input.kind, content_type, intent_id);
    let intent = repo::insert_upload_intent(
        &state.db,
        intent_id,
        auth.id,
        &input.kind,
        &oss_key,
        content_type,
        oss::OSS_UPLOAD_MAX_BYTES,
        &callback_token,
        expires_at,
    )
    .await?;
    let provider = AliyunStsProvider::default();
    let creds = oss::generate_sts_credentials(
        &provider,
        &oss_config,
        auth.id,
        intent.id,
        &intent.oss_key,
        &intent.callback_token,
        intent.expires_at,
    )
    .await?;
    Ok(Json(creds))
}

/// POST /api/v2/media/callback — OSS callback endpoint
///
/// Called by OSS after a successful upload. Creates a media.uploads row with
/// status `pending` and consumes the upload intent atomically.
pub async fn callback(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body_bytes: axum::body::Bytes,
) -> AppResult<Json<serde_json::Value>> {
    let oss_config = require_oss_config(&state)?;
    let public_key_url = oss::callback_public_key_url(&headers)?;
    let callback_client = reqwest::Client::builder()
        .timeout(oss::callback_http_timeout())
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?;
    let public_key_response =
        callback_client.get(public_key_url).send().await.map_err(|error| {
            tracing::warn!(?error, "oss callback public key fetch failed");
            MediaError::Unavailable("callback public key unavailable".into())
        })?;
    if !public_key_response.status().is_success()
        || public_key_response.content_length().unwrap_or(0)
            > oss::callback_public_key_max_bytes() as u64
    {
        return Err(MediaError::Unavailable("callback public key unavailable".into()).into());
    }
    let public_key_bytes = public_key_response.bytes().await.map_err(|error| {
        tracing::warn!(?error, "oss callback public key read failed");
        MediaError::Unavailable("callback public key unavailable".into())
    })?;
    if public_key_bytes.len() > oss::callback_public_key_max_bytes() {
        return Err(MediaError::Unavailable("callback public key unavailable".into()).into());
    }
    let public_key_pem = std::str::from_utf8(&public_key_bytes)
        .map_err(|_| MediaError::BadRequest("invalid callback public key".into()))?;
    verify_callback_signature_for_uri(&headers, &uri, &body_bytes, public_key_pem)?;

    let input: crate::dto::UploadCallbackInput = serde_json::from_slice(&body_bytes)
        .map_err(|error| AppError::BadRequest(format!("invalid callback body: {error}")))?;
    let intent_id = input
        .upload_intent_id
        .parse::<uuid::Uuid>()
        .map_err(|_| AppError::BadRequest("invalid uploadIntentId".into()))?;

    let mut tx = state.db.begin().await?;
    let intent = repo::lock_upload_intent(&mut tx, intent_id).await?.ok_or(MediaError::NotFound)?;
    if let Some(upload_id) = intent.upload_id {
        tx.commit().await?;
        return Ok(Json(serde_json::json!({ "ok": true, "uploadId": upload_id.to_string() })));
    }
    if intent.expires_at <= chrono::Utc::now() {
        return Err(MediaError::BadRequest("upload intent expired".into()).into());
    }
    if intent.callback_token != input.callback_token {
        return Err(MediaError::BadRequest("upload intent mismatch".into()).into());
    }
    oss::validate_callback_metadata(
        &intent.oss_key,
        &intent.content_type,
        intent.max_bytes,
        &input.oss_key,
        input.bytes,
        &input.mime,
        &input.sha256,
    )?;
    let trusted_url = oss::generate_url(&oss_config, &intent.oss_key);

    let row = repo::insert_upload_in_tx(
        &mut tx,
        intent.account_id,
        &intent.kind,
        &intent.oss_key,
        &trusted_url,
        input.bytes,
        &intent.content_type,
        &input.sha256,
    )
    .await?;
    repo::consume_upload_intent(&mut tx, intent.id, row.id).await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "ok": true, "uploadId": row.id.to_string() })))
}

fn verify_callback_signature_for_uri(
    headers: &HeaderMap,
    uri: &Uri,
    body: &[u8],
    public_key_pem: &str,
) -> AppResult<()> {
    oss::verify_callback_signature(headers, uri, body, public_key_pem).map_err(Into::into)
}

/// GET /api/v2/media/{id}/url — get media URL
///
/// Returns the CDN / signed URL for a media upload. Requires authentication.
pub async fn get_url(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<UploadUrlDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let row = repo::find_upload(&state.db, id).await?.ok_or(MediaError::NotFound)?;
    if !can_read_upload_url(&auth, &row) {
        return Err(MediaError::NotFound.into());
    }

    let oss_config = require_oss_config(&state)?;
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
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let (rows, next_cursor) = repo::list_pending(&state.db, q.cursor.as_deref(), q.limit).await?;

    let items: Vec<UploadDto> = rows.iter().map(upload_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/admin/media/uploads/{id}/approve — approve a pending upload
pub async fn approve_upload(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ModerateUploadInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_moderation_reason(&body.reason)?;
    moderate_upload(&state, &auth, id, "clean", reason).await?;
    tracing::info!(upload_id = id, moderator_id = auth.id, "upload approved");

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/v2/admin/media/uploads/{id}/block — block a pending upload
pub async fn block_upload(
    State(state): State<AppState>,
    Extension(object_store): Extension<Arc<dyn UploadObjectStore>>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ModerateUploadInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_moderation_reason(&body.reason)?;
    quarantine_upload(&state, &auth, id, reason, object_store.as_ref()).await?;

    tracing::info!(upload_id = id, moderator_id = auth.id, "upload blocked");
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use shared::AuthAccount;

    use super::{can_read_upload_url, UploadRow};
    use crate::quarantine::require_independent_moderator;

    fn account(id: i64, role: &str) -> AuthAccount {
        AuthAccount { id, role: role.into(), status: "active".into() }
    }

    fn upload(account_id: i64, status: &str) -> UploadRow {
        UploadRow {
            id: 1,
            account_id,
            kind: "image".into(),
            oss_key: "uploads/1/image/file.png".into(),
            url: "https://example.invalid/file.png".into(),
            bytes: 10,
            mime: "image/png".into(),
            sha256: "a".repeat(64),
            status: status.into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn pending_url_is_limited_to_owner_or_staff() {
        let pending = upload(10, "pending");
        assert!(can_read_upload_url(&account(10, "user"), &pending));
        assert!(!can_read_upload_url(&account(11, "user"), &pending));
        assert!(can_read_upload_url(&account(11, "mod"), &pending));
    }

    #[test]
    fn blocked_url_is_never_returned_even_to_staff() {
        let blocked = upload(10, "blocked");
        assert!(!can_read_upload_url(&account(10, "user"), &blocked));
        assert!(!can_read_upload_url(&account(11, "admin"), &blocked));
    }

    #[test]
    fn clean_url_is_available_to_authenticated_accounts() {
        let clean = upload(10, "clean");
        assert!(can_read_upload_url(&account(11, "user"), &clean));
    }

    #[test]
    fn staff_cannot_moderate_their_own_upload() {
        assert!(matches!(
            require_independent_moderator(&account(10, "admin"), 10),
            Err(shared::AppError::Forbidden)
        ));
        assert!(require_independent_moderator(&account(11, "mod"), 10).is_ok());
    }
}

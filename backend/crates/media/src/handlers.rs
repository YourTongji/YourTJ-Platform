//! HTTP handlers for the media domain.
//!
//! Routes are registered in [`crate::routes`]. Each handler follows the
//! established pattern: extract `State<AppState>`, authenticate, validate,
//! call repo/oss, build DTO, return `AppResult<Json<…>>`.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::bindings::{sync_asset_binding, AssetBindingType};
use crate::deletion::schedule_upload_deletion;
use crate::dto::{
    MediaUsage, MyUploadDto, ProfileAssetInput, ReleaseRetentionHoldInput, RetentionHoldInput,
    UploadCredentialsDto, UploadDto, UploadIntentInput, UploadUrlDto,
};
use crate::error::MediaError;
use crate::models::{ModerationUploadRow, UploadRow};
use crate::moderation::require_strictly_lower_owner;
use crate::oss::{self, AliyunStsProvider, OssConfig};
use crate::preview::{consume_preview_grant, create_preview_grant, PREVIEW_TOKEN_HEADER};
use crate::quarantine::UploadObjectStore;
use crate::repo;

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
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyUploadListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    usage: Option<MediaUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionHoldListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionJobListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    status: Option<String>,
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

fn upload_to_dto(row: &ModerationUploadRow) -> UploadDto {
    let approval_requirement = if row.status != "pending" {
        "none"
    } else if row.kind != "image" {
        "scanner"
    } else if row.has_reviewer_evidence {
        "satisfied"
    } else {
        "image_preview"
    };
    UploadDto {
        id: row.id.to_string(),
        account_id: row.account_id.to_string(),
        kind: row.kind.clone(),
        bytes: row.bytes,
        mime: row.mime.clone(),
        status: row.status.clone(),
        usage: row.usage.clone(),
        image_width: row.image_width,
        image_height: row.image_height,
        approval_requirement: approval_requirement.into(),
        deletion_state: row.deletion_state.clone(),
        retention_held: row.retention_held,
        retention_state: row.retention_state.clone(),
        retention_expires_at: row.retention_expires_at.map(|value| value.timestamp()),
        created_at: row.created_at.timestamp(),
    }
}

fn upload_to_owner_dto(row: &UploadRow) -> MyUploadDto {
    MyUploadDto {
        id: row.id.to_string(),
        kind: row.kind.clone(),
        usage: row.usage.clone(),
        bytes: row.bytes,
        mime: row.mime.clone(),
        status: row.status.clone(),
        image_width: row.image_width,
        image_height: row.image_height,
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

fn validate_upload_usage(kind: &str, usage: Option<MediaUsage>) -> AppResult<Option<&'static str>> {
    match usage {
        Some(_) if kind != "image" => {
            Err(MediaError::BadRequest("image media usage requires an image".into()).into())
        }
        Some(usage) => Ok(Some(usage.as_str())),
        None => Ok(None),
    }
}

fn validate_page_limit(limit: i64) -> AppResult<i64> {
    if (1..=100).contains(&limit) {
        Ok(limit)
    } else {
        Err(AppError::BadRequest("limit must be between 1 and 100".into()))
    }
}

fn validate_moderation_status(status: Option<&str>) -> AppResult<&str> {
    let status = status.unwrap_or("pending");
    if matches!(status, "pending" | "clean" | "quarantined" | "blocked") {
        Ok(status)
    } else {
        Err(AppError::BadRequest("invalid media moderation status".into()))
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
    reason: &str,
) -> AppResult<()> {
    let mut tx = state.db.begin().await?;
    let (owner_id, owner) = crate::locking::lock_upload_owner(&mut tx, upload_id).await?;
    let upload: Option<(String, i64, String, bool)> = sqlx::query_as(
        "SELECT upload.status, upload.account_id, upload.kind, \
                EXISTS ( \
                  SELECT 1 FROM media.moderation_evidence evidence \
                  WHERE evidence.upload_id = upload.id \
                    AND evidence.evidence_kind = 'trusted_image_preview' \
                    AND evidence.actor_account_id = $2 \
                ) \
         FROM media.uploads upload WHERE upload.id = $1",
    )
    .bind(upload_id)
    .bind(auth.id)
    .fetch_optional(&mut *tx)
    .await?;
    let (current_status, locked_owner_id, kind, has_reviewer_evidence) =
        upload.ok_or(MediaError::NotFound)?;
    if locked_owner_id != owner_id {
        return Err(AppError::Internal(anyhow::anyhow!("locked media owner changed")));
    }
    require_strictly_lower_owner(auth, owner_id, &owner.role)?;
    if current_status != "pending" {
        return Err(AppError::Conflict(format!("upload is already {current_status}")));
    }
    if kind != "image" {
        return Err(AppError::Conflict(
            "file approval requires malware and sandbox scanner evidence".into(),
        ));
    }
    if !has_reviewer_evidence {
        return Err(AppError::Conflict(
            "the approving moderator must complete a trusted image preview first".into(),
        ));
    }
    sqlx::query("UPDATE media.uploads SET status = 'clean' WHERE id = $1")
        .bind(upload_id)
        .execute(&mut *tx)
        .await?;
    let metadata = serde_json::json!({
        "oldStatus": current_status,
        "newStatus": "clean",
        "evidence": "trusted_image_preview",
    });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "media.upload.approved",
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
        "pending" => upload.account_id == auth.id,
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
    let usage = validate_upload_usage(&input.kind, input.usage)?;
    let content_type = oss::validate_content_type(&input.kind, &input.content_type)?;
    let oss_config = require_oss_config(&state)?;
    let callback_token = oss::new_callback_token();
    let intent = crate::reserve_upload_intent(
        &state.db,
        auth.id,
        &input.kind,
        content_type,
        usage,
        &callback_token,
    )
    .await?;
    let provider = AliyunStsProvider::default();
    let creds = oss::generate_sts_credentials(
        &provider,
        &oss_config,
        auth.id,
        intent.id,
        &intent.oss_key,
        &callback_token,
        intent.expires_at,
    )
    .await;
    let creds = match creds {
        Ok(creds) => creds,
        Err(error) => {
            if let Err(revoke_error) =
                repo::revoke_upload_intent_after_provider_failure(&state.db, intent.id).await
            {
                tracing::warn!(?revoke_error, intent_id = %intent.id, "failed to revoke upload intent after STS failure");
            }
            return Err(error.into());
        }
    };
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
    if !oss::verify_callback_token_hash(&intent.callback_token_hash, &input.callback_token) {
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
        intent.usage.as_deref(),
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
/// When CDN is configured, returns a signed CDN URL; otherwise falls back
/// to the direct OSS URL (which requires a public bucket or CDN origin).
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

    // Try CDN signed URL first.
    if let Some(cdn_config) = crate::cdn::CdnConfig::from_config(&state.config) {
        let path = format!("/uploads/{}", row.oss_key);
        if let Some(signed_url) =
            crate::cdn::sign_url(&cdn_config, &path, cdn_config.url_ttl_seconds)
        {
            return Ok(Json(UploadUrlDto { url: signed_url }));
        }
    }

    // Fallback: direct OSS URL (private bucket will return 403 without CDN).
    let url = oss::generate_url(&oss_config, &row.oss_key);
    Ok(Json(UploadUrlDto { url }))
}

/// GET /api/v2/me/media/uploads — list the current account's resumable upload states.
pub async fn list_my_uploads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MyUploadListQuery>,
) -> AppResult<Json<Page<MyUploadDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let limit = validate_page_limit(query.limit)?;
    let usage = query.usage.map(MediaUsage::as_str);
    let (rows, next_cursor) =
        repo::list_owned(&state.db, auth.id, usage, query.cursor.as_deref(), limit).await?;
    Ok(Json(Page::new(rows.iter().map(upload_to_owner_dto).collect(), next_cursor)))
}

/// GET /api/v2/me/media/uploads/{id} — poll one owned upload's moderation state.
pub async fn get_my_upload(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<MyUploadDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let upload_id = id_str.parse::<i64>().map_err(|_| AppError::NotFound)?;
    let upload = repo::find_owned_upload(&state.db, auth.id, upload_id)
        .await?
        .ok_or(MediaError::NotFound)?;
    Ok(Json(upload_to_owner_dto(&upload)))
}

async fn bind_profile_asset(
    state: &AppState,
    headers: &HeaderMap,
    input: ProfileAssetInput,
    kind: identity::profiles::ProfileAssetKind,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let asset_id = input.asset_id.parse::<i64>().map_err(|_| AppError::NotFound)?;
    let mut tx = state.db.begin().await?;
    identity::public_accounts::lock_active_account_for_owned_mutation(&mut tx, auth.id).await?;
    let binding_type = match kind {
        identity::profiles::ProfileAssetKind::Avatar => AssetBindingType::ProfileAvatar,
        identity::profiles::ProfileAssetKind::Banner => AssetBindingType::ProfileBanner,
    };
    sync_asset_binding(&mut tx, auth.id, binding_type, auth.id, Some(asset_id), "replaced").await?;
    identity::profiles::set_profile_asset(&mut tx, auth.id, kind, Some(asset_id)).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn clear_profile_asset(
    state: &AppState,
    headers: &HeaderMap,
    kind: identity::profiles::ProfileAssetKind,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let mut tx = state.db.begin().await?;
    identity::public_accounts::lock_active_account_for_owned_mutation(&mut tx, auth.id).await?;
    let binding_type = match kind {
        identity::profiles::ProfileAssetKind::Avatar => AssetBindingType::ProfileAvatar,
        identity::profiles::ProfileAssetKind::Banner => AssetBindingType::ProfileBanner,
    };
    sync_asset_binding(&mut tx, auth.id, binding_type, auth.id, None, "cleared").await?;
    identity::profiles::set_profile_asset(&mut tx, auth.id, kind, None).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/v2/me/profile/avatar
pub async fn bind_profile_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ProfileAssetInput>,
) -> AppResult<StatusCode> {
    bind_profile_asset(&state, &headers, input, identity::profiles::ProfileAssetKind::Avatar).await
}

/// DELETE /api/v2/me/profile/avatar
pub async fn clear_profile_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    clear_profile_asset(&state, &headers, identity::profiles::ProfileAssetKind::Avatar).await
}

/// PUT /api/v2/me/profile/banner
pub async fn bind_profile_banner(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ProfileAssetInput>,
) -> AppResult<StatusCode> {
    bind_profile_asset(&state, &headers, input, identity::profiles::ProfileAssetKind::Banner).await
}

/// DELETE /api/v2/me/profile/banner
pub async fn clear_profile_banner(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    clear_profile_asset(&state, &headers, identity::profiles::ProfileAssetKind::Banner).await
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

    let limit = validate_page_limit(q.limit)?;
    let status = validate_moderation_status(q.status.as_deref())?;
    let (rows, next_cursor) =
        repo::list_moderatable(&state.db, auth.id, &auth.role, status, q.cursor.as_deref(), limit)
            .await?;

    let items: Vec<UploadDto> = rows.iter().map(upload_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// GET /api/v2/admin/media/retention-holds — list operations-only hold details by expiry.
pub async fn list_retention_holds(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RetentionHoldListQuery>,
) -> AppResult<Response> {
    let auth_context = identity::auth_middleware::authenticate_context(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth_context
        .account
        .require_capability(shared::auth::Capability::RunOperations)
        .map_err(|_| AppError::Forbidden)?;
    let limit = validate_page_limit(query.limit)?;
    let hold_state = query.state.as_deref().unwrap_or("active");
    let mut transaction = state.db.begin().await?;
    identity::auth_middleware::require_recent_auth_tx(&auth_context, &mut transaction).await?;
    let page =
        crate::retention::list_holds(&mut transaction, hold_state, query.cursor.as_deref(), limit)
            .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        "media.retention_hold_inventory.viewed",
        "media_retention_inventory",
        hold_state,
        "authorized media retention inventory viewed",
        Some(&serde_json::json!({ "resultCount": page.items.len() })),
    )
    .await?;
    transaction.commit().await?;
    let mut response = Json(page).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("private, no-store"));
    response.headers_mut().insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    Ok(response)
}

/// GET /api/v2/admin/media/deletion-jobs — list durable system deletion work.
pub async fn list_deletion_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DeletionJobListQuery>,
) -> AppResult<Response> {
    let auth_context = identity::auth_middleware::authenticate_context(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth_context
        .account
        .require_capability(shared::auth::Capability::RunOperations)
        .map_err(|_| AppError::Forbidden)?;
    let limit = validate_page_limit(query.limit)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid media deletion job cursor".into()))?;
    let job_status = query.status.as_deref().unwrap_or("dead_letter");
    let mut transaction = state.db.begin().await?;
    identity::auth_middleware::require_recent_auth_tx(&auth_context, &mut transaction).await?;
    let page =
        crate::deletion::list_system_deletion_jobs(&mut transaction, job_status, cursor, limit)
            .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        "media.deletion_job_inventory.viewed",
        "media_deletion_inventory",
        job_status,
        "authorized media deletion inventory viewed",
        Some(&serde_json::json!({ "resultCount": page.items.len() })),
    )
    .await?;
    transaction.commit().await?;
    let mut response = Json(page).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("private, no-store"));
    response.headers_mut().insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    Ok(response)
}

/// POST /api/v2/admin/media/deletion-jobs/{id}/retry — requeue one exhausted system job.
pub async fn retry_deletion_job(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ModerateUploadInput>,
) -> AppResult<StatusCode> {
    let auth_context = identity::auth_middleware::authenticate_context(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth_context
        .account
        .require_capability(shared::auth::Capability::RunOperations)
        .map_err(|_| AppError::Forbidden)?;
    let job_id = id_str.parse::<i64>().map_err(|_| AppError::NotFound)?;
    let reason = validate_moderation_reason(&body.reason)?;
    crate::deletion::retry_system_deletion_job(&state.db, &auth_context, job_id, reason).await?;
    Ok(StatusCode::ACCEPTED)
}

/// POST /api/v2/admin/media/uploads/{id}/preview-grants — issue one short-lived read grant.
pub async fn create_upload_preview_grant(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ModerateUploadInput>,
) -> AppResult<Response> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let upload_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_moderation_reason(&body.reason)?;
    let grant = create_preview_grant(&state, &auth, upload_id, reason).await?;
    let mut response = Json(grant).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("private, no-store"));
    response.headers_mut().insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    Ok(response)
}

/// GET /api/v2/admin/media/uploads/{id}/preview — consume a grant and proxy image bytes.
pub async fn preview_upload(
    State(state): State<AppState>,
    Extension(object_store): Extension<Arc<dyn UploadObjectStore>>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let upload_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let token = headers
        .get(PREVIEW_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::NotFound)?;
    let preview =
        consume_preview_grant(&state, &auth, upload_id, token, object_store.as_ref()).await?;
    tracing::info!(upload_id, moderator_id = auth.id, "media preview authorized");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, preview.content_type)
        .header(header::CONTENT_LENGTH, preview.content_length)
        .header(header::CACHE_CONTROL, "private, no-store, max-age=0")
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .header(header::CONTENT_DISPOSITION, "inline")
        .header("x-content-type-options", "nosniff")
        .header("cross-origin-resource-policy", "same-origin")
        .header("content-security-policy", "default-src 'none'; sandbox")
        .body(preview.body)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))
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
    moderate_upload(&state, &auth, id, reason).await?;
    tracing::info!(upload_id = id, moderator_id = auth.id, "upload approved");

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/v2/admin/media/uploads/{id}/block — block a pending upload
pub async fn block_upload(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ModerateUploadInput>,
) -> AppResult<StatusCode> {
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
    schedule_upload_deletion(&state, &auth, id, reason).await?;

    tracing::info!(upload_id = id, moderator_id = auth.id, "upload quarantined for deletion");
    Ok(StatusCode::ACCEPTED)
}

/// POST /api/v2/admin/media/uploads/{id}/retention-hold — pause deletion for a bounded purpose.
pub async fn place_retention_hold(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RetentionHoldInput>,
) -> AppResult<StatusCode> {
    let auth_context = identity::auth_middleware::authenticate_context(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth_context
        .account
        .require_capability(shared::auth::Capability::RunOperations)
        .map_err(|_| AppError::Forbidden)?;
    let upload_id = id_str.parse::<i64>().map_err(|_| AppError::NotFound)?;
    crate::retention::place_hold(&state.db, &auth_context, upload_id, body).await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/v2/admin/media/uploads/{id}/retention-hold — release a deletion hold.
pub async fn release_retention_hold(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ReleaseRetentionHoldInput>,
) -> AppResult<StatusCode> {
    let auth_context = identity::auth_middleware::authenticate_context(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_response| AppError::Unauthorized)?;
    auth_context
        .account
        .require_capability(shared::auth::Capability::RunOperations)
        .map_err(|_| AppError::Forbidden)?;
    let upload_id = id_str.parse::<i64>().map_err(|_| AppError::NotFound)?;
    crate::retention::release_hold(
        &state.db,
        &auth_context,
        upload_id,
        &body.expected_hold_id,
        &body.reason,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use shared::AuthAccount;

    use super::{can_read_upload_url, validate_upload_usage, MediaUsage, UploadRow};
    use crate::moderation::require_strictly_lower_owner;

    fn account(id: i64, role: &str) -> AuthAccount {
        AuthAccount { id, role: role.into(), status: "active".into() }
    }

    fn upload(account_id: i64, status: &str) -> UploadRow {
        UploadRow {
            id: 1,
            account_id,
            kind: "image".into(),
            oss_key: "uploads/1/image/file.png".into(),
            bytes: 10,
            mime: "image/png".into(),
            status: status.into(),
            usage: None,
            image_width: None,
            image_height: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn pending_url_is_limited_to_owner_and_staff_must_use_audited_preview() {
        let pending = upload(10, "pending");
        assert!(can_read_upload_url(&account(10, "user"), &pending));
        assert!(!can_read_upload_url(&account(11, "user"), &pending));
        assert!(!can_read_upload_url(&account(11, "mod"), &pending));
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
    fn profile_usage_is_restricted_to_images() {
        assert!(validate_upload_usage("image", Some(MediaUsage::ProfileAvatar)).is_ok());
        assert!(validate_upload_usage("file", Some(MediaUsage::ProfileBanner)).is_err());
        assert!(validate_upload_usage("image", Some(MediaUsage::ForumThread)).is_ok());
        assert!(validate_upload_usage("file", Some(MediaUsage::ForumComment)).is_err());
    }

    #[test]
    fn staff_cannot_moderate_their_own_upload() {
        assert!(matches!(
            require_strictly_lower_owner(&account(10, "admin"), 10, "user"),
            Err(shared::AppError::Forbidden)
        ));
        assert!(require_strictly_lower_owner(&account(11, "mod"), 10, "user").is_ok());
        assert!(require_strictly_lower_owner(&account(11, "mod"), 12, "mod").is_err());
        assert!(require_strictly_lower_owner(&account(13, "admin"), 12, "mod").is_ok());
    }
}

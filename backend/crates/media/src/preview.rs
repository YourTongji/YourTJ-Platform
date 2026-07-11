//! One-time, audited moderation access to bounded OSS image evidence.

use base64::Engine;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult, AppState, AuthAccount};
use sqlx::FromRow;
use uuid::Uuid;

use crate::dto::ModerationPreviewGrantDto;
use crate::error::MediaError;
use crate::moderation::require_strictly_lower_owner;
use crate::oss;
use crate::quarantine::{UploadObjectPreview, UploadObjectStore};

pub(crate) const PREVIEW_TOKEN_HEADER: &str = "x-media-preview-token";
const PREVIEW_GRANT_TTL_SECONDS: i64 = 60;
const PREVIEW_MAX_BYTES: u64 = oss::OSS_UPLOAD_MAX_BYTES as u64;
const PREVIEW_MAX_DIMENSION: u32 = 20_000;
const PREVIEW_MAX_PIXELS: u64 = 40_000_000;

#[derive(Debug, FromRow)]
struct PreviewableUploadRow {
    account_id: i64,
    kind: String,
    mime: String,
    bytes: i64,
    status: String,
}

#[derive(Debug, FromRow)]
struct PreviewGrantRow {
    grant_id: i64,
    account_id: i64,
    oss_key: String,
    mime: String,
    bytes: i64,
    reason: String,
}

#[derive(Debug, FromRow)]
struct FinalizePreviewUploadRow {
    account_id: i64,
    status: String,
    image_width: Option<i32>,
    image_height: Option<i32>,
}

fn new_preview_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn preview_token_hash(token: &str) -> AppResult<String> {
    if token.len() != 43 {
        return Err(AppError::NotFound);
    }
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| AppError::NotFound)?;
    if decoded.len() != 32 {
        return Err(AppError::NotFound);
    }
    Ok(hex::encode(Sha256::digest(decoded)))
}

fn validate_previewable_upload(upload: &PreviewableUploadRow) -> AppResult<()> {
    if upload.status != "pending"
        || upload.kind != "image"
        || !(1..=PREVIEW_MAX_BYTES as i64).contains(&upload.bytes)
        || oss::validate_content_type("image", &upload.mime).is_err()
    {
        return Err(AppError::Conflict("upload is not available for image review".into()));
    }
    Ok(())
}

/// Issue a short-lived one-time grant without disclosing a provider URL or object key.
pub(crate) async fn create_preview_grant(
    state: &AppState,
    auth: &AuthAccount,
    upload_id: i64,
    reason: &str,
) -> AppResult<ModerationPreviewGrantDto> {
    let mut transaction = state.db.begin().await?;
    let (owner_id, owner) = crate::locking::lock_upload_owner(&mut transaction, upload_id).await?;
    let upload = sqlx::query_as::<_, PreviewableUploadRow>(
        "SELECT upload.account_id, upload.kind, upload.mime, \
                upload.bytes, upload.status \
         FROM media.uploads upload WHERE upload.id = $1",
    )
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(MediaError::NotFound)?;
    if upload.account_id != owner_id {
        return Err(AppError::Internal(anyhow::anyhow!("locked media owner changed")));
    }
    require_strictly_lower_owner(auth, owner_id, &owner.role)?;
    validate_previewable_upload(&upload)?;

    sqlx::query(
        "DELETE FROM media.moderation_preview_grants \
         WHERE expires_at < now() - interval '1 day'",
    )
    .execute(&mut *transaction)
    .await?;
    let token = new_preview_token();
    let token_hash = preview_token_hash(&token)?;
    let expires_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO media.moderation_preview_grants \
         (token_hash, upload_id, moderator_account_id, reason, expires_at) \
         VALUES ($1, $2, $3, $4, now() + ($5 * interval '1 second')) \
         RETURNING expires_at",
    )
    .bind(token_hash)
    .bind(upload_id)
    .bind(auth.id)
    .bind(reason)
    .bind(PREVIEW_GRANT_TTL_SECONDS)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(ModerationPreviewGrantDto { token, expires_at: expires_at.timestamp() })
}

/// Consume a one-time grant, append the evidence-read audit, then open a bounded provider stream.
pub(crate) async fn consume_preview_grant(
    state: &AppState,
    auth: &AuthAccount,
    upload_id: i64,
    token: &str,
    object_store: &dyn UploadObjectStore,
) -> AppResult<UploadObjectPreview> {
    let token_hash = preview_token_hash(token)?;
    let mut transaction = state.db.begin().await?;
    let (owner_id, owner) = crate::locking::lock_upload_owner(&mut transaction, upload_id).await?;
    let grant = sqlx::query_as::<_, PreviewGrantRow>(
        "SELECT preview_grant.id AS grant_id, upload.account_id, \
                upload.oss_key, upload.mime, upload.bytes, preview_grant.reason \
         FROM media.moderation_preview_grants preview_grant \
         JOIN media.uploads upload ON upload.id = preview_grant.upload_id \
         WHERE preview_grant.token_hash = $1 AND preview_grant.upload_id = $2 \
           AND preview_grant.moderator_account_id = $3 AND preview_grant.consumed_at IS NULL \
           AND preview_grant.expires_at > now() AND upload.status = 'pending' \
           AND upload.kind = 'image' AND upload.bytes BETWEEN 1 AND $4 \
         FOR UPDATE OF preview_grant",
    )
    .bind(token_hash)
    .bind(upload_id)
    .bind(auth.id)
    .bind(PREVIEW_MAX_BYTES as i64)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    if grant.account_id != owner_id {
        return Err(AppError::Internal(anyhow::anyhow!("locked media owner changed")));
    }
    require_strictly_lower_owner(auth, owner_id, &owner.role)?;
    if oss::validate_content_type("image", &grant.mime).is_err() {
        return Err(AppError::NotFound);
    }

    sqlx::query("UPDATE media.moderation_preview_grants SET consumed_at = now() WHERE id = $1")
        .bind(grant.grant_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    let preview = object_store
        .read_image_for_moderation(
            &grant.oss_key,
            &grant.mime,
            grant.bytes as u64,
            PREVIEW_MAX_BYTES,
        )
        .await
        .inspect_err(|error| {
            tracing::warn!(
                ?error,
                upload_id,
                moderator_id = auth.id,
                "media preview stream failed"
            );
        })?;
    let preview_pixels =
        u64::from(preview.image_width).saturating_mul(u64::from(preview.image_height));
    if preview.image_width == 0
        || preview.image_height == 0
        || preview.image_width > PREVIEW_MAX_DIMENSION
        || preview.image_height > PREVIEW_MAX_DIMENSION
        || preview_pixels > PREVIEW_MAX_PIXELS
    {
        return Err(AppError::BadRequest("image dimensions exceed preview limits".into()));
    }
    let preview_width = i32::try_from(preview.image_width)
        .map_err(|_| AppError::BadRequest("invalid preview width".into()))?;
    let preview_height = i32::try_from(preview.image_height)
        .map_err(|_| AppError::BadRequest("invalid preview height".into()))?;
    let mut transaction = state.db.begin().await?;
    let (owner_id, owner) = crate::locking::lock_upload_owner(&mut transaction, upload_id).await?;
    let upload = sqlx::query_as::<_, FinalizePreviewUploadRow>(
        "SELECT upload.account_id, upload.status, \
                upload.image_width, upload.image_height \
         FROM media.uploads upload WHERE upload.id = $1",
    )
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(MediaError::NotFound)?;
    if upload.account_id != owner_id {
        return Err(AppError::Internal(anyhow::anyhow!("locked media owner changed")));
    }
    require_strictly_lower_owner(auth, owner_id, &owner.role)?;
    if upload.status != "pending" {
        return Err(AppError::Conflict("upload left the pending review state".into()));
    }
    match (upload.image_width, upload.image_height) {
        (Some(width), Some(height)) if width == preview_width && height == preview_height => {}
        (None, None) => {
            sqlx::query(
                "UPDATE media.uploads SET image_width = $2, image_height = $3 WHERE id = $1",
            )
            .bind(upload_id)
            .bind(preview_width)
            .bind(preview_height)
            .execute(&mut *transaction)
            .await?;
        }
        _ => {
            return Err(AppError::Conflict(
                "preview dimensions do not match stored evidence".into(),
            ));
        }
    }
    let metadata = serde_json::json!({
        "purpose": "moderation_review",
        "contentType": grant.mime,
        "declaredBytes": grant.bytes,
        "imageWidth": preview.image_width,
        "imageHeight": preview.image_height,
    });
    sqlx::query(
        "INSERT INTO media.moderation_evidence \
         (upload_id, evidence_kind, verdict, actor_account_id, observed_mime, \
          image_width, image_height) \
         VALUES ($1, 'trusted_image_preview', 'observed', $2, $3, $4, $5)",
    )
    .bind(upload_id)
    .bind(auth.id)
    .bind(&grant.mime)
    .bind(preview_width)
    .bind(preview_height)
    .execute(&mut *transaction)
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "media.upload.previewed",
        "upload",
        &upload_id.to_string(),
        &grant.reason,
        Some(&metadata),
    )
    .await?;
    transaction.commit().await?;
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::preview_token_hash;

    #[test]
    fn preview_tokens_are_canonical_and_bounded() {
        let token = super::new_preview_token();
        assert_eq!(token.len(), 43);
        assert_eq!(preview_token_hash(&token).expect("preview token hash").len(), 64);
        assert!(preview_token_hash("not-a-grant").is_err());
    }
}

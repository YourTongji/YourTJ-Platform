//! Atomic upload-intent reservation and database-authoritative abuse controls.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::MediaError;
use crate::{oss, repo};

/// Persisted intent facts needed to request one exact-key STS credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadIntentReservation {
    pub id: Uuid,
    pub oss_key: String,
    pub expires_at: DateTime<Utc>,
}

/// Result of atomically consuming one verified OSS callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadCallbackCompletion {
    pub upload_id: i64,
    pub was_auto_approved: bool,
}

fn validate_usage(kind: &str, usage: Option<&str>) -> AppResult<()> {
    if !matches!(kind, "image" | "file") {
        return Err(AppError::BadRequest("invalid upload kind".into()));
    }
    if let Some(usage) = usage {
        if kind != "image"
            || !matches!(
                usage,
                "profile_avatar" | "profile_banner" | "forum_thread" | "forum_comment"
            )
        {
            return Err(AppError::BadRequest("invalid media usage for upload kind".into()));
        }
    }
    Ok(())
}

/// Reserve quota and persist only the callback digest in the same account-locked transaction.
pub async fn reserve_upload_intent(
    pool: &PgPool,
    account_id: i64,
    kind: &str,
    content_type: &str,
    usage: Option<&str>,
    callback_token: &str,
) -> AppResult<UploadIntentReservation> {
    validate_usage(kind, usage)?;
    let content_type = oss::validate_content_type(kind, content_type)?;
    let intent_id = Uuid::new_v4();
    let oss_key = oss::build_oss_key(account_id, kind, content_type, intent_id);
    let expires_at = oss::upload_intent_expires_at();
    let callback_token_hash = oss::callback_token_hash(callback_token);

    let mut transaction = pool.begin().await?;
    identity::public_accounts::lock_active_account_for_owned_mutation(&mut transaction, account_id)
        .await?;
    repo::consume_upload_credential_quota(&mut transaction, account_id, oss::OSS_UPLOAD_MAX_BYTES)
        .await?;
    let intent = repo::insert_upload_intent(
        &mut transaction,
        intent_id,
        account_id,
        kind,
        &oss_key,
        content_type,
        usage,
        oss::OSS_UPLOAD_MAX_BYTES,
        &callback_token_hash,
        expires_at,
    )
    .await?;
    transaction.commit().await?;

    Ok(UploadIntentReservation {
        id: intent.id,
        oss_key: intent.oss_key,
        expires_at: intent.expires_at,
    })
}

/// Consume a callback token and exact object metadata after the HTTP edge verifies OSS signing.
///
/// Callback replay returns the original upload without repeating moderation, processing enqueue,
/// or audit. The automatic policy is limited to supported raster images; every other upload stays
/// pending.
#[allow(clippy::too_many_arguments)] // reason: every provider callback fact is verified against the locked intent
pub async fn complete_upload_callback(
    pool: &PgPool,
    intent_id: Uuid,
    callback_token: &str,
    oss_key: &str,
    bytes: i64,
    mime: &str,
    sha256: &str,
    is_image_auto_approval_enabled: bool,
) -> AppResult<UploadCallbackCompletion> {
    let mut transaction = pool.begin().await?;
    let intent =
        repo::lock_upload_intent(&mut transaction, intent_id).await?.ok_or(MediaError::NotFound)?;
    if !oss::verify_callback_token_hash(&intent.callback_token_hash, callback_token) {
        return Err(MediaError::BadRequest("upload intent mismatch".into()).into());
    }
    oss::validate_callback_metadata(
        &intent.oss_key,
        &intent.content_type,
        intent.max_bytes,
        oss_key,
        bytes,
        mime,
        sha256,
    )?;
    if let Some(upload_id) = intent.upload_id {
        transaction.commit().await?;
        return Ok(UploadCallbackCompletion { upload_id, was_auto_approved: false });
    }
    if intent.expires_at <= Utc::now() {
        return Err(MediaError::BadRequest("upload intent expired".into()).into());
    }
    let upload = repo::insert_upload_in_tx(
        &mut transaction,
        intent.account_id,
        &intent.kind,
        &intent.oss_key,
        bytes,
        &intent.content_type,
        sha256,
        intent.usage.as_deref(),
    )
    .await?;
    repo::consume_upload_intent(&mut transaction, intent.id, upload.id).await?;
    let was_auto_approved = crate::approval::apply_callback_policy(
        &mut transaction,
        upload.id,
        &upload.kind,
        &upload.mime,
        is_image_auto_approval_enabled,
    )
    .await?;
    transaction.commit().await?;

    Ok(UploadCallbackCompletion { upload_id: upload.id, was_auto_approved })
}

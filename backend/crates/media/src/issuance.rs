//! Atomic upload-intent reservation and database-authoritative abuse controls.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{oss, repo};

/// Persisted intent facts needed to request one exact-key STS credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadIntentReservation {
    pub id: Uuid,
    pub oss_key: String,
    pub expires_at: DateTime<Utc>,
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

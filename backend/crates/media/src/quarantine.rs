//! Upload quarantine service and its object-store boundary.

use shared::{AppError, AppResult, AppState, AuthAccount};

use crate::error::MediaError;
use crate::oss::{AliyunOssClient, ObjectStore, OssConfig};

/// Object deletion boundary used by upload moderation.
#[async_trait::async_trait]
pub trait UploadObjectStore: Send + Sync {
    /// Permanently remove one account-scoped OSS object.
    async fn delete_object(&self, oss_key: &str) -> AppResult<()>;
}

pub(crate) struct AliyunUploadObjectStore {
    client: AliyunOssClient,
    config: Option<OssConfig>,
}

impl AliyunUploadObjectStore {
    pub(crate) fn from_config(config: &shared::Config) -> Self {
        Self { client: AliyunOssClient::default(), config: OssConfig::from_config(config) }
    }
}

#[async_trait::async_trait]
impl UploadObjectStore for AliyunUploadObjectStore {
    async fn delete_object(&self, oss_key: &str) -> AppResult<()> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| MediaError::Unavailable("oss is not configured".into()))?;
        self.client.delete_object(config, oss_key).await?;
        Ok(())
    }
}

/// Delete a rejected object before committing its blocked state and governance audit event.
///
/// The upload remains `pending` when object deletion fails, so a blocked database row never
/// leaves a still-public object behind.
pub(crate) async fn quarantine_upload(
    state: &AppState,
    auth: &AuthAccount,
    upload_id: i64,
    reason: &str,
    object_store: &dyn UploadObjectStore,
) -> AppResult<()> {
    let mut tx = state.db.begin().await?;
    let upload: Option<(String, String, i64)> = sqlx::query_as(
        "SELECT status, oss_key, account_id FROM media.uploads WHERE id = $1 FOR UPDATE",
    )
    .bind(upload_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (current_status, oss_key, owner_id) = upload.ok_or(MediaError::NotFound)?;
    require_independent_moderator(auth, owner_id)?;
    if current_status != "pending" {
        return Err(AppError::Conflict(format!("upload is already {current_status}")));
    }

    object_store.delete_object(&oss_key).await?;

    sqlx::query("UPDATE media.uploads SET status = 'blocked' WHERE id = $1")
        .bind(upload_id)
        .execute(&mut *tx)
        .await?;
    let metadata = serde_json::json!({ "oldStatus": current_status, "newStatus": "blocked" });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "media.upload.blocked",
        "upload",
        &upload_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) fn require_independent_moderator(auth: &AuthAccount, owner_id: i64) -> AppResult<()> {
    if auth.id == owner_id {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

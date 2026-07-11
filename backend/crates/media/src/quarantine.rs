//! Upload quarantine service and its object-store boundary.

use axum::body::Body;
use shared::AppResult;

use crate::error::MediaError;
use crate::oss::{AliyunOssClient, ObjectStore, OssConfig};

/// Object deletion boundary used by upload moderation.
#[async_trait::async_trait]
pub trait UploadObjectStore: Send + Sync {
    /// Permanently remove one account-scoped OSS object.
    async fn delete_object(&self, oss_key: &str) -> AppResult<()>;

    /// Stream one bounded image through the authenticated application boundary for moderation.
    async fn read_image_for_moderation(
        &self,
        _oss_key: &str,
        _expected_content_type: &str,
        _expected_bytes: u64,
        _max_bytes: u64,
    ) -> AppResult<UploadObjectPreview> {
        Err(MediaError::Unavailable("moderation preview is not configured".into()).into())
    }
}

/// Bounded provider response streamed only through a same-origin moderation endpoint.
pub struct UploadObjectPreview {
    /// Allowlisted image MIME returned by the provider.
    pub content_type: String,
    /// Provider length proven equal to the callback evidence and hard limit.
    pub content_length: u64,
    /// Width parsed from the allowlisted image header before any bytes are disclosed.
    pub image_width: u32,
    /// Height parsed from the allowlisted image header before any bytes are disclosed.
    pub image_height: u32,
    /// Bounded same-origin response body; provider identifiers remain private.
    pub body: Body,
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

    async fn read_image_for_moderation(
        &self,
        oss_key: &str,
        expected_content_type: &str,
        expected_bytes: u64,
        max_bytes: u64,
    ) -> AppResult<UploadObjectPreview> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| MediaError::Unavailable("oss is not configured".into()))?;
        let object = self
            .client
            .read_object(config, oss_key, expected_content_type, expected_bytes, max_bytes)
            .await?;
        Ok(UploadObjectPreview {
            content_type: object.content_type,
            content_length: object.content_length,
            image_width: object.image_width,
            image_height: object.image_height,
            body: object.body,
        })
    }
}

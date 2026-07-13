//! Upload quarantine service and its object-store boundary.

use axum::body::Body;
use shared::AppResult;

use crate::delivery::DeliveryConfig;
use crate::error::MediaError;
use crate::oss::{AliyunOssClient, ObjectStore, OssConfig};

/// Provider-normalized completion state for one persisted CDN purge task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryPurgeTaskState {
    Complete,
    Refreshing,
    Failed,
}

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

    /// Read the private Ingest source into a bounded worker buffer.
    async fn read_image_for_processing(
        &self,
        _oss_key: &str,
        _expected_content_type: &str,
        _expected_bytes: u64,
        _max_bytes: u64,
    ) -> AppResult<Vec<u8>> {
        Err(MediaError::Unavailable("media processing is not configured".into()).into())
    }

    /// Create one immutable private Delivery object.
    async fn put_delivery_object(
        &self,
        _object_key: &str,
        _content_type: &str,
        _bytes: Vec<u8>,
    ) -> AppResult<()> {
        Err(MediaError::Unavailable("media Delivery is not configured".into()).into())
    }

    /// Verify the exact private Delivery object written by a retryable worker.
    async fn head_delivery_object(
        &self,
        _object_key: &str,
        _content_type: &str,
        _expected_bytes: u64,
        _expected_sha256: &str,
    ) -> AppResult<()> {
        Err(MediaError::Unavailable("media Delivery is not configured".into()).into())
    }

    /// Remove one private Delivery object while preserving a held Ingest original.
    async fn delete_delivery_object(&self, _object_key: &str) -> AppResult<()> {
        Err(MediaError::Unavailable("media Delivery is not configured".into()).into())
    }

    /// Submit one force-purge task and return its provider task id for durable polling.
    async fn submit_delivery_purge(&self, _object_key: &str) -> AppResult<String> {
        Err(MediaError::Unavailable("media CDN purge is not configured".into()).into())
    }

    /// Query one persisted purge task without disclosing its provider id outside Media.
    async fn delivery_purge_task_state(
        &self,
        _provider_task_id: &str,
    ) -> AppResult<DeliveryPurgeTaskState> {
        Err(MediaError::Unavailable("media CDN purge is not configured".into()).into())
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
    ingest_config: Option<OssConfig>,
    delivery_config: Option<DeliveryConfig>,
    provider_client: Option<reqwest::Client>,
}

impl AliyunUploadObjectStore {
    pub(crate) fn from_config(config: &shared::Config) -> Self {
        let delivery_config = match DeliveryConfig::from_env(&config.oss_region) {
            Ok(delivery_config) => delivery_config,
            Err(error) => {
                tracing::warn!(?error, "media Delivery configuration rejected");
                None
            }
        };
        let provider_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| {
                tracing::warn!(component = "cdn_purge", "media provider client build failed");
            })
            .ok();
        Self {
            client: AliyunOssClient::default(),
            ingest_config: OssConfig::from_config(config),
            delivery_config,
            provider_client,
        }
    }

    fn ingest_config(&self) -> AppResult<&OssConfig> {
        self.ingest_config.as_ref().ok_or_else(|| {
            MediaError::Unavailable("media Ingest OSS is not configured".into()).into()
        })
    }

    fn delivery_oss_config(&self) -> AppResult<OssConfig> {
        let delivery = self.delivery_config.as_ref().ok_or_else(|| {
            MediaError::Unavailable("media Delivery OSS is not configured".into())
        })?;
        Ok(OssConfig {
            region: delivery.region.clone(),
            bucket: delivery.bucket.clone(),
            access_key_id: delivery.access_key_id.clone(),
            access_key_secret: delivery.access_key_secret.clone(),
            role_arn: String::new(),
            callback_base_url: String::new(),
        })
    }
}

#[async_trait::async_trait]
impl UploadObjectStore for AliyunUploadObjectStore {
    async fn delete_object(&self, oss_key: &str) -> AppResult<()> {
        let config = self.ingest_config()?;
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
        let config = self.ingest_config()?;
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

    async fn read_image_for_processing(
        &self,
        oss_key: &str,
        expected_content_type: &str,
        expected_bytes: u64,
        max_bytes: u64,
    ) -> AppResult<Vec<u8>> {
        Ok(self
            .client
            .read_object_bytes(
                self.ingest_config()?,
                oss_key,
                expected_content_type,
                expected_bytes,
                max_bytes,
            )
            .await?)
    }

    async fn put_delivery_object(
        &self,
        object_key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> AppResult<()> {
        self.client
            .put_object(&self.delivery_oss_config()?, object_key, content_type, bytes)
            .await?;
        Ok(())
    }

    async fn head_delivery_object(
        &self,
        object_key: &str,
        content_type: &str,
        expected_bytes: u64,
        expected_sha256: &str,
    ) -> AppResult<()> {
        self.client
            .head_object(
                &self.delivery_oss_config()?,
                object_key,
                content_type,
                expected_bytes,
                expected_sha256,
            )
            .await?;
        Ok(())
    }

    async fn delete_delivery_object(&self, object_key: &str) -> AppResult<()> {
        self.client.delete_object(&self.delivery_oss_config()?, object_key).await?;
        Ok(())
    }

    async fn submit_delivery_purge(&self, object_key: &str) -> AppResult<String> {
        let delivery = self.delivery_config.as_ref().ok_or_else(|| {
            MediaError::Unavailable("media Delivery CDN is not configured".into())
        })?;
        let client = self.provider_client.as_ref().ok_or_else(|| {
            MediaError::Unavailable("media CDN purge client is unavailable".into())
        })?;
        Ok(delivery.submit_purge(client, object_key).await?)
    }

    async fn delivery_purge_task_state(
        &self,
        provider_task_id: &str,
    ) -> AppResult<DeliveryPurgeTaskState> {
        let delivery = self.delivery_config.as_ref().ok_or_else(|| {
            MediaError::Unavailable("media Delivery CDN is not configured".into())
        })?;
        let client = self.provider_client.as_ref().ok_or_else(|| {
            MediaError::Unavailable("media CDN purge client is unavailable".into())
        })?;
        Ok(delivery.purge_task_state(client, provider_task_id).await?)
    }
}

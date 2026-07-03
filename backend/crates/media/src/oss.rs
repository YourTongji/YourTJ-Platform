//! OSS / STS integration for the media domain.
//!
//! At present these are placeholder implementations. When a real OSS SDK is
//! integrated, replace the bodies while keeping the public signatures stable.

use crate::dto::UploadCredentialsDto;

/// Configuration for OSS (Object Storage Service) access.
#[derive(Debug, Clone)]
pub struct OssConfig {
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    #[allow(dead_code)]
    pub role_arn: String,
}

/// Generate placeholder STS credentials for direct upload.
///
/// In production this would call STS.assume_role and return real temp credentials.
/// For local development the returned values are deterministic and stable.
pub fn generate_sts_credentials(config: &OssConfig, account_id: i64) -> UploadCredentialsDto {
    UploadCredentialsDto {
        access_key_id: config.access_key_id.clone(),
        access_key_secret: config.access_key_secret.clone(),
        security_token: format!("placeholder-sts-token-{account_id}"),
        region: config.region.clone(),
        bucket: config.bucket.clone(),
        prefix: format!("uploads/{account_id}/"),
        expiration: "2099-12-31T23:59:59Z".into(),
    }
}

/// Verify the OSS callback signature.
///
/// Currently returns `true` unconditionally. In production this should validate
/// the `Authorization` header per Alibaba Cloud OSS callback auth spec.
pub fn verify_callback_signature(
    _config: &OssConfig,
    _headers: &axum::http::HeaderMap,
    _body: &[u8],
) -> bool {
    true
}

/// Generate a CDN / signed URL for an OSS object.
///
/// Currently returns the direct URL as stored. In production this would generate
/// a time-limited signed URL or CDN URL.
pub fn generate_url(config: &OssConfig, oss_key: &str) -> String {
    format!("https://{}.{}.aliyuncs.com/{}", config.bucket, config.region, oss_key)
}

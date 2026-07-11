//! OSS / STS integration for the media domain.

use std::collections::BTreeMap;

use axum::http::{HeaderMap, Uri};
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Verifier;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Deserialize;
use sha1::Sha1;
use uuid::Uuid;

use crate::dto::UploadCredentialsDto;
use crate::error::MediaError;

const OSS_INTENT_TTL_SECONDS: i64 = 900;
const OSS_POLICY_MAX_BYTES: i64 = OSS_UPLOAD_MAX_BYTES;
pub const OSS_UPLOAD_MAX_BYTES: i64 = 20 * 1024 * 1024;
const OSS_PUBLIC_KEY_MAX_BYTES: usize = 16 * 1024;
const OSS_HTTP_TIMEOUT_SECONDS: u64 = 5;
const OSS_CALLBACK_PUBLIC_KEY_HOST: &str = "gosspublic.alicdn.com";
const PERCENT_ENCODE_SET: &AsciiSet =
    &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');

type HmacSha1 = Hmac<Sha1>;

/// Configuration for OSS and STS access.
#[derive(Debug, Clone)]
pub struct OssConfig {
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub role_arn: String,
    pub callback_base_url: String,
}

/// Temporary credentials returned by Alibaba Cloud STS.
#[derive(Debug, Clone)]
pub struct StsCredentials {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub security_token: String,
    pub expiration: DateTime<Utc>,
}

/// Provider abstraction for STS, allowing protocol-vector tests without network access.
#[async_trait::async_trait]
pub trait StsProvider: Send + Sync {
    async fn assume_upload_role(
        &self,
        config: &OssConfig,
        session_name: &str,
        policy: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<StsCredentials, MediaError>;
}

/// Real Alibaba Cloud STS provider.
pub struct AliyunStsProvider {
    client: reqwest::Client,
}

/// Object deletion boundary used by moderation to quarantine rejected uploads.
#[async_trait::async_trait]
pub trait ObjectStore: Send + Sync {
    async fn delete_object(&self, config: &OssConfig, oss_key: &str) -> Result<(), MediaError>;
}

/// Authenticated Alibaba Cloud OSS client for moderation operations.
pub struct AliyunOssClient {
    client: reqwest::Client,
}

impl Default for AliyunOssClient {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(OSS_HTTP_TIMEOUT_SECONDS))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

#[async_trait::async_trait]
impl ObjectStore for AliyunOssClient {
    async fn delete_object(&self, config: &OssConfig, oss_key: &str) -> Result<(), MediaError> {
        let signed = build_delete_object_request(config, oss_key, Utc::now())?;
        let response = self
            .client
            .delete(signed.url)
            .header("Date", signed.date)
            .header("Authorization", signed.authorization)
            .send()
            .await
            .map_err(|error| {
                MediaError::Unavailable(format!("oss delete request failed: {error}"))
            })?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(())
        } else {
            tracing::warn!(status = %response.status(), "oss delete object failed");
            Err(MediaError::Unavailable("oss object quarantine failed".into()))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DeleteObjectRequest {
    url: String,
    date: String,
    authorization: String,
}

impl Default for AliyunStsProvider {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(OSS_HTTP_TIMEOUT_SECONDS))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

#[async_trait::async_trait]
impl StsProvider for AliyunStsProvider {
    async fn assume_upload_role(
        &self,
        config: &OssConfig,
        session_name: &str,
        policy: &str,
        _expires_at: DateTime<Utc>,
    ) -> Result<StsCredentials, MediaError> {
        let url = build_assume_role_url(config, session_name, policy, Utc::now())?;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| MediaError::Unavailable(format!("sts request failed: {error}")))?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "sts assume role failed");
            return Err(MediaError::Unavailable("sts unavailable".into()));
        }
        let payload: AssumeRoleResponse = response
            .json()
            .await
            .map_err(|error| MediaError::Unavailable(format!("invalid sts response: {error}")))?;
        let expiration = DateTime::parse_from_rfc3339(&payload.credentials.expiration)
            .map_err(|error| MediaError::Unavailable(format!("invalid sts expiration: {error}")))?
            .with_timezone(&Utc);
        Ok(StsCredentials {
            access_key_id: payload.credentials.access_key_id,
            access_key_secret: payload.credentials.access_key_secret,
            security_token: payload.credentials.security_token,
            expiration,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AssumeRoleResponse {
    credentials: AssumeRoleCredentials,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AssumeRoleCredentials {
    access_key_id: String,
    access_key_secret: String,
    security_token: String,
    expiration: String,
}

impl OssConfig {
    /// Build OSS config from runtime config, returning `None` when media is disabled.
    pub fn from_config(config: &shared::Config) -> Option<Self> {
        let oss_config = Self {
            region: config.oss_region.trim().to_string(),
            bucket: config.oss_bucket.trim().to_string(),
            access_key_id: config.oss_access_key_id.trim().to_string(),
            access_key_secret: config.oss_access_key_secret.trim().to_string(),
            role_arn: config.oss_role_arn.trim().to_string(),
            callback_base_url: config
                .oss_callback_base_url
                .trim()
                .trim_end_matches('/')
                .to_string(),
        };
        if oss_config.is_enabled() {
            Some(oss_config)
        } else {
            None
        }
    }

    fn is_enabled(&self) -> bool {
        !self.region.is_empty()
            && !self.bucket.is_empty()
            && !self.access_key_id.is_empty()
            && !self.access_key_secret.is_empty()
            && !self.role_arn.is_empty()
            && !self.callback_base_url.is_empty()
    }
}

/// Returns the expiry timestamp for a newly issued upload intent.
pub fn upload_intent_expires_at() -> DateTime<Utc> {
    Utc::now() + Duration::seconds(OSS_INTENT_TTL_SECONDS)
}

/// Validate and normalize a supported upload content type for a media kind.
pub fn validate_content_type(kind: &str, content_type: &str) -> Result<&'static str, MediaError> {
    match (kind, content_type.trim().to_ascii_lowercase().as_str()) {
        ("image", "image/jpeg") => Ok("image/jpeg"),
        ("image", "image/png") => Ok("image/png"),
        ("image", "image/gif") => Ok("image/gif"),
        ("image", "image/webp") => Ok("image/webp"),
        ("file", "application/pdf") => Ok("application/pdf"),
        _ => Err(MediaError::BadRequest("unsupported upload content type".into())),
    }
}

/// Validate callback metadata against a server-issued upload intent.
pub fn validate_callback_metadata(
    expected_key: &str,
    expected_content_type: &str,
    max_bytes: i64,
    oss_key: &str,
    bytes: i64,
    mime: &str,
    sha256: &str,
) -> Result<(), MediaError> {
    let is_sha256 = sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
    if oss_key != expected_key
        || mime != expected_content_type
        || bytes <= 0
        || bytes > max_bytes
        || !is_sha256
    {
        return Err(MediaError::BadRequest("upload callback metadata mismatch".into()));
    }
    Ok(())
}

/// Maximum accepted callback public-key document size.
pub fn callback_public_key_max_bytes() -> usize {
    OSS_PUBLIC_KEY_MAX_BYTES
}

/// Timeout for Alibaba Cloud callback public-key retrieval.
pub fn callback_http_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(OSS_HTTP_TIMEOUT_SECONDS)
}

/// Build an account-scoped OSS object key.
pub fn build_oss_key(account_id: i64, kind: &str, content_type: &str, intent_id: Uuid) -> String {
    let extension = extension_for_content_type(content_type);
    format!("uploads/{account_id}/{kind}/{intent_id}.{extension}")
}

/// Build an opaque callback token.
pub fn new_callback_token() -> String {
    let bytes = *Uuid::new_v4().as_bytes();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Generate real STS credentials constrained to the exact upload key prefix.
pub async fn generate_sts_credentials(
    provider: &dyn StsProvider,
    config: &OssConfig,
    account_id: i64,
    intent_id: Uuid,
    oss_key: &str,
    callback_token: &str,
    expires_at: DateTime<Utc>,
) -> Result<UploadCredentialsDto, MediaError> {
    let policy = build_upload_policy(config, oss_key);
    let session_name = format!("yourtj-upload-{account_id}-{intent_id}");
    let credentials =
        provider.assume_upload_role(config, &session_name, &policy, expires_at).await?;
    Ok(UploadCredentialsDto {
        upload_intent_id: intent_id.to_string(),
        access_key_id: credentials.access_key_id,
        access_key_secret: credentials.access_key_secret,
        security_token: credentials.security_token,
        region: config.region.clone(),
        bucket: config.bucket.clone(),
        prefix: account_prefix(account_id),
        oss_key: oss_key.to_string(),
        callback_url: format!("{}/api/v2/media/callback", config.callback_base_url),
        callback_body: build_callback_body(intent_id, callback_token),
        expiration: credentials.expiration.timestamp(),
    })
}

/// Verify the OSS callback signature using Alibaba Cloud's callback canonical string.
pub fn verify_callback_signature(
    headers: &HeaderMap,
    uri: &Uri,
    body: &[u8],
    public_key_pem: &str,
) -> Result<(), MediaError> {
    let signature = callback_signature(headers)?;
    let canonical = build_callback_canonical_string(uri, body)?;
    let public_key = PKey::public_key_from_pem(public_key_pem.as_bytes())
        .map_err(|_| MediaError::BadRequest("invalid callback public key".into()))?;
    let mut verifier = Verifier::new(MessageDigest::md5(), &public_key)
        .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?;
    verifier
        .update(canonical.as_bytes())
        .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?;
    match verifier.verify(&signature) {
        Ok(true) => Ok(()),
        Ok(false) => Err(MediaError::BadRequest("invalid callback signature".into())),
        Err(error) => Err(MediaError::Internal(anyhow::Error::new(error))),
    }
}

/// Decode and validate the callback public-key URL from OSS headers.
pub fn callback_public_key_url(headers: &HeaderMap) -> Result<String, MediaError> {
    let encoded = header_str(headers, "x-oss-pub-key-url")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| MediaError::BadRequest("invalid callback public key url".into()))?;
    let url = String::from_utf8(decoded)
        .map_err(|_| MediaError::BadRequest("invalid callback public key url".into()))?;
    validate_public_key_url(&url)?;
    Ok(url)
}

/// Generate a direct OSS URL for an object.
pub fn generate_url(config: &OssConfig, oss_key: &str) -> String {
    format!("https://{}.oss-{}.aliyuncs.com/{}", config.bucket, config.region, oss_key)
}

fn build_delete_object_request(
    config: &OssConfig,
    oss_key: &str,
    request_time: DateTime<Utc>,
) -> Result<DeleteObjectRequest, MediaError> {
    if oss_key.is_empty() || oss_key.starts_with('/') || oss_key.contains("..") {
        return Err(MediaError::BadRequest("invalid oss object key".into()));
    }
    let date = request_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let canonical_resource = format!("/{}/{}", config.bucket, oss_key);
    let string_to_sign = format!("DELETE\n\n\n{date}\n{canonical_resource}");
    let mut mac = HmacSha1::new_from_slice(config.access_key_secret.as_bytes())
        .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    Ok(DeleteObjectRequest {
        url: generate_url(config, oss_key),
        date,
        authorization: format!("OSS {}:{signature}", config.access_key_id),
    })
}

/// Build the least-privilege STS policy for one object.
pub fn build_upload_policy(config: &OssConfig, oss_key: &str) -> String {
    serde_json::json!({
        "Version": "1",
        "Statement": [{
            "Effect": "Allow",
            "Action": ["oss:PutObject"],
            "Resource": [format!("acs:oss:*:*:{}/{}", config.bucket, oss_key)],
            "Condition": {
                "NumericLessThanEquals": { "oss:ContentLength": OSS_POLICY_MAX_BYTES }
            }
        }]
    })
    .to_string()
}

fn build_assume_role_url(
    config: &OssConfig,
    session_name: &str,
    policy: &str,
    request_time: DateTime<Utc>,
) -> Result<String, MediaError> {
    let mut params = BTreeMap::from([
        ("AccessKeyId".to_string(), config.access_key_id.clone()),
        ("Action".to_string(), "AssumeRole".to_string()),
        ("DurationSeconds".to_string(), OSS_INTENT_TTL_SECONDS.to_string()),
        ("Format".to_string(), "JSON".to_string()),
        ("Policy".to_string(), policy.to_string()),
        ("RoleArn".to_string(), config.role_arn.clone()),
        ("RoleSessionName".to_string(), session_name.to_string()),
        ("SignatureMethod".to_string(), "HMAC-SHA1".to_string()),
        ("SignatureNonce".to_string(), Uuid::new_v4().to_string()),
        ("SignatureVersion".to_string(), "1.0".to_string()),
        ("Timestamp".to_string(), request_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        ("Version".to_string(), "2015-04-01".to_string()),
    ]);
    let canonical_parameters = canonical_query(&params);
    let string_to_sign = format!("GET&%2F&{}", percent_encode(&canonical_parameters));
    let mut mac = HmacSha1::new_from_slice(format!("{}&", config.access_key_secret).as_bytes())
        .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    params.insert("Signature".to_string(), signature);
    Ok(format!("https://sts.aliyuncs.com/?{}", canonical_query(&params)))
}

fn canonical_query(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    utf8_percent_encode(value, PERCENT_ENCODE_SET).to_string()
}

fn account_prefix(account_id: i64) -> String {
    format!("uploads/{account_id}/")
}

fn build_callback_body(intent_id: Uuid, callback_token: &str) -> String {
    format!(
        r#"{{"uploadIntentId":"{}","callbackToken":"{}","ossKey":"${{object}}","url":"${{bucket}}.${{host}}/${{object}}","bytes":${{size}},"mime":"${{mimeType}}","sha256":"${{x:sha256}}"}}"#,
        intent_id, callback_token
    )
}

fn extension_for_content_type(content_type: &str) -> &'static str {
    match content_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "application/pdf" => "pdf",
        _ => "bin",
    }
}

fn callback_signature(headers: &HeaderMap) -> Result<Vec<u8>, MediaError> {
    let header = header_str(headers, "authorization")?;
    base64::engine::general_purpose::STANDARD
        .decode(header)
        .map_err(|_| MediaError::BadRequest("invalid callback signature".into()))
}

fn build_callback_canonical_string(uri: &Uri, body: &[u8]) -> Result<String, MediaError> {
    let path_and_query = uri.path_and_query().map(|value| value.as_str()).unwrap_or(uri.path());
    let decoded_path = percent_decode(path_and_query)?;
    let body = std::str::from_utf8(body)
        .map_err(|_| MediaError::BadRequest("invalid callback body encoding".into()))?;
    Ok(format!("{decoded_path}\n{body}"))
}

fn percent_decode(value: &str) -> Result<String, MediaError> {
    let mut bytes = Vec::with_capacity(value.len());
    let input = value.as_bytes();
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'%' {
            if index + 2 >= input.len() {
                return Err(MediaError::BadRequest("invalid callback path encoding".into()));
            }
            let hex = std::str::from_utf8(&input[index + 1..index + 3])
                .map_err(|_| MediaError::BadRequest("invalid callback path encoding".into()))?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| MediaError::BadRequest("invalid callback path encoding".into()))?;
            bytes.push(byte);
            index += 3;
        } else {
            bytes.push(input[index]);
            index += 1;
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| MediaError::BadRequest("invalid callback path encoding".into()))
}

fn validate_public_key_url(url: &str) -> Result<(), MediaError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| MediaError::BadRequest("invalid callback public key url".into()))?;
    let is_allowed = parsed.scheme() == "https"
        && parsed.host_str() == Some(OSS_CALLBACK_PUBLIC_KEY_HOST)
        && parsed.port().is_none()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.fragment().is_none();
    if is_allowed {
        Ok(())
    } else {
        Err(MediaError::BadRequest("invalid callback public key url".into()))
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, MediaError> {
    headers
        .get(name)
        .ok_or_else(|| MediaError::BadRequest(format!("missing {name}")))?
        .to_str()
        .map_err(|_| MediaError::BadRequest(format!("invalid {name}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_policy_scopes_to_single_object() {
        let config = OssConfig {
            region: "cn-shanghai".into(),
            bucket: "yourtj".into(),
            access_key_id: "ak".into(),
            access_key_secret: "secret".into(),
            role_arn: "acs:ram::1:role/upload".into(),
            callback_base_url: "https://api.example.test".into(),
        };
        let policy = build_upload_policy(&config, "uploads/42/image/file.png");
        assert!(policy.contains("acs:oss:*:*:yourtj/uploads/42/image/file.png"));
        assert!(!policy.contains("uploads/43"));
        assert!(policy.contains("oss:ContentLength"));
    }

    #[test]
    fn sts_percent_encoding_uses_rfc3986_unreserved_set() {
        assert_eq!(percent_encode("AZaz09-_.~"), "AZaz09-_.~");
        assert_eq!(percent_encode(" +/=:{}\""), "%20%2B%2F%3D%3A%7B%7D%22");
    }

    #[test]
    fn delete_object_request_uses_oss_v1_authorization() {
        let config = OssConfig {
            region: "cn-shanghai".into(),
            bucket: "yourtj".into(),
            access_key_id: "test-ak".into(),
            access_key_secret: "test-secret".into(),
            role_arn: "acs:ram::1:role/upload".into(),
            callback_base_url: "https://api.example.test".into(),
        };
        let request_time = DateTime::parse_from_rfc3339("2026-07-11T08:09:10Z")
            .expect("request timestamp")
            .with_timezone(&Utc);
        let request = build_delete_object_request(
            &config,
            "uploads/42/image/00000000-0000-0000-0000-000000000000.png",
            request_time,
        )
        .expect("signed delete request");
        assert_eq!(
            request.url,
            "https://yourtj.oss-cn-shanghai.aliyuncs.com/uploads/42/image/00000000-0000-0000-0000-000000000000.png"
        );
        assert_eq!(request.date, "Sat, 11 Jul 2026 08:09:10 GMT");
        assert_eq!(request.authorization, "OSS test-ak:9fVdWj+aDQKmkJOAI5uUIrVEPwY=");
    }

    #[test]
    fn delete_object_rejects_non_account_scoped_path_traversal() {
        let config = OssConfig {
            region: "cn-shanghai".into(),
            bucket: "yourtj".into(),
            access_key_id: "test-ak".into(),
            access_key_secret: "test-secret".into(),
            role_arn: "acs:ram::1:role/upload".into(),
            callback_base_url: "https://api.example.test".into(),
        };
        assert!(build_delete_object_request(&config, "../other-object", Utc::now()).is_err());
    }

    #[test]
    fn rejects_callback_public_key_url_ssrf_variants() {
        for url in [
            "http://gosspublic.alicdn.com/key",
            "https://gosspublic.alicdn.com.evil.test/key",
            "https://gosspublic.alicdn.com@evil.test/key",
            "https://evil.test/key",
        ] {
            assert!(validate_public_key_url(url).is_err());
        }
        assert!(validate_public_key_url("https://gosspublic.alicdn.com/key").is_ok());
    }

    #[test]
    fn validates_kind_specific_content_types() {
        assert_eq!(validate_content_type("image", "IMAGE/PNG").expect("png"), "image/png");
        assert_eq!(
            validate_content_type("file", "application/pdf").expect("pdf"),
            "application/pdf"
        );
        assert!(validate_content_type("image", "application/pdf").is_err());
        assert!(validate_content_type("file", "text/html").is_err());
    }

    #[test]
    fn rejects_callback_metadata_outside_intent() {
        let digest = "a".repeat(64);
        assert!(validate_callback_metadata(
            "uploads/1/image/a.png",
            "image/png",
            100,
            "uploads/1/image/a.png",
            99,
            "image/png",
            &digest,
        )
        .is_ok());
        assert!(validate_callback_metadata(
            "uploads/1/image/a.png",
            "image/png",
            100,
            "uploads/2/image/a.png",
            99,
            "image/png",
            &digest,
        )
        .is_err());
        assert!(validate_callback_metadata(
            "uploads/1/image/a.png",
            "image/png",
            100,
            "uploads/1/image/a.png",
            101,
            "image/png",
            &digest,
        )
        .is_err());
    }

    #[test]
    fn callback_canonical_string_uses_decoded_path_and_body() {
        let uri: Uri = "/api/v2/media/callback%2Ftest?x=1".parse().expect("valid uri");
        let canonical =
            build_callback_canonical_string(&uri, br#"{"ok":true}"#).expect("canonical string");
        assert_eq!(canonical, "/api/v2/media/callback/test?x=1\n{\"ok\":true}");
    }

    #[test]
    fn verifies_callback_signature_and_rejects_body_mutation() {
        let rsa = openssl::rsa::Rsa::generate(2048).expect("rsa key");
        let private_key = PKey::from_rsa(rsa).expect("private key");
        let public_key_pem =
            String::from_utf8(private_key.public_key_to_pem().expect("public key pem"))
                .expect("utf8 pem");
        let uri: Uri = "/api/v2/media/callback".parse().expect("valid uri");
        let body = br#"{"uploadIntentId":"00000000-0000-0000-0000-000000000000","callbackToken":"token","ossKey":"uploads/1/image/a.png","url":"bucket.host/uploads/1/image/a.png","bytes":12,"mime":"image/png","sha256":"abc"}"#;
        let canonical = build_callback_canonical_string(&uri, body).expect("canonical string");
        let mut signer =
            openssl::sign::Signer::new(MessageDigest::md5(), &private_key).expect("signer");
        signer.update(canonical.as_bytes()).expect("sign bytes");
        let signature = signer.sign_to_vec().expect("signature");
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            base64::engine::general_purpose::STANDARD
                .encode(signature)
                .parse()
                .expect("signature header"),
        );

        assert!(verify_callback_signature(&headers, &uri, body, &public_key_pem).is_ok());
        assert!(verify_callback_signature(
            &headers,
            &uri,
            br#"{"tampered":true}"#,
            &public_key_pem
        )
        .is_err());
    }
}

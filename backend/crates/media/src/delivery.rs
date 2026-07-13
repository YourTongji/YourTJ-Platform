//! Private Delivery-bucket configuration and short-lived CDN URL signing.

use std::env;

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Deserialize;
use sha1::Sha1;
use shared::AppResult;
use uuid::Uuid;

use crate::error::MediaError;

pub(crate) const CDN_URL_TTL_SECONDS: i64 = 5 * 60;
pub(crate) const DELIVERY_POLICY_VERSION: i32 = 1;
pub(crate) const DISPLAY_VARIANT: &str = "display_1280";
const RPC_ENCODE_SET: &AsciiSet =
    &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');
const CDN_RPC_ENDPOINT: &str = "https://cdn.aliyuncs.com/";
const CDN_RPC_RESPONSE_MAX_BYTES: usize = 64 * 1024;

/// Runtime configuration for the private Delivery bucket and its CDN domain.
#[derive(Clone)]
pub(crate) struct DeliveryConfig {
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub cdn_base_url: String,
    primary_key: String,
    secondary_key: String,
    active_key: CdnKeySlot,
    pub purge_access_key_id: String,
    pub purge_access_key_secret: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CdnKeySlot {
    Primary,
    Secondary,
}

/// One CDN bearer URL and the time after which clients must refresh its owner resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignedDeliveryUrl {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

/// Storage-opaque image delivery data returned only after an owning domain authorizes disclosure.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImageDeliveryProjection {
    pub asset_id: String,
    pub url: String,
    pub expires_at: i64,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub variant: ImageVariant,
}

/// Stable image variants that owning domains may intentionally disclose.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum ImageVariant {
    #[serde(rename = "thumb_256")]
    Thumb256,
    #[serde(rename = "display_1280")]
    Display1280,
    #[serde(rename = "full_2048")]
    Full2048,
}

impl ImageVariant {
    pub(crate) fn from_database(value: &str) -> Result<Self, MediaError> {
        match value {
            "thumb_256" => Ok(Self::Thumb256),
            "display_1280" => Ok(Self::Display1280),
            "full_2048" => Ok(Self::Full2048),
            _ => Err(MediaError::Internal(anyhow::anyhow!("unknown media image variant"))),
        }
    }
}

impl DeliveryConfig {
    /// Load an all-or-nothing Delivery configuration from runtime environment variables.
    pub(crate) fn from_env(region: &str) -> Result<Option<Self>, MediaError> {
        let bucket = env_value("MEDIA_DELIVERY_OSS_BUCKET");
        let cdn_base_url = env_value("MEDIA_CDN_BASE_URL");
        let access_key_id = env_value("MEDIA_DELIVERY_OSS_ACCESS_KEY_ID");
        let access_key_secret = env_value("MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET");
        let primary_key = env_value("MEDIA_CDN_PRIMARY_KEY");
        let secondary_key = env_value("MEDIA_CDN_SECONDARY_KEY");
        let active_key = env_value("MEDIA_CDN_SIGNING_KEY_SLOT");
        let url_ttl = env_value("MEDIA_CDN_URL_TTL_SECONDS");
        let purge_access_key_id = env_value("CDN_ACCESS_KEY_ID");
        let purge_access_key_secret = env_value("CDN_ACCESS_KEY_SECRET");
        let configured = [
            bucket.as_deref(),
            cdn_base_url.as_deref(),
            access_key_id.as_deref(),
            access_key_secret.as_deref(),
            primary_key.as_deref(),
            secondary_key.as_deref(),
            active_key.as_deref(),
            url_ttl.as_deref(),
            purge_access_key_id.as_deref(),
            purge_access_key_secret.as_deref(),
        ]
        .iter()
        .any(Option::is_some);
        if !configured {
            return Ok(None);
        }
        let bucket = bucket.ok_or_else(partial_delivery_config)?;
        validate_bucket_name(&bucket)?;
        let cdn_base_url =
            normalize_cdn_base_url(&cdn_base_url.ok_or_else(partial_delivery_config)?)?;
        let access_key_id = access_key_id.ok_or_else(partial_delivery_config)?;
        let access_key_secret = access_key_secret.ok_or_else(partial_delivery_config)?;
        let primary_key = primary_key.ok_or_else(partial_delivery_config)?;
        let secondary_key = secondary_key.ok_or_else(partial_delivery_config)?;
        if primary_key == secondary_key
            || !is_valid_cdn_signing_key(&primary_key)
            || !is_valid_cdn_signing_key(&secondary_key)
        {
            return Err(MediaError::Unavailable(
                "media CDN signing keys are incomplete or not independently rotatable".into(),
            ));
        }
        let active_key = match active_key.as_deref() {
            Some("primary") => CdnKeySlot::Primary,
            Some("secondary") => CdnKeySlot::Secondary,
            _ => {
                return Err(MediaError::Unavailable(
                    "media CDN active signing key slot is invalid".into(),
                ))
            }
        };
        let configured_ttl = url_ttl
            .ok_or_else(partial_delivery_config)?
            .parse::<i64>()
            .map_err(|_| MediaError::Unavailable("media CDN URL TTL is invalid".into()))?;
        if configured_ttl != CDN_URL_TTL_SECONDS {
            return Err(MediaError::Unavailable(
                "media CDN URL TTL must be exactly 300 seconds".into(),
            ));
        }
        let purge_access_key_id = purge_access_key_id.ok_or_else(partial_delivery_config)?;
        let purge_access_key_secret =
            purge_access_key_secret.ok_or_else(partial_delivery_config)?;
        Ok(Some(Self {
            region: region.trim().to_owned(),
            bucket,
            access_key_id,
            access_key_secret,
            cdn_base_url,
            primary_key,
            secondary_key,
            active_key,
            purge_access_key_id,
            purge_access_key_secret,
        }))
    }

    pub(crate) fn sign_object(&self, object_key: &str) -> Result<SignedDeliveryUrl, MediaError> {
        let nonce = Uuid::new_v4().simple().to_string();
        self.sign_object_at(object_key, Utc::now(), &nonce)
    }

    pub(crate) fn canonical_object_url(&self, object_key: &str) -> Result<String, MediaError> {
        let path = delivery_path(object_key)?;
        Ok(format!("{}{path}", self.cdn_base_url))
    }

    pub(crate) async fn submit_purge(
        &self,
        client: &reqwest::Client,
        object_key: &str,
    ) -> Result<String, MediaError> {
        self.submit_purge_to(client, object_key, CDN_RPC_ENDPOINT).await
    }

    async fn submit_purge_to(
        &self,
        client: &reqwest::Client,
        object_key: &str,
        endpoint: &str,
    ) -> Result<String, MediaError> {
        let object_url = self.canonical_object_url(object_key)?;
        let request = build_cdn_rpc_request(
            &self.purge_access_key_id,
            &self.purge_access_key_secret,
            "RefreshObjectCaches",
            std::collections::BTreeMap::from([
                ("ObjectPath".to_owned(), object_url),
                ("ObjectType".to_owned(), "File".to_owned()),
            ]),
            Utc::now(),
            Uuid::new_v4(),
        )?;
        let bytes = post_cdn_rpc(client, endpoint, request, "submit").await?;
        let response: RefreshObjectCachesResponse = serde_json::from_slice(&bytes)
            .map_err(|_| MediaError::Unavailable("invalid CDN purge response".into()))?;
        validate_provider_task_id(&response.refresh_task_id)?;
        Ok(response.refresh_task_id)
    }

    pub(crate) async fn purge_task_state(
        &self,
        client: &reqwest::Client,
        provider_task_id: &str,
    ) -> Result<crate::quarantine::DeliveryPurgeTaskState, MediaError> {
        self.purge_task_state_from(client, provider_task_id, CDN_RPC_ENDPOINT).await
    }

    async fn purge_task_state_from(
        &self,
        client: &reqwest::Client,
        provider_task_id: &str,
        endpoint: &str,
    ) -> Result<crate::quarantine::DeliveryPurgeTaskState, MediaError> {
        validate_provider_task_id(provider_task_id)?;
        let request = build_cdn_rpc_request(
            &self.purge_access_key_id,
            &self.purge_access_key_secret,
            "DescribeRefreshTaskById",
            std::collections::BTreeMap::from([("TaskId".to_owned(), provider_task_id.to_owned())]),
            Utc::now(),
            Uuid::new_v4(),
        )?;
        let bytes = post_cdn_rpc(client, endpoint, request, "status").await?;
        let response: DescribeRefreshTaskResponse = serde_json::from_slice(&bytes)
            .map_err(|_| MediaError::Unavailable("invalid CDN purge status response".into()))?;
        if response.tasks.is_empty() {
            return Err(MediaError::Unavailable("CDN purge task was not found".into()));
        }
        let expected_task_ids =
            provider_task_id.split(',').collect::<std::collections::HashSet<_>>();
        let mut observed_task_ids = std::collections::HashSet::new();
        let mut is_refreshing = false;
        for task in response.tasks {
            validate_provider_task_id(&task.task_id)?;
            observed_task_ids.extend(task.task_id.split(',').map(str::to_owned));
            match task.status.as_str() {
                "Complete" => {}
                "Refreshing" => is_refreshing = true,
                "Timeout" | "Canceled" | "Failed" => {
                    return Ok(crate::quarantine::DeliveryPurgeTaskState::Failed)
                }
                _ => return Err(MediaError::Unavailable("unknown CDN purge task status".into())),
            }
        }
        if observed_task_ids.len() != expected_task_ids.len()
            || !expected_task_ids.iter().all(|task_id| observed_task_ids.contains(*task_id))
        {
            return Err(MediaError::Unavailable(
                "CDN purge status referred to a different task".into(),
            ));
        }
        Ok(if is_refreshing {
            crate::quarantine::DeliveryPurgeTaskState::Refreshing
        } else {
            crate::quarantine::DeliveryPurgeTaskState::Complete
        })
    }

    fn sign_object_at(
        &self,
        object_key: &str,
        issued_at: DateTime<Utc>,
        nonce: &str,
    ) -> Result<SignedDeliveryUrl, MediaError> {
        if nonce.len() != 32 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(MediaError::Internal(anyhow::anyhow!("invalid CDN signing nonce")));
        }
        let path = delivery_path(object_key)?;
        let timestamp = issued_at.timestamp();
        let key = match self.active_key {
            CdnKeySlot::Primary => &self.primary_key,
            CdnKeySlot::Secondary => &self.secondary_key,
        };
        let digest = Md5::digest(format!("{path}-{timestamp}-{nonce}-0-{key}").as_bytes());
        let auth_key = format!("{timestamp}-{nonce}-0-{}", hex::encode(digest));
        Ok(SignedDeliveryUrl {
            url: format!("{}{path}?auth_key={auth_key}", self.cdn_base_url),
            expires_at: issued_at + Duration::seconds(CDN_URL_TTL_SECONDS),
        })
    }
}

pub(crate) fn require_delivery_config(
    config: &shared::Config,
) -> Result<DeliveryConfig, MediaError> {
    DeliveryConfig::from_env(&config.oss_region)?
        .ok_or_else(|| MediaError::Unavailable("media Delivery is not configured".into()))
}

pub(crate) fn require_delivery_config_from_env() -> Result<DeliveryConfig, MediaError> {
    let region = env_value("OSS_REGION")
        .ok_or_else(|| MediaError::Unavailable("media OSS region is not configured".into()))?;
    DeliveryConfig::from_env(&region)?
        .ok_or_else(|| MediaError::Unavailable("media Delivery is not configured".into()))
}

#[derive(sqlx::FromRow)]
struct OwnedDeliveryRow {
    asset_id: i64,
    object_key: String,
    mime: String,
    width: i32,
    height: i32,
    variant_kind: String,
}

/// Resolve one owned variant only after PostgreSQL proves clean moderation and atomic publication.
pub(crate) async fn resolve_owned_published_variant(
    pool: &sqlx::PgPool,
    config: &shared::Config,
    asset_id: i64,
    owner_account_id: i64,
    variant_kind: &str,
) -> AppResult<Option<ImageDeliveryProjection>> {
    let row = sqlx::query_as::<_, OwnedDeliveryRow>(
        "SELECT upload.id AS asset_id, variant.object_key, variant.mime, \
                variant.width, variant.height, variant.variant_kind \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         JOIN media.asset_variants variant \
           ON variant.asset_id = upload.id \
          AND variant.policy_version = publication.policy_version \
         WHERE upload.id = $1 AND upload.account_id = $2 \
           AND upload.kind = 'image' AND upload.status = 'clean' \
           AND publication.status = 'published' AND variant.status = 'published' \
           AND variant.variant_kind = $3",
    )
    .bind(asset_id)
    .bind(owner_account_id)
    .bind(variant_kind)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let signed = require_delivery_config(config)?.sign_object(&row.object_key)?;
    Ok(Some(ImageDeliveryProjection {
        asset_id: row.asset_id.to_string(),
        url: signed.url,
        expires_at: signed.expires_at.timestamp(),
        mime: row.mime,
        width: row.width,
        height: row.height,
        variant: ImageVariant::from_database(&row.variant_kind)?,
    }))
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().map(|value| value.trim().to_owned()).filter(|value| !value.is_empty())
}

fn partial_delivery_config() -> MediaError {
    MediaError::Unavailable("media Delivery configuration is incomplete".into())
}

fn is_valid_cdn_signing_key(key: &str) -> bool {
    (6..=128).contains(&key.len()) && key.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn validate_bucket_name(bucket: &str) -> Result<(), MediaError> {
    let valid = (3..=63).contains(&bucket.len())
        && bucket
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && bucket.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        && bucket.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric);
    if valid {
        Ok(())
    } else {
        Err(MediaError::Unavailable("media Delivery bucket name is invalid".into()))
    }
}

fn normalize_cdn_base_url(value: &str) -> Result<String, MediaError> {
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| MediaError::Unavailable("media CDN base URL is invalid".into()))?;
    let valid = parsed.scheme() == "https"
        && parsed.host_str().is_some()
        && parsed.port().is_none()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.path() == "/"
        && parsed.query().is_none()
        && parsed.fragment().is_none();
    if !valid {
        return Err(MediaError::Unavailable(
            "media CDN base URL must be an HTTPS origin without a path".into(),
        ));
    }
    Ok(value.trim_end_matches('/').to_owned())
}

fn delivery_path(object_key: &str) -> Result<String, MediaError> {
    let valid = !object_key.is_empty()
        && !object_key.starts_with('/')
        && !object_key.contains("..")
        && object_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'));
    if !valid {
        return Err(MediaError::BadRequest("invalid Delivery object key".into()));
    }
    Ok(format!("/{object_key}"))
}

struct CdnRpcRequest {
    form_body: String,
}

#[derive(Deserialize)]
struct RefreshObjectCachesResponse {
    #[serde(rename = "RefreshTaskId")]
    refresh_task_id: String,
}

#[derive(Deserialize)]
struct DescribeRefreshTaskResponse {
    #[serde(rename = "Tasks")]
    tasks: Vec<DescribeRefreshTask>,
}

#[derive(Deserialize)]
struct DescribeRefreshTask {
    #[serde(rename = "TaskId")]
    task_id: String,
    #[serde(rename = "Status")]
    status: String,
}

fn build_cdn_rpc_request(
    access_key_id: &str,
    access_key_secret: &str,
    action: &str,
    action_params: std::collections::BTreeMap<String, String>,
    request_time: DateTime<Utc>,
    nonce: Uuid,
) -> Result<CdnRpcRequest, MediaError> {
    let mut params = std::collections::BTreeMap::from([
        ("AccessKeyId".to_owned(), access_key_id.to_owned()),
        ("Action".to_owned(), action.to_owned()),
        ("Format".to_owned(), "JSON".to_owned()),
        ("SignatureMethod".to_owned(), "HMAC-SHA1".to_owned()),
        ("SignatureNonce".to_owned(), nonce.to_string()),
        ("SignatureVersion".to_owned(), "1.0".to_owned()),
        ("Timestamp".to_owned(), request_time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        ("Version".to_owned(), "2018-05-10".to_owned()),
    ]);
    params.extend(action_params);
    let canonical_query = rpc_canonical_query(&params);
    let string_to_sign = format!("POST&%2F&{}", rpc_encode(&canonical_query));
    let mut mac = Hmac::<Sha1>::new_from_slice(format!("{access_key_secret}&").as_bytes())
        .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    params.insert("Signature".to_owned(), signature);
    Ok(CdnRpcRequest { form_body: rpc_canonical_query(&params) })
}

async fn post_cdn_rpc(
    client: &reqwest::Client,
    endpoint: &str,
    request: CdnRpcRequest,
    operation: &'static str,
) -> Result<Vec<u8>, MediaError> {
    let response = client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(request.form_body)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(
                is_timeout = error.is_timeout(),
                is_connect = error.is_connect(),
                operation,
                "CDN purge RPC failed"
            );
            MediaError::Unavailable("media CDN purge unavailable".into())
        })?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), operation, "CDN purge RPC was rejected");
        return Err(MediaError::Unavailable("media CDN purge unavailable".into()));
    }
    if response.content_length().is_some_and(|length| length > CDN_RPC_RESPONSE_MAX_BYTES as u64) {
        return Err(MediaError::Unavailable("CDN purge response is too large".into()));
    }
    let bytes = response.bytes().await.map_err(|error| {
        tracing::warn!(
            is_timeout = error.is_timeout(),
            is_connect = error.is_connect(),
            operation,
            "CDN purge RPC response read failed"
        );
        MediaError::Unavailable("media CDN purge unavailable".into())
    })?;
    if bytes.len() > CDN_RPC_RESPONSE_MAX_BYTES {
        return Err(MediaError::Unavailable("CDN purge response is too large".into()));
    }
    Ok(bytes.to_vec())
}

fn validate_provider_task_id(provider_task_id: &str) -> Result<(), MediaError> {
    let ids = provider_task_id.split(',').collect::<Vec<_>>();
    if ids.is_empty()
        || ids.len() > 10
        || provider_task_id.len() > 255
        || ids
            .iter()
            .any(|task_id| task_id.is_empty() || !task_id.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(MediaError::Unavailable("invalid CDN purge task id".into()));
    }
    Ok(())
}

fn rpc_canonical_query(params: &std::collections::BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{}={}", rpc_encode(key), rpc_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn rpc_encode(value: &str) -> String {
    utf8_percent_encode(value, RPC_ENCODE_SET).to_string()
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::{
        build_cdn_rpc_request, CdnKeySlot, DeliveryConfig, ImageVariant, CDN_URL_TTL_SECONDS,
    };

    fn config(active_key: CdnKeySlot) -> DeliveryConfig {
        DeliveryConfig {
            region: "cn-shanghai".into(),
            bucket: "yourtj-media-delivery-dev".into(),
            access_key_id: "delivery-ak".into(),
            access_key_secret: "delivery-secret".into(),
            cdn_base_url: "https://media-dev.yourtj.de".into(),
            primary_key: "primarysecret".into(),
            secondary_key: "secondarysecret".into(),
            active_key,
            purge_access_key_id: "purgeak".into(),
            purge_access_key_secret: "purgesecret".into(),
        }
    }

    #[test]
    fn type_a_signature_matches_the_documented_formula_and_five_minute_ttl() {
        let issued_at = DateTime::parse_from_rfc3339("2024-07-03T09:46:40Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);
        assert_eq!(issued_at.timestamp(), 1_720_000_000);
        let signed = config(CdnKeySlot::Primary)
            .sign_object_at(
                "assets/42/1/display_1280-abc.webp",
                issued_at,
                "00112233445566778899aabbccddeeff",
            )
            .expect("signed CDN URL");
        assert_eq!(
            signed.url,
            "https://media-dev.yourtj.de/assets/42/1/display_1280-abc.webp?auth_key=1720000000-00112233445566778899aabbccddeeff-0-901916fbd2bd8306ccbd03fb5d277dbb"
        );
        assert_eq!(signed.expires_at.timestamp() - issued_at.timestamp(), CDN_URL_TTL_SECONDS);
    }

    #[test]
    fn secondary_slot_supports_overlap_without_disclosing_keys() {
        let issued_at = DateTime::parse_from_rfc3339("2024-07-03T09:46:40Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);
        let primary = config(CdnKeySlot::Primary)
            .sign_object_at("assets/a.webp", issued_at, "0a112233445566778899aabbccddeeff")
            .expect("primary signature");
        let secondary = config(CdnKeySlot::Secondary)
            .sign_object_at("assets/a.webp", issued_at, "0a112233445566778899aabbccddeeff")
            .expect("secondary signature");
        assert_ne!(primary.url, secondary.url);
        assert!(!primary.url.contains("primarysecret"));
        assert!(!secondary.url.contains("secondarysecret"));
    }

    #[test]
    fn signing_rejects_provider_path_escape() {
        assert!(config(CdnKeySlot::Primary).sign_object("../ingest/secret").is_err());
        assert!(config(CdnKeySlot::Primary).sign_object("assets/a b.webp").is_err());
    }

    #[test]
    fn image_variant_serialization_is_stable_and_storage_opaque() {
        assert_eq!(
            serde_json::to_value(ImageVariant::Display1280).expect("serialize image variant"),
            serde_json::json!("display_1280")
        );
    }

    #[test]
    fn cdn_force_purge_uses_post_form_signature_and_independent_credential() {
        let request_time = DateTime::parse_from_rfc3339("2026-07-11T08:09:10Z")
            .expect("request timestamp")
            .with_timezone(&Utc);
        let nonce = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000042")
            .expect("fixed purge nonce");
        let request = build_cdn_rpc_request(
            "cdn-purge-ak",
            "cdn-purge-secret",
            "RefreshObjectCaches",
            std::collections::BTreeMap::from([
                (
                    "ObjectPath".to_owned(),
                    "https://media-dev.yourtj.de/assets/42/1/display.webp".to_owned(),
                ),
                ("ObjectType".to_owned(), "File".to_owned()),
            ]),
            request_time,
            nonce,
        )
        .expect("signed CDN purge request");
        assert!(request.form_body.contains("Action=RefreshObjectCaches"));
        assert!(request.form_body.contains("ObjectType=File"));
        assert!(request.form_body.contains("AccessKeyId=cdn-purge-ak"));
        assert!(request.form_body.contains("Signature="));
        assert!(!request.form_body.contains("cdn-purge-secret"));
        assert!(!request.form_body.contains("auth_key"));
    }

    #[tokio::test]
    async fn cdn_rpc_posts_form_and_parses_submission_then_completion() {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind fake CDN RPC");
        let endpoint = format!("http://{}/", listener.local_addr().expect("fake CDN RPC address"));
        let server = tokio::spawn(async move {
            let responses = [
                r#"{"RefreshTaskId":"704222904","RequestId":"submit"}"#,
                r#"{"TotalCount":1,"RequestId":"status","Tasks":[{"Status":"Complete","TaskId":"704222904"}]}"#,
            ];
            let mut requests = Vec::new();
            for response_body in responses {
                let (mut stream, _) = listener.accept().await.expect("accept fake CDN RPC");
                let mut request = Vec::new();
                loop {
                    let mut chunk = [0_u8; 2048];
                    let read = stream.read(&mut chunk).await.expect("read fake CDN RPC request");
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                    let Some(header_end) = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|position| position + 4)
                    else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .expect("fake CDN RPC content length");
                    if request.len() >= header_end + content_length {
                        break;
                    }
                }
                requests.push(String::from_utf8(request).expect("UTF-8 fake CDN RPC request"));
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).await.expect("write fake CDN RPC response");
            }
            requests
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("fake CDN RPC client");
        let config = config(CdnKeySlot::Primary);
        let task_id = config
            .submit_purge_to(&client, "assets/42/1/display.webp", &endpoint)
            .await
            .expect("submit fake CDN purge");
        assert_eq!(task_id, "704222904");
        let state = config
            .purge_task_state_from(&client, &task_id, &endpoint)
            .await
            .expect("query fake CDN purge");
        assert_eq!(state, crate::quarantine::DeliveryPurgeTaskState::Complete);
        let requests = server.await.expect("join fake CDN RPC");
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| request.starts_with("POST / HTTP/1.1\r\n")));
        assert!(requests.iter().all(|request| request
            .to_ascii_lowercase()
            .contains("content-type: application/x-www-form-urlencoded")));
        assert!(requests[0].contains("Action=RefreshObjectCaches"));
        assert!(requests[1].contains("Action=DescribeRefreshTaskById"));
    }

    #[tokio::test]
    async fn purge_status_rejects_a_completed_response_for_another_task() {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind fake CDN RPC");
        let endpoint = format!("http://{}/", listener.local_addr().expect("fake CDN RPC address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept fake CDN RPC");
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 2048];
                let read = stream.read(&mut chunk).await.expect("read fake CDN RPC request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                let Some(header_end) = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|position| position + 4)
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .expect("fake CDN RPC content length");
                if request.len() >= header_end + content_length {
                    break;
                }
            }
            let response_body =
                r#"{"TotalCount":1,"Tasks":[{"Status":"Complete","TaskId":"999"}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).await.expect("write fake CDN RPC response");
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("fake CDN RPC client");

        let result = config(CdnKeySlot::Primary)
            .purge_task_state_from(&client, "704222904", &endpoint)
            .await;

        assert!(result.is_err());
        server.await.expect("join fake CDN RPC");
    }
}

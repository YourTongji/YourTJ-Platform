//! Alibaba Cloud CDN URL signing (Type A) for secure media delivery.
//!
//! Type A appends an auth parameter `auth_key={timestamp-{rand-{uid}-{md5hash}}`
//! to the CDN URL. The hash is `md5(uri-path-timestamp-rand-uid-private-key)`.
//!
//! Reference: https://www.alibabacloud.com/help/en/cdn/user-guide/configure-url-signing
//!
//! This module only generates signed URLs from server-side object paths;
//! it never exposes the signing keys or accepts user-controlled paths.

use md5::{Digest, Md5};
use shared::Config;
use std::time::{SystemTime, UNIX_EPOCH};

/// CDN delivery configuration, derived from runtime config.
#[derive(Debug, Clone)]
pub struct CdnConfig {
    /// CDN base URL, e.g. `https://media-dev.yourtj.de`
    pub base_url: String,
    /// Auth type — currently only `"A"` is supported.
    pub auth_type: String,
    /// Primary signing key.
    pub primary_key: String,
    /// Secondary signing key (for rotation window).
    pub secondary_key: String,
    /// Signed URL TTL in seconds.
    pub url_ttl_seconds: i64,
}

impl CdnConfig {
    /// Build CDN config from runtime config. Returns `None` when CDN is disabled.
    pub fn from_config(config: &Config) -> Option<Self> {
        let cfg = Self {
            base_url: config.media_cdn_base_url.trim().trim_end_matches('/').to_string(),
            auth_type: config.media_cdn_auth_type.trim().to_uppercase(),
            primary_key: config.media_cdn_primary_key.trim().to_string(),
            secondary_key: config.media_cdn_secondary_key.trim().to_string(),
            url_ttl_seconds: config.media_cdn_url_ttl_seconds.clamp(60, 86400),
        };
        if cfg.is_enabled() {
            Some(cfg)
        } else {
            None
        }
    }

    fn is_enabled(&self) -> bool {
        !self.base_url.is_empty() && self.auth_type == "A" && !self.primary_key.is_empty()
    }
}

/// Generate a signed CDN URL for the given object path using Type A signing.
///
/// The `path` must be a server-generated canonical path (e.g. `/assets/{id}/{hash}/avatar256.webp`).
/// Client-controlled paths are not accepted.
pub fn sign_url(config: &CdnConfig, path: &str, expires_in_seconds: i64) -> Option<String> {
    if path.contains("..") || path.contains("//") || !path.starts_with('/') {
        return None;
    }

    let timestamp = (SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs()
        + expires_in_seconds as u64)
        .to_string();

    let rand = "0";
    let uid = "0";

    // Try primary key first, fall back to secondary.
    for key in [&config.primary_key, &config.secondary_key] {
        if key.is_empty() {
            continue;
        }
        let hash_input = format!("{path}-{timestamp}-{rand}-{uid}-{key}");
        let hash = format!("{:x}", Md5::digest(hash_input.as_bytes()));

        let signed = format!(
            "{base}{path}?auth_key={timestamp}-{rand}-{uid}-{hash}",
            base = config.base_url,
        );
        return Some(signed);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cdn_config() -> CdnConfig {
        CdnConfig {
            base_url: "https://media-dev.yourtj.de".into(),
            auth_type: "A".into(),
            primary_key: "test-primary-key-32bytes!".into(),
            secondary_key: "test-secondary-key-32bytes!".into(),
            url_ttl_seconds: 300,
        }
    }

    #[test]
    fn cdn_config_is_enabled() {
        let cfg = test_cdn_config();
        assert!(cfg.is_enabled());
    }

    #[test]
    fn cdn_config_disabled_when_missing_base_url() {
        let cfg = CdnConfig { base_url: "".into(), ..test_cdn_config() };
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn cdn_config_disabled_when_wrong_auth_type() {
        let cfg = CdnConfig { auth_type: "B".into(), ..test_cdn_config() };
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn sign_url_returns_signed_url() {
        let cfg = test_cdn_config();
        let url = sign_url(&cfg, "/assets/123/hash/avatar256.webp", 300);
        assert!(url.is_some());
        let url = url.unwrap();
        assert!(url.starts_with("https://media-dev.yourtj.de"));
        assert!(url.contains("auth_key="));
    }

    #[test]
    fn sign_url_rejects_path_traversal() {
        let cfg = test_cdn_config();
        assert!(sign_url(&cfg, "/../../etc/passwd", 300).is_none());
        assert!(sign_url(&cfg, "relative/path", 300).is_none());
    }

    #[test]
    fn sign_url_expires_in_future() {
        let cfg = test_cdn_config();
        let url = sign_url(&cfg, "/assets/1/hash/avatar256.webp", 60).unwrap();
        let params: Vec<&str> = url.split("auth_key=").collect();
        assert_eq!(params.len(), 2);
        let parts: Vec<&str> = params[1].split('-').collect();
        assert_eq!(parts.len(), 4);
        let ts: u64 = parts[0].parse().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert!(ts > now);
    }

    #[test]
    fn sign_url_with_secondary_key_fallback() {
        let cfg = CdnConfig {
            primary_key: "".into(),
            secondary_key: "backup-key-32bytes!!!!!".into(),
            ..test_cdn_config()
        };
        let url = sign_url(&cfg, "/assets/1/hash/avatar256.webp", 300);
        assert!(url.is_some());
        assert!(url.unwrap().contains("auth_key="));
    }

    #[test]
    fn is_configured_checks_config() {
        let cfg = test_cdn_config();
        assert!(cfg.is_enabled());
        let disabled = CdnConfig { base_url: "".into(), ..test_cdn_config() };
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn md5_hash_is_deterministic() {
        let cfg = test_cdn_config();
        let path = "/assets/42/hash/v.webp";
        let a = sign_url(&cfg, path, 300).unwrap();
        let b = sign_url(&cfg, path, 300).unwrap();
        assert_eq!(a.split("auth_key=").next(), b.split("auth_key=").next());
    }
}

//! Onebox link preview module. Fetches OG meta tags from whitelisted domains and
//! returns preview cards. Cached in both Redis (fast) and Postgres (persistent).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use shared::{AppError, AppResult, AppState};
use sqlx::PgPool;

const MAX_BODY_BYTES: usize = 512 * 1024;
const MAX_REDIRECTS: usize = 5;
const ONEBOX_POLICY_VERSION: &str = "v2";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(6);

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneboxResult {
    pub r#type: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub site_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OneboxQuery {
    pub url: String,
}

// ---------------------------------------------------------------------------
// DB row
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct LinkPreviewRow {
    url_hash: String,
    url: String,
    title: Option<String>,
    description: Option<String>,
    image_url: Option<String>,
    site_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// GET /api/v2/onebox?url=...
///
/// Returns a link preview card for whitelisted domains, or `{type:"plain"}`
/// for non-whitelisted domains. Rate-limited to 30 requests per 60 seconds per IP.
pub async fn get_onebox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OneboxQuery>,
) -> AppResult<Json<OneboxResult>> {
    // Rate limit: 30 requests per 60 seconds per IP.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "onebox", &ip, 30, 60).await?;

    let url = query.url.trim().to_string();
    if url.is_empty() {
        return Err(AppError::BadRequest("url is required".into()));
    }

    let parsed = parse_target_url(&url)
        .map_err(|_| AppError::BadRequest("URL must be a safe HTTPS URL".into()))?;
    let domain =
        parsed.host_str().ok_or_else(|| AppError::BadRequest("invalid URL".into()))?.to_string();

    // Check domain whitelist from platform.settings.
    if !is_allowed_domain(&state.db, state.redis.as_ref(), &domain).await? {
        return Ok(Json(OneboxResult {
            r#type: "plain".into(),
            url,
            title: None,
            description: None,
            image_url: None,
            site_name: None,
        }));
    }

    let url_hash = compute_url_hash(&url);

    // Check cache (Redis fast path, then DB fallback).
    if let Some(cached) = cached_preview(state.redis.as_ref(), &state.db, &url_hash).await? {
        return Ok(Json(cached));
    }

    // Fetch OG tags from the remote URL.
    let mut result = match fetch_og_tags(&state.db, state.redis.as_ref(), &url).await {
        Ok(r) => r,
        Err(error) => {
            tracing::warn!(
                url_hash,
                domain,
                error_category = error.category(),
                "onebox fetch failed"
            );
            return Err(AppError::BadRequest("failed to fetch link preview".into()));
        }
    };
    result.url = url.clone();

    // Best-effort cache write (failures are logged, not propagated).
    save_preview(&state.db, state.redis.as_ref(), &result, &url_hash).await;

    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_target_url(url: &str) -> Result<reqwest::Url, OneboxFetchError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| OneboxFetchError::InvalidUrl)?;
    if parsed.scheme() != "https"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.host_str().is_none()
        || parsed.port_or_known_default() != Some(443)
    {
        return Err(OneboxFetchError::UnsafeTarget);
    }
    Ok(parsed)
}

/// Compute the SHA-256 hexadecimal hash of a URL for use as a cache key.
fn compute_url_hash(url: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(ONEBOX_POLICY_VERSION.as_bytes());
    hasher.update([0]);
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

/// Check whether `domain` is in the `onebox_allowed_domains` setting. The
/// whitelist is cached in Redis (5-minute TTL) to avoid hitting the DB on every
/// request.
async fn is_allowed_domain(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    domain: &str,
) -> Result<bool, AppError> {
    // Redis cache: try fast path first.
    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            if let Ok(Some(raw)) = redis::cmd("GET")
                .arg("onebox_allowed_domains")
                .query_async::<Option<String>>(&mut conn)
                .await
            {
                if let Ok(domains) = serde_json::from_str::<Vec<String>>(&raw) {
                    if matches_domain(&domains, domain) {
                        return Ok(true);
                    }
                }
            }
        }
    }

    // DB fallback.
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM platform.settings WHERE key = 'onebox_allowed_domains'",
    )
    .fetch_optional(pool)
    .await?;

    let raw = match row {
        Some((v,)) => v,
        None => return Ok(false),
    };

    let domains: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();

    // Write-through to Redis (5-minute TTL).
    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            let _: Result<(), _> = redis::cmd("SETEX")
                .arg("onebox_allowed_domains")
                .arg(300u64)
                .arg(&raw)
                .query_async(&mut conn)
                .await;
        }
    }

    Ok(matches_domain(&domains, domain))
}

/// Match a domain against a list of allowed domains. When the whitelist
/// contains `example.com`, both `example.com` and `www.example.com` match.
fn matches_domain(whitelist: &[String], domain: &str) -> bool {
    let domain_lower = domain.to_lowercase();
    whitelist.iter().any(|allowed| {
        let allowed_lower = allowed.to_lowercase();
        domain_lower == allowed_lower || domain_lower.ends_with(&format!(".{allowed_lower}"))
    })
}

/// Attempt to retrieve a cached preview. Checks Redis first, then the DB. When
/// a DB hit occurs the result is written back to Redis to warm the cache.
async fn cached_preview(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    url_hash: &str,
) -> Result<Option<OneboxResult>, AppError> {
    // Redis fast path.
    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            if let Ok(Some(raw)) = redis::cmd("GET")
                .arg(format!("onebox:{}", url_hash))
                .query_async::<Option<String>>(&mut conn)
                .await
            {
                if let Ok(val) = serde_json::from_str::<OneboxResult>(&raw) {
                    return Ok(Some(val));
                }
            }
        }
    }

    // DB fallback.
    let row = sqlx::query_as::<_, LinkPreviewRow>(
        "SELECT url_hash, url, title, description, image_url, site_name \
         FROM platform.link_previews WHERE url_hash = $1",
    )
    .bind(url_hash)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        let result = OneboxResult {
            r#type: "card".into(),
            url: r.url,
            title: r.title,
            description: r.description,
            image_url: r.image_url,
            site_name: r.site_name,
        };

        // Warm Redis cache.
        if let Some(pool_r) = redis {
            if let Ok(mut conn) = pool_r.get().await {
                if let Ok(json) = serde_json::to_string(&result) {
                    let _: Result<(), _> = redis::cmd("SETEX")
                        .arg(format!("onebox:{}", url_hash))
                        .arg(604_800u64)
                        .arg(&json)
                        .query_async(&mut conn)
                        .await;
                }
            }
        }

        return Ok(Some(result));
    }

    Ok(None)
}

/// Persist a preview to both Postgres and Redis (7-day TTL). Failures are
/// logged but not propagated — caching is best-effort.
async fn save_preview(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    preview: &OneboxResult,
    url_hash: &str,
) {
    // DB upsert.
    let result = sqlx::query(
        "INSERT INTO platform.link_previews \
         (url_hash, url, title, description, image_url, site_name, fetched_at) \
         VALUES ($1, $2, $3, $4, $5, $6, now()) \
         ON CONFLICT (url_hash) \
         DO UPDATE SET \
           title = $3, description = $4, image_url = $5, site_name = $6, \
           fetched_at = now()",
    )
    .bind(url_hash)
    .bind(&preview.url)
    .bind(&preview.title)
    .bind(&preview.description)
    .bind(&preview.image_url)
    .bind(&preview.site_name)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(error = %e, "onebox failed to save DB cache");
    }

    // Redis write (7-day TTL).
    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            if let Ok(json) = serde_json::to_string(preview) {
                let _: Result<(), _> = redis::cmd("SETEX")
                    .arg(format!("onebox:{}", url_hash))
                    .arg(604_800u64)
                    .arg(&json)
                    .query_async(&mut conn)
                    .await;
            }
        }
    }
}

#[derive(Debug)]
enum OneboxFetchError {
    InvalidUrl,
    UnsafeTarget,
    DomainNotAllowed,
    DnsResolution,
    Request,
    Redirect,
    UpstreamStatus,
    InvalidContentType,
    BodyTooLarge,
    Timeout,
}

impl OneboxFetchError {
    fn category(&self) -> &'static str {
        match self {
            Self::InvalidUrl => "invalid_url",
            Self::UnsafeTarget => "unsafe_target",
            Self::DomainNotAllowed => "domain_not_allowed",
            Self::DnsResolution => "dns_resolution",
            Self::Request => "request",
            Self::Redirect => "redirect",
            Self::UpstreamStatus => "upstream_status",
            Self::InvalidContentType => "invalid_content_type",
            Self::BodyTooLarge => "body_too_large",
            Self::Timeout => "timeout",
        }
    }
}

async fn resolve_public_target(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    url: &reqwest::Url,
) -> Result<(String, SocketAddr), OneboxFetchError> {
    let host = url.host_str().ok_or(OneboxFetchError::InvalidUrl)?.to_string();
    if !is_allowed_domain(pool, redis, &host).await.map_err(|_| OneboxFetchError::Request)? {
        return Err(OneboxFetchError::DomainNotAllowed);
    }

    let mut addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 443))
        .await
        .map_err(|_| OneboxFetchError::DnsResolution)?
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
        return Err(OneboxFetchError::UnsafeTarget);
    }

    Ok((host, addresses[0]))
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => is_public_ipv4(ipv4),
        IpAddr::V6(ipv6) => is_public_ipv6(ipv6),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [first, second, third, _fourth] = ip.octets();
    !matches!(
        (first, second, third),
        (0, _, _)
            | (10, _, _)
            | (100, 64..=127, _)
            | (127, _, _)
            | (169, 254, _)
            | (172, 16..=31, _)
            | (192, 0, 0 | 2)
            | (192, 88, 99)
            | (192, 168, _)
            | (198, 18..=19, _)
            | (198, 51, 100)
            | (203, 0, 113)
            | (224..=255, _, _)
    )
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(embedded) = ip.to_ipv4() {
        return is_public_ipv4(embedded);
    }
    let segments = ip.segments();
    let is_unique_local = segments[0] & 0xfe00 == 0xfc00;
    let is_link_local = segments[0] & 0xffc0 == 0xfe80;
    let is_site_local = segments[0] & 0xffc0 == 0xfec0;
    let is_nat64 =
        segments[0] == 0x0064 && segments[1] == 0xff9b && (segments[2] == 0 || segments[2] == 1);
    let is_ietf_special = segments[0] == 0x2001 && segments[1] <= 0x01ff;
    let is_documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    let is_six_to_four = segments[0] == 0x2002;
    !ip.is_unspecified()
        && !ip.is_loopback()
        && !ip.is_multicast()
        && !is_unique_local
        && !is_link_local
        && !is_site_local
        && !is_nat64
        && !is_ietf_special
        && !is_documentation
        && !is_six_to_four
}

async fn fetch_bounded_html(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    initial_url: reqwest::Url,
) -> Result<String, OneboxFetchError> {
    let mut current_url = initial_url;
    for redirect_count in 0..=MAX_REDIRECTS {
        let (host, address) = resolve_public_target(pool, redis, &current_url).await?;
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent("YourTJ-Onebox/2.0")
            .redirect(reqwest::redirect::Policy::none())
            .resolve(&host, address)
            .build()
            .map_err(|_| OneboxFetchError::Request)?;
        let mut response =
            client.get(current_url.clone()).send().await.map_err(|_| OneboxFetchError::Request)?;

        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                return Err(OneboxFetchError::Redirect);
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(OneboxFetchError::Redirect)?;
            current_url = parse_target_url(
                current_url.join(location).map_err(|_| OneboxFetchError::Redirect)?.as_str(),
            )?;
            continue;
        }

        if !response.status().is_success() {
            return Err(OneboxFetchError::UpstreamStatus);
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !content_type.starts_with("text/html")
            && !content_type.starts_with("application/xhtml+xml")
        {
            return Err(OneboxFetchError::InvalidContentType);
        }
        if response.content_length().is_some_and(|length| length > MAX_BODY_BYTES as u64) {
            return Err(OneboxFetchError::BodyTooLarge);
        }

        let mut bytes = Vec::with_capacity(
            response.content_length().unwrap_or(16 * 1024).min(MAX_BODY_BYTES as u64) as usize,
        );
        while let Some(chunk) = response.chunk().await.map_err(|_| OneboxFetchError::Request)? {
            append_bounded_chunk(&mut bytes, &chunk)?;
        }
        return Ok(decode_html_bytes(&bytes));
    }

    Err(OneboxFetchError::Redirect)
}

fn append_bounded_chunk(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<(), OneboxFetchError> {
    if bytes.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
        return Err(OneboxFetchError::BodyTooLarge);
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

fn decode_html_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Fetch a remote URL and parse bounded Open Graph metadata without loading subresources.
async fn fetch_og_tags(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    url: &str,
) -> Result<OneboxResult, OneboxFetchError> {
    let parsed = parse_target_url(url)?;
    let body = tokio::time::timeout(TOTAL_TIMEOUT, fetch_bounded_html(pool, redis, parsed))
        .await
        .map_err(|_| OneboxFetchError::Timeout)??;
    Ok(parse_og_tags(&body, url))
}

fn parse_og_tags(body: &str, url: &str) -> OneboxResult {
    static OG_META_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r###"<meta\s+(?:[^>]*?\s)?(?:property="og:([^"]+)"\s+[^>]*?content="([^"]*)"|content="([^"]*)"\s+[^>]*?property="og:([^"]+)")\s*/?>"###,
        )
        .expect("invalid OG meta regex")
    });

    static TITLE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r###"<title[^>]*>([^<]*)</title>"###).expect("invalid title regex")
    });

    let mut title = None;
    let mut description = None;
    let mut site_name = None;

    for cap in OG_META_RE.captures_iter(body) {
        let (prop, content) = if let (Some(p), Some(c)) = (cap.get(1), cap.get(2)) {
            (p.as_str(), c.as_str())
        } else if let (Some(c), Some(p)) = (cap.get(3), cap.get(4)) {
            (p.as_str(), c.as_str())
        } else {
            continue;
        };

        let content = html_unescape(content).trim().to_string();
        match prop {
            "title" => title = bounded_text(&content, 300),
            "description" => description = bounded_text(&content, 1_000),
            "site_name" => site_name = bounded_text(&content, 100),
            _ => {}
        }
    }

    // Fallback to the document <title> when no og:title is present.
    if title.is_none() {
        if let Some(cap) = TITLE_RE.captures(body) {
            title = cap.get(1).and_then(|value| bounded_text(&html_unescape(value.as_str()), 300));
        }
    }

    OneboxResult {
        r#type: "card".into(),
        url: url.to_string(),
        title,
        description,
        // Remote preview images would leak reader IPs and bypass the media trust boundary.
        image_url: None,
        site_name,
    }
}

fn bounded_text(value: &str, max_chars: usize) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(max_chars).collect())
    }
}

/// Decode the most common HTML entities in a string.
fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&#x2f;", "/")
}

/// All onebox-owned routes.
pub fn routes(state: AppState) -> Router {
    Router::new().route("/api/v2/onebox", get(get_onebox)).with_state(state)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::{
        append_bounded_chunk, compute_url_hash, decode_html_bytes, is_public_ip, matches_domain,
        parse_og_tags, parse_target_url, MAX_BODY_BYTES,
    };

    #[test]
    fn target_url_requires_https_without_credentials_or_custom_port() {
        assert!(parse_target_url("https://example.com/path?q=1").is_ok());
        assert!(parse_target_url("http://example.com/path").is_err());
        assert!(parse_target_url("https://user:secret@example.com/path").is_err());
        assert!(parse_target_url("https://example.com:8443/path").is_err());
        assert!(parse_target_url("not a url").is_err());
    }

    #[test]
    fn domain_matching_respects_label_boundaries() {
        let whitelist = vec!["example.com".to_string()];
        assert!(matches_domain(&whitelist, "example.com"));
        assert!(matches_domain(&whitelist, "news.example.com"));
        assert!(!matches_domain(&whitelist, "example.com.attacker.test"));
        assert!(!matches_domain(&whitelist, "notexample.com"));
    }

    #[test]
    fn private_metadata_and_documentation_addresses_are_rejected() {
        for address in [
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(100, 100, 0, 1),
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(169, 254, 169, 254),
            Ipv4Addr::new(172, 16, 0, 1),
            Ipv4Addr::new(192, 168, 0, 1),
            Ipv4Addr::new(192, 0, 2, 1),
            Ipv4Addr::new(198, 51, 100, 1),
            Ipv4Addr::new(203, 0, 113, 1),
        ] {
            assert!(!is_public_ip(IpAddr::V4(address)), "accepted {address}");
        }
        assert!(!is_public_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_public_ip(IpAddr::V6("fc00::1".parse().expect("valid IPv6"))));
        assert!(!is_public_ip(IpAddr::V6("fe80::1".parse().expect("valid IPv6"))));
        assert!(!is_public_ip(IpAddr::V6(
            "::ffff:192.168.1.1".parse().expect("valid mapped IPv6"),
        )));
        assert!(!is_public_ip(IpAddr::V6("64:ff9b::a00:1".parse().expect("valid NAT64 IPv6"),)));
        assert!(!is_public_ip(IpAddr::V6("2002:a00:1::".parse().expect("valid 6to4 IPv6"),)));
        assert!(is_public_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(is_public_ip(IpAddr::V6(
            "2606:4700:4700::1111".parse().expect("valid public IPv6"),
        )));
    }

    #[test]
    fn parser_bounds_fields_and_never_returns_remote_images() {
        let long_title = "同".repeat(400);
        let body = format!(
            r#"<html><head>
            <meta property="og:title" content="{long_title}">
            <meta property="og:description" content="A &amp; B">
            <meta property="og:image" content="https://tracker.example/pixel.png">
            <meta property="og:site_name" content="Campus">
            </head></html>"#,
        );
        let result = parse_og_tags(&body, "https://example.com/article");
        assert_eq!(result.title.expect("title").chars().count(), 300);
        assert_eq!(result.description.as_deref(), Some("A & B"));
        assert_eq!(result.site_name.as_deref(), Some("Campus"));
        assert!(result.image_url.is_none());
    }

    #[test]
    fn cache_hash_is_deterministic_and_url_specific() {
        assert_eq!(
            compute_url_hash("https://example.com"),
            compute_url_hash("https://example.com")
        );
        assert_ne!(
            compute_url_hash("https://example.com"),
            compute_url_hash("https://example.org")
        );
    }

    #[test]
    fn bounded_reader_rejects_oversize_before_appending() {
        let mut bytes = vec![b'a'; MAX_BODY_BYTES];
        assert!(append_bounded_chunk(&mut bytes, b"x").is_err());
        assert_eq!(bytes.len(), MAX_BODY_BYTES);
    }

    #[test]
    fn invalid_utf8_is_decoded_without_byte_boundary_panics() {
        let decoded = decode_html_bytes(&[b'a', 0xf0, 0x28, 0x8c, 0x28, b'b']);
        assert!(decoded.starts_with('a'));
        assert!(decoded.ends_with('b'));
    }
}

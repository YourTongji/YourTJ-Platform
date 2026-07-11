//! Onebox link preview module. Fetches OG meta tags from whitelisted domains and
//! returns preview cards. Cached in both Redis (fast) and Postgres (persistent).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use shared::{AppError, AppResult, AppState};
use sqlx::PgPool;

mod network;

const ONEBOX_POLICY_VERSION: &str = "v4";
const ERROR_CACHE_TTL_SECONDS: i64 = 120;
const SUCCESS_CACHE_TTL_SECONDS: i64 = 604_800;

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
struct LinkPreviewRow {
    url: String,
    title: Option<String>,
    description: Option<String>,
    site_name: Option<String>,
    status: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

enum CachedPreview {
    Ready(OneboxResult),
    Failed,
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

    if query.url.trim().is_empty() {
        return Err(AppError::BadRequest("url is required".into()));
    }

    let parsed = normalize_target_url(query.url.trim())
        .map_err(|_| AppError::BadRequest("URL must be a safe HTTPS URL".into()))?;
    let domain =
        parsed.host_str().ok_or_else(|| AppError::BadRequest("invalid URL".into()))?.to_string();
    let is_cacheable = is_persistently_cacheable_url(&parsed);
    let url = parsed.to_string();

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

    if let Some(cached) = cached_preview(state.redis.as_ref(), &state.db, &url_hash).await? {
        return match cached {
            CachedPreview::Ready(preview) => Ok(Json(preview)),
            CachedPreview::Failed => {
                Err(AppError::BadRequest("failed to fetch link preview".into()))
            }
        };
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
            save_failed_preview(
                &state.db,
                state.redis.as_ref(),
                &url,
                &url_hash,
                error.category(),
                is_cacheable,
            )
            .await;
            return Err(AppError::BadRequest("failed to fetch link preview".into()));
        }
    };
    result.url = url.clone();

    if is_cacheable {
        save_preview(&state.db, state.redis.as_ref(), &result, &url_hash).await;
    }

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

fn normalize_target_url(url: &str) -> Result<reqwest::Url, OneboxFetchError> {
    let mut parsed = parse_target_url(url)?;
    parsed.set_fragment(None);
    Ok(parsed)
}

fn is_persistently_cacheable_url(url: &reqwest::Url) -> bool {
    url.query().is_none()
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
) -> Result<Option<CachedPreview>, AppError> {
    // Redis fast path.
    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            if redis::cmd("EXISTS")
                .arg(format!("onebox:error:{url_hash}"))
                .query_async::<bool>(&mut conn)
                .await
                .unwrap_or(false)
            {
                return Ok(Some(CachedPreview::Failed));
            }
            if let Ok(Some(raw)) = redis::cmd("GET")
                .arg(format!("onebox:{url_hash}"))
                .query_async::<Option<String>>(&mut conn)
                .await
            {
                if let Ok(val) = serde_json::from_str::<OneboxResult>(&raw) {
                    return Ok(Some(CachedPreview::Ready(val)));
                }
            }
        }
    }

    // DB fallback.
    let row = sqlx::query_as::<_, LinkPreviewRow>(
        "SELECT url, title, description, site_name, status, expires_at \
         FROM platform.link_previews WHERE url_hash = $1",
    )
    .bind(url_hash)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        if r.expires_at <= chrono::Utc::now() {
            let _ = sqlx::query(
                "DELETE FROM platform.link_previews WHERE url_hash = $1 AND expires_at <= now()",
            )
            .bind(url_hash)
            .execute(pool)
            .await;
            return Ok(None);
        }
        let cache_seconds = (r.expires_at - chrono::Utc::now())
            .num_seconds()
            .clamp(1, SUCCESS_CACHE_TTL_SECONDS) as u64;
        if r.status == "error" {
            if let Some(pool_r) = redis {
                if let Ok(mut conn) = pool_r.get().await {
                    let _: Result<(), _> = redis::cmd("SETEX")
                        .arg(format!("onebox:error:{url_hash}"))
                        .arg(cache_seconds)
                        .arg("1")
                        .query_async(&mut conn)
                        .await;
                }
            }
            return Ok(Some(CachedPreview::Failed));
        }
        let result = OneboxResult {
            r#type: "card".into(),
            url: r.url,
            title: r.title,
            description: r.description,
            image_url: None,
            site_name: r.site_name,
        };

        // Warm Redis cache.
        if let Some(pool_r) = redis {
            if let Ok(mut conn) = pool_r.get().await {
                if let Ok(json) = serde_json::to_string(&result) {
                    let _: Result<(), _> = redis::cmd("SETEX")
                        .arg(format!("onebox:{url_hash}"))
                        .arg(cache_seconds)
                        .arg(&json)
                        .query_async(&mut conn)
                        .await;
                }
            }
        }

        return Ok(Some(CachedPreview::Ready(result)));
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
         (url_hash, url, title, description, site_name, fetched_at, \
          status, error_category, expires_at) \
         VALUES ($1, $2, $3, $4, $5, now(), 'ready', NULL, now() + interval '7 days') \
         ON CONFLICT (url_hash) \
         DO UPDATE SET \
           title = $3, description = $4, image_url = NULL, site_name = $5, \
           status = 'ready', error_category = NULL, fetched_at = now(), \
           expires_at = now() + interval '7 days'",
    )
    .bind(url_hash)
    .bind(&preview.url)
    .bind(&preview.title)
    .bind(&preview.description)
    .bind(&preview.site_name)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(error = %e, url_hash, "onebox failed to save DB cache");
    }

    // Redis write (7-day TTL).
    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            if let Ok(json) = serde_json::to_string(preview) {
                let _: Result<(), _> = redis::cmd("DEL")
                    .arg(format!("onebox:error:{url_hash}"))
                    .query_async(&mut conn)
                    .await;
                let _: Result<(), _> = redis::cmd("SETEX")
                    .arg(format!("onebox:{url_hash}"))
                    .arg(SUCCESS_CACHE_TTL_SECONDS as u64)
                    .arg(&json)
                    .query_async(&mut conn)
                    .await;
            }
        }
    }
}

async fn save_failed_preview(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    url: &str,
    url_hash: &str,
    error_category: &str,
    persist_url: bool,
) {
    if persist_url {
        let result = sqlx::query(
            "INSERT INTO platform.link_previews \
             (url_hash, url, title, description, image_url, site_name, fetched_at, \
              status, error_category, expires_at) \
             VALUES ($1, $2, NULL, NULL, NULL, NULL, now(), 'error', $3, \
                     now() + interval '120 seconds') \
             ON CONFLICT (url_hash) DO UPDATE SET \
               url = $2, title = NULL, description = NULL, image_url = NULL, site_name = NULL, \
               status = 'error', error_category = $3, fetched_at = now(), \
               expires_at = now() + interval '120 seconds'",
        )
        .bind(url_hash)
        .bind(url)
        .bind(error_category)
        .execute(pool)
        .await;
        if let Err(error) = result {
            tracing::warn!(%error, url_hash, "onebox failed to save negative cache");
        }
    }

    if let Some(pool_r) = redis {
        if let Ok(mut conn) = pool_r.get().await {
            let _: Result<(), _> =
                redis::cmd("DEL").arg(format!("onebox:{url_hash}")).query_async(&mut conn).await;
            let _: Result<(), _> = redis::cmd("SETEX")
                .arg(format!("onebox:error:{url_hash}"))
                .arg(ERROR_CACHE_TTL_SECONDS as u64)
                .arg("1")
                .query_async(&mut conn)
                .await;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OneboxFetchError {
    InvalidUrl,
    UnsafeTarget,
    DomainNotAllowed,
    DnsResolution,
    Request,
    RequestTimeout,
    Redirect,
    UpstreamStatus,
    InvalidContentType,
    InvalidCharset,
    BodyTooLarge,
    TotalTimeout,
}

impl OneboxFetchError {
    fn category(&self) -> &'static str {
        match self {
            Self::InvalidUrl => "invalid_url",
            Self::UnsafeTarget => "unsafe_target",
            Self::DomainNotAllowed => "domain_not_allowed",
            Self::DnsResolution => "dns_resolution",
            Self::Request => "request",
            Self::RequestTimeout => "request_timeout",
            Self::Redirect => "redirect",
            Self::UpstreamStatus => "upstream_status",
            Self::InvalidContentType => "invalid_content_type",
            Self::InvalidCharset => "invalid_charset",
            Self::BodyTooLarge => "body_too_large",
            Self::TotalTimeout => "total_timeout",
        }
    }
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

/// Fetch a remote URL and parse bounded Open Graph metadata without loading subresources.
async fn fetch_og_tags(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    url: &str,
) -> Result<OneboxResult, OneboxFetchError> {
    let parsed = parse_target_url(url)?;
    let body = network::fetch_bounded_html(pool, redis, parsed).await?;
    Ok(parse_og_tags(&body, url))
}

fn parse_og_tags(body: &str, url: &str) -> OneboxResult {
    let document = Html::parse_document(body);
    let mut title = None;
    let mut description = None;
    let mut fallback_description = None;
    let mut site_name = None;

    if let Ok(meta_selector) = Selector::parse("meta") {
        for element in document.select(&meta_selector) {
            let Some(content) = element.value().attr("content") else {
                continue;
            };
            let property =
                element.value().attr("property").map(str::trim).map(str::to_ascii_lowercase);
            match property.as_deref() {
                Some("og:title") if title.is_none() => title = bounded_text(content, 300),
                Some("og:description") if description.is_none() => {
                    description = bounded_text(content, 1_000)
                }
                Some("og:site_name") if site_name.is_none() => {
                    site_name = bounded_text(content, 100)
                }
                _ => {}
            }
            if fallback_description.is_none()
                && element
                    .value()
                    .attr("name")
                    .is_some_and(|name| name.eq_ignore_ascii_case("description"))
            {
                fallback_description = bounded_text(content, 1_000);
            }
        }
    }

    if description.is_none() {
        description = fallback_description;
    }

    if title.is_none() {
        if let Ok(title_selector) = Selector::parse("title") {
            title = document.select(&title_selector).next().and_then(|element| {
                let text = element.text().collect::<String>();
                bounded_text(&text, 300)
            });
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
    let without_controls: String = value
        .chars()
        .map(|character| if character.is_control() { ' ' } else { character })
        .collect();
    let normalized = without_controls.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.chars().take(max_chars).collect())
    }
}

/// All onebox-owned routes.
pub fn routes(state: AppState) -> Router {
    Router::new().route("/api/v2/onebox", get(get_onebox)).with_state(state)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::{
        compute_url_hash, is_persistently_cacheable_url, is_public_ip, matches_domain,
        normalize_target_url, parse_og_tags, parse_target_url, OneboxResult,
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
    fn normalized_url_removes_fragments_and_canonicalizes_the_host() {
        let normalized =
            normalize_target_url("https://EXAMPLE.com:443/a%2Fb?q=1#section").expect("safe URL");
        assert_eq!(normalized.as_str(), "https://example.com/a%2Fb?q=1");
        assert_eq!(
            compute_url_hash(normalized.as_str()),
            compute_url_hash("https://example.com/a%2Fb?q=1")
        );
        assert!(!is_persistently_cacheable_url(&normalized));
        assert!(is_persistently_cacheable_url(
            &normalize_target_url("https://example.com/article#comments").expect("safe URL")
        ));
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
    fn result_serializes_the_public_contract_field_names() {
        let result = OneboxResult {
            r#type: "card".into(),
            url: "https://example.com/".into(),
            title: Some("Example".into()),
            description: None,
            image_url: None,
            site_name: Some("Example Site".into()),
        };

        let value = serde_json::to_value(result).expect("Onebox result must serialize");
        assert_eq!(value["type"], "card");
        assert_eq!(value["url"], "https://example.com/");
        assert_eq!(value["imageUrl"], serde_json::Value::Null);
        assert_eq!(value["siteName"], "Example Site");
        assert!(value.get("image").is_none());
        assert!(value.get("image_url").is_none());
    }

    #[test]
    fn html5_parser_handles_attribute_order_case_entities_and_malformed_markup() {
        let body = r#"<HTML><head>
            <meta CONTENT='A &amp; B' PROPERTY='OG:TITLE'>
            <meta content='fallback should lose' name='description'>
            <meta content='Canonical description' property='og:description'>
            <meta property='og:site_name' content=' Campus&#10; News '>
            <title>Ignored <b>fallback</b></title>
            <meta property='og:image' content='https://tracker.example/pixel.png'>
            <meta property='og:title' content='later title'>
        </head><body><p>unclosed"#;
        let result = parse_og_tags(body, "https://example.com/article");
        assert_eq!(result.title.as_deref(), Some("A & B"));
        assert_eq!(result.description.as_deref(), Some("Canonical description"));
        assert_eq!(result.site_name.as_deref(), Some("Campus News"));
        assert!(result.image_url.is_none());
    }

    #[test]
    fn parser_uses_document_title_and_meta_description_fallbacks() {
        let body = r#"<title>Campus &amp; update</title>
                      <meta name="description" content="  Important   notice  ">"#;
        let result = parse_og_tags(body, "https://example.com/article");
        assert_eq!(result.title.as_deref(), Some("Campus & update"));
        assert_eq!(result.description.as_deref(), Some("Important notice"));
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
}

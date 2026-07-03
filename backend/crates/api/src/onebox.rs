//! Onebox link preview module. Fetches OG meta tags from whitelisted domains and
//! returns preview cards. Cached in both Redis (fast) and Postgres (persistent).

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

    let domain = extract_domain(&url).ok_or_else(|| AppError::BadRequest("invalid URL".into()))?;

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
    let mut result = match fetch_og_tags(&url).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, %url, "onebox fetch failed");
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

/// Extract the hostname from a URL string. Returns `None` if the URL has no
/// recognised scheme or no hostname.
fn extract_domain(url: &str) -> Option<String> {
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))?;
    let hostname = rest.split(['/', '?', '#']).next()?;
    let hostname = hostname.split(':').next()?;
    if hostname.is_empty() {
        None
    } else {
        Some(hostname.to_lowercase())
    }
}

/// Compute the SHA-256 hexadecimal hash of a URL for use as a cache key.
fn compute_url_hash(url: &str) -> String {
    let mut hasher = sha2::Sha256::new();
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

/// Fetch a remote URL and parse Open Graph meta tags from its HTML.
/// - 3-second HTTP timeout
/// - 512 KB body limit
/// - No JavaScript execution
/// - User-Agent: YourTJ-Onebox/1.0
async fn fetch_og_tags(url: &str) -> Result<OneboxResult, anyhow::Error> {
    static OG_META_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r###"<meta\s+(?:[^>]*?\s)?(?:property="og:([^"]+)"\s+[^>]*?content="([^"]*)"|content="([^"]*)"\s+[^>]*?property="og:([^"]+)")\s*/?>"###,
        )
        .expect("invalid OG meta regex")
    });

    static TITLE_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r###"<title[^>]*>([^<]*)</title>"###).expect("invalid title regex")
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent("YourTJ-Onebox/1.0")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let resp = client.get(url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status}");
    }

    let body = resp.text().await?;

    // Truncate to 512 KB to limit memory usage.
    let body = if body.len() > 524_288 { &body[..524_288] } else { body.as_str() };

    let mut title = None;
    let mut description = None;
    let mut image_url = None;
    let mut site_name = None;

    for cap in OG_META_RE.captures_iter(body) {
        let (prop, content) = if let (Some(p), Some(c)) = (cap.get(1), cap.get(2)) {
            (p.as_str(), c.as_str())
        } else if let (Some(c), Some(p)) = (cap.get(3), cap.get(4)) {
            (p.as_str(), c.as_str())
        } else {
            continue;
        };

        let content = html_unescape(content);
        match prop {
            "title" => title = Some(content),
            "description" => description = Some(content),
            "image" => image_url = Some(content),
            "site_name" => site_name = Some(content),
            _ => {}
        }
    }

    // Fallback to the document <title> when no og:title is present.
    if title.is_none() {
        if let Some(cap) = TITLE_RE.captures(body) {
            title = cap.get(1).map(|m| html_unescape(m.as_str()));
        }
    }

    Ok(OneboxResult {
        r#type: "card".into(),
        url: url.to_string(),
        title,
        description,
        image_url,
        site_name,
    })
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

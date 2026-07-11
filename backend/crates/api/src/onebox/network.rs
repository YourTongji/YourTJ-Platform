//! Network trust boundary for Onebox HTTPS fetches.

use std::net::SocketAddr;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::{Response, Url};
use sqlx::PgPool;

use super::{is_allowed_domain, is_public_ip, normalize_target_url, OneboxFetchError};

const MAX_BODY_BYTES: usize = 512 * 1024;
const MAX_REDIRECTS: usize = 5;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Clone, Copy)]
struct FetchPolicy {
    max_body_bytes: usize,
    max_redirects: usize,
    request_timeout: Duration,
    total_timeout: Duration,
}

const PRODUCTION_FETCH_POLICY: FetchPolicy = FetchPolicy {
    max_body_bytes: MAX_BODY_BYTES,
    max_redirects: MAX_REDIRECTS,
    request_timeout: REQUEST_TIMEOUT,
    total_timeout: TOTAL_TIMEOUT,
};

#[async_trait]
trait DomainAllowlist: Send + Sync {
    async fn contains(&self, host: &str) -> Result<bool, OneboxFetchError>;
}

struct SettingsDomainAllowlist<'a> {
    pool: &'a PgPool,
    redis: Option<&'a deadpool_redis::Pool>,
}

#[async_trait]
impl DomainAllowlist for SettingsDomainAllowlist<'_> {
    async fn contains(&self, host: &str) -> Result<bool, OneboxFetchError> {
        is_allowed_domain(self.pool, self.redis, host).await.map_err(|_| OneboxFetchError::Request)
    }
}

#[async_trait]
trait HostResolver: Send + Sync {
    async fn resolve(&self, host: &str) -> Result<Vec<SocketAddr>, OneboxFetchError>;
}

struct SystemHostResolver;

#[async_trait]
impl HostResolver for SystemHostResolver {
    async fn resolve(&self, host: &str) -> Result<Vec<SocketAddr>, OneboxFetchError> {
        tokio::net::lookup_host((host, 443))
            .await
            .map(|addresses| addresses.collect())
            .map_err(|_| OneboxFetchError::DnsResolution)
    }
}

#[async_trait]
trait HttpsTransport: Send + Sync {
    async fn get(
        &self,
        url: Url,
        host: &str,
        verified_address: SocketAddr,
        timeout: Duration,
    ) -> Result<Response, OneboxFetchError>;
}

#[derive(Default)]
struct ReqwestHttpsTransport {
    #[cfg(test)]
    fixture: Option<FixtureTransportOverride>,
}

#[cfg(test)]
#[derive(Clone)]
struct FixtureTransportOverride {
    address: SocketAddr,
    root_certificate: reqwest::Certificate,
    observed_pins: std::sync::Arc<std::sync::Mutex<Vec<(String, SocketAddr)>>>,
}

#[cfg(test)]
impl ReqwestHttpsTransport {
    fn for_fixture(
        address: SocketAddr,
        root_certificate: reqwest::Certificate,
        observed_pins: std::sync::Arc<std::sync::Mutex<Vec<(String, SocketAddr)>>>,
    ) -> Self {
        Self {
            fixture: Some(FixtureTransportOverride { address, root_certificate, observed_pins }),
        }
    }
}

#[async_trait]
impl HttpsTransport for ReqwestHttpsTransport {
    async fn get(
        &self,
        url: Url,
        host: &str,
        verified_address: SocketAddr,
        timeout: Duration,
    ) -> Result<Response, OneboxFetchError> {
        #[cfg(not(test))]
        let (builder, connection_address) = (reqwest::Client::builder(), verified_address);
        #[cfg(test)]
        let (builder, connection_address) = match &self.fixture {
            Some(fixture) => {
                fixture
                    .observed_pins
                    .lock()
                    .expect("fixture pin recorder lock must remain usable")
                    .push((host.to_string(), verified_address));
                (
                    reqwest::Client::builder()
                        .add_root_certificate(fixture.root_certificate.clone()),
                    fixture.address,
                )
            }
            None => (reqwest::Client::builder(), verified_address),
        };

        let client = builder
            .timeout(timeout)
            .user_agent("YourTJ-Onebox/2.0")
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve(host, connection_address)
            .build()
            .map_err(|_| OneboxFetchError::Request)?;

        client.get(url).send().await.map_err(classify_request_error)
    }
}

fn classify_request_error(error: reqwest::Error) -> OneboxFetchError {
    if error.is_timeout() {
        OneboxFetchError::RequestTimeout
    } else {
        OneboxFetchError::Request
    }
}

pub(super) async fn fetch_bounded_html(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    initial_url: Url,
) -> Result<String, OneboxFetchError> {
    let allowlist = SettingsDomainAllowlist { pool, redis };
    let resolver = SystemHostResolver;
    let transport = ReqwestHttpsTransport::default();
    fetch_bounded_html_with(&allowlist, &resolver, &transport, initial_url, PRODUCTION_FETCH_POLICY)
        .await
}

async fn fetch_bounded_html_with<A, R, T>(
    allowlist: &A,
    resolver: &R,
    transport: &T,
    initial_url: Url,
    policy: FetchPolicy,
) -> Result<String, OneboxFetchError>
where
    A: DomainAllowlist,
    R: HostResolver,
    T: HttpsTransport,
{
    tokio::time::timeout(
        policy.total_timeout,
        fetch_bounded_html_inner(allowlist, resolver, transport, initial_url, policy),
    )
    .await
    .map_err(|_| OneboxFetchError::TotalTimeout)?
}

async fn fetch_bounded_html_inner<A, R, T>(
    allowlist: &A,
    resolver: &R,
    transport: &T,
    initial_url: Url,
    policy: FetchPolicy,
) -> Result<String, OneboxFetchError>
where
    A: DomainAllowlist,
    R: HostResolver,
    T: HttpsTransport,
{
    let mut current_url = initial_url;
    for redirect_count in 0..=policy.max_redirects {
        let (host, address) = resolve_public_target(allowlist, resolver, &current_url).await?;
        let mut response =
            transport.get(current_url.clone(), &host, address, policy.request_timeout).await?;

        if response.status().is_redirection() {
            if redirect_count == policy.max_redirects {
                return Err(OneboxFetchError::Redirect);
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(OneboxFetchError::Redirect)?;
            current_url = normalize_target_url(
                current_url.join(location).map_err(|_| OneboxFetchError::Redirect)?.as_str(),
            )?;
            continue;
        }

        if !response.status().is_success() {
            return Err(OneboxFetchError::UpstreamStatus);
        }
        validate_html_content_type(response.headers())?;
        if response.content_length().is_some_and(|length| length > policy.max_body_bytes as u64) {
            return Err(OneboxFetchError::BodyTooLarge);
        }

        let mut bytes = Vec::with_capacity(
            response.content_length().unwrap_or(16 * 1024).min(policy.max_body_bytes as u64)
                as usize,
        );
        while let Some(chunk) = response.chunk().await.map_err(classify_request_error)? {
            append_bounded_chunk(&mut bytes, &chunk, policy.max_body_bytes)?;
        }
        return Ok(decode_html_bytes(&bytes));
    }

    Err(OneboxFetchError::Redirect)
}

async fn resolve_public_target<A, R>(
    allowlist: &A,
    resolver: &R,
    url: &Url,
) -> Result<(String, SocketAddr), OneboxFetchError>
where
    A: DomainAllowlist,
    R: HostResolver,
{
    let host = url.host_str().ok_or(OneboxFetchError::InvalidUrl)?.to_string();
    if !allowlist.contains(&host).await? {
        return Err(OneboxFetchError::DomainNotAllowed);
    }

    let mut addresses = resolver.resolve(&host).await?;
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty()
        || addresses.iter().any(|address| address.port() != 443 || !is_public_ip(address.ip()))
    {
        return Err(OneboxFetchError::UnsafeTarget);
    }

    Ok((host, addresses[0]))
}

fn validate_html_content_type(headers: &HeaderMap) -> Result<(), OneboxFetchError> {
    let raw = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(OneboxFetchError::InvalidContentType)?;
    let mut segments = raw.split(';');
    let media_type = segments.next().unwrap_or_default().trim();
    if !media_type.eq_ignore_ascii_case("text/html")
        && !media_type.eq_ignore_ascii_case("application/xhtml+xml")
    {
        return Err(OneboxFetchError::InvalidContentType);
    }

    for parameter in segments {
        let Some((name, value)) = parameter.split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("charset") {
            continue;
        }
        let charset = value.trim().trim_matches('"');
        if charset.is_empty()
            || (!charset.eq_ignore_ascii_case("utf-8") && !charset.eq_ignore_ascii_case("utf8"))
        {
            return Err(OneboxFetchError::InvalidCharset);
        }
    }
    Ok(())
}

fn append_bounded_chunk(
    bytes: &mut Vec<u8>,
    chunk: &[u8],
    max_body_bytes: usize,
) -> Result<(), OneboxFetchError> {
    if bytes.len().saturating_add(chunk.len()) > max_body_bytes {
        return Err(OneboxFetchError::BodyTooLarge);
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

fn decode_html_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
#[path = "../../tests/helpers/onebox_https_fixture.rs"]
mod tests;

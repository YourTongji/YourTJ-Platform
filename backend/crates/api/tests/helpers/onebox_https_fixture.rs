//! Controlled HTTPS regression tests for the Onebox production state machine.

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

use super::{
    fetch_bounded_html_with, DomainAllowlist, FetchPolicy, HostResolver, OneboxFetchError,
    ReqwestHttpsTransport, MAX_BODY_BYTES, PRODUCTION_FETCH_POLICY,
};
use crate::onebox::{normalize_target_url, parse_og_tags};

const ALLOWED_HOST: &str = "allowed.test";
const REDIRECT_HOST: &str = "redirect.test";
const CONTENT_HOST: &str = "content.test";

#[derive(Clone, Debug)]
struct RequestRecord {
    sni: Option<String>,
    host: String,
    path: String,
}

#[derive(Clone)]
enum FixtureBody {
    Fixed(Vec<u8>),
    Chunked { chunks: Vec<Vec<u8>>, delay_between_chunks: Duration },
}

#[derive(Clone)]
struct FixtureResponse {
    status: &'static str,
    headers: Vec<(String, String)>,
    body: FixtureBody,
    header_delay: Duration,
}

impl FixtureResponse {
    fn html(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: "200 OK",
            headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            body: FixtureBody::Fixed(body.into()),
            header_delay: Duration::ZERO,
        }
    }

    fn redirect(location: &str) -> Self {
        Self {
            status: "302 Found",
            headers: vec![("Location".into(), location.to_string())],
            body: FixtureBody::Fixed(Vec::new()),
            header_delay: Duration::ZERO,
        }
    }

    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.header_delay = delay;
        self
    }
}

struct HttpsFixture {
    address: SocketAddr,
    root_certificate: reqwest::Certificate,
    requests: Arc<Mutex<Vec<RequestRecord>>>,
    accept_task: JoinHandle<()>,
}

impl HttpsFixture {
    async fn start(routes: HashMap<String, FixtureResponse>) -> Self {
        let CertifiedKey { cert, key_pair } = generate_simple_self_signed(vec![
            ALLOWED_HOST.to_string(),
            REDIRECT_HOST.to_string(),
            CONTENT_HOST.to_string(),
        ])
        .expect("fixture certificate must be generated");
        let certificate_der = cert.der().clone();
        let private_key = PrivatePkcs8KeyDer::from(key_pair.serialize_der());
        let mut server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate_der.clone()], private_key.into())
            .expect("fixture certificate and key must match");
        server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("fixture listener must bind locally");
        let address = listener.local_addr().expect("fixture must expose its address");
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let routes = Arc::new(routes);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let task_requests = Arc::clone(&requests);
        let accept_task = tokio::spawn(async move {
            while let Ok((stream, _peer)) = listener.accept().await {
                let connection_acceptor = acceptor.clone();
                let connection_routes = Arc::clone(&routes);
                let connection_requests = Arc::clone(&task_requests);
                tokio::spawn(async move {
                    let Ok(tls_stream) = connection_acceptor.accept(stream).await else {
                        return;
                    };
                    serve_request(tls_stream, &connection_routes, &connection_requests).await;
                });
            }
        });

        Self {
            address,
            root_certificate: reqwest::Certificate::from_der(certificate_der.as_ref())
                .expect("fixture certificate must be accepted by reqwest"),
            requests,
            accept_task,
        }
    }

    fn transport(
        &self,
        observed_pins: Arc<Mutex<Vec<(String, SocketAddr)>>>,
    ) -> ReqwestHttpsTransport {
        ReqwestHttpsTransport::for_fixture(
            self.address,
            self.root_certificate.clone(),
            observed_pins,
        )
    }

    fn requests(&self) -> Vec<RequestRecord> {
        self.requests.lock().expect("fixture request recorder lock must remain usable").clone()
    }
}

impl Drop for HttpsFixture {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

async fn serve_request(
    mut tls_stream: TlsStream<TcpStream>,
    routes: &HashMap<String, FixtureResponse>,
    requests: &Mutex<Vec<RequestRecord>>,
) {
    let mut request_bytes = Vec::new();
    let mut buffer = [0_u8; 1_024];
    while request_bytes.len() < 16 * 1_024 {
        let Ok(read) = tls_stream.read(&mut buffer).await else {
            return;
        };
        if read == 0 {
            return;
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let request_text = String::from_utf8_lossy(&request_bytes);
    let mut lines = request_text.lines();
    let path =
        lines.next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("/").to_string();
    let host = lines
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _value)| name.eq_ignore_ascii_case("host"))
                .map(|(_name, value)| value.trim().trim_end_matches(":443").to_string())
        })
        .unwrap_or_default();
    let record = RequestRecord {
        sni: tls_stream.get_ref().1.server_name().map(str::to_string),
        host: host.clone(),
        path: path.clone(),
    };
    requests.lock().expect("fixture request recorder lock must remain usable").push(record);

    let key = format!("{host}{path}");
    let response = routes.get(&key).cloned().unwrap_or(FixtureResponse {
        status: "404 Not Found",
        headers: vec![("Content-Type".into(), "text/plain".into())],
        body: FixtureBody::Fixed(b"not found".to_vec()),
        header_delay: Duration::ZERO,
    });
    tokio::time::sleep(response.header_delay).await;

    let has_content_length =
        response.headers.iter().any(|(name, _value)| name.eq_ignore_ascii_case("content-length"));
    let mut response_head = format!("HTTP/1.1 {}\r\nConnection: close\r\n", response.status);
    for (name, value) in &response.headers {
        response_head.push_str(name);
        response_head.push_str(": ");
        response_head.push_str(value);
        response_head.push_str("\r\n");
    }
    match &response.body {
        FixtureBody::Fixed(body) if !has_content_length => {
            response_head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        FixtureBody::Chunked { .. } => {
            response_head.push_str("Transfer-Encoding: chunked\r\n");
        }
        FixtureBody::Fixed(_) => {}
    }
    response_head.push_str("\r\n");
    if tls_stream.write_all(response_head.as_bytes()).await.is_err() {
        return;
    }

    match response.body {
        FixtureBody::Fixed(body) => {
            let _result = tls_stream.write_all(&body).await;
        }
        FixtureBody::Chunked { chunks, delay_between_chunks } => {
            for chunk in chunks {
                let chunk_head = format!("{:X}\r\n", chunk.len());
                if tls_stream.write_all(chunk_head.as_bytes()).await.is_err()
                    || tls_stream.write_all(&chunk).await.is_err()
                    || tls_stream.write_all(b"\r\n").await.is_err()
                {
                    return;
                }
                tokio::time::sleep(delay_between_chunks).await;
            }
            let _result = tls_stream.write_all(b"0\r\n\r\n").await;
        }
    }
    let _result = tls_stream.shutdown().await;
}

struct StaticAllowlist {
    allowed: HashSet<String>,
    observed_hosts: Arc<Mutex<Vec<String>>>,
}

impl StaticAllowlist {
    fn new(hosts: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            allowed: hosts.into_iter().map(str::to_string).collect(),
            observed_hosts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn observed_hosts(&self) -> Vec<String> {
        self.observed_hosts.lock().expect("allowlist recorder lock must remain usable").clone()
    }
}

#[async_trait]
impl DomainAllowlist for StaticAllowlist {
    async fn contains(&self, host: &str) -> Result<bool, OneboxFetchError> {
        self.observed_hosts
            .lock()
            .expect("allowlist recorder lock must remain usable")
            .push(host.to_string());
        Ok(self.allowed.contains(host))
    }
}

struct ScriptedResolver {
    answers: Mutex<HashMap<String, VecDeque<Vec<SocketAddr>>>>,
    observed_hosts: Arc<Mutex<Vec<String>>>,
}

impl ScriptedResolver {
    fn new(entries: impl IntoIterator<Item = (&'static str, Vec<Vec<SocketAddr>>)>) -> Self {
        Self {
            answers: Mutex::new(
                entries
                    .into_iter()
                    .map(|(host, answers)| (host.to_string(), answers.into()))
                    .collect(),
            ),
            observed_hosts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn observed_hosts(&self) -> Vec<String> {
        self.observed_hosts.lock().expect("resolver recorder lock must remain usable").clone()
    }
}

#[async_trait]
impl HostResolver for ScriptedResolver {
    async fn resolve(&self, host: &str) -> Result<Vec<SocketAddr>, OneboxFetchError> {
        self.observed_hosts
            .lock()
            .expect("resolver recorder lock must remain usable")
            .push(host.to_string());
        self.answers
            .lock()
            .expect("resolver script lock must remain usable")
            .get_mut(host)
            .and_then(VecDeque::pop_front)
            .ok_or(OneboxFetchError::DnsResolution)
    }
}

fn public_address(last_octet: u8) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::new(8, 8, 8, last_octet), 443))
}

fn fixture_url(host: &str, path: &str) -> reqwest::Url {
    normalize_target_url(&format!("https://{host}{path}"))
        .expect("fixture URL must satisfy production URL policy")
}

#[tokio::test]
async fn tls_fixture_preserves_host_and_sni_while_using_the_verified_dns_pin() {
    let fixture = HttpsFixture::start(HashMap::from([(
        format!("{ALLOWED_HOST}/ok"),
        FixtureResponse::html(br#"<meta property="og:title" content="Pinned TLS">"#.to_vec()),
    )]))
    .await;
    let allowlist = StaticAllowlist::new([ALLOWED_HOST]);
    let pinned_address = public_address(8);
    let resolver = ScriptedResolver::new([(ALLOWED_HOST, vec![vec![pinned_address]])]);
    let observed_pins = Arc::new(Mutex::new(Vec::new()));
    let transport = fixture.transport(Arc::clone(&observed_pins));

    let body = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(ALLOWED_HOST, "/ok"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect("trusted fixture page must load");
    let preview = parse_og_tags(&body, "https://allowed.test/ok");

    assert_eq!(preview.title.as_deref(), Some("Pinned TLS"));
    let requests = fixture.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].sni.as_deref(), Some(ALLOWED_HOST));
    assert_eq!(requests[0].host, ALLOWED_HOST);
    assert_eq!(requests[0].path, "/ok");
    assert_eq!(
        *observed_pins.lock().expect("pin recorder lock must remain usable"),
        vec![(ALLOWED_HOST.to_string(), pinned_address)]
    );
}

#[tokio::test]
async fn tls_hostname_verification_rejects_a_trusted_certificate_for_another_host() {
    let fixture = HttpsFixture::start(HashMap::new()).await;
    let host = "wrong-host.test";
    let allowlist = StaticAllowlist::new([host]);
    let resolver = ScriptedResolver::new([(host, vec![vec![public_address(8)]])]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(host, "/"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect_err("hostname mismatch must fail TLS validation");

    assert_eq!(error, OneboxFetchError::Request);
    assert!(fixture.requests().is_empty());
}

#[tokio::test]
async fn every_redirect_rechecks_allowlist_dns_and_the_public_address_pin() {
    let fixture = HttpsFixture::start(HashMap::from([
        (
            format!("{ALLOWED_HOST}/start"),
            FixtureResponse::redirect(&format!("https://{REDIRECT_HOST}/final")),
        ),
        (format!("{REDIRECT_HOST}/final"), FixtureResponse::html("<title>redirected</title>")),
    ]))
    .await;
    let allowlist = StaticAllowlist::new([ALLOWED_HOST, REDIRECT_HOST]);
    let first_pin = public_address(8);
    let second_pin = public_address(9);
    let resolver = ScriptedResolver::new([
        (ALLOWED_HOST, vec![vec![first_pin]]),
        (REDIRECT_HOST, vec![vec![second_pin]]),
    ]);
    let observed_pins = Arc::new(Mutex::new(Vec::new()));
    let transport = fixture.transport(Arc::clone(&observed_pins));

    let body = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(ALLOWED_HOST, "/start"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect("allowed redirect chain must load");

    assert!(body.contains("redirected"));
    assert_eq!(allowlist.observed_hosts(), vec![ALLOWED_HOST, REDIRECT_HOST]);
    assert_eq!(resolver.observed_hosts(), vec![ALLOWED_HOST, REDIRECT_HOST]);
    assert_eq!(
        *observed_pins.lock().expect("pin recorder lock must remain usable"),
        vec![(ALLOWED_HOST.to_string(), first_pin), (REDIRECT_HOST.to_string(), second_pin),]
    );
    assert_eq!(
        fixture
            .requests()
            .iter()
            .map(|request| (request.host.as_str(), request.sni.as_deref()))
            .collect::<Vec<_>>(),
        vec![(ALLOWED_HOST, Some(ALLOWED_HOST)), (REDIRECT_HOST, Some(REDIRECT_HOST)),]
    );
}

#[tokio::test]
async fn redirect_to_a_disallowed_host_stops_before_dns_or_network_access() {
    let blocked_host = "blocked.test";
    let fixture = HttpsFixture::start(HashMap::from([(
        format!("{ALLOWED_HOST}/start"),
        FixtureResponse::redirect(&format!("https://{blocked_host}/final")),
    )]))
    .await;
    let allowlist = StaticAllowlist::new([ALLOWED_HOST]);
    let resolver = ScriptedResolver::new([(ALLOWED_HOST, vec![vec![public_address(8)]])]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(ALLOWED_HOST, "/start"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect_err("disallowed redirect target must fail closed");

    assert_eq!(error, OneboxFetchError::DomainNotAllowed);
    assert_eq!(allowlist.observed_hosts(), vec![ALLOWED_HOST, blocked_host]);
    assert_eq!(resolver.observed_hosts(), vec![ALLOWED_HOST]);
    assert_eq!(fixture.requests().len(), 1);
}

#[tokio::test]
async fn dns_rebinding_to_a_private_address_fails_before_the_second_request() {
    let fixture = HttpsFixture::start(HashMap::from([(
        format!("{ALLOWED_HOST}/start"),
        FixtureResponse::redirect("/final"),
    )]))
    .await;
    let allowlist = StaticAllowlist::new([ALLOWED_HOST]);
    let resolver = ScriptedResolver::new([(
        ALLOWED_HOST,
        vec![vec![public_address(8)], vec![SocketAddr::from((Ipv4Addr::LOCALHOST, 443))]],
    )]);
    let observed_pins = Arc::new(Mutex::new(Vec::new()));
    let transport = fixture.transport(Arc::clone(&observed_pins));

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(ALLOWED_HOST, "/start"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect_err("private rebound address must fail closed");

    assert_eq!(error, OneboxFetchError::UnsafeTarget);
    assert_eq!(resolver.observed_hosts(), vec![ALLOWED_HOST, ALLOWED_HOST]);
    assert_eq!(fixture.requests().len(), 1);
    assert_eq!(observed_pins.lock().expect("pin recorder lock must remain usable").len(), 1);
}

#[tokio::test]
async fn any_private_address_in_a_dns_answer_fails_closed_before_transport() {
    let fixture = HttpsFixture::start(HashMap::new()).await;
    let allowlist = StaticAllowlist::new([ALLOWED_HOST]);
    let resolver = ScriptedResolver::new([(
        ALLOWED_HOST,
        vec![vec![public_address(8), SocketAddr::from((Ipv4Addr::new(169, 254, 169, 254), 443))]],
    )]);
    let observed_pins = Arc::new(Mutex::new(Vec::new()));
    let transport = fixture.transport(Arc::clone(&observed_pins));

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(ALLOWED_HOST, "/"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect_err("mixed public and metadata DNS answers must fail closed");

    assert_eq!(error, OneboxFetchError::UnsafeTarget);
    assert!(observed_pins.lock().expect("pin recorder lock must remain usable").is_empty());
    assert!(fixture.requests().is_empty());
}

#[tokio::test]
async fn non_html_and_html_prefix_content_types_are_rejected() {
    let fixture = HttpsFixture::start(HashMap::from([
        (
            format!("{CONTENT_HOST}/plain"),
            FixtureResponse {
                status: "200 OK",
                headers: vec![("Content-Type".into(), "text/plain".into())],
                body: FixtureBody::Fixed(b"plain".to_vec()),
                header_delay: Duration::ZERO,
            },
        ),
        (
            format!("{CONTENT_HOST}/prefix"),
            FixtureResponse {
                status: "200 OK",
                headers: vec![("Content-Type".into(), "text/htmlx".into())],
                body: FixtureBody::Fixed(b"not html".to_vec()),
                header_delay: Duration::ZERO,
            },
        ),
    ]))
    .await;
    let allowlist = StaticAllowlist::new([CONTENT_HOST]);
    let resolver = ScriptedResolver::new([(
        CONTENT_HOST,
        vec![vec![public_address(8)], vec![public_address(8)]],
    )]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));

    for path in ["/plain", "/prefix"] {
        let error = fetch_bounded_html_with(
            &allowlist,
            &resolver,
            &transport,
            fixture_url(CONTENT_HOST, path),
            PRODUCTION_FETCH_POLICY,
        )
        .await
        .expect_err("non-exact HTML media type must be rejected");
        assert_eq!(error, OneboxFetchError::InvalidContentType);
    }
}

#[tokio::test]
async fn oversized_content_length_is_rejected_before_reading_the_body() {
    let fixture = HttpsFixture::start(HashMap::from([(
        format!("{CONTENT_HOST}/length"),
        FixtureResponse::html(Vec::new())
            .with_header("Content-Length", &(MAX_BODY_BYTES as u64 + 1).to_string()),
    )]))
    .await;
    let allowlist = StaticAllowlist::new([CONTENT_HOST]);
    let resolver = ScriptedResolver::new([(CONTENT_HOST, vec![vec![public_address(8)]])]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(CONTENT_HOST, "/length"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect_err("oversized declared body must be rejected");

    assert_eq!(error, OneboxFetchError::BodyTooLarge);
}

#[tokio::test]
async fn chunked_stream_cannot_cross_the_production_body_limit() {
    let fixture = HttpsFixture::start(HashMap::from([(
        format!("{CONTENT_HOST}/chunked"),
        FixtureResponse {
            status: "200 OK",
            headers: vec![("Content-Type".into(), "text/html".into())],
            body: FixtureBody::Chunked {
                chunks: vec![vec![b'a'; MAX_BODY_BYTES], vec![b'b']],
                delay_between_chunks: Duration::ZERO,
            },
            header_delay: Duration::ZERO,
        },
    )]))
    .await;
    let allowlist = StaticAllowlist::new([CONTENT_HOST]);
    let resolver = ScriptedResolver::new([(CONTENT_HOST, vec![vec![public_address(8)]])]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(CONTENT_HOST, "/chunked"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect_err("chunked body must be bounded while streaming");

    assert_eq!(error, OneboxFetchError::BodyTooLarge);
}

#[tokio::test]
async fn invalid_utf8_is_lossy_but_non_utf8_charsets_fail_closed() {
    let invalid_utf8 =
        [br#"<meta property="og:title" content="a"#.as_slice(), &[0xff], br#"b">"#.as_slice()]
            .concat();
    let fixture = HttpsFixture::start(HashMap::from([
        (format!("{CONTENT_HOST}/invalid-utf8"), FixtureResponse::html(invalid_utf8)),
        (
            format!("{CONTENT_HOST}/legacy-charset"),
            FixtureResponse {
                status: "200 OK",
                headers: vec![("Content-Type".into(), "text/html; charset=windows-1252".into())],
                body: FixtureBody::Fixed(b"<title>legacy</title>".to_vec()),
                header_delay: Duration::ZERO,
            },
        ),
        (
            format!("{CONTENT_HOST}/empty-charset"),
            FixtureResponse {
                status: "200 OK",
                headers: vec![("Content-Type".into(), "text/html; charset=\"\"".into())],
                body: FixtureBody::Fixed(b"<title>empty</title>".to_vec()),
                header_delay: Duration::ZERO,
            },
        ),
    ]))
    .await;
    let allowlist = StaticAllowlist::new([CONTENT_HOST]);
    let resolver = ScriptedResolver::new([(
        CONTENT_HOST,
        vec![vec![public_address(8)], vec![public_address(8)], vec![public_address(8)]],
    )]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));

    let body = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(CONTENT_HOST, "/invalid-utf8"),
        PRODUCTION_FETCH_POLICY,
    )
    .await
    .expect("declared UTF-8 is decoded without panicking on malformed bytes");
    assert_eq!(
        parse_og_tags(&body, "https://content.test/invalid-utf8").title.as_deref(),
        Some("a�b")
    );

    for path in ["/legacy-charset", "/empty-charset"] {
        let error = fetch_bounded_html_with(
            &allowlist,
            &resolver,
            &transport,
            fixture_url(CONTENT_HOST, path),
            PRODUCTION_FETCH_POLICY,
        )
        .await
        .expect_err("unsupported or empty charset must fail closed");
        assert_eq!(error, OneboxFetchError::InvalidCharset);
    }
}

#[tokio::test]
async fn per_request_timeout_is_distinct_from_the_total_deadline() {
    let fixture = HttpsFixture::start(HashMap::from([(
        format!("{CONTENT_HOST}/request-timeout"),
        FixtureResponse::html("<title>late</title>").with_delay(Duration::from_millis(300)),
    )]))
    .await;
    let allowlist = StaticAllowlist::new([CONTENT_HOST]);
    let resolver = ScriptedResolver::new([(CONTENT_HOST, vec![vec![public_address(8)]])]);
    let transport = fixture.transport(Arc::new(Mutex::new(Vec::new())));
    let policy = FetchPolicy {
        request_timeout: Duration::from_millis(120),
        total_timeout: Duration::from_secs(1),
        ..PRODUCTION_FETCH_POLICY
    };

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(CONTENT_HOST, "/request-timeout"),
        policy,
    )
    .await
    .expect_err("one slow upstream request must hit its own deadline");

    assert_eq!(error, OneboxFetchError::RequestTimeout);
}

#[tokio::test]
async fn redirect_chain_cannot_extend_the_total_deadline() {
    let fixture = HttpsFixture::start(HashMap::from([
        (
            format!("{CONTENT_HOST}/total-start"),
            FixtureResponse::redirect("/total-final").with_delay(Duration::from_millis(300)),
        ),
        (
            format!("{CONTENT_HOST}/total-final"),
            FixtureResponse::html("<title>too late</title>").with_delay(Duration::from_millis(300)),
        ),
    ]))
    .await;
    let allowlist = StaticAllowlist::new([CONTENT_HOST]);
    let resolver = ScriptedResolver::new([(
        CONTENT_HOST,
        vec![vec![public_address(8)], vec![public_address(9)]],
    )]);
    let observed_pins = Arc::new(Mutex::new(Vec::new()));
    let transport = fixture.transport(Arc::clone(&observed_pins));
    let policy = FetchPolicy {
        request_timeout: Duration::from_millis(450),
        total_timeout: Duration::from_millis(500),
        ..PRODUCTION_FETCH_POLICY
    };

    let error = fetch_bounded_html_with(
        &allowlist,
        &resolver,
        &transport,
        fixture_url(CONTENT_HOST, "/total-start"),
        policy,
    )
    .await
    .expect_err("redirects must share one total deadline");

    assert_eq!(error, OneboxFetchError::TotalTimeout);
    assert_eq!(resolver.observed_hosts(), vec![CONTENT_HOST, CONTENT_HOST]);
    assert_eq!(observed_pins.lock().expect("pin recorder lock must remain usable").len(), 2);
}

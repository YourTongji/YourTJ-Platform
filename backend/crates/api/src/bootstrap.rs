//! Application bootstrap: config, tracing, DB connection, router composition.

use std::net::SocketAddr;

use axum::extract::{Request, State};
use axum::http::header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, PRAGMA};
use axum::http::{HeaderName, HeaderValue, Method};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::sse::SsePayload;
use shared::AppState;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
const SINGLE_ACTIVE_WALLET_KEY_MIGRATION: i64 = 67;

fn request_log_path(request: &Request) -> &str {
    request.uri().path()
}

#[derive(Debug, Default, PartialEq, Eq)]
struct StartupOptions {
    enforce_controlled_wallet_migration: bool,
    wallet_key_cutover_drained: bool,
}

/// Full bootstrap: init tracing, load config, connect DB, build and serve.
pub async fn run() -> anyhow::Result<()> {
    init_tracing();
    let startup_options = parse_startup_options(std::env::args_os().skip(1))?;
    let config = shared::Config::from_env()?;

    // Reject default JWT secret in production.
    if config.jwt_secret.is_empty() || config.jwt_secret == "change-me-please" {
        anyhow::bail!(
            "JWT_SECRET is empty or set to a default value. Set a strong random secret before deploying."
        );
    }

    let db = sqlx::PgPool::connect(&config.database_url).await?;
    run_migrations(&db, &startup_options).await?;
    tracing::info!("connected to database");

    // Connect Redis (optional — app degrades gracefully if unavailable).
    let redis_pool = match deadpool_redis::Config::from_url(&config.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
    {
        Ok(pool) => {
            match pool.get().await {
                Ok(mut conn) => {
                    let _: String = redis::cmd("PING").query_async(&mut conn).await?;
                    tracing::info!("connected to Redis");
                }
                Err(_) => {
                    tracing::warn!(
                        "Redis pool created but connection failed — continuing without Redis"
                    );
                }
            };
            Some(pool)
        }
        Err(e) => {
            tracing::warn!("Failed to create Redis pool: {e} — continuing without Redis");
            None
        }
    };

    // Decode system Ed25519 private key and derive public key.
    let (system_private_key, system_public_key_b64) =
        derive_system_key(&config.credit_system_private_key)?;
    if startup_options.wallet_key_cutover_drained {
        let verification = credit::repo::verify_full_ledger(&db, &system_public_key_b64).await?;
        if !verification.ok {
            anyhow::bail!("credit ledger verification failed after wallet key cutover");
        }
        tracing::info!("credit ledger verified before serving wallet-key cutover");
    }

    // SSE broadcast channel (capacity 128, wrapping).
    let (sse_tx, _sse_rx) = broadcast::channel::<SsePayload>(128);
    forum::sse::init_global(sse_tx.clone());

    let email_encryption = shared::email_crypto::EmailEncryption::from_keys(
        config.email_encryption_active_version,
        &config.email_encryption_active_aead_hex,
        &config.email_encryption_active_blind_hex,
        &[], // legacy pairs loaded from env in future rotations
    )?;
    match email_encryption.as_ref() {
        Some(encryption) => {
            identity::backfill_email_encryption(&db, encryption).await?;
            if config.email_encryption_strict && identity::has_unencrypted_email_rows(&db).await? {
                anyhow::bail!("EMAIL_ENCRYPTION_STRICT=true but plaintext email rows remain");
            }
        }
        None if config.email_encryption_strict => {
            anyhow::bail!("EMAIL_ENCRYPTION_STRICT=true but no encryption keys are configured");
        }
        None => {}
    }

    // Captcha verifier (fail closed when not configured).
    let captcha_verifier: Option<std::sync::Arc<dyn shared::captcha::CaptchaVerifier>> =
        if config.captcha_siteverify_url.is_empty() {
            None
        } else {
            Some(std::sync::Arc::new(shared::captcha::YourTongjiCaptcha::new(
                config.captcha_siteverify_url.clone(),
                std::time::Duration::from_secs(5),
            )))
        };

    let state = AppState {
        db,
        config: config.clone(),
        jwt_secret: config.jwt_secret.clone(),
        jwt_ttl: config.jwt_ttl,
        refresh_ttl: config.refresh_ttl,
        meili_url: config.meili_url.clone(),
        meili_master_key: config.meili_master_key.clone(),
        redis: redis_pool,
        system_private_key,
        system_public_key_b64,
        email_encryption,
        captcha_verifier,
        sse_tx: Some(sse_tx),
    };
    media::validate_delivery_runtime(&state.config)?;
    tracing::info!("media Delivery runtime configuration validated");
    // Meilisearch index setup and readiness check on startup.
    {
        let meili_url = state.meili_url.clone();
        let meili_key = state.meili_master_key.clone();
        if !meili_url.is_empty() {
            tokio::spawn(async move {
                if let Err(e) = courses::meili::setup_course_index(&meili_url, &meili_key).await {
                    tracing::warn!(error = %e, "meilisearch course index setup failed");
                } else {
                    tracing::info!("meilisearch course index ready");
                }
                if let Err(e) = courses::meili::setup_selection_index(&meili_url, &meili_key).await
                {
                    tracing::warn!(error = %e, "meilisearch selection index setup failed");
                }
            });
        } else {
            tracing::warn!("MEILI_URL is empty — meilisearch is not configured; search will return empty results");
        }
    }

    // --- Forum background tasks ---

    // 1. Hot rank refresh (every 5 minutes).
    if let Some(ref redis_pool) = state.redis {
        let redis = redis_pool.clone();
        let db = state.db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                tracing::debug!("running hot rank refresh");
                if let Err(e) = forum::repo::refresh_hot_rank(&redis, &db).await {
                    tracing::error!(error = %e, "hot rank refresh failed");
                }
            }
        });
        tracing::info!("forum hot rank refresh scheduled (every 5 min)");
    }

    // 2. Trust level promotion (once per Shanghai day, lease-fenced in PostgreSQL).
    let db = state.db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            tracing::debug!("checking scheduled trust level promotion");
            let (promoted, demoted) = forum::trust_levels::run_daily_tl_promotion(&db).await;
            if promoted > 0 || demoted > 0 {
                tracing::info!(promoted, demoted, "trust level changes applied");
            }
        }
    });
    tracing::info!("forum trust level promotion scheduled (hourly lease check)");

    // 3. Watched words initialization (on startup, once).
    forum::watched_words::init_watched_words(&state.db).await;
    tracing::info!("forum watched words loaded");

    // 4. Seed standard badges.
    platform::achievements::seed_achievements(&state.db).await?;
    tracing::info!("platform achievements seeded");
    crate::notification_worker::start(&state);
    {
        let worker_state = state.clone();
        tokio::spawn(identity::email_delivery::run_email_delivery_worker(worker_state));
        tracing::info!("identity email delivery worker scheduled");
    }

    // 5. Auto-archive stale threads (daily).
    {
        let db = state.db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(86400)).await;
                tracing::info!("running daily auto-archive of stale threads");
                forum::repo::auto_archive_stale(&db).await;
            }
        });
        tracing::info!("forum auto-archive scheduled (every 24h)");
    }

    // 6. Weekly email digest (every 7 days).
    {
        let db = state.db.clone();
        let config = state.config.clone();
        tokio::spawn(async move {
            // Start the first digest after a short initial delay (1 hour),
            // then every 7 days thereafter. The exact alignment to Sunday
            // 00:00 UTC is a nice-to-have that can be tuned later.
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            loop {
                tracing::info!("running weekly email digest");
                forum::digest::run_digest(&db, &config).await;
                tokio::time::sleep(std::time::Duration::from_secs(7 * 86400)).await;
            }
        });
        tracing::info!("forum email digest scheduled (every 7 days)");
    }

    // 7. Promotion event receipt retention.
    {
        let db = state.db.clone();
        tokio::spawn(async move {
            loop {
                match platform::purge_expired_promotion_event_receipts(&db).await {
                    Ok(removed) if removed > 0 => {
                        tracing::info!(removed, "expired promotion event receipts removed");
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(?error, "promotion event receipt retention failed");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
        tracing::info!("promotion event receipt retention scheduled (hourly)");
    }

    // Durable media object deletion never holds a database lock across provider I/O.
    {
        let worker_state = state.clone();
        tokio::spawn(media::run_deletion_worker(worker_state));
        tracing::info!("media variant processing and object deletion worker scheduled");
    }
    if state.config.media_retention_gc_enabled {
        let worker_state = state.clone();
        tokio::spawn(media::run_retention_gc_worker(worker_state));
        tracing::info!("media retention GC worker scheduled");
    } else {
        tracing::info!("media retention GC worker disabled pending rollout reconciliation");
    }
    {
        let worker_state = state.clone();
        tokio::spawn(media::run_retention_housekeeping_worker(worker_state));
        tracing::info!("media retention metadata housekeeping scheduled");
    }

    // 8. Badge credit mint bridge (every 60 seconds).
    {
        let db = state.db.clone();
        let system_seed = state.system_private_key.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let pending: Vec<(i64, i64, i64, String)> = match sqlx::query_as(
                    "SELECT id, account_id, amount, idempotency_key \
                     FROM platform.pending_mints WHERE minted_at IS NULL \
                     ORDER BY id LIMIT 50",
                )
                .fetch_all(&db)
                .await
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        tracing::warn!(error = %e, "badge mint bridge: query failed");
                        continue;
                    }
                };

                for (id, account_id, amount, idempotency_key) in pending {
                    match credit::mint_for_contribution(
                        &db,
                        account_id,
                        amount,
                        &idempotency_key,
                        "badge award",
                        &system_seed,
                    )
                    .await
                    {
                        Ok(_) => {
                            let _ = sqlx::query(
                                "UPDATE platform.pending_mints SET minted_at = now() WHERE id = $1",
                            )
                            .bind(id)
                            .execute(&db)
                            .await;
                            tracing::info!(
                                id,
                                account_id,
                                amount,
                                idempotency_key,
                                "badge mint completed"
                            );
                        }
                        Err(e) => tracing::warn!(
                            error = %e, id, account_id, idempotency_key,
                            "badge mint failed (will retry)"
                        ),
                    }
                }
            }
        });
        tracing::info!("badge credit mint bridge scheduled (every 60s)");
    }

    crate::account_data::spawn_workers(state.clone());
    tracing::info!("account lifecycle and data-export workers scheduled");

    let app = build_router(state)?;
    let addr = SocketAddr::new(config.bind_address, config.port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "yourtj-platform api listening");

    axum::serve(listener, app).await?;
    Ok(())
}

/// Compose the full application router from per-domain routers.
fn build_router(state: AppState) -> anyhow::Result<Router> {
    let tip_target_resolver = std::sync::Arc::new(crate::tip_targets::ContentTipTargetResolver);
    let allowed_origins = state
        .config
        .cors_allowed_origins
        .iter()
        .map(|origin| HeaderValue::from_str(origin))
        .collect::<Result<Vec<_>, _>>()?;
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            ACCEPT,
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("idempotency-key"),
            HeaderName::from_static("x-export-token"),
            HeaderName::from_static("x-media-preview-token"),
            HeaderName::from_static("x-recovery-token"),
            HeaderName::from_static("x-wallet-intent"),
            HeaderName::from_static("x-wallet-sig"),
            HeaderName::from_static("x-request-id"),
        ])
        .allow_credentials(true);

    let request_id_layer = SetRequestIdLayer::x_request_id(MakeRequestUuid);

    // Security headers: prevent clickjacking, MIME sniffing, and referrer leakage.

    // Limit request body to 256 KB.
    let body_limit = RequestBodyLimitLayer::new(256_000);
    let readiness_routes = Router::new()
        .route("/ready", get(ready))
        .route("/api/v2/ready", get(ready))
        .with_state(state.clone());

    Ok(Router::new()
        .route("/health", get(health))
        .route("/api/v2/health", get(health))
        .merge(readiness_routes)
        .merge(platform::routes(state.clone()))
        .merge(crate::admin::routes(state.clone()))
        .merge(crate::appeals::routes(state.clone()))
        .merge(crate::account_data::routes(state.clone()))
        .merge(identity::routes(state.clone(), std::sync::Arc::new(reviews::LegacyReviewClaimer)))
        .merge(activity::routes(state.clone()))
        .merge(search::routes(state.clone()))
        .merge(courses::routes(state.clone()))
        .merge(reviews::routes(state.clone()))
        .merge(credit::routes(state.clone(), tip_target_resolver))
        .merge(forum::routes(state.clone()))
        .merge(media::routes(state.clone()))
        .merge(crate::onebox::routes(state.clone()))
        .layer(cors)
        .layer(request_id_layer)
        .layer(TraceLayer::new_for_http().make_span_with(|request: &Request| {
            tracing::debug_span!(
                "request",
                method = %request.method(),
                path = request_log_path(request),
            )
        }))
        .layer(
            // Security headers: prevent clickjacking, MIME sniffing, and referrer leakage.
            tower::ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_FRAME_OPTIONS,
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                .layer(SetResponseHeaderLayer::if_not_present(
                    CACHE_CONTROL,
                    HeaderValue::from_static("private, no-store"),
                ))
                .layer(SetResponseHeaderLayer::if_not_present(
                    PRAGMA,
                    HeaderValue::from_static("no-cache"),
                )),
        )
        .layer(body_limit))
}

/// Liveness probe used by SAE / load balancers.
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "yourtj-platform", "version": "2.0.0" }))
}

/// Readiness requires the authoritative database and the latest migration.
async fn ready(State(state): State<AppState>) -> shared::AppResult<Json<Value>> {
    let expected_version = MIGRATOR.iter().map(|migration| migration.version).max().unwrap_or(0);
    let applied_version: Option<i64> =
        match sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success = TRUE")
            .fetch_one(&state.db)
            .await
        {
            Ok(version) => version,
            Err(error) => {
                tracing::warn!(?error, "readiness database check failed");
                return Err(shared::AppError::ServiceUnavailable);
            }
        };
    if applied_version.unwrap_or(0) < expected_version {
        tracing::warn!(applied_version, expected_version, "readiness migration check failed");
        return Err(shared::AppError::ServiceUnavailable);
    }
    Ok(Json(json!({
        "status": "ok",
        "service": "yourtj-platform",
        "version": "2.0.0",
    })))
}

fn parse_startup_options(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> anyhow::Result<StartupOptions> {
    let mut options = StartupOptions::default();
    for argument in arguments {
        match argument.to_str() {
            Some("--enforce-controlled-wallet-migration") => {
                options.enforce_controlled_wallet_migration = true;
            }
            Some("--wallet-key-cutover-drained") => {
                options.wallet_key_cutover_drained = true;
            }
            Some(_) | None => anyhow::bail!("unknown api startup argument"),
        }
    }
    if options.wallet_key_cutover_drained && !options.enforce_controlled_wallet_migration {
        anyhow::bail!("wallet key cutover drain requires controlled migration enforcement");
    }
    Ok(options)
}

async fn run_migrations(
    pool: &sqlx::PgPool,
    startup_options: &StartupOptions,
) -> anyhow::Result<()> {
    baseline_legacy_database(pool).await?;
    let wallet_migration_applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations \
         WHERE version = $1 AND success = TRUE)",
    )
    .bind(SINGLE_ACTIVE_WALLET_KEY_MIGRATION)
    .fetch_one(pool)
    .await?;
    if startup_options.enforce_controlled_wallet_migration
        && !wallet_migration_applied
        && !startup_options.wallet_key_cutover_drained
    {
        anyhow::bail!("wallet key migration requires a stopped-writer drain before migration 0067");
    }
    MIGRATOR.run(pool).await?;
    Ok(())
}

async fn baseline_legacy_database(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _sqlx_migrations ( \
            version BIGINT PRIMARY KEY, \
            description TEXT NOT NULL, \
            installed_on TIMESTAMPTZ NOT NULL DEFAULT now(), \
            success BOOLEAN NOT NULL, \
            checksum BYTEA NOT NULL, \
            execution_time BIGINT NOT NULL \
        )",
    )
    .execute(pool)
    .await?;

    for migration in MIGRATOR.iter() {
        if legacy_marker_exists(pool, migration.version).await? {
            sqlx::query(
                "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
                 VALUES ($1, $2, TRUE, $3, 0) \
                 ON CONFLICT (version) DO NOTHING",
            )
            .bind(migration.version)
            .bind(migration.description.as_ref())
            .bind(migration.checksum.as_ref())
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn legacy_marker_exists(pool: &sqlx::PgPool, version: i64) -> anyhow::Result<bool> {
    let Some(query) = legacy_marker_query(version) else {
        return Ok(false);
    };

    let exists = sqlx::query_scalar(query).fetch_one(pool).await?;
    Ok(exists)
}

fn legacy_marker_query(version: i64) -> Option<&'static str> {
    match version {
        1 => Some("SELECT to_regclass('identity.accounts') IS NOT NULL"),
        2 => Some("SELECT to_regtype('credit.task_status') IS NOT NULL"),
        3 => Some("SELECT to_regclass('platform.announcements') IS NOT NULL"),
        4 => Some("SELECT to_regclass('forum.votes') IS NOT NULL"),
        5 => Some("SELECT to_regclass('forum.tags') IS NOT NULL"),
        6 => Some("SELECT to_regclass('platform.badges') IS NOT NULL"),
        7 => Some("SELECT to_regclass('forum.threads') IS NOT NULL"),
        8 => Some("SELECT to_regclass('platform.pending_mints') IS NOT NULL"),
        9 => Some("SELECT to_regclass('selection.pk_calendars') IS NOT NULL"),
        10 => Some("SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'reviews' AND table_name = 'reviews' AND column_name = 'reviewer_name')"),
        11 => Some("SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'identity' AND table_name = 'accounts' AND column_name = 'password_hash')"),
        12 => Some("SELECT to_regclass('selection.campuses_id_seq') IS NOT NULL"),
        13 => Some("SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'selection' AND table_name = 'courses' AND column_name = 'teacher_names')"),
        _ => None,
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(true).init();
}

/// Decode the hex-encoded system Ed25519 private key seed and derive the
/// corresponding public key. Returns `(private_key_bytes, public_key_b64)`.
fn derive_system_key(hex_key: &str) -> anyhow::Result<(Vec<u8>, String)> {
    if hex_key.is_empty() {
        anyhow::bail!("CREDIT_SYSTEM_PRIVATE_KEY is not set");
    }
    let seed = hex::decode(hex_key)
        .map_err(|e| anyhow::anyhow!("CREDIT_SYSTEM_PRIVATE_KEY is not valid hex: {e}"))?;
    if seed.len() != 32 {
        anyhow::bail!(
            "CREDIT_SYSTEM_PRIVATE_KEY must be 32 bytes (64 hex chars), got {} bytes",
            seed.len()
        );
    }
    use ring::signature::KeyPair;
    let key_pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&seed)
        .map_err(|e| anyhow::anyhow!("invalid Ed25519 seed: {e}"))?;
    use base64::Engine as _;
    let pk_b64 = base64::engine::general_purpose::STANDARD.encode(key_pair.public_key().as_ref());
    Ok((seed, pk_b64))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::header::{
        ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_METHOD, CACHE_CONTROL, ORIGIN,
    };
    use axum::http::{HeaderValue, Request, StatusCode};
    use shared::AppState;
    use tower::ServiceExt as _;

    use super::{
        build_router, legacy_marker_query, parse_startup_options, request_log_path, StartupOptions,
    };

    fn test_state() -> AppState {
        AppState {
            db: sqlx::PgPool::connect_lazy("postgres://user:password@localhost/test")
                .expect("valid lazy postgres URL"),
            config: shared::Config::from_env().expect("test Config::from_env"),
            jwt_secret: "integration-test-secret-32bytes!".into(),
            jwt_ttl: 900,
            refresh_ttl: 604800,
            meili_url: String::new(),
            meili_master_key: String::new(),
            redis: None,
            system_private_key: vec![0u8; 32],
            system_public_key_b64: String::new(),
            email_encryption: None,
            captcha_verifier: None,
            sse_tx: None,
        }
    }

    #[test]
    fn request_trace_path_excludes_query_string() {
        let request = Request::builder()
            .uri("/api/v2/selection/courses/search?calendarId=1&q=private-input")
            .body(Body::empty())
            .expect("request builds");

        assert_eq!(request_log_path(&request), "/api/v2/selection/courses/search");
    }

    #[tokio::test]
    async fn health_is_available_under_versioned_api_path() {
        let response = build_router(test_state())
            .expect("router configuration is valid")
            .oneshot(
                Request::builder()
                    .uri("/api/v2/health")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("request succeeds");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("private, no-store"))
        );
    }

    #[tokio::test]
    async fn cors_echoes_only_configured_exact_origins() {
        let allowed = build_router(test_state())
            .expect("router configuration is valid")
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/v2/health")
                    .header(ORIGIN, "https://pf-dev.yourtj.de")
                    .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .body(Body::empty())
                    .expect("allowed preflight builds"),
            )
            .await
            .expect("allowed preflight succeeds");
        assert_eq!(
            allowed.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("https://pf-dev.yourtj.de"))
        );

        let denied = build_router(test_state())
            .expect("router configuration is valid")
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/v2/health")
                    .header(ORIGIN, "https://evil.example")
                    .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .body(Body::empty())
                    .expect("denied preflight builds"),
            )
            .await
            .expect("denied preflight completes");
        assert!(denied.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
    }

    #[test]
    fn legacy_marker_query_only_baselines_pre_migrator_schema() {
        assert_eq!(
            legacy_marker_query(1),
            Some("SELECT to_regclass('identity.accounts') IS NOT NULL")
        );
        assert_eq!(legacy_marker_query(4), Some("SELECT to_regclass('forum.votes') IS NOT NULL"));
        assert_eq!(
            legacy_marker_query(9),
            Some("SELECT to_regclass('selection.pk_calendars') IS NOT NULL")
        );
        assert_eq!(legacy_marker_query(14), None);
    }

    #[test]
    fn wallet_cutover_drain_requires_controlled_migration_enforcement() {
        let options = parse_startup_options([
            "--enforce-controlled-wallet-migration".into(),
            "--wallet-key-cutover-drained".into(),
        ])
        .expect("controlled cutover arguments");
        assert_eq!(
            options,
            StartupOptions {
                enforce_controlled_wallet_migration: true,
                wallet_key_cutover_drained: true,
            }
        );

        assert!(
            parse_startup_options(["--wallet-key-cutover-drained".into()]).is_err(),
            "a caller cannot assert drain without enabling the migration guard"
        );
        assert!(parse_startup_options(["--unknown".into()]).is_err());
    }
}

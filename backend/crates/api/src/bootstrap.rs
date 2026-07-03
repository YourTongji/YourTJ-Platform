//! Application bootstrap: config, tracing, DB connection, router composition.

use std::net::SocketAddr;

use axum::http::HeaderValue;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::AppState;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

/// Full bootstrap: init tracing, load config, connect DB, build and serve.
pub async fn run() -> anyhow::Result<()> {
    init_tracing();
    let config = shared::Config::from_env()?;

    // Reject default JWT secret in production.
    if config.jwt_secret.is_empty() || config.jwt_secret == "change-me-please" {
        anyhow::bail!(
            "JWT_SECRET is empty or set to a default value. Set a strong random secret before deploying."
        );
    }

    let db = sqlx::PgPool::connect(&config.database_url).await?;
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
    };

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

    // 2. Trust level promotion (every 24 hours).
    let db = state.db.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(86400)).await;
            tracing::info!("running daily trust level promotion");
            let (promoted, demoted) = forum::trust_levels::run_daily_tl_promotion(&db).await;
            if promoted > 0 || demoted > 0 {
                tracing::info!(promoted, demoted, "trust level changes applied");
            }
        }
    });
    tracing::info!("forum trust level promotion scheduled (every 24h)");

    // 3. Watched words initialization (on startup, once).
    forum::watched_words::init_watched_words(&state.db).await;
    tracing::info!("forum watched words loaded");

    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "yourtj-platform api listening");

    axum::serve(listener, app).await?;
    Ok(())
}

/// Compose the full application router from per-domain routers.
fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let request_id_layer = SetRequestIdLayer::x_request_id(MakeRequestUuid);

    // Security headers: prevent clickjacking, MIME sniffing, and referrer leakage.

    // Limit request body to 256 KB.
    let body_limit = RequestBodyLimitLayer::new(256_000);

    Router::new()
        .route("/health", get(health))
        .merge(crate::platform::routes(state.clone()))
        .merge(crate::admin::routes(state.clone()))
        .merge(identity::routes(state.clone()))
        .merge(courses::routes(state.clone()))
        .merge(reviews::routes(state.clone()))
        .merge(credit::routes(state.clone()))
        .merge(forum::routes(state.clone()))
        .layer(cors)
        .layer(request_id_layer)
        .layer(TraceLayer::new_for_http())
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
                )),
        )
        .layer(body_limit)
}

/// Liveness probe used by SAE / load balancers.
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "yourtj-platform", "version": "2.0.0" }))
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

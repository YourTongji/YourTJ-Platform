//! `api` is the single Axum gateway binary. It owns process startup (config,
//! tracing, server) and composes one router per domain crate. Business logic
//! lives in the domain crates, never here.

mod admin;
mod platform;

use std::net::SocketAddr;

use axum::Router;
use shared::AppState;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = shared::Config::from_env()?;

    let db = sqlx::PgPool::connect(&config.database_url).await?;
    tracing::info!("connected to database");

    let state = AppState {
        db,
        jwt_secret: config.jwt_secret.clone(),
        jwt_ttl: config.jwt_ttl,
        refresh_ttl: config.refresh_ttl,
        meili_url: config.meili_url.clone(),
        meili_master_key: config.meili_master_key.clone(),
    };

    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "yourtj-platform api listening");

    axum::serve(listener, app).await?;
    Ok(())
}

fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let request_id_layer = SetRequestIdLayer::x_request_id(MakeRequestUuid);

    platform::routes(state.clone())
        .merge(admin::routes(state.clone()))
        .merge(identity::routes(state.clone()))
        .merge(courses::routes(state.clone()))
        .merge(reviews::routes(state.clone()))
        .merge(credit::routes(state.clone()))
        .merge(forum::routes(state.clone()))
        .layer(cors)
        .layer(request_id_layer)
        .layer(TraceLayer::new_for_http())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(true).init();
}

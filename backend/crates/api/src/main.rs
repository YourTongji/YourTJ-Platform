//! `api` is the single Axum gateway binary. It owns process startup (config,
//! tracing, server) and composes one router per domain crate. Business logic
//! lives in the domain crates, never here.

use std::net::SocketAddr;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::Config;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::from_env()?;

    let app = build_router();
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "yourtj-platform api listening");

    axum::serve(listener, app).await?;
    Ok(())
}

/// Compose the full application router from per-domain routers.
fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(identity::routes())
        .merge(courses::routes())
        .merge(reviews::routes())
        .merge(credit::routes())
        .merge(forum::routes())
        .layer(TraceLayer::new_for_http())
}

/// Liveness probe used by SAE / load balancers.
async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "yourtj-platform" }))
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(true).init();
}

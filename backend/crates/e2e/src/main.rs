//! End-to-end test driver for YourTJ Platform.
//!
//! Spins against a running API server (postgres + redis + meilisearch + api)
//! and executes journey-based tests that verify real HTTP behavior against
//! database invariants.
//!
//! ## Usage
//!
//! ```bash
//! # Requires docker-compose stack to be running:
//! #   docker compose --profile e2e up -d
//!
//! export E2E_API_BASE=http://localhost:8080
//! export E2E_DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj
//! cargo run -p e2e
//! ```
//!
//! ## Architecture
//!
//! Each test suite (S1-S8) is a separate module. Tests within a suite share
//! an isolated namespace (email domain per run) to allow parallel execution.
//!
//! The driver uses `reqwest` to call the API and `sqlx` to verify database
//! invariants directly.

use std::env;

mod s2_identity;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "e2e=debug,reqwest=warn".into()))
        .init();

    let base_url = env::var("E2E_API_BASE").unwrap_or_else(|_| "http://localhost:8080".into());
    let db_url = env::var("E2E_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://yourtj:yourtj@localhost:5432/yourtj".into());

    tracing::info!(base_url, "E2E test driver starting");

    // S2: Identity journey
    s2_identity::run(&base_url, &db_url).await?;

    tracing::info!("All E2E tests passed!");
    Ok(())
}

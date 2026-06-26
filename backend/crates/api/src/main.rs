//! `api` is the single Axum gateway binary. Process startup is delegated to
//! [`bootstrap::run`]; business logic lives in the domain crates, never here.

mod admin;
mod bootstrap;
mod platform;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::run().await
}

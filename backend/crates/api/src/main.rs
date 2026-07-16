//! `api` is the single Axum gateway binary. Process startup is delegated to
//! [`bootstrap::run`]; business logic lives in the domain crates, never here.

mod account_data;
mod account_eligibility;
mod admin;
mod appeals;
mod bootstrap;
mod notification_worker;
mod onebox;
mod tip_targets;
mod wallet_keys;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::run().await
}

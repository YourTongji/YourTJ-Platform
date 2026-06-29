//! Credit domain — Web2.5 closed-loop points.
//!
//! HARD COMPLIANCE RULES (闭环虚拟权益, see `docs/REWRITE_V2_DESIGN.md` §6.3):
//! - NO recharge, NO withdrawal, NO fiat conversion, NO cashout.
//! - NO unrestricted peer transfer — value only moves inside controlled flows
//!   (tip / bounty / escrow). Do NOT add a free `transfer` endpoint.
//! - Points are earned by contribution only (system-signed `mint`).
//!
//! Ledger model: `credit.ledger` is append-only, monotonic `seq`, `prev_hash`
//! chained, and every entry carries an Ed25519 signature (system key for mint,
//! the account key for tip/bounty/escrow). Balance is a derived cache, never the
//! source of truth. Appends are serialized (advisory lock) to keep the chain linear.

pub mod auth;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod ledger;
pub mod models;
pub mod repo;

use axum::routing::{get, post};
use axum::Router;
use shared::AppState;

/// All routes owned by the credit domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // Wallet
        .route("/api/v2/wallet", get(handlers::get_wallet))
        // Canonical: POST /api/v2/credit/tip — alias: POST /api/v2/wallet/tip
        .route("/api/v2/credit/tip", post(handlers::tip))
        .route("/api/v2/wallet/tip", post(handlers::tip))
        .route("/api/v2/wallet/ledger", get(handlers::get_ledger))
        .route("/api/v2/wallet/ledger/verify", get(handlers::verify_ledger))
        // Tasks
        .route("/api/v2/credit/tasks", get(handlers::list_tasks).post(handlers::create_task))
        .route("/api/v2/credit/tasks/{id}/accept", post(handlers::accept_task))
        .route("/api/v2/credit/tasks/{id}/action", post(handlers::action_task))
        // Products
        .route(
            "/api/v2/credit/products",
            get(handlers::list_products).post(handlers::create_product),
        )
        .route("/api/v2/credit/products/{id}/purchase", post(handlers::purchase_product))
        // Purchases
        .route("/api/v2/credit/purchases", get(handlers::list_purchases))
        .route("/api/v2/credit/purchases/{id}/action", post(handlers::action_purchase))
        .with_state(state)
}

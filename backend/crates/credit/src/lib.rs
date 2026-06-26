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

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::AppState;

/// All routes owned by the credit domain.
pub fn routes(_state: AppState) -> Router {
    Router::new()
        .route("/api/v2/wallet", get(wallet))
        .route("/api/v2/wallet/ledger/verify", get(verify_ledger))
        .route("/api/v2/credit/tasks", get(tasks))
        .route("/api/v2/credit/products", get(products))
}

async fn wallet() -> Json<Value> {
    Json(json!({ "todo": "credit.wallet" }))
}

async fn tasks() -> Json<Value> {
    // TODO(P2): escrow task square; money via ledger escrow_hold/release only.
    Json(json!({ "todo": "credit.tasks" }))
}

async fn products() -> Json<Value> {
    // TODO(P2): product listings; purchase via ledger escrow.
    Json(json!({ "todo": "credit.products" }))
}

async fn verify_ledger() -> Json<Value> {
    // TODO(P2): recompute the hash chain + verify each signature; return latest seq/hash.
    Json(json!({ "todo": "credit.ledger.verify" }))
}

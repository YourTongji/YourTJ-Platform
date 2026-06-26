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

pub mod dto;
pub mod error;
pub mod ledger;
pub mod models;
pub mod repo;

use axum::Router;
use shared::AppState;

/// All routes owned by the credit domain.
pub fn routes(_state: AppState) -> Router {
    Router::new()
}

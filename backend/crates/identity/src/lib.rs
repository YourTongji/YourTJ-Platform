//! Identity domain: campus-email accounts, verification codes, JWT sessions, and
//! the account-bound Ed25519 keys used to sign credit operations.
//!
//! Invariants (see `docs/REWRITE_V2_DESIGN.md`):
//! - The public handle is shown to users; the real email is server-only (moderation).
//! - The server stores only Ed25519 *public* keys — never private keys or secrets.
//! - Old wallets are merged via a signed challenge (`/wallet/claim`), not by import.

mod dto;
mod error;
mod models;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::AppState;

/// All routes owned by the identity domain.
pub fn routes(_state: AppState) -> Router {
    Router::new().route("/api/v2/me", get(me))
}

async fn me() -> Json<Value> {
    // TODO(P1): resolve the authenticated account from the bearer session.
    Json(json!({ "todo": "identity.me" }))
}

//! Identity domain: campus-email accounts, verification codes, JWT sessions, and
//! the account-bound Ed25519 keys used to sign credit operations.
//!
//! Invariants (see `docs/REWRITE_V2_DESIGN.md`):
//! - The public handle is shown to users; the real email is server-only (moderation).
//! - The server stores only Ed25519 *public* keys — never private keys or secrets.
//! - Old wallets are merged via a signed challenge (`/wallet/claim`), not by import.

pub mod auth;
pub mod auth_middleware;
mod email_code;
mod handlers;
mod ledger;
mod password;
mod repo;
pub mod sanctions;

mod dto;
mod error;
mod models;

use axum::routing::{get, post};
use axum::Router;
use shared::AppState;

/// All routes owned by the identity domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // Auth
        .route("/api/v2/auth/email/request-code", post(handlers::request_code))
        .route("/api/v2/auth/email/verify", post(handlers::verify_email))
        .route("/api/v2/auth/refresh", post(handlers::refresh))
        .route("/api/v2/auth/logout", post(handlers::logout))
        .route("/api/v2/auth/password/login", post(handlers::password_login))
        .route("/api/v2/auth/password/forgot", post(handlers::password_forgot))
        .route("/api/v2/auth/password/reset", post(handlers::password_reset))
        .route("/api/v2/auth/password/change", post(handlers::password_change))
        // Profile
        .route("/api/v2/me", get(handlers::get_me).patch(handlers::update_me))
        .route("/api/v2/users/{handle}", get(handlers::get_user_profile))
        .route("/api/v2/users/{handle}/threads", get(handlers::list_user_threads))
        .route("/api/v2/users/{handle}/comments", get(handlers::list_user_comments))
        // Wallet
        .route("/api/v2/wallet/bind", post(handlers::bind_key))
        .route("/api/v2/wallet/claim-challenge", get(handlers::claim_challenge))
        .route("/api/v2/wallet/claim", post(handlers::claim_wallet))
        // Admin
        .route("/api/v2/admin/users/{id}/silence", post(handlers::silence_user))
        .route("/api/v2/admin/users/{id}/suspend", post(handlers::suspend_user))
        .route("/api/v2/admin/users/{id}/unsanction", post(handlers::unsanction_user))
        .route("/api/v2/admin/users/{id}/sanctions", get(handlers::list_user_sanctions))
        .with_state(state)
}

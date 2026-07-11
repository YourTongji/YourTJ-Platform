//! Identity domain: campus-email accounts, verification codes, JWT sessions, and
//! the account-bound Ed25519 keys used to sign credit operations.
//!
//! Invariants (see `docs/product/identity-and-access.md` and `AGENTS.md`):
//! - The public handle is shown to users; the real email is server-only (moderation).
//! - The server stores only Ed25519 *public* keys — never private keys or secrets.
//! - Old wallets are merged via a signed challenge (`/wallet/claim`), not by import.

use shared::AppState;
pub mod auth;
pub mod auth_middleware;
mod email_code;
mod email_templates;
mod handlers;
mod ledger;
mod password;
pub mod profiles;
pub mod public_accounts;
mod repo;
pub mod sanctions;

mod dto;
mod error;
mod models;

use axum::routing::{delete, get, patch, post};
use axum::Router;

/// All routes owned by the identity domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // Auth
        .route("/api/v2/auth/email/request-code", post(handlers::request_code))
        .route("/api/v2/auth/email/verify", post(handlers::verify_email))
        .route("/api/v2/auth/refresh", post(handlers::refresh))
        .route("/api/v2/auth/logout", post(handlers::logout))
        .route("/api/v2/auth/logout-all", post(handlers::logout_all))
        .route("/api/v2/auth/password/login", post(handlers::password_login))
        .route("/api/v2/auth/password/forgot", post(handlers::password_forgot))
        .route("/api/v2/auth/password/reset", post(handlers::password_reset))
        .route("/api/v2/auth/password/change", post(handlers::password_change))
        // Profile
        .route("/api/v2/me", get(handlers::get_me).patch(handlers::update_me))
        .route(
            "/api/v2/me/profile",
            get(handlers::get_my_profile).put(handlers::replace_my_profile),
        )
        .route(
            "/api/v2/me/privacy",
            get(handlers::get_my_privacy).put(handlers::replace_my_privacy),
        )
        .route("/api/v2/me/sessions", get(handlers::list_sessions))
        .route("/api/v2/me/sessions/revoke-others", post(handlers::revoke_other_sessions))
        .route("/api/v2/me/sessions/{id}", delete(handlers::revoke_named_session))
        // Wallet
        .route("/api/v2/wallet/bind", post(handlers::bind_key))
        .route("/api/v2/wallet/claim-challenge", get(handlers::claim_challenge))
        .route("/api/v2/wallet/claim", post(handlers::claim_wallet))
        // Admin
        .route(
            "/api/v2/admin/users",
            get(handlers::admin::list_users).post(handlers::admin::invite_user),
        )
        .route("/api/v2/admin/users/{id}/role", patch(handlers::admin::change_role))
        .route("/api/v2/admin/users/{id}/sessions/revoke", post(handlers::admin::revoke_sessions))
        .route("/api/v2/admin/users/{id}/silence", post(handlers::admin::silence_user))
        .route("/api/v2/admin/users/{id}/suspend", post(handlers::admin::suspend_user))
        .route("/api/v2/admin/users/{id}/unsanction", post(handlers::admin::unsanction_user))
        .route("/api/v2/admin/users/{id}/sanctions", get(handlers::admin::list_user_sanctions))
        .with_state(state)
}

/// Encrypt legacy plaintext identity emails before the application accepts traffic.
pub async fn backfill_email_encryption(
    pool: &sqlx::PgPool,
    encryption: &shared::email_crypto::EmailEncryption,
) -> shared::AppResult<()> {
    repo::backfill_email_encryption(pool, encryption).await
}

/// Report whether any identity email is still stored outside the encrypted path.
pub async fn has_unencrypted_email_rows(pool: &sqlx::PgPool) -> shared::AppResult<bool> {
    repo::has_unencrypted_email_rows(pool).await
}

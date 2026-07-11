//! Platform-owned announcements, promotions, staff-issued verifications, and runtime settings.
//!
//! Announcements and promotions remain first-party platform content. Privileged writes require a
//! named capability, an audit reason, optimistic concurrency, and an audit event in the same
//! transaction. Public reads apply lifecycle and audience policy at the database boundary.

mod announcements;
mod auth;
mod promotions;
mod settings;
mod validation;
pub mod verifications;

use axum::Router;
use shared::AppState;

/// Compose every route owned by the platform domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(announcements::routes())
        .merge(promotions::routes())
        .merge(settings::routes())
        .merge(verifications::routes())
        .with_state(state)
}

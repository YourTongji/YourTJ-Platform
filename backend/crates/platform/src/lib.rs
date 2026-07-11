//! Platform-owned announcements, promotions, staff-issued verifications, and runtime settings.
//!
//! Announcements and promotions remain first-party platform content. Privileged writes require a
//! named capability, an audit reason, optimistic concurrency, and an audit event in the same
//! transaction. Public reads apply lifecycle and audience policy at the database boundary.

pub mod achievements;
mod announcements;
mod auth;
mod promotions;
mod settings;
mod validation;
pub mod verifications;

pub use promotions::purge_expired_promotion_event_receipts;

use axum::Router;
use shared::AppState;

/// Compose every route owned by the platform domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .merge(achievements::routes())
        .merge(announcements::routes())
        .merge(promotions::routes())
        .merge(settings::routes())
        .merge(verifications::routes())
        .with_state(state)
}

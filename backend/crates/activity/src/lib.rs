//! Daily public activity heatmaps, versioned scoring policy, and unified trust levels.
//!
//! Counts are projected at write time from idempotent activation/reversal
//! events. Policy changes reinterpret those counts and never mint credit.
//! Trust levels 1–6 are derived from lifetime effective activity totals.

pub mod data_export;
mod dto;
mod handlers;
mod models;
mod repo;

pub mod contributions;
pub mod trust;

use axum::routing::{get, patch};
use axum::Router;
use shared::AppState;

/// Routes owned by the activity domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/me/activity", get(handlers::get_my_activity))
        .route("/api/v2/me/trust-progress", get(handlers::get_my_trust_progress))
        .route(
            "/api/v2/admin/activity-policy",
            get(handlers::get_activity_policy).put(handlers::update_activity_policy),
        )
        .route("/api/v2/admin/activity-policy/history", get(handlers::get_activity_policy_history))
        .route(
            "/api/v2/admin/trust-policy",
            get(handlers::get_trust_policy).put(handlers::update_trust_policy),
        )
        .route("/api/v2/admin/trust-policy/history", get(handlers::get_trust_policy_history))
        .route(
            "/api/v2/admin/users/{id}/trust-level",
            patch(handlers::adjust_user_trust_level),
        )
        .route(
            "/api/v2/admin/users/{id}/trust-events",
            get(handlers::get_user_trust_events),
        )
        .with_state(state)
}

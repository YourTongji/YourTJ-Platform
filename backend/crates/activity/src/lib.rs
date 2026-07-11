//! Daily public activity heatmaps and versioned scoring policy.
//!
//! Counts are projected at write time from idempotent activation/reversal
//! events. Policy changes reinterpret those counts and never mint credit.

mod dto;
mod handlers;
mod models;
mod repo;

pub mod contributions;

use axum::routing::get;
use axum::Router;
use shared::AppState;

/// Routes owned by the activity domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/me/activity", get(handlers::get_my_activity))
        .route(
            "/api/v2/admin/activity-policy",
            get(handlers::get_activity_policy).put(handlers::update_activity_policy),
        )
        .route("/api/v2/admin/activity-policy/history", get(handlers::get_activity_policy_history))
        .with_state(state)
}

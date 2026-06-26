//! Reviews domain: course reviews, likes, reports, and the moderation queue.
//!
//! Invariants:
//! - A review is keyed to an `account_id`; the public author is the pseudonymous handle.
//! - `courses.review_count` / `review_avg` are maintained incrementally on write —
//!   never recomputed with `AVG()` on the read path.

// TODO: remove once D5 (admin handlers) is complete.
#![allow(dead_code)]
pub(crate) mod dto;
pub(crate) mod error;
mod handlers;
pub(crate) mod models;
pub(crate) mod repo;

use axum::routing::{get, patch, post};
use axum::Router;
use shared::AppState;

/// All routes owned by the reviews domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v2/courses/{id}/reviews",
            get(handlers::list_reviews).post(handlers::create_review),
        )
        .route("/api/v2/reviews/{id}", patch(handlers::edit_review))
        .route("/api/v2/reviews/{id}/like", post(handlers::like_review))
        .route("/api/v2/reviews/{id}/unlike", post(handlers::unlike_review))
        .route("/api/v2/reviews/{id}/report", post(handlers::report_review))
        .with_state(state)
}

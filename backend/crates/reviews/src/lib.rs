//! Reviews domain: course reviews, likes, reports, and the moderation queue.
//!
//! Invariants:
//! - A review is keyed to an `account_id`; the public author is the pseudonymous handle.
//! - `courses.review_count` / `review_avg` are maintained incrementally on write —
//!   never recomputed with `AVG()` on the read path.

mod admin_handlers;
pub mod appeals;
pub mod data_export;
pub(crate) mod dto;
pub(crate) mod error;
mod handlers;
pub(crate) mod models;
pub(crate) mod repo;
pub mod search;
pub mod tip_targets;

use std::future::Future;
use std::pin::Pin;

use axum::routing::{get, patch, post};
use axum::Router;
pub use repo::claim_legacy_reviews_tx;
use shared::AppState;

/// Reviews-owned implementation of Identity's atomic legacy-claim port.
pub struct LegacyReviewClaimer;

impl identity::LegacyReviewClaimer for LegacyReviewClaimer {
    fn claim<'a>(
        &'a self,
        connection: &'a mut sqlx::PgConnection,
        legacy_user_hash: &'a str,
        account_id: i64,
    ) -> Pin<Box<dyn Future<Output = shared::AppResult<u64>> + Send + 'a>> {
        Box::pin(claim_legacy_reviews_tx(connection, legacy_user_hash, account_id))
    }
}

/// All routes owned by the reviews domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // Public
        .route(
            "/api/v2/courses/{id}/reviews",
            get(handlers::list_reviews).post(handlers::create_review),
        )
        .route("/api/v2/reviews/{id}", patch(handlers::edit_review))
        // POST like, DELETE like (canonical), POST unlike (alias)
        .route(
            "/api/v2/reviews/{id}/like",
            post(handlers::like_review).delete(handlers::unlike_review),
        )
        .route("/api/v2/reviews/{id}/unlike", post(handlers::unlike_review))
        .route("/api/v2/reviews/{id}/report", post(handlers::report_review))
        // Admin
        .route("/api/v2/admin/reviews", get(admin_handlers::admin_list_reviews))
        .route(
            "/api/v2/admin/reviews/{id}",
            axum::routing::delete(admin_handlers::admin_delete_review),
        )
        .route("/api/v2/admin/reviews/{id}/toggle", post(admin_handlers::admin_toggle_review))
        .route("/api/v2/admin/reports", get(admin_handlers::admin_list_reports))
        .route("/api/v2/admin/reports/{id}/resolve", post(admin_handlers::admin_resolve_report))
        .with_state(state)
}

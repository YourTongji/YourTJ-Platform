//! Reviews domain: course reviews, likes, reports, and the moderation queue.
//!
//! Invariants:
//! - A review is keyed to an `account_id`; the public author is the pseudonymous handle.
//! - `courses.review_count` / `review_avg` are maintained incrementally on write —
//!   never recomputed with `AVG()` on the read path.

// TODO: remove when all types are in use.
#![allow(dead_code)]

pub(crate) mod dto;
pub(crate) mod error;
pub(crate) mod models;
pub(crate) mod repo;

use axum::Router;
use shared::AppState;

/// All routes owned by the reviews domain.
pub fn routes(_state: AppState) -> Router {
    Router::new()
}

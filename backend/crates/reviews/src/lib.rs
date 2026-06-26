//! Reviews domain: course reviews, likes, reports, and the moderation queue.
//!
//! Invariants:
//! - A review is keyed to an `account_id`; the public author is the pseudonymous handle.
//! - `courses.review_count` / `review_avg` are maintained incrementally on write —
//!   never recomputed with `AVG()` on the read path.

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

/// All routes owned by the reviews domain.
pub fn routes() -> Router {
    Router::new()
        .route("/api/v2/courses/{id}/reviews", get(list).post(create))
        .route("/api/v2/reviews/{id}/report", post(report))
}

async fn list() -> Json<Value> {
    Json(json!({ "todo": "reviews.list" }))
}

async fn create() -> Json<Value> {
    // TODO(P2): captcha + idempotency + incremental stats update + Meili upsert.
    Json(json!({ "todo": "reviews.create" }))
}

async fn report() -> Json<Value> {
    Json(json!({ "todo": "reviews.report" }))
}

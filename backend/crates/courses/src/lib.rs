//! Courses domain: course catalogue, teachers, departments, the 选课 (PK) mirror
//! tables synced from 一系统, and the realtime search surface.
//!
//! Performance contract: realtime search is served by Meilisearch (pinyin /
//! initials / alias fields), never by `LIKE %q%` over the DB. Browse/list and
//! detail endpoints are cached (short TTL + SWR) and invalidated by version bump.
pub mod dto;
pub mod error;
pub(crate) mod models;
pub(crate) mod repo;
pub(crate) mod selection_repo;
pub mod selection;

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use shared::AppState;

/// All routes owned by the courses domain.
pub fn routes(_state: AppState) -> Router {
    Router::new()
        .route("/api/v2/courses", get(list))
        .route("/api/v2/search", get(search))
        .route("/api/v2/selection/calendars", get(selection_calendars))
}

async fn list() -> Json<Value> {
    // TODO(P2): cursor-paginated browse with cache.
    Json(json!({ "todo": "courses.list" }))
}

async fn search() -> Json<Value> {
    // TODO(P2): proxy to Meilisearch (courses + reviews indices).
    Json(json!({ "todo": "courses.search" }))
}

async fn selection_calendars() -> Json<Value> {
    // TODO(P2): 选课 (PK) mirror — read from the `selection` schema.
    Json(json!({ "todo": "selection.calendars" }))
}

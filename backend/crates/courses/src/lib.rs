//! Courses domain: course catalogue, teachers, departments, the 选课 (PK) mirror
//! tables synced from 一系统, and the realtime search surface.
//!
//! Performance contract: realtime search is served by Meilisearch (pinyin /
//! initials / alias fields), never by `LIKE %q%` over the DB. Browse/list and
//! detail endpoints are cached (short TTL + SWR) and invalidated by version bump.

pub mod dto;
pub mod error;
pub(crate) mod handlers;
pub mod meili;
pub mod models;
pub mod pinyin;
pub mod repo;
pub mod selection;
pub(crate) mod selection_handlers;
pub mod selection_repo;

use axum::routing::get;
use axum::Router;
use shared::AppState;

/// All routes owned by the courses domain, merged under the Axum router.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // --- courses catalogue ---
        .route("/api/v2/courses", get(handlers::list_courses))
        .route("/api/v2/courses/{id}", get(handlers::get_course))
        .route("/api/v2/courses/code/{code}", get(handlers::get_course_by_code))
        .route("/api/v2/courses/{id}/related", get(handlers::list_related_courses))
        .route("/api/v2/departments", get(handlers::list_departments))
        .route("/api/v2/courses/{id}/ai-summary", get(handlers::get_ai_summary))
        // --- global search ---
        .route("/api/v2/search", get(handlers::global_search))
        // --- selection (选课) mirror ---
        .route("/api/v2/selection/calendars", get(selection_handlers::selection_calendars))
        .route("/api/v2/selection/campuses", get(selection_handlers::selection_campuses))
        .route("/api/v2/selection/faculties", get(selection_handlers::selection_faculties))
        .route("/api/v2/selection/grades", get(selection_handlers::selection_grades))
        .route("/api/v2/selection/majors", get(selection_handlers::selection_majors))
        .route(
            "/api/v2/selection/course-natures",
            get(selection_handlers::selection_course_natures),
        )
        .route(
            "/api/v2/selection/courses-by-major",
            get(selection_handlers::selection_courses_by_major),
        )
        .route(
            "/api/v2/selection/courses-by-nature",
            get(selection_handlers::selection_courses_by_nature),
        )
        .route(
            "/api/v2/selection/courses/search",
            get(selection_handlers::selection_courses_search),
        )
        .route(
            "/api/v2/selection/courses/{code}",
            get(selection_handlers::selection_course_by_code),
        )
        .route(
            "/api/v2/selection/courses/{code}/timeslots",
            get(selection_handlers::selection_courses_by_time),
        )
        .route("/api/v2/selection/latest-update", get(selection_handlers::selection_latest_update))
        .with_state(state)
}

//! Courses domain: course catalogue, teachers, departments, the 选课 (PK) mirror
//! tables synced from 一系统, and the realtime search surface.
//!
//! Performance contract: realtime search is served by Meilisearch (pinyin /
//! initials / alias fields), never by `LIKE %q%` over the DB. Browse/list and
//! detail endpoints are cached (short TTL + SWR) and invalidated by version bump.

pub mod admin_handlers;
pub(crate) mod admin_repo;
pub mod dto;
pub mod error;
pub(crate) mod handlers;
pub mod meili;
pub mod models;
pub mod pinyin;
pub mod public_search;
pub mod repo;
pub mod selection;
pub mod selection_admin;
pub(crate) mod selection_handlers;
pub mod selection_repo;
pub mod sync;

use axum::routing::{get, put};
use axum::Router;
use shared::AppState;

/// All routes owned by the courses domain, merged under the Axum router.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // --- courses catalogue ---
        .route("/api/v2/courses", get(handlers::list_courses))
        .route("/api/v2/courses/{id}", get(handlers::get_course))
        // Canonical: GET /api/v2/courses/by-code/{code} — alias: GET /api/v2/courses/code/{code}
        .route("/api/v2/courses/by-code/{code}", get(handlers::get_course_by_code))
        .route("/api/v2/courses/code/{code}", get(handlers::get_course_by_code))
        .route("/api/v2/courses/{id}/related", get(handlers::list_related_courses))
        .route("/api/v2/departments", get(handlers::list_departments))
        .route("/api/v2/courses/{id}/ai-summary", get(handlers::get_ai_summary))
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
        .route("/api/v2/selection/offerings", get(selection_handlers::selection_offerings))
        .route(
            "/api/v2/selection/offerings/{offering_id}",
            get(selection_handlers::selection_offering),
        )
        .route(
            "/api/v2/selection/offerings/{offering_id}/timeslots",
            get(selection_handlers::selection_offering_timeslots),
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
            "/api/v2/selection/courses/{teachingClassId}",
            get(selection_handlers::selection_course_by_id),
        )
        .route(
            "/api/v2/selection/courses/{teachingClassId}/timeslots",
            get(selection_handlers::selection_course_timeslots),
        )
        .route("/api/v2/selection/latest-update", get(selection_handlers::selection_latest_update))
        // --- durable selection operations ---
        .route("/api/v2/admin/selection/sync", axum::routing::post(selection_admin::enqueue_sync))
        .route("/api/v2/admin/selection/sync-jobs", get(selection_admin::list_sync_jobs))
        .route("/api/v2/admin/selection/sync-jobs/{id}", get(selection_admin::get_sync_job))
        .route(
            "/api/v2/admin/selection/sync-jobs/{id}/retry",
            axum::routing::post(selection_admin::retry_sync_job),
        )
        // --- admin course CRUD ---
        .route(
            "/api/v2/admin/courses",
            get(admin_handlers::admin_list_courses).post(admin_handlers::admin_create_course),
        )
        .route(
            "/api/v2/admin/courses/{id}",
            put(admin_handlers::admin_update_course).delete(admin_handlers::admin_delete_course),
        )
        .with_state(state)
}

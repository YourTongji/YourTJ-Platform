//! Axum handlers for the courses domain. Every handler accepts `State<AppState>`
//! and returns `AppResult<Json<…>>`. Errors are mapped from `CoursesError`
//! through the `From` impl in `crate::error`.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use shared::{AppResult, AppState, Page};

use crate::dto::{CourseDetailDto, CourseDto, DepartmentDto, TeacherDto};
use crate::error::CoursesError;
use crate::repo;

/// Query parameters for `GET /api/v2/courses`.
#[derive(Debug, Deserialize)]
pub struct ListCoursesQuery {
    pub dept: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

fn decode_cursor(cursor: &str) -> Result<i64, CoursesError> {
    use std::str;
    let bytes = base64_decode(cursor)?;
    let s =
        str::from_utf8(&bytes).map_err(|_| CoursesError::InvalidSort("invalid cursor".into()))?;
    s.parse::<i64>().map_err(|_| CoursesError::InvalidSort("invalid cursor".into()))
}

fn encode_cursor(id: i64) -> String {
    base64_encode(id.to_string().as_bytes())
}

fn base64_decode(input: &str) -> Result<Vec<u8>, CoursesError> {
    // Use a simple approach: URL-safe base64 or standard base64
    // Try standard + URL-safe + URL-safe without padding
    let decoded = base64_url_decode(input).or_else(|| standard_base64_decode(input));
    decoded.ok_or_else(|| CoursesError::InvalidSort("invalid cursor encoding".into()))
}

fn base64_encode(input: &[u8]) -> String {
    let mut buf = String::new();
    // Simple home-grown base64url (no padding) using config
    let config = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    config.encode_string(input, &mut buf);
    buf
}

fn base64_url_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    // Try URL-safe without padding
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input).ok().or_else(|| {
        // Try URL-safe with padding
        base64::engine::general_purpose::URL_SAFE.decode(input).ok()
    })
}

fn standard_base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(input).ok()
}

/// `GET /api/v2/courses` — cursor-paginated course browse.
pub async fn list_courses(
    State(state): State<AppState>,
    Query(params): Query<ListCoursesQuery>,
) -> AppResult<Json<Page<CourseDto>>> {
    let sort = params.sort.as_deref().unwrap_or("new");
    if !matches!(sort, "hot" | "rating" | "new") {
        return Err(CoursesError::InvalidSort(sort.to_string()).into());
    }

    let cursor_id = params.cursor.as_deref().map(decode_cursor).transpose()?;

    let (rows, next_id) =
        repo::list_courses(&state.db, params.dept.as_deref(), sort, cursor_id, params.limit)
            .await?;

    let items: Vec<CourseDto> = rows.into_iter().map(|r| row_to_course_dto(r, None)).collect();

    let next_cursor = next_id.map(encode_cursor);
    Ok(Json(Page::new(items, next_cursor)))
}

pub async fn get_course(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<CourseDetailDto>> {
    let detail =
        shared::cache::cached_json(state.redis.as_ref(), "course", &id.to_string(), 300, async {
            let row = repo::find_course_by_id(&state.db, id)
                .await?
                .ok_or(CoursesError::CourseNotFound)?;

            let teacher_rows = repo::find_teachers_by_course(&state.db, row.id).await?;
            let aliases = repo::find_aliases(&state.db, row.id).await?;

            let course = row_to_course_dto(row, None);
            let teachers: Vec<TeacherDto> = teacher_rows
                .into_iter()
                .map(|t| TeacherDto {
                    id: t.id.to_string(),
                    name: t.name,
                    title: t.title,
                    department: t.department,
                })
                .collect();

            Ok::<_, CoursesError>(CourseDetailDto { course, teachers, aliases })
        })
        .await?;
    Ok(Json(detail))
}

/// `GET /api/v2/courses/code/:code` — single course by code.
pub async fn get_course_by_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> AppResult<Json<CourseDetailDto>> {
    let code_clone = code.clone();
    let detail =
        shared::cache::cached_json(state.redis.as_ref(), "course_code", &code, 300, async {
            let row = repo::find_course_by_code(&state.db, &code_clone)
                .await?
                .ok_or(CoursesError::CourseNotFound)?;

            let teacher_rows = repo::find_teachers_by_course(&state.db, row.id).await?;
            let aliases = repo::find_aliases(&state.db, row.id).await?;

            let course = row_to_course_dto(row, None);
            let teachers: Vec<TeacherDto> = teacher_rows
                .into_iter()
                .map(|t| TeacherDto {
                    id: t.id.to_string(),
                    name: t.name,
                    title: t.title,
                    department: t.department,
                })
                .collect();

            Ok::<_, CoursesError>(CourseDetailDto { course, teachers, aliases })
        })
        .await?;
    Ok(Json(detail))
}

/// `GET /api/v2/courses/:id/related` — related courses.
pub async fn list_related_courses(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<CourseDto>>> {
    let rows = repo::list_related_courses(&state.db, id).await?;
    let items: Vec<CourseDto> = rows.into_iter().map(|r| row_to_course_dto(r, None)).collect();
    Ok(Json(items))
}

/// `GET /api/v2/courses/:id/ai-summary` — stub returning a placeholder.
pub async fn get_ai_summary(
    State(_state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "courseId": id.to_string(),
        "summary": "AI summaries available soon",
        "model": "pending",
        "updatedAt": chrono::Utc::now().to_rfc3339()
    })))
}

/// `GET /api/v2/departments` — department picker.
pub async fn list_departments(state: State<AppState>) -> AppResult<Json<Vec<DepartmentDto>>> {
    let items =
        shared::cache::cached_json(state.redis.as_ref(), "departments", "all", 3600, async {
            let rows = repo::list_departments(&state.db).await?;
            let depts: Vec<DepartmentDto> = rows
                .into_iter()
                .enumerate()
                .map(|(i, r)| DepartmentDto {
                    id: (i + 1).to_string(),
                    name: r.department.unwrap_or_default(),
                })
                .collect();
            Ok::<_, CoursesError>(depts)
        })
        .await?;
    Ok(Json(items))
}

// Global search handler ---------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
    /// Search type: "course" | "review" | "thread" | "all" (default)
    #[serde(rename = "type", default = "default_query_type")]
    pub query_type: String,
}

fn default_query_type() -> String {
    "all".into()
}

fn default_search_limit() -> usize {
    10
}

/// Structured search result with separated courses, reviews, and forum threads.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub courses: Vec<serde_json::Value>,
    pub reviews: Vec<serde_json::Value>,
    pub threads: Vec<serde_json::Value>,
}

/// GET /api/v2/search — global Meilisearch search across courses, reviews, and forum threads.
pub async fn global_search(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<SearchQuery>,
) -> AppResult<Json<SearchResultDto>> {
    // Rate-limit search: 30 requests per 10 seconds per client IP.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .unwrap_or("unknown");
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "search", ip, 30, 10).await?;

    use crate::meili;

    let results = meili::search_courses_and_reviews(
        &state.meili_url,
        &state.meili_master_key,
        &params.q,
        params.limit,
    )
    .await;

    let mut courses = Vec::new();
    let mut reviews = Vec::new();

    for r in results {
        let item = serde_json::json!({
            "id": r.id,
            "name": r.name,
            "code": r.code,
            "kind": r.kind,
        });
        if r.kind == "review" {
            reviews.push(item);
        } else {
            courses.push(item);
        }
    }

    // Search forum threads when type includes "thread" or is "all" (default).
    let threads = if params.query_type == "thread" || params.query_type == "all" {
        forum::meili::search_threads(
            &state.db,
            &state.meili_url,
            &state.meili_master_key,
            &params.q,
            params.limit,
        )
        .await?
    } else {
        Vec::new()
    };

    Ok(Json(SearchResultDto { courses, reviews, threads }))
}

// Helpers ---------------------------------------------------------------

fn row_to_course_dto(r: repo::CourseWithTeacherRow, _teacher_name: Option<String>) -> CourseDto {
    CourseDto {
        id: r.id.to_string(),
        code: r.code,
        name: r.name,
        credit: r.credit,
        department: r.department,
        teacher_name: r.teacher_name,
        review_count: r.review_count,
        review_avg: r.review_avg,
    }
}

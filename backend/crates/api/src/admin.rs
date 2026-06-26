//! Admin cross-cutting endpoints: selection sync, review reindex, and courses CRUD.
//! Lives in the api crate because these endpoints span multiple domains.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState, AuthAccount};

// ---------------------------------------------------------------------------
// Stub handlers
// ---------------------------------------------------------------------------

/// POST /api/v2/admin/selection/sync — stub (queued)
pub async fn selection_sync_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({ "status": "queued" }))))
}

/// POST /api/v2/admin/reviews/reindex — stub (queued)
pub async fn reviews_reindex_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({ "status": "queued" }))))
}

// ---------------------------------------------------------------------------
// Courses admin DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCourseDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_name: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCourseInput {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub credit: Option<f64>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub teacher_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCourseInput {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub credit: Option<f64>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub teacher_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Admin course handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/admin/courses — list all courses (wide view)
pub async fn admin_list_courses(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AdminCourseDto>>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    // Use the courses crate's repo directly.
    // We need to reach into courses::repo which exposes list_courses.
    // Since courses::repo is pub, we can call its functions.
    // But list_courses is cursor-paginated. For admin we want all courses.
    // We'll query directly here with a simple SELECT.

    let rows = sqlx::query_as::<_, RawCourseRow>(
        "SELECT c.id, c.code, c.name, c.credit, c.department, \
         t.name AS teacher_name, c.review_count, c.review_avg \
         FROM courses.courses c \
         LEFT JOIN courses.teachers t ON c.teacher_id = t.id \
         ORDER BY c.id",
    )
    .fetch_all(&state.db)
    .await?;

    let items: Vec<AdminCourseDto> = rows
        .into_iter()
        .map(|r| AdminCourseDto {
            id: r.id.to_string(),
            code: r.code,
            name: r.name,
            credit: r.credit,
            department: r.department,
            teacher_name: r.teacher_name,
            review_count: r.review_count,
            review_avg: r.review_avg,
        })
        .collect();

    Ok(Json(items))
}

/// POST /api/v2/admin/courses — create a course
pub async fn admin_create_course(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateCourseInput>,
) -> AppResult<(StatusCode, Json<AdminCourseDto>)> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    // Insert teacher if teacher_name provided, or use NULL
    let teacher_id: Option<i64> = if let Some(ref name) = body.teacher_name {
        // Upsert teacher: INSERT ... ON CONFLICT DO NOTHING, then SELECT id
        let id: Option<(i64,)> = sqlx::query_as(
            "INSERT INTO courses.teachers (name) VALUES ($1) \
             ON CONFLICT DO NOTHING",
        )
        .bind(name)
        .fetch_optional(&state.db)
        .await?;

        // If already exists, look up by name
        if let Some((tid,)) = id {
            Some(tid)
        } else {
            let tid: (i64,) = sqlx::query_as("SELECT id FROM courses.teachers WHERE name = $1")
                .bind(name)
                .fetch_one(&state.db)
                .await?;
            Some(tid.0)
        }
    } else {
        None
    };

    let row = sqlx::query_as::<_, RawCourseRow>(
        "INSERT INTO courses.courses (code, name, credit, department, teacher_id) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, code, name, credit, department, \
         NULL::text AS teacher_name, 0 AS review_count, 0.0::float8 AS review_avg",
    )
    .bind(&body.code)
    .bind(&body.name)
    .bind(body.credit)
    .bind(&body.department)
    .bind(teacher_id)
    .fetch_one(&state.db)
    .await?;

    let teacher_name = body.teacher_name.clone();
    let dto = AdminCourseDto {
        id: row.id.to_string(),
        code: row.code,
        name: row.name,
        credit: row.credit,
        department: row.department,
        teacher_name,
        review_count: 0,
        review_avg: None,
    };

    Ok((StatusCode::CREATED, Json(dto)))
}

/// PUT /api/v2/admin/courses/{id} — update a course
pub async fn admin_update_course(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(body): Json<UpdateCourseInput>,
) -> AppResult<Json<AdminCourseDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::BadRequest("invalid course id".into()))?;

    // Build dynamic update
    let mut set_clauses = Vec::new();
    let mut idx = 0u32;

    if let Some(ref code) = body.code {
        idx += 1;
        set_clauses.push((format!("code = ${idx}"), code.clone()));
    }
    if let Some(ref name) = body.name {
        idx += 1;
        set_clauses.push((format!("name = ${idx}"), name.clone()));
    }
    if let Some(credit) = body.credit {
        idx += 1;
        set_clauses.push((format!("credit = ${idx}"), credit.to_string()));
    }
    if let Some(ref dept) = body.department {
        idx += 1;
        set_clauses.push((format!("department = ${idx}"), dept.clone()));
    }
    if let Some(ref teacher_name) = body.teacher_name {
        // Upsert teacher
        let tid: Option<(i64,)> = sqlx::query_as(
            "INSERT INTO courses.teachers (name) VALUES ($1) \
             ON CONFLICT DO NOTHING",
        )
        .bind(teacher_name)
        .fetch_optional(&state.db)
        .await?;
        let teacher_id = if let Some((tid_val,)) = tid {
            tid_val
        } else {
            let tid: (i64,) = sqlx::query_as("SELECT id FROM courses.teachers WHERE name = $1")
                .bind(teacher_name)
                .fetch_one(&state.db)
                .await?;
            tid.0
        };
        idx += 1;
        set_clauses.push((format!("teacher_id = ${idx}"), teacher_id.to_string()));
    }

    if set_clauses.is_empty() {
        // No fields to update — fetch and return current
        let row = sqlx::query_as::<_, RawCourseRow>(
            "SELECT c.id, c.code, c.name, c.credit, c.department, \
             t.name AS teacher_name, c.review_count, c.review_avg \
             FROM courses.courses c \
             LEFT JOIN courses.teachers t ON c.teacher_id = t.id \
             WHERE c.id = $1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
        return Ok(Json(AdminCourseDto {
            id: row.id.to_string(),
            code: row.code,
            name: row.name,
            credit: row.credit,
            department: row.department,
            teacher_name: row.teacher_name,
            review_count: row.review_count,
            review_avg: row.review_avg,
        }));
    }

    // Build SQL
    let parts: Vec<&str> = set_clauses.iter().map(|(c, _)| c.as_str()).collect();
    let mut sql = String::from("UPDATE courses.courses SET ");
    sql.push_str(&parts.join(", "));
    idx += 1;
    sql.push_str(&format!(" WHERE id = ${idx} RETURNING id, code, name, credit, department"));

    let mut q = sqlx::query_as::<_, RawCourseRow>(&sql);
    for (_, val) in &set_clauses {
        q = q.bind(val);
    }
    let row = q.bind(id).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    // Fetch teacher name for response
    let teacher_name = if let Some(ref tn) = body.teacher_name {
        Some(tn.clone())
    } else {
        // Look up current teacher
        let tn: Option<(String,)> = sqlx::query_as(
            "SELECT t.name FROM courses.teachers t \
             JOIN courses.courses c ON c.teacher_id = t.id WHERE c.id = $1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

        tn.map(|(n,)| n)
    };

    Ok(Json(AdminCourseDto {
        id: row.id.to_string(),
        code: row.code,
        name: row.name,
        credit: row.credit,
        department: row.department,
        teacher_name,
        review_count: row.review_count,
        review_avg: row.review_avg,
    }))
}

/// DELETE /api/v2/admin/courses/{id} — delete a course
pub async fn admin_delete_course(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::BadRequest("invalid course id".into()))?;

    let rows = sqlx::query("DELETE FROM courses.courses WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Raw row for admin queries
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
struct RawCourseRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_name: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// All admin routes.
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/admin/selection/sync", post(selection_sync_handler))
        .route("/api/v2/admin/reviews/reindex", post(reviews_reindex_handler))
        .route("/api/v2/admin/courses", get(admin_list_courses).post(admin_create_course))
        .route("/api/v2/admin/courses/{id}", put(admin_update_course).delete(admin_delete_course))
        .with_state(state)
}

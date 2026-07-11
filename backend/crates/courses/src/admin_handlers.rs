//! Axum handlers for admin course CRUD.
//!
//! Every handler requires a `mod` or `admin` role. Auth is resolved via
//! `identity::auth_middleware::authenticate`.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState, Page};

use crate::admin_repo;

// ---------------------------------------------------------------------------
// DTOs
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
    pub reason: String,
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
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReasonInput {
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminListCoursesQuery {
    pub cursor: Option<i64>,
    #[serde(default = "default_admin_limit")]
    pub limit: i64,
}

fn default_admin_limit() -> i64 {
    20
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
}

fn validate_text<'a>(
    value: Option<&'a str>,
    maximum: usize,
    field: &str,
) -> AppResult<Option<&'a str>> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if value.is_some_and(|value| value.chars().count() > maximum) {
        return Err(AppError::BadRequest(format!("{field} is too long")));
    }
    Ok(value)
}

fn validate_credit(credit: Option<f64>) -> AppResult<()> {
    if credit.is_some_and(|credit| !credit.is_finite() || !(0.0..=100.0).contains(&credit)) {
        return Err(AppError::BadRequest("credit must be between 0 and 100".into()));
    }
    Ok(())
}

/// GET /api/v2/admin/courses — list all courses (wide admin view), cursor-paginated
pub async fn admin_list_courses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<AdminListCoursesQuery>,
) -> AppResult<Json<Page<AdminCourseDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCourses)
        .map_err(|_| AppError::Forbidden)?;

    let page = admin_repo::admin_list_courses(&state.db, params.cursor, params.limit).await?;

    let items: Vec<AdminCourseDto> = page
        .items
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

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/admin/courses — create a course
pub async fn admin_create_course(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateCourseInput>,
) -> AppResult<(StatusCode, Json<AdminCourseDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCourses)
        .map_err(|_| AppError::Forbidden)?;

    let reason = validate_reason(&body.reason)?;
    let code = validate_text(Some(&body.code), 64, "code")?
        .ok_or_else(|| AppError::BadRequest("code is required".into()))?;
    let name = validate_text(Some(&body.name), 200, "name")?
        .ok_or_else(|| AppError::BadRequest("name is required".into()))?;
    let department = validate_text(body.department.as_deref(), 200, "department")?;
    let teacher_name = validate_text(body.teacher_name.as_deref(), 200, "teacherName")?;
    validate_credit(body.credit)?;
    let mut tx = state.db.begin().await?;
    let row =
        admin_repo::admin_create_course(&mut tx, code, name, body.credit, department, teacher_name)
            .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "courses.course.created",
        "course",
        &row.id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    crate::meili::reconcile_course_in_background(&state, row.id);

    let dto = AdminCourseDto {
        id: row.id.to_string(),
        code: row.code,
        name: row.name,
        credit: row.credit,
        department: row.department,
        teacher_name: teacher_name.map(str::to_string),
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
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCourses)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::BadRequest("invalid course id".into()))?;
    let reason = validate_reason(&body.reason)?;
    let code = validate_text(body.code.as_deref(), 64, "code")?;
    let name = validate_text(body.name.as_deref(), 200, "name")?;
    let department = validate_text(body.department.as_deref(), 200, "department")?;
    let teacher_name_input = validate_text(body.teacher_name.as_deref(), 200, "teacherName")?;
    validate_credit(body.credit)?;
    let changed_fields = [
        code.map(|_| "code"),
        name.map(|_| "name"),
        body.credit.map(|_| "credit"),
        department.map(|_| "department"),
        teacher_name_input.map(|_| "teacherName"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if changed_fields.is_empty() {
        return Err(AppError::BadRequest("at least one course field is required".into()));
    }
    let mut tx = state.db.begin().await?;
    let row = admin_repo::admin_update_course(
        &mut tx,
        id,
        code,
        name,
        body.credit,
        department,
        teacher_name_input,
    )
    .await?
    .ok_or(AppError::NotFound)?;
    let metadata = serde_json::json!({ "changedFields": changed_fields });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "courses.course.updated",
        "course",
        &id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    crate::meili::reconcile_course_in_background(&state, id);

    let teacher_name = if let Some(teacher_name) = teacher_name_input {
        Some(teacher_name.to_string())
    } else {
        admin_repo::find_teacher_name_by_course(&state.db, id).await?
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
    Json(body): Json<AdminReasonInput>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCourses)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::BadRequest("invalid course id".into()))?;
    let reason = validate_reason(&body.reason)?;
    let mut tx = state.db.begin().await?;
    let deleted = admin_repo::admin_delete_course(&mut tx, id).await?;
    if !deleted {
        return Err(AppError::NotFound);
    }
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "courses.course.deleted",
        "course",
        &id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    crate::meili::reconcile_course_in_background(&state, id);

    Ok(StatusCode::NO_CONTENT)
}

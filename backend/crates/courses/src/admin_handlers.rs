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
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

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
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let row = admin_repo::admin_create_course(
        &state.db,
        &body.code,
        &body.name,
        body.credit,
        body.department.as_deref(),
        body.teacher_name.as_deref(),
    )
    .await?;

    let dto = AdminCourseDto {
        id: row.id.to_string(),
        code: row.code,
        name: row.name,
        credit: row.credit,
        department: row.department,
        teacher_name: body.teacher_name.clone(),
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
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::BadRequest("invalid course id".into()))?;

    let row = admin_repo::admin_update_course(
        &state.db,
        id,
        body.code.as_deref(),
        body.name.as_deref(),
        body.credit,
        body.department.as_deref(),
        body.teacher_name.as_deref(),
    )
    .await?
    .ok_or(AppError::NotFound)?;

    let teacher_name = if let Some(ref tn) = body.teacher_name {
        Some(tn.clone())
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
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::BadRequest("invalid course id".into()))?;

    let deleted = admin_repo::admin_delete_course(&state.db, id).await?;
    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

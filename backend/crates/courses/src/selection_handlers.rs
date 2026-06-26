//! Axum handlers for the selection (选课) domain. Each handler maps to a GET
//! route under `/api/v2/selection/...` and delegates to the selection repo layer.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use shared::{AppResult, AppState};

use crate::error::CoursesError;
use crate::selection::dto::{
    CalendarDto, CampusDto, CourseNatureDto, FacultyDto, LatestUpdateDto, MajorDto,
    SelectionCourseDto, TimeSlotDto,
};
use crate::selection_repo;

/// `GET /api/v2/selection/calendars`
pub async fn selection_calendars(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CalendarDto>>> {
    let rows = selection_repo::list_calendars(&state.db).await?;
    let items: Vec<CalendarDto> = rows
        .into_iter()
        .map(|r| CalendarDto { id: r.id.to_string(), name: r.name, is_current: r.is_current })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/campuses`
pub async fn selection_campuses(State(state): State<AppState>) -> AppResult<Json<Vec<CampusDto>>> {
    let rows = selection_repo::list_campuses(&state.db).await?;
    let items: Vec<CampusDto> =
        rows.into_iter().map(|r| CampusDto { id: r.id.to_string(), name: r.name }).collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/faculties`
pub async fn selection_faculties(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<FacultyDto>>> {
    let rows = selection_repo::list_faculties(&state.db).await?;
    let items: Vec<FacultyDto> = rows
        .into_iter()
        .map(|r| FacultyDto {
            id: r.id.to_string(),
            name: r.name,
            campus_id: r.campus_id.map(|v| v.to_string()),
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/grades?calendarId=...`
#[derive(Debug, Deserialize)]
pub struct GradesQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
}

pub async fn selection_grades(
    State(state): State<AppState>,
    Query(params): Query<GradesQuery>,
) -> AppResult<Json<Vec<String>>> {
    let grades = selection_repo::list_grades(&state.db, params.calendar_id).await?;
    Ok(Json(grades))
}

/// `GET /api/v2/selection/majors?grade=...`
#[derive(Debug, Deserialize)]
pub struct MajorsQuery {
    pub grade: String,
}

pub async fn selection_majors(
    State(state): State<AppState>,
    Query(params): Query<MajorsQuery>,
) -> AppResult<Json<Vec<MajorDto>>> {
    let rows = selection_repo::list_majors(&state.db, &params.grade).await?;
    let items: Vec<MajorDto> = rows
        .into_iter()
        .map(|r| MajorDto {
            id: r.id.to_string(),
            name: r.name,
            faculty_id: r.faculty_id.map(|v| v.to_string()),
            grade: r.grade,
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/course-natures`
pub async fn selection_course_natures(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CourseNatureDto>>> {
    let rows = selection_repo::list_course_natures(&state.db).await?;
    let items: Vec<CourseNatureDto> =
        rows.into_iter().map(|r| CourseNatureDto { id: r.id.to_string(), name: r.name }).collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses-by-major?majorId=...&grade=...`
#[derive(Debug, Deserialize)]
pub struct CoursesByMajorQuery {
    #[serde(rename = "majorId")]
    pub major_id: i64,
    pub grade: String,
}

pub async fn selection_courses_by_major(
    State(state): State<AppState>,
    Query(params): Query<CoursesByMajorQuery>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let rows =
        selection_repo::list_courses_by_major(&state.db, params.major_id, &params.grade).await?;
    let items: Vec<SelectionCourseDto> = rows
        .into_iter()
        .map(|r| SelectionCourseDto {
            id: r.id.to_string(),
            code: r.code,
            name: r.name,
            credit: r.credit,
            nature_id: r.nature_id.map(|v| v.to_string()),
            campus_id: r.campus_id.map(|v| v.to_string()),
            teacher_name: r.teacher_name,
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses-by-nature?natureId=...`
#[derive(Debug, Deserialize)]
pub struct CoursesByNatureQuery {
    #[serde(rename = "natureId")]
    pub nature_id: i64,
}

pub async fn selection_courses_by_nature(
    State(state): State<AppState>,
    Query(params): Query<CoursesByNatureQuery>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let rows = selection_repo::list_courses_by_nature(&state.db, params.nature_id).await?;
    let items: Vec<SelectionCourseDto> = rows
        .into_iter()
        .map(|r| SelectionCourseDto {
            id: r.id.to_string(),
            code: r.code,
            name: r.name,
            credit: r.credit,
            nature_id: r.nature_id.map(|v| v.to_string()),
            campus_id: r.campus_id.map(|v| v.to_string()),
            teacher_name: r.teacher_name,
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses/:code`
pub async fn selection_course_by_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> AppResult<Json<SelectionCourseDto>> {
    let row = selection_repo::find_selection_course_by_code(&state.db, &code)
        .await?
        .ok_or(CoursesError::SelectionCourseNotFound)?;
    Ok(Json(SelectionCourseDto {
        id: row.id.to_string(),
        code: row.code,
        name: row.name,
        credit: row.credit,
        nature_id: row.nature_id.map(|v| v.to_string()),
        campus_id: row.campus_id.map(|v| v.to_string()),
        teacher_name: row.teacher_name,
    }))
}

/// `GET /api/v2/selection/courses/search?q=...`
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn selection_courses_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let rows = selection_repo::search_selection_courses(&state.db, &params.q).await?;
    let items: Vec<SelectionCourseDto> = rows
        .into_iter()
        .map(|r| SelectionCourseDto {
            id: r.id.to_string(),
            code: r.code,
            name: r.name,
            credit: r.credit,
            nature_id: r.nature_id.map(|v| v.to_string()),
            campus_id: r.campus_id.map(|v| v.to_string()),
            teacher_name: r.teacher_name,
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses/:code/timeslots`
pub async fn selection_courses_by_time(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> AppResult<Json<Vec<TimeSlotDto>>> {
    // First find the course by code to get its id
    let course = selection_repo::find_selection_course_by_code(&state.db, &code)
        .await?
        .ok_or(CoursesError::SelectionCourseNotFound)?;
    let rows = selection_repo::list_timeslots(&state.db, course.id).await?;
    let items: Vec<TimeSlotDto> = rows
        .into_iter()
        .map(|r| TimeSlotDto {
            course_id: r.course_id.to_string(),
            teacher_name: r.teacher_name,
            weekday: r.weekday,
            start_slot: r.start_slot,
            end_slot: r.end_slot,
            weeks: r.weeks,
            location: r.location,
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/latest-update`
pub async fn selection_latest_update(
    State(state): State<AppState>,
) -> AppResult<Json<LatestUpdateDto>> {
    let updated_at = selection_repo::find_latest_update(&state.db).await?;
    Ok(Json(LatestUpdateDto { updated_at }))
}

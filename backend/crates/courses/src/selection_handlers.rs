//! Axum handlers for the selection (选课) domain. Each handler maps to a GET
//! route under `/api/v2/selection/...` and delegates to the selection repo layer.

use std::collections::HashMap;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

use crate::error::CoursesError;
use crate::selection::dto::{
    CalendarDto, CampusDto, CourseNatureDto, FacultyDto, LatestUpdateDto, MajorDto,
    SelectionCourseDto, TimeSlotDto,
};
use crate::selection_repo;

/// `GET /api/v2/selection/calendars`
pub async fn selection_calendars(state: State<AppState>) -> AppResult<Json<Vec<CalendarDto>>> {
    let items = shared::cache::cached_json(state.redis.as_ref(), "calendars", "all", 600, async {
        let rows = selection_repo::list_calendars(&state.db).await?;
        let cal: Vec<CalendarDto> = rows
            .into_iter()
            .map(|r| CalendarDto { id: r.id.to_string(), name: r.name, is_current: r.is_current })
            .collect();
        Ok::<_, CoursesError>(cal)
    })
    .await?;
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
pub async fn selection_faculties(state: State<AppState>) -> AppResult<Json<Vec<FacultyDto>>> {
    let items = shared::cache::cached_json(state.redis.as_ref(), "faculties", "all", 600, async {
        let rows = selection_repo::list_faculties(&state.db).await?;
        let fac: Vec<FacultyDto> = rows
            .into_iter()
            .map(|r| FacultyDto {
                id: r.id.to_string(),
                name: r.name,
                campus_id: r.campus_id.map(|v| v.to_string()),
            })
            .collect();
        Ok::<_, CoursesError>(fac)
    })
    .await?;
    Ok(Json(items))
}

/// `GET /api/v2/selection/grades?calendarId=...`
#[derive(Debug, Deserialize)]
pub struct GradesQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
}

fn validate_calendar_id(calendar_id: i64) -> AppResult<()> {
    if calendar_id <= 0 {
        return Err(AppError::BadRequest("invalid selection calendar id".into()));
    }
    Ok(())
}

fn parse_selection_query<T>(query: Result<Query<T>, QueryRejection>) -> AppResult<T> {
    query
        .map(|Query(params)| params)
        .map_err(|_| AppError::BadRequest("invalid selection query".into()))
}

pub async fn selection_grades(
    State(state): State<AppState>,
    query: Result<Query<GradesQuery>, QueryRejection>,
) -> AppResult<Json<Vec<String>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let grades = selection_repo::list_grades(&state.db, params.calendar_id).await?;
    Ok(Json(grades))
}

/// `GET /api/v2/selection/majors?calendarId=...&grade=...`
#[derive(Debug, Deserialize)]
pub struct MajorsQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
    pub grade: String,
}

pub async fn selection_majors(
    State(state): State<AppState>,
    query: Result<Query<MajorsQuery>, QueryRejection>,
) -> AppResult<Json<Vec<MajorDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let rows = selection_repo::list_majors(&state.db, params.calendar_id, &params.grade).await?;
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
    state: State<AppState>,
) -> AppResult<Json<Vec<CourseNatureDto>>> {
    let items = shared::cache::cached_json(state.redis.as_ref(), "natures", "all", 600, async {
        let rows = selection_repo::list_course_natures(&state.db).await?;
        let nats: Vec<CourseNatureDto> = rows
            .into_iter()
            .map(|r| CourseNatureDto { id: r.id.to_string(), name: r.name })
            .collect();
        Ok::<_, CoursesError>(nats)
    })
    .await?;
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses-by-major?calendarId=...&majorId=...&grade=...`
#[derive(Debug, Deserialize)]
pub struct CoursesByMajorQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
    #[serde(rename = "majorId")]
    pub major_id: i64,
    pub grade: String,
}

pub async fn selection_courses_by_major(
    State(state): State<AppState>,
    query: Result<Query<CoursesByMajorQuery>, QueryRejection>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let rows = selection_repo::list_courses_by_major(
        &state.db,
        params.calendar_id,
        params.major_id,
        &params.grade,
    )
    .await?;
    let items: Vec<SelectionCourseDto> = rows
        .into_iter()
        .map(|r| SelectionCourseDto {
            id: r.id.to_string(),
            code: r.code,
            name: r.name,
            credit: r.credit,
            nature_id: r.nature_id.map(|v| v.to_string()),
            calendar_id: r.calendar_id.map(|v| v.to_string()),
            campus_id: r.campus_id.map(|v| v.to_string()),
            teacher_name: r.teacher_name,
            teacher_names: r.teacher_names.unwrap_or_default(),
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses-by-nature?calendarId=...&natureId=...`
#[derive(Debug, Deserialize)]
pub struct CoursesByNatureQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
    #[serde(rename = "natureId")]
    pub nature_id: i64,
}

pub async fn selection_courses_by_nature(
    State(state): State<AppState>,
    query: Result<Query<CoursesByNatureQuery>, QueryRejection>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let rows =
        selection_repo::list_courses_by_nature(&state.db, params.calendar_id, params.nature_id)
            .await?;
    let items: Vec<SelectionCourseDto> = rows
        .into_iter()
        .map(|r| SelectionCourseDto {
            id: r.id.to_string(),
            code: r.code,
            name: r.name,
            credit: r.credit,
            nature_id: r.nature_id.map(|v| v.to_string()),
            calendar_id: r.calendar_id.map(|v| v.to_string()),
            campus_id: r.campus_id.map(|v| v.to_string()),
            teacher_name: r.teacher_name,
            teacher_names: r.teacher_names.unwrap_or_default(),
        })
        .collect();
    Ok(Json(items))
}

fn parse_teaching_class_id(raw_id: &str) -> AppResult<i64> {
    let teaching_class_id = raw_id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid teaching class id".into()))?;
    if teaching_class_id <= 0 {
        return Err(AppError::BadRequest("invalid teaching class id".into()));
    }
    Ok(teaching_class_id)
}

/// `GET /api/v2/selection/courses/:teachingClassId`
pub async fn selection_course_by_id(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> AppResult<Json<SelectionCourseDto>> {
    let teaching_class_id = parse_teaching_class_id(&raw_id)?;
    let row = selection_repo::find_selection_course_by_id(&state.db, teaching_class_id)
        .await?
        .ok_or(CoursesError::SelectionCourseNotFound)?;
    Ok(Json(SelectionCourseDto {
        id: row.id.to_string(),
        code: row.code,
        name: row.name,
        credit: row.credit,
        nature_id: row.nature_id.map(|v| v.to_string()),
        calendar_id: row.calendar_id.map(|v| v.to_string()),
        campus_id: row.campus_id.map(|v| v.to_string()),
        teacher_name: row.teacher_name,
        teacher_names: row.teacher_names.unwrap_or_default(),
    }))
}

/// `GET /api/v2/selection/courses/search?calendarId=...&q=...`
#[derive(Debug, Deserialize)]
pub struct SelectionSearchQuery {
    #[serde(rename = "calendarId")]
    pub calendar_id: i64,
    pub q: String,
}

pub async fn selection_courses_search(
    State(state): State<AppState>,
    query: Result<Query<SelectionSearchQuery>, QueryRejection>,
) -> AppResult<Json<Vec<SelectionCourseDto>>> {
    use crate::meili;

    let params = parse_selection_query(query)?;
    validate_calendar_id(params.calendar_id)?;
    let results = meili::search_selection_courses(
        &state.meili_url,
        &state.meili_master_key,
        &params.q,
        params.calendar_id,
        20,
    )
    .await;

    let candidate_ids =
        results.iter().filter_map(|result| result.id.parse::<i64>().ok()).collect::<Vec<_>>();
    let canonical_rows = selection_repo::find_selection_courses_by_ids(
        &state.db,
        params.calendar_id,
        &candidate_ids,
    )
    .await?;
    let mut canonical_by_id =
        canonical_rows.into_iter().map(|row| (row.id.to_string(), row)).collect::<HashMap<_, _>>();
    let items = results
        .into_iter()
        .filter_map(|candidate| canonical_by_id.remove(&candidate.id))
        .map(|row| SelectionCourseDto {
            id: row.id.to_string(),
            code: row.code,
            name: row.name,
            credit: row.credit,
            nature_id: row.nature_id.map(|value| value.to_string()),
            calendar_id: row.calendar_id.map(|value| value.to_string()),
            campus_id: row.campus_id.map(|value| value.to_string()),
            teacher_name: row.teacher_name,
            teacher_names: row.teacher_names.unwrap_or_default(),
        })
        .collect();
    Ok(Json(items))
}

/// `GET /api/v2/selection/courses/:teachingClassId/timeslots`
pub async fn selection_course_timeslots(
    State(state): State<AppState>,
    Path(raw_id): Path<String>,
) -> AppResult<Json<Vec<TimeSlotDto>>> {
    let teaching_class_id = parse_teaching_class_id(&raw_id)?;
    selection_repo::find_selection_course_by_id(&state.db, teaching_class_id)
        .await?
        .ok_or(CoursesError::SelectionCourseNotFound)?;
    let rows = selection_repo::list_timeslots(&state.db, teaching_class_id).await?;
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

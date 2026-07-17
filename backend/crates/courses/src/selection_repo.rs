//! Repository layer for the selection (选课) mirror.
//!
//! PostgreSQL is authoritative for every public response. Meilisearch may rank
//! candidate offering ids, but candidates are always rehydrated and filtered
//! through this module before they leave the service.

use std::collections::HashMap;

use sqlx::PgPool;

use crate::error::CoursesError;
use crate::selection::models::{
    CalendarRow, CampusRow, CourseNatureRow, FacultyRow, MajorRow, SelectionCourseRow,
    SelectionFreshnessRow, TimeslotRow,
};

const OFFERING_COLUMNS: &str = "c.id, c.code, c.teaching_class_code, c.name, c.credit, \
    c.nature_id, c.calendar_id, c.campus_id, c.faculty_name, c.teaching_language, \
    c.teacher_name, c.teacher_names, c.start_week, c.end_week, c.weeks_unknown, \
    c.schedule_unknown, c.status, c.catalogue_course_id, c.review_count, c.review_avg, \
    c.review_scope";

/// Canonical browse/search filters. Time fields are either all present or all
/// absent after handler validation.
#[derive(Debug, Clone, Default)]
pub struct OfferingFilter {
    pub calendar_id: Option<i64>,
    pub major_id: Option<i64>,
    pub grade: Option<String>,
    pub nature_id: Option<i64>,
    pub campus_id: Option<i64>,
    pub course_code: Option<String>,
    pub weekday: Option<i32>,
    pub start_slot: Option<i32>,
    pub end_slot: Option<i32>,
    pub week: Option<i32>,
    pub include_unknown_schedule: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferingCursor {
    pub code: String,
    pub id: i64,
}

/// List all selection calendars, current first and then newest first.
pub async fn list_calendars(pool: &PgPool) -> Result<Vec<CalendarRow>, CoursesError> {
    Ok(sqlx::query_as::<_, CalendarRow>(
        "SELECT id, name, is_current FROM selection.calendars \
         ORDER BY is_current DESC, id DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn list_campuses(pool: &PgPool) -> Result<Vec<CampusRow>, CoursesError> {
    Ok(sqlx::query_as::<_, CampusRow>("SELECT id, name FROM selection.campuses ORDER BY name, id")
        .fetch_all(pool)
        .await?)
}

pub async fn list_faculties(pool: &PgPool) -> Result<Vec<FacultyRow>, CoursesError> {
    Ok(sqlx::query_as::<_, FacultyRow>(
        "SELECT id, name, campus_id FROM selection.faculties ORDER BY name, id",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn list_grades(pool: &PgPool, calendar_id: i64) -> Result<Vec<String>, CoursesError> {
    let rows: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT DISTINCT binding.grade \
         FROM selection.major_courses AS binding \
         JOIN selection.courses AS course ON binding.course_id = course.id \
         WHERE course.calendar_id = $1 AND binding.grade IS NOT NULL \
         ORDER BY binding.grade",
    )
    .bind(calendar_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().filter_map(|(grade,)| grade).collect())
}

pub async fn list_majors(
    pool: &PgPool,
    calendar_id: i64,
    grade: &str,
) -> Result<Vec<MajorRow>, CoursesError> {
    Ok(sqlx::query_as::<_, MajorRow>(
        "SELECT DISTINCT major.id, major.name, major.faculty_id, major.grade \
         FROM selection.majors AS major \
         JOIN selection.major_courses AS binding ON binding.major_id = major.id \
         JOIN selection.courses AS course ON course.id = binding.course_id \
         WHERE course.calendar_id = $1 AND binding.grade = $2 \
         ORDER BY major.name, major.id LIMIT 500",
    )
    .bind(calendar_id)
    .bind(grade)
    .fetch_all(pool)
    .await?)
}

pub async fn list_course_natures(
    pool: &PgPool,
    calendar_id: i64,
) -> Result<Vec<CourseNatureRow>, CoursesError> {
    Ok(sqlx::query_as::<_, CourseNatureRow>(
        "SELECT DISTINCT nature.id, nature.name \
         FROM selection.course_natures AS nature \
         JOIN selection.courses AS course ON course.nature_id = nature.id \
         WHERE course.calendar_id = $1 ORDER BY nature.name, nature.id",
    )
    .bind(calendar_id)
    .fetch_all(pool)
    .await?)
}

/// Compatibility list with a hard upper bound. New clients use list_offerings.
pub async fn list_courses_by_major(
    pool: &PgPool,
    calendar_id: i64,
    major_id: i64,
    grade: &str,
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    let sql = format!(
        "SELECT {OFFERING_COLUMNS} FROM selection.courses AS c \
         JOIN selection.major_courses AS binding ON binding.course_id = c.id \
         WHERE c.calendar_id = $1 AND binding.major_id = $2 AND binding.grade = $3 \
         ORDER BY c.code, c.id LIMIT 100"
    );
    Ok(sqlx::query_as::<_, SelectionCourseRow>(&sql)
        .bind(calendar_id)
        .bind(major_id)
        .bind(grade)
        .fetch_all(pool)
        .await?)
}

/// Compatibility list with a hard upper bound. New clients use list_offerings.
pub async fn list_courses_by_nature(
    pool: &PgPool,
    calendar_id: i64,
    nature_id: i64,
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    let sql = format!(
        "SELECT {OFFERING_COLUMNS} FROM selection.courses AS c \
         WHERE c.calendar_id = $1 AND c.nature_id = $2 ORDER BY c.code, c.id LIMIT 100"
    );
    Ok(sqlx::query_as::<_, SelectionCourseRow>(&sql)
        .bind(calendar_id)
        .bind(nature_id)
        .fetch_all(pool)
        .await?)
}

/// Find one teaching class by its stable upstream identifier.
pub async fn find_selection_course_by_id(
    pool: &PgPool,
    teaching_class_id: i64,
) -> Result<Option<SelectionCourseRow>, CoursesError> {
    find_offering_by_id(pool, teaching_class_id).await
}

pub async fn find_offering_by_id(
    pool: &PgPool,
    offering_id: i64,
) -> Result<Option<SelectionCourseRow>, CoursesError> {
    let sql = format!("SELECT {OFFERING_COLUMNS} FROM selection.courses AS c WHERE c.id = $1");
    Ok(sqlx::query_as::<_, SelectionCourseRow>(&sql).bind(offering_id).fetch_optional(pool).await?)
}

/// Rehydrate current teaching-class facts for candidate ids in one calendar.
pub async fn find_selection_courses_by_ids(
    pool: &PgPool,
    calendar_id: i64,
    teaching_class_ids: &[i64],
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    if teaching_class_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT {OFFERING_COLUMNS} FROM selection.courses AS c \
         WHERE c.calendar_id = $1 AND c.id = ANY($2) ORDER BY c.id"
    );
    Ok(sqlx::query_as::<_, SelectionCourseRow>(&sql)
        .bind(calendar_id)
        .bind(teaching_class_ids)
        .fetch_all(pool)
        .await?)
}

async fn query_offerings(
    pool: &PgPool,
    filter: &OfferingFilter,
    candidate_ids: Option<Vec<i64>>,
    cursor: Option<&OfferingCursor>,
    fetch_limit: i64,
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    let time_filter = filter.weekday.is_some();
    let sql = format!(
        "SELECT {OFFERING_COLUMNS} \
         FROM selection.courses AS c \
         WHERE ($1::bigint IS NULL OR c.calendar_id = $1) \
           AND ($2::bigint IS NULL OR EXISTS ( \
             SELECT 1 FROM selection.major_courses AS binding \
             WHERE binding.course_id = c.id AND binding.major_id = $2 \
               AND ($3::text IS NULL OR binding.grade = $3) \
           )) \
           AND ($2::bigint IS NOT NULL OR $3::text IS NULL OR EXISTS ( \
             SELECT 1 FROM selection.major_courses AS binding \
             WHERE binding.course_id = c.id AND binding.grade = $3 \
           )) \
           AND ($4::bigint IS NULL OR c.nature_id = $4) \
           AND ($5::bigint IS NULL OR c.campus_id = $5) \
           AND ($6::text IS NULL OR c.code = $6) \
           AND (NOT $7::boolean OR ( \
             ($8::boolean AND c.schedule_unknown) OR ( \
               ($8::boolean OR NOT c.schedule_unknown) AND EXISTS ( \
               SELECT 1 FROM selection.timeslots AS slot \
               WHERE slot.course_id = c.id AND slot.weekday = $9 \
                 AND slot.start_slot <= $11 AND slot.end_slot >= $10 \
                 AND ($8::boolean OR NOT slot.weeks_unknown) \
                 AND ($12::integer IS NULL OR slot.weeks_unknown \
                      OR $12 = ANY(slot.week_numbers)) \
               ) \
             ) \
           )) \
           AND ($13::bigint[] IS NULL OR c.id = ANY($13)) \
           AND ($14::text IS NULL OR (c.code, c.id) > ($14, $15)) \
         ORDER BY c.code, c.id LIMIT $16"
    );
    Ok(sqlx::query_as::<_, SelectionCourseRow>(&sql)
        .bind(filter.calendar_id)
        .bind(filter.major_id)
        .bind(filter.grade.as_deref())
        .bind(filter.nature_id)
        .bind(filter.campus_id)
        .bind(filter.course_code.as_deref())
        .bind(time_filter)
        .bind(filter.include_unknown_schedule)
        .bind(filter.weekday)
        .bind(filter.start_slot)
        .bind(filter.end_slot)
        .bind(filter.week)
        .bind(candidate_ids)
        .bind(cursor.map(|value| value.code.as_str()))
        .bind(cursor.map(|value| value.id))
        .bind(fetch_limit)
        .fetch_all(pool)
        .await?)
}

pub async fn list_offerings(
    pool: &PgPool,
    filter: &OfferingFilter,
    cursor: Option<&OfferingCursor>,
    limit: i64,
) -> Result<(Vec<SelectionCourseRow>, Option<OfferingCursor>), CoursesError> {
    let limit = limit.clamp(1, 100);
    let mut rows = query_offerings(pool, filter, None, cursor, limit + 1).await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.pop();
    }
    let next_cursor = if has_more {
        rows.last().map(|last| OfferingCursor { code: last.code.clone(), id: last.id })
    } else {
        None
    };
    Ok((rows, next_cursor))
}

/// Rehydrate ranked search candidates and reapply every current PostgreSQL
/// filter. Missing/stale index documents disappear rather than leaking.
pub async fn find_offerings_by_candidate_ids(
    pool: &PgPool,
    filter: &OfferingFilter,
    candidate_ids: &[i64],
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    if candidate_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = query_offerings(
        pool,
        filter,
        Some(candidate_ids.to_vec()),
        None,
        candidate_ids.len().min(1_000) as i64,
    )
    .await?;
    let mut by_id: HashMap<i64, SelectionCourseRow> =
        rows.into_iter().map(|row| (row.id, row)).collect();
    Ok(candidate_ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

pub async fn list_timeslots(
    pool: &PgPool,
    offering_id: i64,
) -> Result<Vec<TimeslotRow>, CoursesError> {
    let rows = sqlx::query_as::<_, TimeslotRow>(
        "SELECT course_id, teacher_name, weekday, start_slot, end_slot, weeks, \
                week_numbers, weeks_unknown, location, location_unknown \
         FROM selection.timeslots WHERE course_id = $1 \
         ORDER BY weekday, start_slot, end_slot, location NULLS LAST, teacher_name NULLS LAST \
         LIMIT 101",
    )
    .bind(offering_id)
    .fetch_all(pool)
    .await?;
    if rows.len() > 100 {
        return Err(CoursesError::SelectionScheduleTooLarge);
    }
    Ok(rows)
}

pub async fn find_latest_update(pool: &PgPool) -> Result<SelectionFreshnessRow, CoursesError> {
    Ok(sqlx::query_as::<_, SelectionFreshnessRow>(
        "SELECT (SELECT MAX(fetched_at) FROM selection.fetchlog) AS updated_at, \
                (SELECT MAX(imported_at) FROM selection.import_runs) AS imported_at",
    )
    .fetch_one(pool)
    .await?)
}

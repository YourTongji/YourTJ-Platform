//! Repository layer for the selection (选课) mirror — all queries target the
//! `selection` PostgreSQL schema. Functions accept a `&PgPool` and return domain
//! row types. Every error is wrapped in `CoursesError` before reaching handlers.

use sqlx::PgPool;

use crate::error::CoursesError;
use crate::selection::models::{
    CalendarRow, CampusRow, CourseNatureRow, FacultyRow, MajorRow, SelectionCourseRow, TimeslotRow,
};

/// List all selection calendars, current first.
pub async fn list_calendars(pool: &PgPool) -> Result<Vec<CalendarRow>, CoursesError> {
    let rows = sqlx::query_as::<_, CalendarRow>(
        "SELECT id, name, is_current FROM selection.calendars ORDER BY is_current DESC, id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List all campuses.
pub async fn list_campuses(pool: &PgPool) -> Result<Vec<CampusRow>, CoursesError> {
    let rows =
        sqlx::query_as::<_, CampusRow>("SELECT id, name FROM selection.campuses ORDER BY id")
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

/// List all faculties.
pub async fn list_faculties(pool: &PgPool) -> Result<Vec<FacultyRow>, CoursesError> {
    let rows = sqlx::query_as::<_, FacultyRow>(
        "SELECT id, name, campus_id FROM selection.faculties ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List distinct grades available for a given calendar (from major_courses).
pub async fn list_grades(pool: &PgPool, calendar_id: i64) -> Result<Vec<String>, CoursesError> {
    let rows: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT DISTINCT mc.grade \
         FROM selection.major_courses mc \
         JOIN selection.courses c ON mc.course_id = c.id \
         WHERE c.calendar_id = $1 AND mc.grade IS NOT NULL \
         ORDER BY mc.grade",
    )
    .bind(calendar_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().filter_map(|(g,)| g).collect())
}

/// List majors for a given grade.
pub async fn list_majors(pool: &PgPool, grade: &str) -> Result<Vec<MajorRow>, CoursesError> {
    let rows = sqlx::query_as::<_, MajorRow>(
        "SELECT id, name, faculty_id, grade FROM selection.majors WHERE grade = $1 ORDER BY id",
    )
    .bind(grade)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List all course natures (课程性质).
pub async fn list_course_natures(pool: &PgPool) -> Result<Vec<CourseNatureRow>, CoursesError> {
    let rows = sqlx::query_as::<_, CourseNatureRow>(
        "SELECT id, name FROM selection.course_natures ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List selection courses for a given major + grade.
pub async fn list_courses_by_major(
    pool: &PgPool,
    major_id: i64,
    grade: &str,
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    let rows = sqlx::query_as::<_, SelectionCourseRow>(
        "SELECT c.id, c.code, c.name, c.credit, c.nature_id, c.campus_id, c.teacher_name, c.teacher_names \
         FROM selection.courses c \
         JOIN selection.major_courses mc ON mc.course_id = c.id \
         WHERE mc.major_id = $1 AND mc.grade = $2 \
         ORDER BY c.code",
    )
    .bind(major_id)
    .bind(grade)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List selection courses for a given nature.
pub async fn list_courses_by_nature(
    pool: &PgPool,
    nature_id: i64,
) -> Result<Vec<SelectionCourseRow>, CoursesError> {
    let rows = sqlx::query_as::<_, SelectionCourseRow>(
        "SELECT id, code, name, credit, nature_id, campus_id, teacher_name, teacher_names \
         FROM selection.courses \
         WHERE nature_id = $1 \
         ORDER BY code",
    )
    .bind(nature_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find a single selection course by its code.
pub async fn find_selection_course_by_code(
    pool: &PgPool,
    code: &str,
) -> Result<Option<SelectionCourseRow>, CoursesError> {
    let row = sqlx::query_as::<_, SelectionCourseRow>(
        "SELECT id, code, name, credit, nature_id, campus_id, teacher_name, teacher_names \
         FROM selection.courses \
         WHERE code = $1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// List all timeslots for a given selection course.
pub async fn list_timeslots(
    pool: &PgPool,
    course_id: i64,
) -> Result<Vec<TimeslotRow>, CoursesError> {
    let rows = sqlx::query_as::<_, TimeslotRow>(
        "SELECT course_id, teacher_name, weekday, start_slot, end_slot, weeks, location \
         FROM selection.timeslots \
         WHERE course_id = $1 \
         ORDER BY weekday, start_slot",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// The most recent fetch timestamp from selection.fetchlog, or None.
pub async fn find_latest_update(pool: &PgPool) -> Result<Option<String>, CoursesError> {
    let row: Option<(chrono::NaiveDateTime,)> = sqlx::query_as(
        "SELECT fetched_at FROM selection.fetchlog ORDER BY fetched_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(dt,)| dt.and_utc().to_rfc3339()))
}

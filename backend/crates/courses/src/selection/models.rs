//! Database row structs for the selection (选课) mirror tables. All tables live
//! in the `selection` PostgreSQL schema.

use sqlx::FromRow;

/// A row from `selection.calendars`.
#[derive(Debug, Clone, FromRow)]
pub struct CalendarRow {
    pub id: i64,
    pub name: String,
    pub is_current: bool,
}

/// A row from `selection.campuses`.
#[derive(Debug, Clone, FromRow)]
pub struct CampusRow {
    pub id: i64,
    pub name: String,
}

/// A row from `selection.faculties`.
#[derive(Debug, Clone, FromRow)]
pub struct FacultyRow {
    pub id: i64,
    pub name: String,
    pub campus_id: Option<i64>,
}

/// A row from `selection.majors`.
#[derive(Debug, Clone, FromRow)]
pub struct MajorRow {
    pub id: i64,
    pub name: String,
    pub faculty_id: Option<i64>,
    pub grade: Option<String>,
}

/// A row from `selection.course_natures`.
#[derive(Debug, Clone, FromRow)]
pub struct CourseNatureRow {
    pub id: i64,
    pub name: String,
}

/// A row from `selection.courses`.
#[derive(Debug, Clone, FromRow)]
pub struct SelectionCourseRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub nature_id: Option<i64>,
    pub calendar_id: Option<i64>,
    pub campus_id: Option<i64>,
    pub teacher_name: Option<String>,
    pub teacher_names: Option<Vec<String>>,
}

/// A row from `selection.timeslots`.
#[derive(Debug, Clone, FromRow)]
pub struct TimeslotRow {
    pub course_id: i64,
    pub teacher_name: Option<String>,
    pub weekday: i32,
    pub start_slot: i32,
    pub end_slot: i32,
    pub weeks: Option<String>,
    pub location: Option<String>,
}

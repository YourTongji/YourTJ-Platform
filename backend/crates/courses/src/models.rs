//! Database row structs for the courses domain. Every struct derives `sqlx::FromRow`
//! so it can be materialised directly from a query.

use sqlx::FromRow;

/// A row from `courses.teachers`.
#[derive(Debug, Clone, FromRow)]
pub struct TeacherRow {
    pub id: i64,
    pub name: String,
    pub title: Option<String>,
    pub department: Option<String>,
    pub name_pinyin: Option<String>,
    pub name_initials: Option<String>,
}

/// A row from `courses.courses`.
#[derive(Debug, Clone, FromRow)]
pub struct CourseRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_id: Option<i64>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
    pub name_pinyin: Option<String>,
    pub name_initials: Option<String>,
    pub search_keywords: Option<String>,
}

/// A row from `courses.course_aliases`.
#[derive(Debug, Clone, FromRow)]
pub struct CourseAliasRow {
    pub course_id: i64,
    pub alias: String,
}

/// Virtual row — materialised from `SELECT DISTINCT department FROM courses.courses`.
#[derive(Debug, Clone, FromRow)]
pub struct DepartmentRow {
    pub department: Option<String>,
}

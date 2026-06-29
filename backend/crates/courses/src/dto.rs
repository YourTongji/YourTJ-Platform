//! Public API types for the courses domain. Every struct uses `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format matches OpenAPI conventions. IDs are transmitted as strings to avoid
//! JavaScript integer precision issues with large BIGINT values.

use serde::{Deserialize, Serialize};

/// A department — one row in the department picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentDto {
    pub id: String,
    pub name: String,
}

/// A teacher shown in course details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeacherDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub department: Option<String>,
}

/// A course in a browse / search list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_name: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
}

/// Full detail for a single course page: the course itself, its teachers, and its aliases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseDetailDto {
    #[serde(flatten)]
    pub course: CourseDto,
    pub teachers: Vec<TeacherDto>,
    pub aliases: Vec<String>,
}

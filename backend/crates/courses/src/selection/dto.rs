//! Public API types for the selection (选课) domain. Every struct uses
//! camelCase JSON serialization. IDs are strings to avoid JavaScript
//! integer precision issues.

use serde::{Deserialize, Serialize};

/// A selection calendar (semester).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDto {
    pub id: String,
    pub name: String,
    pub is_current: bool,
}

/// A campus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampusDto {
    pub id: String,
    pub name: String,
}

/// A faculty / college.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacultyDto {
    pub id: String,
    pub name: String,
    pub campus_id: Option<String>,
}

/// A major.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorDto {
    pub id: String,
    pub name: String,
    pub faculty_id: Option<String>,
    pub grade: Option<String>,
}

/// A course nature (必修/选修 etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseNatureDto {
    pub id: String,
    pub name: String,
}

/// A selection course in a list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCourseDto {
    pub id: String,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub nature_id: Option<String>,
    pub campus_id: Option<String>,
    pub teacher_name: Option<String>,
    pub teacher_names: Vec<String>,
}

/// A time-slot for a selection course.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSlotDto {
    pub course_id: String,
    pub teacher_name: Option<String>,
    pub weekday: i32,
    pub start_slot: i32,
    pub end_slot: i32,
    pub weeks: Option<String>,
    pub location: Option<String>,
}

/// Timestamp of the latest data fetch from 一系统.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestUpdateDto {
    pub updated_at: Option<String>,
}

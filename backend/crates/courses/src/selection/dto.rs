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

/// One teaching class in the selection mirror.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCourseDto {
    /// Stable upstream teaching-class identifier.
    pub id: String,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub nature_id: Option<String>,
    pub calendar_id: Option<String>,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LatestUpdateDto, SelectionCourseDto, TimeSlotDto};

    #[test]
    fn serializes_selection_contract_with_explicit_nullable_fields() {
        let course = SelectionCourseDto {
            id: "42".into(),
            code: "CS100".into(),
            name: "课程".into(),
            credit: None,
            nature_id: None,
            calendar_id: Some("7".into()),
            campus_id: None,
            teacher_name: None,
            teacher_names: Vec::new(),
        };
        assert_eq!(
            serde_json::to_value(course).expect("serialize selection course"),
            json!({
                "id": "42",
                "code": "CS100",
                "name": "课程",
                "credit": null,
                "natureId": null,
                "calendarId": "7",
                "campusId": null,
                "teacherName": null,
                "teacherNames": []
            })
        );

        let timeslot = TimeSlotDto {
            course_id: "42".into(),
            teacher_name: None,
            weekday: 1,
            start_slot: 3,
            end_slot: 4,
            weeks: None,
            location: None,
        };
        assert_eq!(
            serde_json::to_value(timeslot).expect("serialize selection timeslot"),
            json!({
                "courseId": "42",
                "teacherName": null,
                "weekday": 1,
                "startSlot": 3,
                "endSlot": 4,
                "weeks": null,
                "location": null
            })
        );

        let latest_update =
            LatestUpdateDto { updated_at: Some("2026-07-14T03:04:05+00:00".into()) };
        assert_eq!(
            serde_json::to_value(latest_update).expect("serialize latest selection update"),
            json!({ "updatedAt": "2026-07-14T03:04:05+00:00" })
        );
    }
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCourseDto {
    pub id: String,
    pub offering_id: String,
    pub code: String,
    pub teaching_class_code: Option<String>,
    pub name: String,
    pub credit: Option<f64>,
    pub nature_id: Option<String>,
    pub calendar_id: String,
    pub campus_id: Option<String>,
    pub faculty_name: Option<String>,
    pub teaching_language: Option<String>,
    pub teacher_name: Option<String>,
    pub teacher_names: Vec<String>,
    pub start_week: Option<i32>,
    pub end_week: Option<i32>,
    pub weeks_unknown: bool,
    pub schedule_unknown: bool,
    pub status: String,
    pub catalogue_course_id: Option<String>,
}

/// A time-slot for a selection course.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSlotDto {
    pub offering_id: String,
    pub course_id: String,
    pub teacher_name: Option<String>,
    pub weekday: i32,
    pub start_slot: i32,
    pub end_slot: i32,
    pub weeks: Option<String>,
    pub week_numbers: Vec<i32>,
    pub weeks_unknown: bool,
    pub location: Option<String>,
    pub location_unknown: bool,
}

/// Timestamp of the latest data fetch from 一系统.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestUpdateDto {
    pub updated_at: Option<String>,
    pub imported_at: Option<String>,
    pub stale: bool,
    pub stale_after_hours: i32,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LatestUpdateDto, SelectionCourseDto, TimeSlotDto};

    #[test]
    fn serializes_selection_contract_with_explicit_nullable_fields() {
        let course = SelectionCourseDto {
            id: "42".into(),
            offering_id: "42".into(),
            code: "CS100".into(),
            teaching_class_code: Some("CS100.01".into()),
            name: "课程".into(),
            credit: None,
            nature_id: None,
            calendar_id: "122".into(),
            campus_id: None,
            faculty_name: None,
            teaching_language: None,
            teacher_name: None,
            teacher_names: Vec::new(),
            start_week: None,
            end_week: None,
            weeks_unknown: true,
            schedule_unknown: true,
            status: "unknown".into(),
            catalogue_course_id: None,
        };
        assert_eq!(
            serde_json::to_value(course).expect("serialize selection course"),
            json!({
                "id": "42",
                "offeringId": "42",
                "code": "CS100",
                "teachingClassCode": "CS100.01",
                "name": "课程",
                "credit": null,
                "natureId": null,
                "calendarId": "122",
                "campusId": null,
                "facultyName": null,
                "teachingLanguage": null,
                "teacherName": null,
                "teacherNames": [],
                "startWeek": null,
                "endWeek": null,
                "weeksUnknown": true,
                "scheduleUnknown": true,
                "status": "unknown",
                "catalogueCourseId": null
            })
        );

        let timeslot = TimeSlotDto {
            offering_id: "42".into(),
            course_id: "42".into(),
            teacher_name: None,
            weekday: 1,
            start_slot: 3,
            end_slot: 4,
            weeks: None,
            week_numbers: Vec::new(),
            weeks_unknown: true,
            location: None,
            location_unknown: true,
        };
        assert_eq!(
            serde_json::to_value(timeslot).expect("serialize selection timeslot"),
            json!({
                "offeringId": "42",
                "courseId": "42",
                "teacherName": null,
                "weekday": 1,
                "startSlot": 3,
                "endSlot": 4,
                "weeks": null,
                "weekNumbers": [],
                "weeksUnknown": true,
                "location": null,
                "locationUnknown": true
            })
        );

        let latest_update = LatestUpdateDto {
            updated_at: Some("2026-07-14T03:04:05+00:00".into()),
            imported_at: Some("2026-07-15T03:04:05+00:00".into()),
            stale: false,
            stale_after_hours: 168,
        };
        assert_eq!(
            serde_json::to_value(latest_update).expect("serialize latest selection update"),
            json!({
                "updatedAt": "2026-07-14T03:04:05+00:00",
                "importedAt": "2026-07-15T03:04:05+00:00",
                "stale": false,
                "staleAfterHours": 168
            })
        );
    }
}

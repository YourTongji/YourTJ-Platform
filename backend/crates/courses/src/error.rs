//! Domain errors for the courses crate. Each variant maps to a stable `AppError`
//! so handlers never leak internal detail to clients.

use shared::AppError;

/// Errors that originate inside the courses domain.
#[derive(Debug, thiserror::Error)]
pub enum CoursesError {
    #[error("course not found")]
    CourseNotFound,

    #[error("invalid department: {0}")]
    InvalidDepartment(String),

    #[error("invalid sort parameter: {0}")]
    InvalidSort(String),

    #[error("selection course not found")]
    SelectionCourseNotFound,

    #[error("selection calendar not found")]
    CalendarNotFound,

    #[error("invalid major or grade")]
    InvalidMajorOrGrade,

    #[error("database error")]
    Database(#[from] sqlx::Error),
}

impl From<CoursesError> for AppError {
    fn from(err: CoursesError) -> Self {
        match err {
            CoursesError::CourseNotFound
            | CoursesError::SelectionCourseNotFound
            | CoursesError::CalendarNotFound => AppError::NotFound,
            CoursesError::InvalidDepartment(_)
            | CoursesError::InvalidSort(_)
            | CoursesError::InvalidMajorOrGrade => AppError::BadRequest(err.to_string()),
            CoursesError::Database(inner) => AppError::Internal(inner.into()),
        }
    }
}

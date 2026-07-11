//! Domain-specific error variants for the reviews crate.
//!
//! [`ReviewsError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

#[derive(Debug, thiserror::Error)]
pub enum ReviewsError {
    #[error("review not found")]
    ReviewNotFound,

    #[error("you can only edit your own review")]
    NotOwnReview,

    #[error("you already have an open report for this review")]
    AlreadyReported,

    #[error("rating must be between 0 and 5")]
    InvalidRating,
}

impl From<ReviewsError> for AppError {
    fn from(err: ReviewsError) -> Self {
        match err {
            ReviewsError::ReviewNotFound => AppError::NotFound,
            ReviewsError::NotOwnReview => AppError::Forbidden,
            ReviewsError::AlreadyReported => AppError::Conflict(err.to_string()),
            ReviewsError::InvalidRating => AppError::BadRequest(err.to_string()),
        }
    }
}

//! Domain-specific error variants for the media crate.
//!
//! [`MediaError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("upload not found")]
    NotFound,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<MediaError> for AppError {
    fn from(e: MediaError) -> Self {
        match e {
            MediaError::NotFound => AppError::NotFound,
            MediaError::Forbidden(msg) => AppError::BadRequest(msg),
            MediaError::BadRequest(msg) => AppError::BadRequest(msg),
            MediaError::Internal(err) => AppError::Internal(err),
        }
    }
}

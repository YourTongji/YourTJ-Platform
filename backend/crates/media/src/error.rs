//! Domain-specific error variants for the media crate.
//!
//! [`MediaError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

#[derive(Debug, thiserror::Error)]
/// Errors raised at the media domain boundary.
pub enum MediaError {
    /// The requested upload does not exist or is intentionally concealed.
    #[error("upload not found")]
    NotFound,

    /// The caller is not allowed to perform the requested media action.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// The request does not satisfy the media contract.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// OSS or STS cannot currently complete the requested operation.
    #[error("media upload service unavailable: {0}")]
    Unavailable(String),

    /// An internal media operation failed without a safe client-facing detail.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<MediaError> for AppError {
    fn from(e: MediaError) -> Self {
        match e {
            MediaError::NotFound => AppError::NotFound,
            MediaError::Forbidden(_) => AppError::Forbidden,
            MediaError::BadRequest(msg) => AppError::BadRequest(msg),
            MediaError::Unavailable(msg) => AppError::BadRequest(msg),
            MediaError::Internal(err) => AppError::Internal(err),
        }
    }
}

//! Domain-specific error variants for the forum crate.
//!
//! [`ForumError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

#[derive(Debug, thiserror::Error)]
pub enum ForumError {
    #[error("board not found")]
    BoardNotFound,

    #[error("thread not found")]
    ThreadNotFound,

    #[error("comment not found")]
    CommentNotFound,
}

impl From<ForumError> for AppError {
    fn from(err: ForumError) -> Self {
        match err {
            ForumError::BoardNotFound
            | ForumError::ThreadNotFound
            | ForumError::CommentNotFound => AppError::NotFound,
        }
    }
}

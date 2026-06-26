//! Domain-specific error variants for the forum crate.
//!
//! [`ForumError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum ForumError {
    #[error("board not found")]
    BoardMissing,

    #[error("thread not found")]
    ThreadMissing,

    #[error("comment not found")]
    CommentMissing,
}

impl From<ForumError> for AppError {
    fn from(err: ForumError) -> Self {
        match err {
            ForumError::BoardMissing | ForumError::ThreadMissing | ForumError::CommentMissing => {
                AppError::NotFound
            }
        }
    }
}

//! Domain-specific error variants for the credit crate.
//!
//! [`CreditError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

#[derive(Debug, thiserror::Error)]
pub enum CreditError {
    #[error("insufficient balance")]
    InsufficientBalance,

    #[error("task not found")]
    TaskNotFound,

    #[error("product not found")]
    ProductNotFound,

    #[error("invalid action: {0}")]
    InvalidAction(String),

    #[error("invalid wallet signature")]
    InvalidSignature,

    #[error("wallet not bound — bind an Ed25519 key first")]
    WalletNotBound,

    #[error("wallet signing intent is unavailable")]
    IntentUnavailable,

    #[error("idempotency key was already used for another request")]
    IdempotencyConflict,
}

impl From<CreditError> for AppError {
    fn from(err: CreditError) -> Self {
        match err {
            CreditError::InsufficientBalance => AppError::BadRequest(err.to_string()),
            CreditError::TaskNotFound | CreditError::ProductNotFound => AppError::NotFound,
            CreditError::InvalidAction(msg) => AppError::BadRequest(msg),
            CreditError::InvalidSignature | CreditError::IntentUnavailable => AppError::Forbidden,
            CreditError::WalletNotBound | CreditError::IdempotencyConflict => {
                AppError::BadRequest(err.to_string())
            }
        }
    }
}

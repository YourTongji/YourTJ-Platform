//! Domain-specific error variants for the identity crate.
//!
//! [`IdentityError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

/// Errors raised by identity domain logic.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("invalid verification code")]
    InvalidCode,

    #[error("verification code has expired")]
    CodeExpired,

    #[error("verification code exhausted (too many attempts)")]
    CodeExhausted,

    #[error("email is already registered")]
    EmailAlreadyUsed,

    #[error("handle is already taken")]
    HandleTaken,

    #[error("handle must be 3–30 characters containing only a-z, 0-9, ., _, and -")]
    InvalidHandle,

    #[error("public key is not a valid Ed25519 key")]
    InvalidPublicKey,

    #[error("key is already bound to this account")]
    KeyAlreadyBound,
}

impl From<IdentityError> for AppError {
    fn from(err: IdentityError) -> Self {
        match err {
            IdentityError::InvalidCode
            | IdentityError::CodeExpired
            | IdentityError::CodeExhausted => AppError::BadRequest(err.to_string()),
            IdentityError::EmailAlreadyUsed | IdentityError::HandleTaken | IdentityError::KeyAlreadyBound => {
                AppError::Conflict(err.to_string())
            }
            IdentityError::InvalidHandle | IdentityError::InvalidPublicKey => {
                AppError::BadRequest(err.to_string())
            }
        }
    }
}

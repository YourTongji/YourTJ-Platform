//! Domain-specific error variants for the identity crate.
//!
//! [`IdentityError`] maps into [`shared::AppError`] via the `From` trait so
//! handlers can use `?` and still return `AppResult<T>`.

use shared::AppError;

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

    #[error("request too frequent — wait 60 seconds")]
    RateLimited,

    #[error("only @tongji.edu.cn email addresses are accepted")]
    InvalidEmailDomain,

    #[error("account invitation has expired; ask an administrator to invite you again")]
    InvitationExpired,

    // Wallet claim errors
    #[error("challenge not found")]
    ChallengeNotFound,

    #[error("challenge expired")]
    ChallengeExpired,

    #[error("challenge already used")]
    ChallengeAlreadyUsed,

    #[error("challenge belongs to another account")]
    ChallengeWrongAccount,

    #[error("legacy wallet link not found")]
    LegacyLinkNotFound,

    #[error("legacy wallet link already claimed")]
    LegacyLinkAlreadyClaimed,

    #[error("legacy wallet link has no public key")]
    LegacyNoPublicKey,

    #[error("invalid signature")]
    InvalidSignature,

    // Password auth errors
    #[error("account already has a password set")]
    AlreadyHasPassword,

    #[error("password does not meet strength requirements")]
    InvalidPassword,

    #[error("wrong password")]
    WrongPassword,

    #[error("no password set on this account")]
    NoPasswordSet,

    #[error("invalid verification code purpose (expected login, registration, or password_reset)")]
    InvalidPurpose,

    #[error("account not found")]
    AccountNotFound,
}

impl From<IdentityError> for AppError {
    fn from(err: IdentityError) -> Self {
        match err {
            IdentityError::InvalidCode
            | IdentityError::CodeExpired
            | IdentityError::CodeExhausted
            | IdentityError::InvalidEmailDomain
            | IdentityError::InvitationExpired => AppError::BadRequest(err.to_string()),
            IdentityError::EmailAlreadyUsed
            | IdentityError::HandleTaken
            | IdentityError::KeyAlreadyBound => AppError::Conflict(err.to_string()),
            IdentityError::RateLimited => AppError::RateLimited,
            IdentityError::InvalidHandle | IdentityError::InvalidPublicKey => {
                AppError::BadRequest(err.to_string())
            }
            IdentityError::ChallengeNotFound
            | IdentityError::LegacyLinkNotFound
            | IdentityError::ChallengeWrongAccount
            | IdentityError::LegacyLinkAlreadyClaimed
            | IdentityError::LegacyNoPublicKey
            | IdentityError::InvalidSignature
            | IdentityError::ChallengeAlreadyUsed => AppError::BadRequest(err.to_string()),
            IdentityError::ChallengeExpired => AppError::BadRequest(err.to_string()),
            IdentityError::AlreadyHasPassword => AppError::BadRequest(err.to_string()),
            IdentityError::InvalidPassword => AppError::BadRequest(err.to_string()),
            IdentityError::WrongPassword => AppError::Unauthorized,
            IdentityError::NoPasswordSet => AppError::BadRequest(err.to_string()),
            IdentityError::InvalidPurpose => AppError::BadRequest(err.to_string()),
            IdentityError::AccountNotFound => AppError::BadRequest(err.to_string()),
        }
    }
}

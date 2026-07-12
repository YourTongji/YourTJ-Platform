//! The single error type crossing the HTTP boundary. Every handler returns
//! `AppResult<T>`; `AppError` renders the platform's stable error envelope:
//! `{ "error": { "code": "...", "message": "..." } }`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Domain-agnostic error. Map specific failures into these variants; never leak
/// internal detail (DB strings, stack traces) to clients — `Internal` is logged
/// server-side and rendered as a generic 500.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("recent authentication required")]
    RecentAuthRequired,

    #[error("{0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("content changed since it was loaded")]
    OptimisticLockConflict { current_version: i64 },

    #[error("rate limited")]
    RateLimited,

    #[error("service unavailable")]
    ServiceUnavailable,

    /// Anything unexpected. The inner error is logged, never serialized.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// HTTP status + stable machine-readable code for this error.
    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::RecentAuthRequired => {
                (StatusCode::PRECONDITION_REQUIRED, "RECENT_AUTH_REQUIRED")
            }
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::OptimisticLockConflict { .. } => (StatusCode::CONFLICT, "VERSION_CONFLICT"),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
            AppError::ServiceUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE")
            }
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();
        let message = match &self {
            AppError::Internal(err) => {
                tracing::error!(error = ?err, "internal error");
                "internal server error".to_string()
            }
            other => other.to_string(),
        };
        let details = match self {
            AppError::OptimisticLockConflict { current_version } => {
                Some(json!({ "currentVersion": current_version }))
            }
            _ => None,
        };
        let body = match details {
            Some(details) => {
                json!({ "error": { "code": code, "message": message, "details": details } })
            }
            None => json!({ "error": { "code": code, "message": message } }),
        };
        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Internal(anyhow::Error::new(err))
    }
}

//! Cross-cutting types shared by every domain crate: configuration, the unified
//! error type, and pagination primitives. Keep this crate dependency-light — it
//! is compiled by everything.

pub mod config;
pub mod error;
pub mod pagination;

pub use config::Config;
pub use error::AppError;
pub use pagination::Page;

/// The result type returned by handlers and domain services across the platform.
pub type AppResult<T> = Result<T, AppError>;

//! Media domain: file uploads, OSS integration, and upload moderation.
//!
//! This crate handles STS credential issuance for direct-to-OSS uploads,
//! OSS callbacks, and the moderation queue for uploaded files.
//!
//! Currently uses placeholder OSS integration. See `oss.rs` for the stubs
//! that should be replaced with real SDK calls.

mod dto;
mod error;
mod handlers;
mod models;
mod oss;
mod repo;

use axum::routing::{get, post};
use axum::Router;
use shared::AppState;

/// All routes owned by the media domain.
pub fn routes(state: AppState) -> Router {
    Router::new()
        // Upload endpoints
        .route("/api/v2/media/upload-credentials", post(handlers::upload_credentials))
        .route("/api/v2/media/callback", post(handlers::callback))
        .route("/api/v2/media/{id}/url", get(handlers::get_url))
        // Admin moderation endpoints
        .route("/api/v2/admin/media/uploads", get(handlers::list_uploads))
        .route("/api/v2/admin/media/uploads/{id}/approve", post(handlers::approve_upload))
        .route("/api/v2/admin/media/uploads/{id}/block", post(handlers::block_upload))
        .with_state(state)
}

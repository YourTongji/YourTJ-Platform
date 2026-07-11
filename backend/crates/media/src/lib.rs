//! Media domain: file uploads, OSS integration, and upload moderation.
//!
//! This crate handles STS credential issuance for direct-to-OSS uploads,
//! OSS callbacks, and the moderation queue for uploaded files.
//!
//! Currently uses placeholder OSS integration. See `oss.rs` for the stubs
//! that should be replaced with real SDK calls.

use std::sync::Arc;

mod dto;
mod error;
mod handlers;
mod models;
mod oss;
mod quarantine;
mod repo;

use axum::routing::{get, post};
use axum::{Extension, Router};
use shared::AppState;

pub use quarantine::UploadObjectStore;

/// All routes owned by the media domain.
pub fn routes(state: AppState) -> Router {
    let object_store = Arc::new(quarantine::AliyunUploadObjectStore::from_config(&state.config));
    routes_with_object_store(state, object_store)
}

/// Build media routes with an alternate object-store provider.
///
/// This keeps the moderation transaction independent of one OSS implementation and supports
/// deterministic end-to-end verification of object deletion ordering.
pub fn routes_with_object_store(
    state: AppState,
    object_store: Arc<dyn UploadObjectStore>,
) -> Router {
    Router::new()
        // Upload endpoints
        .route("/api/v2/media/upload-credentials", post(handlers::upload_credentials))
        .route("/api/v2/media/callback", post(handlers::callback))
        .route("/api/v2/media/{id}/url", get(handlers::get_url))
        // Admin moderation endpoints
        .route("/api/v2/admin/media/uploads", get(handlers::list_uploads))
        .route("/api/v2/admin/media/uploads/{id}/approve", post(handlers::approve_upload))
        .route("/api/v2/admin/media/uploads/{id}/block", post(handlers::block_upload))
        .layer(Extension(object_store))
        .with_state(state)
}

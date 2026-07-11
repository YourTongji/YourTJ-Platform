//! Media domain: file uploads, OSS integration, and upload moderation.
//!
//! This crate handles STS credential issuance for direct-to-OSS uploads,
//! OSS callbacks, and the moderation queue for uploaded files.
//!
//! STS, callback verification, and moderation object deletion use the concrete
//! Alibaba Cloud HTTP protocols behind testable provider/object-store boundaries.

use std::sync::Arc;

pub mod data_export;
mod deletion;
mod dto;
mod error;
mod handlers;
mod image_header;
mod models;
mod moderation;
mod oss;
mod preview;
mod quarantine;
mod repo;

pub mod attachments;

use axum::routing::{get, post, put};
use axum::{Extension, Router};
use shared::{AppResult, AppState};
use sqlx::PgPool;

pub use deletion::{process_one_deletion_job, process_upload_deletion_job, run_deletion_worker};
pub use quarantine::{UploadObjectPreview, UploadObjectStore};

/// Return whether an upload is a clean image owned by the specified account.
///
/// Business domains use this purpose-limited check before persisting an asset reference. They never
/// read media tables directly or accept a client-supplied object URL.
pub async fn is_clean_image_owned_by(
    pool: &PgPool,
    upload_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let upload = repo::find_upload(pool, upload_id).await?;
    Ok(upload.is_some_and(|row| {
        row.account_id == account_id && row.kind == "image" && row.status == "clean"
    }))
}

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
        .route("/api/v2/me/media/uploads", get(handlers::list_my_uploads))
        .route("/api/v2/me/media/uploads/{id}", get(handlers::get_my_upload))
        .route(
            "/api/v2/me/profile/avatar",
            put(handlers::bind_profile_avatar).delete(handlers::clear_profile_avatar),
        )
        .route(
            "/api/v2/me/profile/banner",
            put(handlers::bind_profile_banner).delete(handlers::clear_profile_banner),
        )
        // Admin moderation endpoints
        .route("/api/v2/admin/media/uploads", get(handlers::list_uploads))
        .route(
            "/api/v2/admin/media/uploads/{id}/preview-grants",
            post(handlers::create_upload_preview_grant),
        )
        .route("/api/v2/admin/media/uploads/{id}/preview", get(handlers::preview_upload))
        .route("/api/v2/admin/media/uploads/{id}/approve", post(handlers::approve_upload))
        .route("/api/v2/admin/media/uploads/{id}/block", post(handlers::block_upload))
        .layer(Extension(object_store))
        .with_state(state)
}

/// Resolve a clean platform-controlled image after the owning domain authorizes disclosure.
pub async fn resolve_clean_profile_image(
    pool: &sqlx::PgPool,
    asset_id: Option<i64>,
) -> shared::AppResult<Option<String>> {
    let Some(asset_id) = asset_id else {
        return Ok(None);
    };
    repo::find_clean_image_url(pool, asset_id).await
}

/// Batch-resolve clean platform-controlled images for an authorized projection.
pub async fn resolve_clean_profile_images(
    pool: &sqlx::PgPool,
    asset_ids: &[i64],
) -> shared::AppResult<std::collections::HashMap<i64, String>> {
    let urls = repo::find_clean_image_urls(pool, asset_ids).await?;
    Ok(urls.into_iter().collect())
}

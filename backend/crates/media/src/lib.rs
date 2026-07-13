//! Media domain: file uploads, OSS integration, and upload moderation.
//!
//! This crate handles STS credential issuance for direct-to-OSS uploads,
//! OSS callbacks, and the moderation queue for uploaded files.
//!
//! STS, callback verification, and moderation object deletion use the concrete
//! Alibaba Cloud HTTP protocols behind testable provider/object-store boundaries.

use std::sync::Arc;

mod approval;
mod bindings;
pub mod data_export;
mod deletion;
mod delivery;
mod dto;
mod error;
mod gc;
mod handlers;
mod image_header;
mod issuance;
mod locking;
mod models;
mod moderation;
mod oss;
mod preview;
mod processing;
mod quarantine;
mod reconciliation;
mod repo;
mod retention;

pub mod attachments;

pub use bindings::{detach_account_profile_bindings, sync_asset_binding, AssetBindingType};

use axum::routing::{get, post, put};
use axum::{Extension, Router};
use shared::{AppResult, AppState};
use sqlx::PgPool;

pub use deletion::{process_one_deletion_job, process_upload_deletion_job, run_deletion_worker};
pub use delivery::{ImageDeliveryProjection, ImageVariant};
pub use gc::{
    prepare_account_media_purge, run_retention_gc_worker,
    schedule_expired_upload_intent_cleanup_batch, schedule_retention_gc_batch,
    AccountMediaPurgeProgress,
};
pub use issuance::{
    complete_upload_callback, reserve_upload_intent, UploadCallbackCompletion,
    UploadIntentReservation,
};
pub use processing::{process_one_variant_job, process_upload_variant_job};
pub use quarantine::{DeliveryPurgeTaskState, UploadObjectPreview, UploadObjectStore};
pub use retention::{
    purge_completed_cleanup_tombstones, purge_expired_asset_bindings, purge_expired_preview_grants,
    purge_upload_credential_attempts, run_retention_housekeeping_worker,
};

/// Reject any partially configured or invalid Delivery runtime before the API starts serving.
///
/// A completely absent Delivery configuration remains valid for provider-free PR previews.
pub fn validate_delivery_runtime(config: &shared::Config) -> AppResult<()> {
    let Some(delivery) = delivery::DeliveryConfig::from_env(&config.oss_region)? else {
        return Ok(());
    };
    if delivery.bucket == config.oss_bucket {
        return Err(error::MediaError::Unavailable(
            "media Ingest and Delivery buckets must be distinct".into(),
        )
        .into());
    }
    if delivery.access_key_id == config.oss_access_key_id
        || delivery.access_key_id == delivery.purge_access_key_id
        || config.oss_access_key_id == delivery.purge_access_key_id
    {
        return Err(error::MediaError::Unavailable(
            "media Ingest, Delivery, and CDN purge identities must be distinct".into(),
        )
        .into());
    }
    Ok(())
}

/// Return whether an upload is a clean image owned by the specified account.
///
/// Business domains use this purpose-limited check before persisting an asset reference. They never
/// read media tables directly or accept a client-supplied object URL.
pub async fn is_clean_image_owned_by(
    pool: &PgPool,
    upload_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let is_publishable: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM media.uploads upload \
           JOIN media.asset_publications publication ON publication.asset_id = upload.id \
           JOIN media.asset_variants variant \
             ON variant.asset_id = upload.id \
            AND variant.policy_version = publication.policy_version \
            AND variant.variant_kind = 'display_1280' \
           WHERE upload.id = $1 AND upload.account_id = $2 \
             AND upload.kind = 'image' AND upload.status = 'clean' \
             AND publication.status = 'published' AND variant.status = 'published' \
         )",
    )
    .bind(upload_id)
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(is_publishable)
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
        .route("/api/v2/me/media/uploads/{id}/preview", get(handlers::preview_my_upload))
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
        .route("/api/v2/admin/media/retention-holds", get(handlers::list_retention_holds))
        .route("/api/v2/admin/media/deletion-jobs", get(handlers::list_deletion_jobs))
        .route("/api/v2/admin/media/reconciliation", get(handlers::reconciliation_report))
        .route("/api/v2/admin/media/deletion-jobs/{id}/retry", post(handlers::retry_deletion_job))
        .route(
            "/api/v2/admin/media/uploads/{id}/processing/retry",
            post(handlers::retry_upload_processing),
        )
        .route(
            "/api/v2/admin/media/uploads/{id}/preview-grants",
            post(handlers::create_upload_preview_grant),
        )
        .route("/api/v2/admin/media/uploads/{id}/preview", get(handlers::preview_upload))
        .route("/api/v2/admin/media/uploads/{id}/approve", post(handlers::approve_upload))
        .route("/api/v2/admin/media/uploads/{id}/block", post(handlers::block_upload))
        .route(
            "/api/v2/admin/media/uploads/{id}/retention-hold",
            post(handlers::place_retention_hold).delete(handlers::release_retention_hold),
        )
        .layer(Extension(object_store))
        .with_state(state)
}

/// Resolve typed clean image delivery after the owning domain authorizes disclosure.
pub async fn resolve_clean_image_delivery(
    pool: &sqlx::PgPool,
    asset_id: Option<i64>,
) -> shared::AppResult<Option<ImageDeliveryProjection>> {
    let Some(asset_id) = asset_id else {
        return Ok(None);
    };
    Ok(repo::find_clean_image_deliveries(pool, &[asset_id])
        .await?
        .into_iter()
        .next()
        .map(|(_, projection)| projection))
}

/// Batch-resolve typed clean image delivery for owner-authorized projections.
pub async fn resolve_clean_image_deliveries(
    pool: &sqlx::PgPool,
    asset_ids: &[i64],
) -> shared::AppResult<std::collections::HashMap<i64, ImageDeliveryProjection>> {
    Ok(repo::find_clean_image_deliveries(pool, asset_ids).await?.into_iter().collect())
}

/// Resolve a clean platform-controlled image URL for compatibility with profile consumers.
pub async fn resolve_clean_profile_image(
    pool: &sqlx::PgPool,
    asset_id: Option<i64>,
) -> shared::AppResult<Option<String>> {
    Ok(resolve_clean_image_delivery(pool, asset_id).await?.map(|projection| projection.url))
}

/// Batch-resolve clean platform-controlled images for an authorized projection.
pub async fn resolve_clean_profile_images(
    pool: &sqlx::PgPool,
    asset_ids: &[i64],
) -> shared::AppResult<std::collections::HashMap<i64, String>> {
    Ok(resolve_clean_image_deliveries(pool, asset_ids)
        .await?
        .into_iter()
        .map(|(asset_id, projection)| (asset_id, projection.url))
        .collect())
}

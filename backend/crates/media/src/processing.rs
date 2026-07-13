//! Durable, deterministic publication of sanitized static image variants.

use std::io::Cursor;

use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader,
};
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::delivery::DELIVERY_POLICY_VERSION;
use crate::oss;
use crate::quarantine::UploadObjectStore;

const PROCESSING_LEASE_SECONDS: i64 = 120;
const MAX_PROCESSING_ATTEMPTS: i32 = 8;
const MAX_IMAGE_DIMENSION: u32 = 20_000;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const MAX_DECODER_ALLOCATION: u64 = 192 * 1024 * 1024;

const VARIANT_SPECS: [(&str, u32); 3] =
    [("thumb_256", 256), ("display_1280", 1_280), ("full_2048", 2_048)];

#[derive(Debug, FromRow)]
struct ClaimedProcessingJob {
    id: i64,
    asset_id: i64,
    policy_version: i32,
    oss_key: String,
    mime: String,
    bytes: i64,
    source_sha256: String,
    attempt_count: i32,
    lease_token: Uuid,
}

#[derive(Debug, Clone)]
struct RenderedVariant {
    variant_kind: &'static str,
    object_key: String,
    content_sha256: String,
    bytes: Vec<u8>,
    width: i32,
    height: i32,
}

struct ProcessingFailure {
    code: &'static str,
    error: AppError,
}

impl ProcessingFailure {
    fn new(code: &'static str, error: impl Into<AppError>) -> Self {
        Self { code, error: error.into() }
    }
}

/// Enqueue the current image policy after moderation succeeds; publication remains fail-closed.
pub(crate) async fn enqueue_variant_processing(
    connection: &mut PgConnection,
    asset_id: i64,
) -> AppResult<()> {
    let eligible: Option<bool> = sqlx::query_scalar(
        "SELECT kind = 'image' AND status = 'clean' \
         AND mime IN ('image/jpeg', 'image/png', 'image/webp') \
         FROM media.uploads WHERE id = $1 FOR UPDATE",
    )
    .bind(asset_id)
    .fetch_optional(&mut *connection)
    .await?;
    if eligible != Some(true) {
        return Err(AppError::Conflict(
            "only approved static images can enter Delivery processing".into(),
        ));
    }
    sqlx::query(
        "UPDATE media.asset_publications \
         SET policy_version = $2, status = 'processing', published_at = NULL, \
             blocked_at = NULL, last_error_code = NULL, updated_at = now() \
         WHERE asset_id = $1 AND status IN ('unpublished', 'processing', 'failed')",
    )
    .bind(asset_id)
    .bind(DELIVERY_POLICY_VERSION)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "INSERT INTO media.variant_processing_jobs (asset_id, policy_version) \
         VALUES ($1, $2) \
         ON CONFLICT (asset_id, policy_version) DO UPDATE \
         SET status = CASE WHEN media.variant_processing_jobs.status = 'succeeded' \
                           THEN 'succeeded' ELSE 'queued' END, \
             attempt_count = CASE WHEN media.variant_processing_jobs.status = 'succeeded' \
                                  THEN media.variant_processing_jobs.attempt_count ELSE 0 END, \
             available_at = now(), lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = NULL, \
             completed_at = CASE WHEN media.variant_processing_jobs.status = 'succeeded' \
                                 THEN media.variant_processing_jobs.completed_at END, \
             updated_at = now()",
    )
    .bind(asset_id)
    .bind(DELIVERY_POLICY_VERSION)
    .execute(connection)
    .await?;
    Ok(())
}

/// Requeue one exhausted job after an operations actor reviews its recorded failure.
pub(crate) async fn retry_failed_processing(
    pool: &PgPool,
    auth_context: &identity::auth_middleware::AuthenticatedContext,
    asset_id: i64,
    reason: &str,
) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    identity::auth_middleware::require_recent_auth_tx(auth_context, &mut transaction).await?;
    let publication: Option<(String, String, String, i32, Option<String>)> = sqlx::query_as(
        "SELECT upload.status, upload.kind, upload.mime, publication.policy_version, \
                publication.last_error_code \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         WHERE upload.id = $1 AND publication.status = 'failed' \
         FOR UPDATE OF upload, publication",
    )
    .bind(asset_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some((upload_status, kind, mime, policy_version, publication_error_code)) = publication
    else {
        return Err(AppError::Conflict(
            "media Delivery processing is not in a retryable failed state".into(),
        ));
    };
    if upload_status != "clean"
        || kind != "image"
        || !matches!(mime.as_str(), "image/jpeg" | "image/png" | "image/webp")
    {
        return Err(AppError::Conflict(
            "media Delivery processing is no longer eligible for retry".into(),
        ));
    }
    let job: Option<(i64, i32, Option<String>)> = sqlx::query_as(
        "SELECT id, attempt_count, last_error_code \
         FROM media.variant_processing_jobs \
         WHERE asset_id = $1 AND policy_version = $2 AND status = 'dead_letter' \
         FOR UPDATE",
    )
    .bind(asset_id)
    .bind(policy_version)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some((job_id, previous_attempt_count, job_error_code)) = job else {
        return Err(AppError::Conflict(
            "media Delivery processing job is not dead-lettered".into(),
        ));
    };
    let job_affected = sqlx::query(
        "UPDATE media.variant_processing_jobs \
         SET status = 'queued', attempt_count = 0, available_at = now(), \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
             completed_at = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'dead_letter'",
    )
    .bind(job_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    let publication_affected = sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'processing', published_at = NULL, blocked_at = NULL, \
             last_error_code = NULL, updated_at = now() \
         WHERE asset_id = $1 AND policy_version = $2 AND status = 'failed'",
    )
    .bind(asset_id)
    .bind(policy_version)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if job_affected != 1 || publication_affected != 1 {
        return Err(AppError::Conflict(
            "media Delivery processing retry lost its state fence".into(),
        ));
    }
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        "media.asset.processing_requeued",
        "upload",
        &asset_id.to_string(),
        reason,
        Some(&serde_json::json!({
            "policyVersion": policy_version,
            "previousAttemptCount": previous_attempt_count,
            "previousPublicationErrorCode": publication_error_code,
            "previousJobErrorCode": job_error_code,
        })),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn claim_processing_job(
    pool: &PgPool,
    asset_id: Option<i64>,
) -> AppResult<Option<ClaimedProcessingJob>> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "UPDATE media.variant_processing_jobs job \
         SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
             last_error_code = 'lease_expired_after_max_attempts', updated_at = now() \
         WHERE job.status = 'leased' AND job.lease_expires_at <= now() \
           AND job.attempt_count >= $1",
    )
    .bind(MAX_PROCESSING_ATTEMPTS)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE media.asset_publications publication \
         SET status = 'failed', published_at = NULL, \
             last_error_code = job.last_error_code, updated_at = now() \
         FROM media.variant_processing_jobs job \
         WHERE job.asset_id = publication.asset_id \
           AND job.policy_version = publication.policy_version \
           AND job.status = 'dead_letter' AND publication.status = 'processing'",
    )
    .execute(&mut *transaction)
    .await?;
    let job_id: Option<i64> = sqlx::query_scalar(
        "SELECT job.id FROM media.variant_processing_jobs job \
         JOIN media.uploads upload ON upload.id = job.asset_id \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         WHERE upload.kind = 'image' AND upload.status = 'clean' \
           AND upload.mime IN ('image/jpeg', 'image/png', 'image/webp') \
           AND publication.status = 'processing' \
           AND publication.policy_version = job.policy_version \
           AND job.attempt_count < $1 AND ($2::bigint IS NULL OR job.asset_id = $2) \
           AND ((job.status = 'queued' AND job.available_at <= now()) \
                OR (job.status = 'leased' AND job.lease_expires_at <= now())) \
         ORDER BY job.available_at, job.id FOR UPDATE OF job SKIP LOCKED LIMIT 1",
    )
    .bind(MAX_PROCESSING_ATTEMPTS)
    .bind(asset_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(job_id) = job_id else {
        transaction.commit().await?;
        return Ok(None);
    };
    let lease_token = Uuid::new_v4();
    let job = sqlx::query_as::<_, ClaimedProcessingJob>(
        "UPDATE media.variant_processing_jobs job \
         SET status = 'leased', attempt_count = job.attempt_count + 1, lease_token = $2, \
             lease_expires_at = now() + ($3 * interval '1 second'), updated_at = now() \
         FROM media.uploads upload \
         WHERE job.id = $1 AND upload.id = job.asset_id AND upload.status = 'clean' \
         RETURNING job.id, job.asset_id, job.policy_version, upload.oss_key, upload.mime, \
                   upload.bytes, upload.sha256 AS source_sha256, \
                   job.attempt_count, job.lease_token",
    )
    .bind(job_id)
    .bind(lease_token)
    .bind(PROCESSING_LEASE_SECONDS)
    .fetch_optional(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(job)
}

async fn register_processing_variants(
    pool: &PgPool,
    job: &ClaimedProcessingJob,
    variants: &[RenderedVariant],
) -> AppResult<bool> {
    let mut transaction = pool.begin().await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(job.asset_id)
            .fetch_optional(&mut *transaction)
            .await?;
    let lease_exists = if status.as_deref() == Some("clean") {
        sqlx::query(
            "UPDATE media.variant_processing_jobs \
             SET lease_expires_at = now() + ($3 * interval '1 second'), updated_at = now() \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2 \
               AND lease_expires_at > now()",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .bind(PROCESSING_LEASE_SECONDS)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1
    } else {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM media.variant_processing_jobs \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2)",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .fetch_one(&mut *transaction)
        .await?
    };
    if !lease_exists {
        return Err(AppError::Conflict("media processing lease was lost".into()));
    }
    let variant_status =
        if status.as_deref() == Some("clean") { "processing" } else { "quarantined" };
    for variant in variants {
        let affected = sqlx::query(
            "INSERT INTO media.asset_variants \
             (asset_id, variant_kind, policy_version, object_key, content_sha256, mime, \
              bytes, width, height, status) \
             VALUES ($1, $2, $3, $4, $5, 'image/webp', $6, $7, $8, $9) \
             ON CONFLICT (asset_id, policy_version, variant_kind) DO UPDATE \
             SET status = EXCLUDED.status \
             WHERE media.asset_variants.object_key = EXCLUDED.object_key \
               AND media.asset_variants.content_sha256 = EXCLUDED.content_sha256 \
               AND media.asset_variants.bytes = EXCLUDED.bytes \
               AND media.asset_variants.width = EXCLUDED.width \
               AND media.asset_variants.height = EXCLUDED.height",
        )
        .bind(job.asset_id)
        .bind(variant.variant_kind)
        .bind(job.policy_version)
        .bind(&variant.object_key)
        .bind(&variant.content_sha256)
        .bind(variant.bytes.len() as i64)
        .bind(variant.width)
        .bind(variant.height)
        .bind(variant_status)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if affected != 1 {
            return Err(AppError::Conflict(
                "media processing output changed for one policy version".into(),
            ));
        }
    }
    if variant_status == "quarantined" {
        enqueue_late_variant_cleanup(&mut transaction, job.asset_id, variants).await?;
        sqlx::query(
            "UPDATE media.variant_processing_jobs \
             SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
                 last_error_code = 'asset_left_clean_state', updated_at = now() \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        return Ok(false);
    }
    transaction.commit().await?;
    Ok(true)
}

async fn enqueue_late_variant_cleanup(
    connection: &mut PgConnection,
    asset_id: i64,
    variants: &[RenderedVariant],
) -> AppResult<()> {
    let deletion_job_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM media.object_deletion_jobs WHERE upload_id = $1 FOR UPDATE",
    )
    .bind(asset_id)
    .fetch_optional(&mut *connection)
    .await?;
    let deletion_job_id = deletion_job_id.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("media asset left clean state without a deletion job"))
    })?;
    sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET status = 'queued', attempt_count = 0, available_at = now(), \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
             completed_at = NULL, updated_at = now() \
         WHERE id = $1",
    )
    .bind(deletion_job_id)
    .execute(&mut *connection)
    .await?;
    for variant in variants {
        for step_kind in ["cdn_purge", "delivery_delete"] {
            sqlx::query(
                "INSERT INTO media.object_cleanup_steps \
                   (deletion_job_id, step_kind, object_key) VALUES ($1, $2, $3) \
                 ON CONFLICT (deletion_job_id, step_kind, object_key) DO UPDATE \
                 SET status = 'queued', attempt_count = 0, available_at = now(), \
                     lease_token = NULL, lease_expires_at = NULL, last_error_code = NULL, \
                     provider_task_id = NULL, provider_task_submitted_at = NULL, \
                     completed_at = NULL, updated_at = now()",
            )
            .bind(deletion_job_id)
            .bind(step_kind)
            .bind(&variant.object_key)
            .execute(&mut *connection)
            .await?;
        }
    }
    Ok(())
}

async fn complete_processing_job(
    pool: &PgPool,
    job: &ClaimedProcessingJob,
    variants: &[RenderedVariant],
) -> AppResult<bool> {
    let mut transaction = pool.begin().await?;
    let upload_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(job.asset_id)
            .fetch_optional(&mut *transaction)
            .await?;
    if upload_status.as_deref() != Some("clean") {
        enqueue_late_variant_cleanup(&mut transaction, job.asset_id, variants).await?;
        let affected = sqlx::query(
            "UPDATE media.variant_processing_jobs \
             SET status = 'dead_letter', lease_token = NULL, lease_expires_at = NULL, \
                 last_error_code = 'asset_left_clean_state', updated_at = now() \
             WHERE id = $1 AND status = 'leased' AND lease_token = $2",
        )
        .bind(job.id)
        .bind(job.lease_token)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        let is_already_cancelled = if affected == 0 {
            sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM media.variant_processing_jobs \
                 WHERE id = $1 AND status = 'dead_letter' \
                   AND last_error_code = 'asset_left_clean_state')",
            )
            .bind(job.id)
            .fetch_one(&mut *transaction)
            .await?
        } else {
            false
        };
        if affected != 1 && !is_already_cancelled {
            return Err(AppError::Conflict("media processing lease was lost".into()));
        }
        transaction.commit().await?;
        return Ok(false);
    }
    for variant in variants {
        let affected = sqlx::query(
            "UPDATE media.asset_variants SET status = 'published', published_at = now() \
             WHERE asset_id = $1 AND policy_version = $2 AND variant_kind = $3 \
               AND status = 'processing' AND object_key = $4 AND content_sha256 = $5",
        )
        .bind(job.asset_id)
        .bind(job.policy_version)
        .bind(variant.variant_kind)
        .bind(&variant.object_key)
        .bind(&variant.content_sha256)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if affected != 1 {
            return Err(AppError::Conflict("media variant publication changed".into()));
        }
    }
    let publication_affected = sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'published', published_at = now(), blocked_at = NULL, \
             last_error_code = NULL, updated_at = now() \
         WHERE asset_id = $1 AND policy_version = $2 AND status = 'processing'",
    )
    .bind(job.asset_id)
    .bind(job.policy_version)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if publication_affected != 1 {
        return Err(AppError::Conflict("media publication state changed".into()));
    }
    let job_affected = sqlx::query(
        "UPDATE media.variant_processing_jobs \
         SET status = 'succeeded', lease_token = NULL, lease_expires_at = NULL, \
             completed_at = now(), last_error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if job_affected != 1 {
        return Err(AppError::Conflict("media processing lease was lost".into()));
    }
    governance::record_system_event_tx(
        &mut transaction,
        "media.asset.published",
        "upload",
        &job.asset_id.to_string(),
        "sanitized media variants published to private Delivery storage",
        Some(&serde_json::json!({
            "policyVersion": job.policy_version,
            "variantCount": variants.len(),
        })),
    )
    .await?;
    transaction.commit().await?;
    Ok(true)
}

async fn record_processing_failure(
    pool: &PgPool,
    job: &ClaimedProcessingJob,
    error_code: &str,
) -> AppResult<()> {
    let is_dead_letter = job.attempt_count >= MAX_PROCESSING_ATTEMPTS;
    let retry_seconds =
        i64::from(15 * 2_i32.pow(job.attempt_count.clamp(1, 8) as u32 - 1)).min(3_600);
    let mut transaction = pool.begin().await?;
    let affected = sqlx::query(
        "UPDATE media.variant_processing_jobs \
         SET status = CASE WHEN $3 THEN 'dead_letter' ELSE 'queued' END, \
             available_at = CASE WHEN $3 THEN available_at \
                                 ELSE now() + ($4 * interval '1 second') END, \
             lease_token = NULL, lease_expires_at = NULL, last_error_code = $5, \
             updated_at = now() \
         WHERE id = $1 AND status = 'leased' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(is_dead_letter)
    .bind(retry_seconds)
    .bind(error_code)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("media processing lease was lost".into()));
    }
    if is_dead_letter {
        sqlx::query(
            "UPDATE media.asset_publications \
             SET status = 'failed', published_at = NULL, last_error_code = $2, \
                 updated_at = now() \
             WHERE asset_id = $1 AND status = 'processing'",
        )
        .bind(job.asset_id)
        .bind(error_code)
        .execute(&mut *transaction)
        .await?;
        governance::record_system_event_tx(
            &mut transaction,
            "media.asset.processing_dead_lettered",
            "upload",
            &job.asset_id.to_string(),
            "media variant processing exhausted its bounded retry budget",
            Some(&serde_json::json!({ "errorCode": error_code })),
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

async fn process_claimed_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
    job: &ClaimedProcessingJob,
) -> Result<(), ProcessingFailure> {
    let source = object_store
        .read_image_for_processing(
            &job.oss_key,
            &job.mime,
            u64::try_from(job.bytes).map_err(|error| {
                ProcessingFailure::new("invalid_source_length", AppError::Internal(error.into()))
            })?,
            oss::OSS_UPLOAD_MAX_BYTES as u64,
        )
        .await
        .map_err(|error| ProcessingFailure::new("ingest_read_failed", error))?;
    let observed_sha256 = hex::encode(Sha256::digest(&source));
    if observed_sha256 != job.source_sha256.to_ascii_lowercase() {
        return Err(ProcessingFailure::new(
            "source_digest_mismatch",
            AppError::BadRequest("media source digest does not match callback evidence".into()),
        ));
    }
    let asset_id = job.asset_id;
    let policy_version = job.policy_version;
    let mime = job.mime.clone();
    let variants = tokio::task::spawn_blocking(move || {
        render_static_variants(asset_id, policy_version, &mime, &source)
    })
    .await
    .map_err(|error| {
        ProcessingFailure::new("image_worker_join_failed", AppError::Internal(error.into()))
    })?
    .map_err(|error| ProcessingFailure::new("image_decode_rejected", error))?;
    let should_publish = register_processing_variants(pool, job, &variants)
        .await
        .map_err(|error| ProcessingFailure::new("variant_registration_failed", error))?;
    if !should_publish {
        return Ok(());
    }
    for variant in &variants {
        object_store
            .put_delivery_object(&variant.object_key, "image/webp", variant.bytes.clone())
            .await
            .map_err(|error| ProcessingFailure::new("delivery_write_failed", error))?;
        object_store
            .head_delivery_object(
                &variant.object_key,
                "image/webp",
                variant.bytes.len() as u64,
                &variant.content_sha256,
            )
            .await
            .map_err(|error| ProcessingFailure::new("delivery_verification_failed", error))?;
    }
    let _was_published = complete_processing_job(pool, job, &variants)
        .await
        .map_err(|error| ProcessingFailure::new("publication_commit_failed", error))?;
    Ok(())
}

/// Process at most one durable variant job without holding a database lock during provider I/O.
pub async fn process_one_variant_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
) -> AppResult<bool> {
    process_variant_job(pool, object_store, None).await
}

/// Process one queued variant job for a known upload, used by bounded operational tests/retries.
pub async fn process_upload_variant_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
    asset_id: i64,
) -> AppResult<bool> {
    process_variant_job(pool, object_store, Some(asset_id)).await
}

async fn process_variant_job(
    pool: &PgPool,
    object_store: &dyn UploadObjectStore,
    asset_id: Option<i64>,
) -> AppResult<bool> {
    let Some(job) = claim_processing_job(pool, asset_id).await? else {
        return Ok(false);
    };
    if let Err(failure) = process_claimed_job(pool, object_store, &job).await {
        tracing::warn!(
            error_code = failure.code,
            ?failure.error,
            asset_id = job.asset_id,
            attempt = job.attempt_count,
            "media variant processing failed"
        );
        record_processing_failure(pool, &job, failure.code).await?;
    }
    Ok(true)
}

fn render_static_variants(
    asset_id: i64,
    policy_version: i32,
    expected_mime: &str,
    source: &[u8],
) -> AppResult<Vec<RenderedVariant>> {
    if source.is_empty() || source.len() > oss::OSS_UPLOAD_MAX_BYTES as usize {
        return Err(AppError::BadRequest("image source exceeds processing limits".into()));
    }
    reject_animated_container(expected_mime, source)?;
    let expected_format = match expected_mime {
        "image/jpeg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        "image/webp" => ImageFormat::WebP,
        _ => return Err(AppError::BadRequest("unsupported static image format".into())),
    };
    if image::guess_format(source).ok() != Some(expected_format) {
        return Err(AppError::BadRequest("image magic does not match callback MIME".into()));
    }
    let mut reader = ImageReader::with_format(Cursor::new(source), expected_format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| AppError::BadRequest("image decoder rejected source".into()))?;
    let orientation = decoder
        .orientation()
        .map_err(|_| AppError::BadRequest("image orientation metadata is invalid".into()))?;
    let (source_width, source_height) = decoder.dimensions();
    validate_decoded_dimensions(source_width, source_height)?;
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|_| AppError::BadRequest("image pixels could not be decoded".into()))?;
    image.apply_orientation(orientation);

    VARIANT_SPECS
        .into_iter()
        .map(|(variant_kind, maximum_dimension)| {
            let variant_image =
                if image.width() > maximum_dimension || image.height() > maximum_dimension {
                    image.resize(maximum_dimension, maximum_dimension, FilterType::Lanczos3)
                } else {
                    image.clone()
                };
            let rgba = variant_image.to_rgba8();
            let mut encoded = Vec::new();
            WebPEncoder::new_lossless(&mut encoded)
                .write_image(rgba.as_raw(), rgba.width(), rgba.height(), ExtendedColorType::Rgba8)
                .map_err(|_| AppError::Internal(anyhow::anyhow!("WebP encoding failed")))?;
            if encoded.is_empty() || encoded.len() > oss::OSS_UPLOAD_MAX_BYTES as usize {
                return Err(AppError::BadRequest(
                    "sanitized image variant exceeds storage limit".into(),
                ));
            }
            let content_sha256 = hex::encode(Sha256::digest(&encoded));
            let object_key =
                format!("assets/{asset_id}/{policy_version}/{variant_kind}-{content_sha256}.webp");
            Ok(RenderedVariant {
                variant_kind,
                object_key,
                content_sha256,
                bytes: encoded,
                width: i32::try_from(rgba.width())
                    .map_err(|error| AppError::Internal(error.into()))?,
                height: i32::try_from(rgba.height())
                    .map_err(|error| AppError::Internal(error.into()))?,
            })
        })
        .collect()
}

fn validate_decoded_dimensions(width: u32, height: u32) -> AppResult<()> {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0
        || height == 0
        || width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || pixels > MAX_IMAGE_PIXELS
    {
        return Err(AppError::BadRequest("image dimensions exceed processing limits".into()));
    }
    Ok(())
}

fn reject_animated_container(expected_mime: &str, source: &[u8]) -> AppResult<()> {
    let animation_marker = match expected_mime {
        "image/png" => png_contains_animation_chunk(source)?,
        "image/webp" => webp_contains_animation_chunk(source)?,
        _ => return Ok(()),
    };
    if animation_marker {
        Err(AppError::BadRequest(
            "animated images are not accepted by the static media pipeline".into(),
        ))
    } else {
        Ok(())
    }
}

fn png_contains_animation_chunk(source: &[u8]) -> AppResult<bool> {
    if !source.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(false);
    }
    let mut cursor = 8_usize;
    while cursor < source.len() {
        let header_end = cursor
            .checked_add(8)
            .ok_or_else(|| AppError::BadRequest("invalid PNG chunk length".into()))?;
        if header_end > source.len() {
            return Err(AppError::BadRequest("truncated PNG chunk header".into()));
        }
        let chunk_length = u32::from_be_bytes([
            source[cursor],
            source[cursor + 1],
            source[cursor + 2],
            source[cursor + 3],
        ]) as usize;
        let chunk_kind = &source[cursor + 4..header_end];
        let chunk_end = header_end
            .checked_add(chunk_length)
            .and_then(|data_end| data_end.checked_add(4))
            .ok_or_else(|| AppError::BadRequest("invalid PNG chunk length".into()))?;
        if chunk_end > source.len() {
            return Err(AppError::BadRequest("truncated PNG chunk body".into()));
        }
        if chunk_kind == b"acTL" {
            return Ok(true);
        }
        cursor = chunk_end;
        if chunk_kind == b"IEND" {
            return Ok(false);
        }
    }
    Err(AppError::BadRequest("PNG end chunk is missing".into()))
}

fn webp_contains_animation_chunk(source: &[u8]) -> AppResult<bool> {
    if source.len() < 12 || &source[..4] != b"RIFF" || &source[8..12] != b"WEBP" {
        return Ok(false);
    }
    let riff_bytes = u32::from_le_bytes([source[4], source[5], source[6], source[7]]) as usize;
    let container_end = riff_bytes
        .checked_add(8)
        .ok_or_else(|| AppError::BadRequest("invalid WebP container length".into()))?;
    if container_end != source.len() {
        return Err(AppError::BadRequest("WebP container length mismatch".into()));
    }
    let mut cursor = 12_usize;
    while cursor < container_end {
        let header_end = cursor
            .checked_add(8)
            .ok_or_else(|| AppError::BadRequest("invalid WebP chunk length".into()))?;
        if header_end > container_end {
            return Err(AppError::BadRequest("truncated WebP chunk header".into()));
        }
        let chunk_kind = &source[cursor..cursor + 4];
        let chunk_length = u32::from_le_bytes([
            source[cursor + 4],
            source[cursor + 5],
            source[cursor + 6],
            source[cursor + 7],
        ]) as usize;
        let chunk_end = header_end
            .checked_add(chunk_length)
            .and_then(|data_end| data_end.checked_add(chunk_length % 2))
            .ok_or_else(|| AppError::BadRequest("invalid WebP chunk length".into()))?;
        if chunk_end > container_end {
            return Err(AppError::BadRequest("truncated WebP chunk body".into()));
        }
        if chunk_kind == b"ANIM" {
            return Ok(true);
        }
        cursor = chunk_end;
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};

    use super::{
        png_contains_animation_chunk, render_static_variants, validate_decoded_dimensions,
        webp_contains_animation_chunk,
    };

    fn jpeg_fixture() -> Vec<u8> {
        let image = RgbaImage::from_pixel(32, 16, Rgba([20, 80, 140, 255]));
        let rgb = image::DynamicImage::ImageRgba8(image).to_rgb8();
        let mut encoded = Vec::new();
        JpegEncoder::new_with_quality(&mut encoded, 90)
            .write_image(rgb.as_raw(), rgb.width(), rgb.height(), ExtendedColorType::Rgb8)
            .expect("encode JPEG fixture");
        encoded
    }

    #[test]
    fn static_variants_are_deterministic_content_addressed_webp() {
        let source = jpeg_fixture();
        let first = render_static_variants(42, 1, "image/jpeg", &source)
            .expect("first deterministic render");
        let second = render_static_variants(42, 1, "image/jpeg", &source)
            .expect("second deterministic render");
        assert_eq!(first.len(), 3);
        for (left, right) in first.iter().zip(second.iter()) {
            assert_eq!(left.object_key, right.object_key);
            assert_eq!(left.content_sha256, right.content_sha256);
            assert_eq!(left.bytes, right.bytes);
            assert!(left.object_key.starts_with("assets/42/1/"));
            assert!(!left.bytes.windows(6).any(|window| window == b"Exif\0\0"));
        }
    }

    #[test]
    fn animated_and_mime_mismatched_inputs_fail_closed() {
        let mut animated_png = b"\x89PNG\r\n\x1a\n".to_vec();
        append_png_chunk(&mut animated_png, b"acTL", &[0; 8]);
        assert!(render_static_variants(1, 1, "image/png", &animated_png).is_err());
        assert!(render_static_variants(1, 1, "image/webp", &jpeg_fixture()).is_err());
    }

    #[test]
    fn animation_detection_parses_container_chunks_instead_of_pixel_bytes() {
        let mut static_png = b"\x89PNG\r\n\x1a\n".to_vec();
        append_png_chunk(&mut static_png, b"IDAT", b"pixel-acTL-bytes");
        append_png_chunk(&mut static_png, b"IEND", &[]);
        assert!(!png_contains_animation_chunk(&static_png).expect("static PNG chunks"));
        let mut animated_png = b"\x89PNG\r\n\x1a\n".to_vec();
        append_png_chunk(&mut animated_png, b"acTL", &[0; 8]);
        append_png_chunk(&mut animated_png, b"IEND", &[]);
        assert!(png_contains_animation_chunk(&animated_png).expect("animated PNG chunks"));

        let static_webp = webp_chunks(&[(b"VP8 ", b"pixel-ANIM-bytes")]);
        assert!(!webp_contains_animation_chunk(&static_webp).expect("static WebP chunks"));
        let animated_webp = webp_chunks(&[(b"ANIM", &[0; 6])]);
        assert!(webp_contains_animation_chunk(&animated_webp).expect("animated WebP chunks"));
    }

    fn append_png_chunk(target: &mut Vec<u8>, chunk_kind: &[u8; 4], data: &[u8]) {
        target.extend_from_slice(&(data.len() as u32).to_be_bytes());
        target.extend_from_slice(chunk_kind);
        target.extend_from_slice(data);
        target.extend_from_slice(&[0; 4]);
    }

    fn webp_chunks(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut body = b"WEBP".to_vec();
        for (chunk_kind, data) in chunks {
            body.extend_from_slice(*chunk_kind);
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(data);
            if data.len() % 2 == 1 {
                body.push(0);
            }
        }
        let mut container = b"RIFF".to_vec();
        container.extend_from_slice(&(body.len() as u32).to_le_bytes());
        container.extend_from_slice(&body);
        container
    }

    #[test]
    fn pixel_bombs_are_rejected_before_allocation() {
        assert!(validate_decoded_dimensions(20_000, 20_000).is_err());
        assert!(validate_decoded_dimensions(8_000, 5_000).is_ok());
        assert!(validate_decoded_dimensions(8_001, 5_000).is_err());
    }
}

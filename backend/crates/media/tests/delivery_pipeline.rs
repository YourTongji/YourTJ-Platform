//! End-to-end database state-machine coverage with an in-process fake media provider.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use media::{
    is_clean_image_owned_by, process_upload_deletion_job, process_upload_variant_job,
    resolve_clean_image_delivery, validate_delivery_runtime, DeliveryPurgeTaskState, ImageVariant,
    UploadObjectStore,
};
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult};
use sqlx::PgPool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

#[derive(Clone)]
struct FakeMediaProvider {
    source: Vec<u8>,
    objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    events: Arc<Mutex<Vec<String>>>,
    purge_tasks: Arc<Mutex<HashMap<String, String>>>,
    block_on_head: Option<Arc<BlockOnHead>>,
}

struct BlockOnHead {
    pool: PgPool,
    asset_id: i64,
    triggered: AtomicBool,
    drain_cleanup_before_resume: bool,
}

impl BlockOnHead {
    async fn trigger_once(&self) -> AppResult<bool> {
        if self.triggered.swap(true, Ordering::SeqCst) {
            return Ok(false);
        }
        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE media.uploads SET status = 'quarantined' WHERE id = $1")
            .bind(self.asset_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "INSERT INTO media.object_deletion_jobs \
             (upload_id, requested_by, requested_role, request_source, reason, previous_status) \
             VALUES ($1, NULL, NULL, 'retention_gc', \
                     'simulate block during Delivery processing', 'clean')",
        )
        .bind(self.asset_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(true)
    }
}

impl FakeMediaProvider {
    fn events(&self) -> Vec<String> {
        self.events.lock().expect("fake event lock").clone()
    }
}

#[async_trait::async_trait]
impl UploadObjectStore for FakeMediaProvider {
    async fn delete_object(&self, oss_key: &str) -> AppResult<()> {
        self.events.lock().expect("fake event lock").push(format!("ingest:{oss_key}"));
        Ok(())
    }

    async fn read_image_for_processing(
        &self,
        _oss_key: &str,
        expected_content_type: &str,
        expected_bytes: u64,
        max_bytes: u64,
    ) -> AppResult<Vec<u8>> {
        assert_eq!(expected_content_type, "image/jpeg");
        assert_eq!(expected_bytes, self.source.len() as u64);
        assert!(expected_bytes <= max_bytes);
        self.events.lock().expect("fake event lock").push("ingest:read".into());
        Ok(self.source.clone())
    }

    async fn put_delivery_object(
        &self,
        object_key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> AppResult<()> {
        assert_eq!(content_type, "image/webp");
        self.objects.lock().expect("fake object lock").insert(object_key.to_owned(), bytes);
        self.events.lock().expect("fake event lock").push(format!("put:{object_key}"));
        Ok(())
    }

    async fn head_delivery_object(
        &self,
        object_key: &str,
        content_type: &str,
        expected_bytes: u64,
        expected_sha256: &str,
    ) -> AppResult<()> {
        assert_eq!(content_type, "image/webp");
        let matches = self
            .objects
            .lock()
            .expect("fake object lock")
            .get(object_key)
            .is_some_and(|bytes| bytes.len() as u64 == expected_bytes);
        if !matches {
            return Err(AppError::Internal(anyhow::anyhow!("fake Delivery HEAD mismatch")));
        }
        let observed_sha256 = {
            let objects = self.objects.lock().expect("fake object lock");
            let bytes = objects
                .get(object_key)
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("fake object missing")))?;
            hex::encode(Sha256::digest(bytes))
        };
        assert_eq!(observed_sha256, expected_sha256);
        self.events.lock().expect("fake event lock").push(format!("head:{object_key}"));
        if let Some(block_on_head) = &self.block_on_head {
            if block_on_head.trigger_once().await? {
                if block_on_head.drain_cleanup_before_resume {
                    sqlx::query(
                        "UPDATE media.variant_processing_jobs \
                         SET lease_expires_at = now() - interval '1 second' \
                         WHERE asset_id = $1 AND status = 'leased'",
                    )
                    .bind(block_on_head.asset_id)
                    .execute(&block_on_head.pool)
                    .await?;
                    for _ in 0..12 {
                        make_cleanup_ready(&block_on_head.pool, block_on_head.asset_id).await;
                        if !process_upload_deletion_job(
                            &block_on_head.pool,
                            self,
                            block_on_head.asset_id,
                        )
                        .await?
                        {
                            break;
                        }
                    }
                } else if process_upload_deletion_job(
                    &block_on_head.pool,
                    self,
                    block_on_head.asset_id,
                )
                .await?
                {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "cleanup crossed an active processing lease"
                    )));
                }
            }
        }
        Ok(())
    }

    async fn delete_delivery_object(&self, object_key: &str) -> AppResult<()> {
        self.objects.lock().expect("fake object lock").remove(object_key);
        self.events.lock().expect("fake event lock").push(format!("delete:{object_key}"));
        Ok(())
    }

    async fn submit_delivery_purge(&self, object_key: &str) -> AppResult<String> {
        let task_id = format!("{}", self.purge_tasks.lock().expect("purge task lock").len() + 1);
        self.purge_tasks
            .lock()
            .expect("purge task lock")
            .insert(task_id.clone(), object_key.to_owned());
        Ok(task_id)
    }

    async fn delivery_purge_task_state(
        &self,
        provider_task_id: &str,
    ) -> AppResult<DeliveryPurgeTaskState> {
        let object_key = self
            .purge_tasks
            .lock()
            .expect("purge task lock")
            .get(provider_task_id)
            .cloned()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("fake purge task missing")))?;
        self.events.lock().expect("fake event lock").push(format!("purge:{object_key}"));
        Ok(DeliveryPurgeTaskState::Complete)
    }
}

fn jpeg_fixture() -> Vec<u8> {
    let image = RgbImage::from_pixel(96, 48, Rgb([20, 90, 180]));
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 90)
        .write_image(image.as_raw(), image.width(), image.height(), ExtendedColorType::Rgb8)
        .expect("encode media integration fixture");
    bytes
}

async fn make_cleanup_ready(pool: &PgPool, asset_id: i64) {
    sqlx::query("UPDATE media.object_deletion_jobs SET available_at = now() WHERE upload_id = $1")
        .bind(asset_id)
        .execute(pool)
        .await
        .expect("advance fake deletion job");
    sqlx::query(
        "UPDATE media.object_cleanup_steps step SET available_at = now() \
         FROM media.object_deletion_jobs job \
         WHERE job.upload_id = $1 AND step.deletion_job_id = job.id",
    )
    .bind(asset_id)
    .execute(pool)
    .await
    .expect("advance fake cleanup steps");
}

#[tokio::test]
async fn clean_asset_publishes_atomically_then_block_purges_delivery_before_ingest() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL for media delivery integration");
    let pool = PgPool::connect(&database_url).await.expect("media delivery test database");
    MIGRATOR.run(&pool).await.expect("media delivery migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("media-delivery-owner-{suffix}@tongji.edu.cn"))
    .bind(format!("media-delivery-owner-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert media delivery owner");
    let moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("media-delivery-mod-{suffix}@tongji.edu.cn"))
    .bind(format!("media-delivery-mod-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert media delivery moderator");
    let source = jpeg_fixture();
    let source_digest = hex::encode(Sha256::digest(&source));
    let ingest_key = format!("uploads/{owner_id}/image/{suffix}.jpg");
    let asset_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads (account_id, kind, oss_key, url, bytes, mime, sha256, status) VALUES ($1, 'image', $2, '', $3, 'image/jpeg', $4, 'clean') RETURNING id",
    )
    .bind(owner_id)
    .bind(&ingest_key)
    .bind(source.len() as i64)
    .bind(source_digest)
    .fetch_one(&pool)
    .await
    .expect("insert clean media source");
    sqlx::query("UPDATE media.asset_publications SET status = 'processing' WHERE asset_id = $1")
        .bind(asset_id)
        .execute(&pool)
        .await
        .expect("prepare media publication");
    sqlx::query(
        "INSERT INTO media.variant_processing_jobs (asset_id, policy_version) VALUES ($1, 1)",
    )
    .bind(asset_id)
    .execute(&pool)
    .await
    .expect("enqueue media processing");
    let provider = FakeMediaProvider {
        source: source.clone(),
        objects: Arc::new(Mutex::new(HashMap::new())),
        events: Arc::new(Mutex::new(Vec::new())),
        purge_tasks: Arc::new(Mutex::new(HashMap::new())),
        block_on_head: None,
    };

    assert!(process_upload_variant_job(&pool, &provider, asset_id)
        .await
        .expect("process sanitized variants"));
    let (publication_status, variant_count, job_status): (String, i64, String) =
        sqlx::query_as(
            "SELECT publication.status, (SELECT count(*)::bigint FROM media.asset_variants variant WHERE variant.asset_id = publication.asset_id AND variant.status = 'published'), job.status FROM media.asset_publications publication JOIN media.variant_processing_jobs job ON job.asset_id = publication.asset_id WHERE publication.asset_id = $1",
        )
        .bind(asset_id)
        .fetch_one(&pool)
        .await
        .expect("published media state");
    assert_eq!(publication_status, "published");
    assert_eq!(variant_count, 3);
    assert_eq!(job_status, "succeeded");
    assert!(is_clean_image_owned_by(&pool, asset_id, owner_id)
        .await
        .expect("published media eligibility"));
    for variable in [
        "MEDIA_DELIVERY_OSS_BUCKET",
        "MEDIA_DELIVERY_OSS_ACCESS_KEY_ID",
        "MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET",
        "MEDIA_CDN_BASE_URL",
        "MEDIA_CDN_PRIMARY_KEY",
        "MEDIA_CDN_SECONDARY_KEY",
        "MEDIA_CDN_SIGNING_KEY_SLOT",
        "MEDIA_CDN_URL_TTL_SECONDS",
        "CDN_ACCESS_KEY_ID",
        "CDN_ACCESS_KEY_SECRET",
    ] {
        std::env::remove_var(variable);
    }
    std::env::set_var("OSS_REGION", "cn-shanghai");
    let mut runtime_config = shared::Config::from_env().expect("media runtime test config");
    runtime_config.oss_region = "cn-shanghai".into();
    runtime_config.oss_bucket = "yourtj-ingest-test".into();
    runtime_config.oss_access_key_id = "ingesttestak".into();
    assert!(validate_delivery_runtime(&runtime_config).is_ok());

    std::env::set_var("MEDIA_DELIVERY_OSS_BUCKET", "yourtj-delivery-test");
    assert!(validate_delivery_runtime(&runtime_config).is_err());
    std::env::set_var("MEDIA_DELIVERY_OSS_ACCESS_KEY_ID", "deliverytestak");
    std::env::set_var("MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET", "deliverytestsecret");
    std::env::set_var("MEDIA_CDN_BASE_URL", "https://media.example.test");
    std::env::set_var("MEDIA_CDN_PRIMARY_KEY", "primarytestkey");
    std::env::set_var("MEDIA_CDN_SECONDARY_KEY", "secondarytestkey");
    std::env::set_var("MEDIA_CDN_SIGNING_KEY_SLOT", "primary");
    std::env::set_var("MEDIA_CDN_URL_TTL_SECONDS", "300");
    std::env::set_var("CDN_ACCESS_KEY_ID", "cdnpurgetestak");
    std::env::set_var("CDN_ACCESS_KEY_SECRET", "cdnpurgetestsecret");
    assert!(validate_delivery_runtime(&runtime_config).is_ok());
    let mut shared_bucket = runtime_config.clone();
    shared_bucket.oss_bucket = "yourtj-delivery-test".into();
    assert!(validate_delivery_runtime(&shared_bucket).is_err());
    let mut shared_identity = runtime_config.clone();
    shared_identity.oss_access_key_id = "deliverytestak".into();
    assert!(validate_delivery_runtime(&shared_identity).is_err());
    let delivery = resolve_clean_image_delivery(&pool, Some(asset_id))
        .await
        .expect("resolve typed Delivery projection")
        .expect("published Delivery projection");
    assert_eq!(delivery.asset_id, asset_id.to_string());
    assert_eq!(delivery.mime, "image/webp");
    assert_eq!((delivery.width, delivery.height), (96, 48));
    assert_eq!(delivery.variant, ImageVariant::Display1280);
    assert!(delivery.url.starts_with("https://media.example.test/assets/"));
    assert!((290..=300).contains(&(delivery.expires_at - chrono::Utc::now().timestamp())));

    let race_ingest_key = format!("uploads/{owner_id}/image/{suffix}-race.jpg");
    let race_asset_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, '', $3, 'image/jpeg', $4, 'clean') RETURNING id",
    )
    .bind(owner_id)
    .bind(race_ingest_key)
    .bind(source.len() as i64)
    .bind(hex::encode(Sha256::digest(&source)))
    .fetch_one(&pool)
    .await
    .expect("insert processing-race media source");
    sqlx::query("UPDATE media.asset_publications SET status = 'processing' WHERE asset_id = $1")
        .bind(race_asset_id)
        .execute(&pool)
        .await
        .expect("prepare processing-race publication");
    sqlx::query(
        "INSERT INTO media.variant_processing_jobs (asset_id, policy_version) VALUES ($1, 1)",
    )
    .bind(race_asset_id)
    .execute(&pool)
    .await
    .expect("enqueue processing-race job");
    let race_provider = FakeMediaProvider {
        source: source.clone(),
        objects: Arc::new(Mutex::new(HashMap::new())),
        events: Arc::new(Mutex::new(Vec::new())),
        purge_tasks: Arc::new(Mutex::new(HashMap::new())),
        block_on_head: Some(Arc::new(BlockOnHead {
            pool: pool.clone(),
            asset_id: race_asset_id,
            triggered: AtomicBool::new(false),
            drain_cleanup_before_resume: true,
        })),
    };
    assert!(process_upload_variant_job(&pool, &race_provider, race_asset_id)
        .await
        .expect("process Delivery publication race"));
    let race_state: (String, String, String, Option<String>, i64) = sqlx::query_as(
        "SELECT upload.status, publication.status, job.status, job.last_error_code, \
                (SELECT count(*)::bigint FROM media.object_cleanup_steps step \
                 JOIN media.object_deletion_jobs deletion \
                   ON deletion.id = step.deletion_job_id \
                 WHERE deletion.upload_id = upload.id) \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         JOIN media.variant_processing_jobs job ON job.asset_id = upload.id \
         WHERE upload.id = $1",
    )
    .bind(race_asset_id)
    .fetch_one(&pool)
    .await
    .expect("processing-race fail-closed state");
    assert_eq!(
        race_state,
        (
            "blocked".into(),
            "blocked".into(),
            "dead_letter".into(),
            Some("asset_left_clean_state".into()),
            6,
        )
    );
    for _ in 0..9 {
        make_cleanup_ready(&pool, race_asset_id).await;
        assert!(process_upload_deletion_job(&pool, &race_provider, race_asset_id)
            .await
            .expect("clean late Delivery race object"));
    }
    assert!(race_provider.objects.lock().expect("race object lock").is_empty());

    let mut transaction = pool.begin().await.expect("begin media block fixture");
    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, expires_at) \
         VALUES ($1, 'moderation', 'preserve private source evidence', $2, \
                 now() + interval '1 day')",
    )
    .bind(asset_id)
    .bind(moderator_id)
    .execute(&mut *transaction)
    .await
    .expect("hold private Ingest evidence");
    sqlx::query("UPDATE media.uploads SET status = 'quarantined' WHERE id = $1")
        .bind(asset_id)
        .execute(&mut *transaction)
        .await
        .expect("quarantine published media");
    let deletion_job_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.object_deletion_jobs (upload_id, requested_by, requested_role, request_source, reason, previous_status) VALUES ($1, $2, 'mod', 'moderation', 'integration block', 'clean') RETURNING id",
    )
    .bind(asset_id)
    .bind(moderator_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("enqueue published media deletion");
    transaction.commit().await.expect("commit media block fixture");
    let cleanup_step_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.object_cleanup_steps WHERE deletion_job_id = $1",
    )
    .bind(deletion_job_id)
    .fetch_one(&pool)
    .await
    .expect("trigger-created complete cleanup plan");
    assert_eq!(cleanup_step_count, 7);
    let legacy_completion =
        sqlx::query("UPDATE media.object_deletion_jobs SET status = 'succeeded' WHERE id = $1")
            .bind(deletion_job_id)
            .execute(&pool)
            .await;
    assert!(legacy_completion.is_err());

    for _ in 0..9 {
        make_cleanup_ready(&pool, asset_id).await;
        assert!(process_upload_deletion_job(&pool, &provider, asset_id)
            .await
            .expect("process one media cleanup step"));
    }
    assert!(!process_upload_deletion_job(&pool, &provider, asset_id)
        .await
        .expect("active hold defers only Ingest deletion"));
    assert!(provider.objects.lock().expect("fake object lock").is_empty());
    assert!(!provider.events().iter().any(|event| event == &format!("ingest:{ingest_key}")));
    sqlx::query(
        "UPDATE media.asset_retention_holds \
         SET released_at = now(), released_by = $2, release_reason = 'evidence copied' \
         WHERE asset_id = $1 AND released_at IS NULL",
    )
    .bind(asset_id)
    .bind(moderator_id)
    .execute(&pool)
    .await
    .expect("release Ingest evidence hold");
    assert!(process_upload_deletion_job(&pool, &provider, asset_id)
        .await
        .expect("delete Ingest after hold release"));
    let upload_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .expect("blocked upload state");
    assert_eq!(upload_status, "blocked");
    let terminal_state: (String, String, i64, i64, i64) = sqlx::query_as(
        "SELECT publication.status, deletion.status, \
                (SELECT count(*)::bigint FROM media.asset_variants \
                 WHERE asset_id = upload.id), \
                (SELECT count(*)::bigint FROM media.object_cleanup_steps \
                 WHERE deletion_job_id = deletion.id), \
                (SELECT count(*)::bigint FROM governance.audit_events \
                 WHERE action = 'media.upload.blocked' AND target_type = 'upload' \
                   AND target_id = upload.id::text) \
         FROM media.uploads upload \
         JOIN media.asset_publications publication ON publication.asset_id = upload.id \
         JOIN media.object_deletion_jobs deletion ON deletion.upload_id = upload.id \
         WHERE upload.id = $1",
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("terminal media deletion state");
    assert_eq!(terminal_state, ("blocked".into(), "succeeded".into(), 0, 0, 1));
    assert!(resolve_clean_image_delivery(&pool, Some(asset_id))
        .await
        .expect("resolve blocked Delivery projection")
        .is_none());
    let events = provider.events();
    let ingress_position = events
        .iter()
        .position(|event| event == &format!("ingest:{ingest_key}"))
        .expect("Ingest deletion event");
    for event in events.iter().filter(|event| event.starts_with("delete:")) {
        let key = event.trim_start_matches("delete:");
        let purge_position = events
            .iter()
            .position(|candidate| candidate == &format!("purge:{key}"))
            .expect("matching CDN purge event");
        let delete_position = events
            .iter()
            .position(|candidate| candidate == event)
            .expect("Delivery deletion event");
        assert!(purge_position < delete_position);
        assert!(delete_position < ingress_position);
    }
    assert!(provider.objects.lock().expect("fake object lock").is_empty());
}

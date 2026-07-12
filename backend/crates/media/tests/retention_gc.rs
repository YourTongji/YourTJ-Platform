//! Database integration coverage for media retention, binding, and provider-deletion fencing.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use media::{
    prepare_account_media_purge, process_upload_deletion_job, purge_completed_cleanup_tombstones,
    purge_expired_asset_bindings, purge_expired_preview_grants, purge_upload_credential_attempts,
    reserve_upload_intent, routes_with_object_store, schedule_expired_upload_intent_cleanup_batch,
    schedule_retention_gc_batch, UploadObjectStore,
};
use sha2::{Digest, Sha256};
use shared::{AppResult, AppState};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Notify;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
const JWT_SECRET: &str = "integration-test-secret-32bytes!";

struct SuccessfulObjectStore;

#[async_trait::async_trait]
impl UploadObjectStore for SuccessfulObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        Ok(())
    }
}

struct AlwaysFailingObjectStore;

#[async_trait::async_trait]
impl UploadObjectStore for AlwaysFailingObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        Err(shared::AppError::Internal(anyhow::anyhow!("object store test failure")))
    }
}

struct RecordingObjectStore {
    deleted_keys: Arc<std::sync::Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl UploadObjectStore for RecordingObjectStore {
    async fn delete_object(&self, oss_key: &str) -> AppResult<()> {
        self.deleted_keys.lock().expect("recording object store lock").push(oss_key.to_owned());
        Ok(())
    }
}

struct PausingObjectStore {
    started: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait::async_trait]
impl UploadObjectStore for PausingObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        self.started.notify_one();
        self.release.notified().await;
        Ok(())
    }
}

fn test_state(pool: PgPool) -> AppState {
    let mut config = shared::Config::from_env().expect("media retention test config");
    config.oss_region = "cn-shanghai".into();
    config.oss_bucket = "yourtj-test".into();
    config.oss_access_key_id = "test-ak".into();
    config.oss_access_key_secret = "test-secret".into();
    config.oss_role_arn = "acs:ram::1:role/upload".into();
    config.oss_callback_base_url = "https://api.example.test".into();
    AppState {
        db: pool,
        config,
        jwt_secret: JWT_SECRET.into(),
        jwt_ttl: 900,
        refresh_ttl: 604800,
        meili_url: String::new(),
        meili_master_key: String::new(),
        redis: None,
        system_private_key: vec![0; 32],
        system_public_key_b64: String::new(),
        email_encryption: None,
        captcha_verifier: None,
        sse_tx: None,
    }
}

fn request(method: Method, uri: String, token: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("media retention request")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes =
        to_bytes(response.into_body(), usize::MAX).await.expect("media retention response body");
    serde_json::from_slice(&bytes).expect("media retention response JSON")
}

async fn serialize_retention_tests(pool: &PgPool) -> Transaction<'static, Postgres> {
    let mut transaction = pool.begin().await.expect("begin retention test lock");
    sqlx::query("SELECT pg_advisory_xact_lock(8570057)")
        .execute(&mut *transaction)
        .await
        .expect("serialize retention integration tests");
    transaction
}

async fn insert_account(pool: &PgPool, suffix: &str, role: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, $3::identity.account_role) RETURNING id",
    )
    .bind(format!("media-retention-{role}-{suffix}@tongji.edu.cn"))
    .bind(format!("media-retention-{role}-{suffix}"))
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("insert media retention account")
}

async fn session_token(pool: &PgPool, account_id: i64, is_recent: bool) -> String {
    let session_id: i64 = sqlx::query_scalar(
         "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, user_agent, expires_at, \
          recent_authenticated_at, recent_auth_method, recent_auth_credential_version) \
         VALUES ($1, $2, $3, 'media-retention-test', now() + interval '1 day', \
                 CASE WHEN $4 THEN now() ELSE NULL END, \
                 CASE WHEN $4 THEN 'password' ELSE NULL END, \
                 CASE WHEN $4 THEN (SELECT credential_version FROM identity.accounts WHERE id = $1) \
                      ELSE NULL END) RETURNING id",
    )
    .bind(account_id)
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(uuid::Uuid::new_v4())
    .bind(is_recent)
    .fetch_one(pool)
    .await
    .expect("insert media retention session");
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await
            .expect("read media retention auth version");
    identity::auth::create_session_access_token(
        account_id,
        session_id,
        auth_version,
        JWT_SECRET,
        3600,
    )
    .expect("create media retention session token")
}

async fn insert_upload(
    pool: &PgPool,
    owner_id: i64,
    suffix: &str,
    label: &str,
    status: &str,
) -> i64 {
    let upload_id = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, created_at) \
         VALUES ($1, 'image', $2, $3, 10, 'image/png', $4, $5, \
                 now() - interval '40 days') RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-{label}.png"))
    .bind(format!("https://cdn.example.test/{suffix}-{label}.png"))
    .bind("a".repeat(64))
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("insert media retention upload");
    if status == "clean" {
        sqlx::query(
            "UPDATE media.uploads SET cleaned_at = now() - interval '40 days' WHERE id = $1",
        )
        .bind(upload_id)
        .execute(pool)
        .await
        .expect("age media approval for retention test");
    }
    upload_id
}

#[tokio::test]
async fn owner_export_excludes_internal_cleanup_tombstones() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = insert_account(&pool, &suffix, "user").await;
    let owner_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, '', 17, 'image/png', repeat('a', 64), 'pending') \
         RETURNING id",
    )
    .bind(account_id)
    .bind(format!("uploads/{account_id}/image/{suffix}-owner.png"))
    .fetch_one(&pool)
    .await
    .expect("insert owner-visible upload");
    let cleanup_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, is_cleanup_tombstone) \
         VALUES ($1, 'image', $2, '', 0, 'image/png', '', 'quarantined', TRUE) \
         RETURNING id",
    )
    .bind(account_id)
    .bind(format!("uploads/{account_id}/image/{suffix}-cleanup.png"))
    .fetch_one(&pool)
    .await
    .expect("insert internal cleanup tombstone");

    let uploads =
        media::data_export::snapshot(&pool, account_id).await.expect("export owner media");
    assert_eq!(uploads.len(), 1);
    let exported_upload = serde_json::to_value(&uploads[0]).expect("serialize owner upload");
    assert_eq!(exported_upload["id"], owner_upload_id);

    sqlx::query("DELETE FROM media.uploads WHERE id = ANY($1)")
        .bind(vec![owner_upload_id, cleanup_upload_id])
        .execute(&pool)
        .await
        .expect("delete media export uploads");
    sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("delete media export account");
}

#[tokio::test]
async fn retention_hold_requires_recent_operations_auth_and_fences_provider_deletion() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = insert_account(&pool, &suffix, "admin").await;
    let second_admin_id = insert_account(&pool, &format!("{suffix}-second"), "admin").await;
    let moderator_id = insert_account(&pool, &suffix, "mod").await;
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let upload_id = insert_upload(&pool, owner_id, &suffix, "held", "clean").await;
    let expired_upload_id = insert_upload(&pool, owner_id, &suffix, "expired-hold", "clean").await;
    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, created_at, expires_at) \
         VALUES ($1, 'moderation', 'expired record requires explicit operations review', $2, \
                 now() - interval '2 days', now() - interval '1 day')",
    )
    .bind(expired_upload_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("insert expired retention record");
    let admin_token = session_token(&pool, admin_id, true).await;
    let second_admin_token = session_token(&pool, second_admin_id, true).await;
    let stale_admin_token = session_token(&pool, admin_id, false).await;
    let moderator_token = session_token(&pool, moderator_id, true).await;
    let app = routes_with_object_store(test_state(pool.clone()), Arc::new(SuccessfulObjectStore));
    let hold_uri = format!("/api/v2/admin/media/uploads/{upload_id}/retention-hold");
    let expiry = chrono::Utc::now().timestamp() + 24 * 60 * 60;
    let hold_body = serde_json::json!({
        "holdKind": "security",
        "expiresAt": expiry,
        "reason": "preserve evidence for an active security investigation",
        "expectedHoldId": null,
    });

    let missing_expectation = app
        .clone()
        .oneshot(request(
            Method::POST,
            hold_uri.clone(),
            &admin_token,
            serde_json::json!({
                "holdKind": "security",
                "expiresAt": expiry,
                "reason": "missing compare and swap expectation",
            }),
        ))
        .await
        .expect("missing hold expectation response");
    assert_eq!(missing_expectation.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let unknown_field = app
        .clone()
        .oneshot(request(
            Method::POST,
            hold_uri.clone(),
            &admin_token,
            serde_json::json!({
                "holdKind": "security",
                "expiresAt": expiry,
                "reason": "unknown hold field must fail closed",
                "expectedHoldId": null,
                "unexpected": true,
            }),
        ))
        .await
        .expect("unknown hold field response");
    assert_eq!(unknown_field.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let denied = app
        .clone()
        .oneshot(request(Method::POST, hold_uri.clone(), &moderator_token, hold_body.clone()))
        .await
        .expect("moderator retention hold response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let step_up = app
        .clone()
        .oneshot(request(Method::POST, hold_uri.clone(), &stale_admin_token, hold_body.clone()))
        .await
        .expect("stale admin retention hold response");
    assert_eq!(step_up.status(), StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(response_json(step_up).await["error"]["code"], "RECENT_AUTH_REQUIRED");

    let placed = app
        .clone()
        .oneshot(request(Method::POST, hold_uri.clone(), &admin_token, hold_body.clone()))
        .await
        .expect("place retention hold response");
    assert_eq!(placed.status(), StatusCode::CREATED);
    let denied_inventory = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/retention-holds?state=active&limit=100".into(),
            &moderator_token,
            serde_json::json!({}),
        ))
        .await
        .expect("moderator retention inventory response");
    assert_eq!(denied_inventory.status(), StatusCode::FORBIDDEN);
    let stale_inventory = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/retention-holds?state=active&limit=100".into(),
            &stale_admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("stale retention inventory response");
    assert_eq!(stale_inventory.status(), StatusCode::PRECONDITION_REQUIRED);
    let inventory = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/retention-holds?state=active&limit=100".into(),
            &admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("retention inventory response");
    assert_eq!(inventory.status(), StatusCode::OK);
    assert_eq!(
        inventory.headers().get(header::CACHE_CONTROL).and_then(|value| value.to_str().ok()),
        Some("private, no-store")
    );
    let inventory_body = response_json(inventory).await;
    let first_hold_id = inventory_body["items"]
        .as_array()
        .expect("retention hold items")
        .iter()
        .find(|item| item["uploadId"].as_str() == Some(upload_id.to_string().as_str()))
        .and_then(|item| item["id"].as_str())
        .expect("placed retention hold id")
        .to_owned();
    let listed = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/uploads?status=clean&limit=100".into(),
            &admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("list held media response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = response_json(listed).await;
    let upload_id_string = upload_id.to_string();
    let item = listed_body["items"]
        .as_array()
        .expect("held media items")
        .iter()
        .find(|item| item["id"].as_str() == Some(upload_id_string.as_str()))
        .expect("held upload is listed");
    assert_eq!(item["retentionHeld"], true);
    assert_eq!(item["retentionState"], "active");
    assert_eq!(item["retentionExpiresAt"], expiry);
    assert!(item.get("holdKind").is_none());
    assert!(item.get("reason").is_none());
    let expired_upload_id_string = expired_upload_id.to_string();
    let expired_item = listed_body["items"]
        .as_array()
        .expect("held media items")
        .iter()
        .find(|item| item["id"].as_str() == Some(expired_upload_id_string.as_str()))
        .expect("expired hold upload is listed");
    assert_eq!(expired_item["retentionHeld"], false);
    assert_eq!(expired_item["retentionState"], "expired");
    assert!(expired_item["retentionExpiresAt"].as_i64().is_some());

    let quarantined_while_held = app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/block"),
            &admin_token,
            serde_json::json!({ "reason": "confirmed unsafe media" }),
        ))
        .await
        .expect("block held media response");
    assert_eq!(quarantined_while_held.status(), StatusCode::ACCEPTED);

    let renewed_expiry = expiry + 24 * 60 * 60;
    let first_renewal = app.clone().oneshot(request(
        Method::POST,
        hold_uri.clone(),
        &admin_token,
        serde_json::json!({
            "holdKind": "security",
            "expiresAt": renewed_expiry,
            "reason": "security investigation remains active",
            "expectedHoldId": first_hold_id,
        }),
    ));
    let second_renewal = app.clone().oneshot(request(
        Method::POST,
        hold_uri.clone(),
        &second_admin_token,
        serde_json::json!({
            "holdKind": "security",
            "expiresAt": renewed_expiry + 60,
            "reason": "second operator reviewed the same security hold",
            "expectedHoldId": first_hold_id,
        }),
    ));
    let (first_renewal, second_renewal) = tokio::join!(first_renewal, second_renewal);
    let renewal_statuses = [
        first_renewal.expect("first concurrent hold renewal").status(),
        second_renewal.expect("second concurrent hold renewal").status(),
    ];
    assert!(renewal_statuses.contains(&StatusCode::CREATED));
    assert!(renewal_statuses.contains(&StatusCode::CONFLICT));
    let hold_lifecycle: (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE released_at IS NOT NULL), \
                count(*) FILTER (WHERE released_at IS NULL) \
         FROM media.asset_retention_holds WHERE asset_id = $1",
    )
    .bind(upload_id)
    .fetch_one(&pool)
    .await
    .expect("atomic media hold renewal state");
    assert_eq!(hold_lifecycle, (1, 1));
    let current_inventory = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/retention-holds?state=active&limit=100".into(),
            &admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("current retention inventory response");
    let current_inventory = response_json(current_inventory).await;
    let current_hold_id = current_inventory["items"]
        .as_array()
        .expect("current retention hold items")
        .iter()
        .find(|item| item["uploadId"].as_str() == Some(upload_id.to_string().as_str()))
        .and_then(|item| item["id"].as_str())
        .expect("current retention hold id")
        .to_owned();
    assert_ne!(current_hold_id, first_hold_id);
    let stale_release = app
        .clone()
        .oneshot(request(
            Method::DELETE,
            hold_uri.clone(),
            &admin_token,
            serde_json::json!({
                "expectedHoldId": first_hold_id,
                "reason": "stale operator must not release a replacement hold",
            }),
        ))
        .await
        .expect("stale hold release response");
    assert_eq!(stale_release.status(), StatusCode::CONFLICT);
    assert!(!process_upload_deletion_job(&pool, &SuccessfulObjectStore, upload_id)
        .await
        .expect("held deletion job is skipped"));
    let released_queue = app
        .clone()
        .oneshot(request(
            Method::DELETE,
            hold_uri.clone(),
            &admin_token,
            serde_json::json!({
                "expectedHoldId": current_hold_id,
                "reason": "queued object can now be deleted",
            }),
        ))
        .await
        .expect("release queued media response");
    assert_eq!(released_queue.status(), StatusCode::NO_CONTENT);

    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let worker_pool = pool.clone();
    let worker_store = PausingObjectStore { started: started.clone(), release: release.clone() };
    let worker = tokio::spawn(async move {
        process_upload_deletion_job(&worker_pool, &worker_store, upload_id).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), started.notified())
        .await
        .expect("provider deletion started");
    let too_late = app
        .oneshot(request(Method::POST, hold_uri, &admin_token, hold_body))
        .await
        .expect("hold leased media response");
    assert_eq!(too_late.status(), StatusCode::CONFLICT);
    release.notify_one();
    assert!(worker.await.expect("media deletion worker").expect("media deletion result"));
    let final_status: String = sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
        .bind(upload_id)
        .fetch_one(&pool)
        .await
        .expect("blocked upload after provider deletion");
    assert_eq!(final_status, "blocked");
}

#[tokio::test]
async fn retention_gc_deletes_only_unreferenced_assets_after_grace_and_hold_end() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let admin_id = insert_account(&pool, &suffix, "admin").await;
    let pending_id = insert_upload(&pool, owner_id, &suffix, "pending", "pending").await;
    let clean_id = insert_upload(&pool, owner_id, &suffix, "clean", "clean").await;
    let bound_id = insert_upload(&pool, owner_id, &suffix, "bound", "clean").await;
    let grace_id = insert_upload(&pool, owner_id, &suffix, "grace", "clean").await;
    let held_id = insert_upload(&pool, owner_id, &suffix, "security", "clean").await;
    let recently_approved_id =
        insert_upload(&pool, owner_id, &suffix, "recently-approved", "pending").await;
    sqlx::query("UPDATE media.uploads SET status = 'clean' WHERE id = $1")
        .bind(recently_approved_id)
        .execute(&pool)
        .await
        .expect("approve old upload now");

    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id) \
         VALUES ($1, $2, 'profile_avatar', $2)",
    )
    .bind(bound_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert active media binding");
    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id, detached_at, detached_reason, \
          gc_eligible_at) \
         VALUES ($1, $2, 'profile_banner', $2, now(), 'cleared', now() + interval '30 days')",
    )
    .bind(grace_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert detached media grace");
    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, expires_at) \
         VALUES ($1, 'security', 'preserve for a bounded security investigation', $2, \
                 now() + interval '30 days')",
    )
    .bind(held_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("insert active media hold");

    assert!(schedule_retention_gc_batch(&pool, 100).await.expect("schedule first GC") >= 1);
    let state: (String, String, String) = sqlx::query_as(
        "SELECT upload.status, job.status, job.request_source \
         FROM media.uploads upload JOIN media.object_deletion_jobs job \
           ON job.upload_id = upload.id WHERE upload.id = $1",
    )
    .bind(clean_id)
    .fetch_one(&pool)
    .await
    .expect("retention GC job");
    assert_eq!(state, ("quarantined".into(), "queued".into(), "retention_gc".into()));
    assert!(process_upload_deletion_job(&pool, &SuccessfulObjectStore, clean_id)
        .await
        .expect("process retention GC deletion"));
    let protected_statuses: Vec<String> =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = ANY($1) ORDER BY id")
            .bind(vec![pending_id, bound_id, grace_id, held_id, recently_approved_id])
            .fetch_all(&pool)
            .await
            .expect("protected media statuses");
    assert_eq!(protected_statuses, vec!["pending", "clean", "clean", "clean", "clean"]);

    sqlx::query(
        "UPDATE media.asset_bindings \
         SET detached_at = COALESCE(detached_at, now()), detached_reason = 'cleared', \
             gc_eligible_at = now() - interval '1 second' \
         WHERE asset_id = ANY($1)",
    )
    .bind(vec![bound_id, grace_id])
    .execute(&pool)
    .await
    .expect("end media binding grace");
    sqlx::query(
        "UPDATE media.asset_retention_holds \
         SET released_at = now(), released_by = $2, release_reason = 'security review ended' \
         WHERE asset_id = $1",
    )
    .bind(held_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("release media security hold");
    assert!(schedule_retention_gc_batch(&pool, 100).await.expect("schedule final GC") >= 3);
    for upload_id in [bound_id, grace_id, held_id] {
        assert!(process_upload_deletion_job(&pool, &SuccessfulObjectStore, upload_id)
            .await
            .expect("process formerly protected deletion"));
    }
    let blocked_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.uploads WHERE id = ANY($1) AND status = 'blocked'",
    )
    .bind(vec![clean_id, bound_id, grace_id, held_id])
    .fetch_one(&pool)
    .await
    .expect("blocked GC upload count");
    assert_eq!(blocked_count, 4);
    let system_audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM governance.audit_events \
         WHERE actor_kind = 'system' AND action = 'media.upload.garbage_collected' \
           AND target_id = ANY($1)",
    )
    .bind(vec![
        clean_id.to_string(),
        bound_id.to_string(),
        grace_id.to_string(),
        held_id.to_string(),
    ])
    .fetch_one(&pool)
    .await
    .expect("media GC system audit count");
    assert_eq!(system_audit_count, 4);
}

#[tokio::test]
async fn retention_gc_waits_for_cloud_draft_reference_then_collects_after_draft_delete() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let draft_asset_id = insert_upload(&pool, owner_id, &suffix, "draft-live", "clean").await;
    sqlx::query("UPDATE media.uploads SET usage = 'forum_thread' WHERE id = $1")
        .bind(draft_asset_id)
        .execute(&pool)
        .await
        .expect("mark draft upload usage");
    sqlx::query(
        "INSERT INTO forum.drafts (account_id, draft_key, payload) VALUES ($1, 'thread:new', $2)",
    )
    .bind(owner_id)
    .bind(serde_json::json!({
        "kind": "thread",
        "boardId": null,
        "title": "retained draft",
        "body": "draft with an image",
        "contentFormat": "markdown_v1",
        "tags": [],
        "pollQuestion": "",
        "pollOptions": [],
        "attachmentAssetIds": [draft_asset_id.to_string()],
    }))
    .execute(&pool)
    .await
    .expect("insert live cloud draft");
    schedule_retention_gc_batch(&pool, 100).await.expect("schedule GC with live draft");
    let protected_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(draft_asset_id)
            .fetch_one(&pool)
            .await
            .expect("draft-protected upload status");
    assert_eq!(protected_status, "clean");

    sqlx::query("DELETE FROM forum.drafts WHERE account_id = $1 AND draft_key = 'thread:new'")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("delete cloud draft");
    schedule_retention_gc_batch(&pool, 100).await.expect("schedule GC after draft delete");
    let queued: (String, String) = sqlx::query_as(
        "SELECT upload.status, job.request_source FROM media.uploads upload \
         JOIN media.object_deletion_jobs job ON job.upload_id = upload.id \
         WHERE upload.id = $1",
    )
    .bind(draft_asset_id)
    .fetch_one(&pool)
    .await
    .expect("draft asset GC job");
    assert_eq!(queued, ("quarantined".into(), "retention_gc".into()));
}

#[tokio::test]
async fn expired_upload_intent_cleanup_deletes_the_exact_key_and_internal_tombstone() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let admin_id = insert_account(&pool, &format!("{suffix}-cleanup-admin"), "admin").await;
    let intent_id = uuid::Uuid::new_v4();
    let exact_key = format!("uploads/{owner_id}/image/{suffix}-abandoned.png");
    sqlx::query(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, usage, max_bytes, callback_token_hash, \
          expires_at) \
         VALUES ($1, $2, 'image', $3, 'image/png', 'profile_avatar', 10, \
                 sha256(convert_to($4, 'UTF8')), \
                 now() - interval '20 minutes')",
    )
    .bind(intent_id)
    .bind(owner_id)
    .bind(&exact_key)
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .execute(&pool)
    .await
    .expect("insert expired no-callback upload intent");

    assert_eq!(
        schedule_expired_upload_intent_cleanup_batch(&pool, 10)
            .await
            .expect("schedule expired upload intent cleanup"),
        1
    );
    let cleanup_upload_id: i64 = sqlx::query_scalar(
        "SELECT upload_id FROM media.upload_intents \
         WHERE id = $1 AND revoked_at IS NOT NULL",
    )
    .bind(intent_id)
    .fetch_one(&pool)
    .await
    .expect("expired intent cleanup upload id");
    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, expires_at) \
         VALUES ($1, 'security', 'inspect abandoned upload object before deletion', $2, \
                 now() + interval '1 day')",
    )
    .bind(cleanup_upload_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("hold upload intent cleanup object");
    let deleted_keys = Arc::new(std::sync::Mutex::new(Vec::new()));
    let object_store = RecordingObjectStore { deleted_keys: deleted_keys.clone() };
    assert!(!process_upload_deletion_job(&pool, &object_store, cleanup_upload_id)
        .await
        .expect("active hold pauses upload intent cleanup"));
    sqlx::query(
        "UPDATE media.asset_retention_holds \
         SET released_at = now(), released_by = $2, release_reason = 'security review completed' \
         WHERE asset_id = $1 AND released_at IS NULL",
    )
    .bind(cleanup_upload_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("release upload intent cleanup hold");
    assert!(process_upload_deletion_job(&pool, &object_store, cleanup_upload_id)
        .await
        .expect("delete expired upload intent key"));
    assert_eq!(deleted_keys.lock().expect("read deleted object keys").as_slice(), &[exact_key]);
    let remaining_intent: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media.upload_intents WHERE id = $1)")
            .bind(intent_id)
            .fetch_one(&pool)
            .await
            .expect("remaining expired upload intent");
    assert!(!remaining_intent);
    let redacted_tombstone: (String, bool, i64, String) = sqlx::query_as(
        "SELECT upload.status, upload.redacted_at IS NOT NULL, job.id, job.status \
         FROM media.uploads upload JOIN media.object_deletion_jobs job \
           ON job.upload_id = upload.id WHERE upload.id = $1",
    )
    .bind(cleanup_upload_id)
    .fetch_one(&pool)
    .await
    .expect("bounded upload intent cleanup tombstone");
    assert_eq!(redacted_tombstone.0, "blocked");
    assert!(redacted_tombstone.1);
    assert_eq!(redacted_tombstone.3, "succeeded");
    let cleanup_job_id = redacted_tombstone.2;
    sqlx::query("UPDATE media.uploads SET redacted_at = now() - interval '31 days' WHERE id = $1")
        .bind(cleanup_upload_id)
        .execute(&pool)
        .await
        .expect("age upload intent cleanup tombstone");
    sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET completed_at = now() - interval '31 days' WHERE upload_id = $1",
    )
    .bind(cleanup_upload_id)
    .execute(&pool)
    .await
    .expect("age upload intent cleanup job");
    assert_eq!(
        purge_completed_cleanup_tombstones(&pool, 10)
            .await
            .expect("purge upload intent cleanup tombstone"),
        0
    );
    sqlx::query("DELETE FROM media.asset_retention_holds WHERE asset_id = $1")
        .bind(cleanup_upload_id)
        .execute(&pool)
        .await
        .expect("expire upload intent cleanup hold history");
    sqlx::query(
        "INSERT INTO media.object_deletion_job_retry_events (job_id, actor_id, reason) \
         VALUES ($1, $2, 'operator retry reason remains retained')",
    )
    .bind(cleanup_job_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("insert cleanup retry history");
    assert_eq!(
        purge_completed_cleanup_tombstones(&pool, 10)
            .await
            .expect("retain tombstone with operator retry history"),
        0
    );
    sqlx::query("DELETE FROM media.object_deletion_job_retry_events WHERE job_id = $1")
        .bind(cleanup_job_id)
        .execute(&pool)
        .await
        .expect("expire cleanup retry history");
    sqlx::query("DELETE FROM media.object_deletion_jobs WHERE id = $1")
        .bind(cleanup_job_id)
        .execute(&pool)
        .await
        .expect("expire succeeded cleanup job history first");
    assert_eq!(
        purge_completed_cleanup_tombstones(&pool, 10)
            .await
            .expect("purge upload intent cleanup tombstone after hold history"),
        1
    );
    let internal_rows: (bool, bool) = sqlx::query_as(
        "SELECT \
           EXISTS(SELECT 1 FROM media.uploads WHERE id = $1), \
           EXISTS(SELECT 1 FROM media.object_deletion_jobs WHERE upload_id = $1)",
    )
    .bind(cleanup_upload_id)
    .fetch_one(&pool)
    .await
    .expect("removed upload intent cleanup facts");
    assert_eq!(internal_rows, (false, false));
    assert_eq!(
        schedule_expired_upload_intent_cleanup_batch(&pool, 10)
            .await
            .expect("repeat expired upload intent cleanup"),
        0
    );
}

#[tokio::test]
async fn upload_credential_quota_bounds_attempts_outstanding_objects_bytes_and_rows() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();

    let persisted_owner = insert_account(&pool, &format!("{suffix}-persisted"), "user").await;
    let callback_token = format!("{suffix}-callback-secret");
    let reservation = reserve_upload_intent(
        &pool,
        persisted_owner,
        "image",
        "image/png",
        Some("profile_avatar"),
        &callback_token,
    )
    .await
    .expect("persist digest-only upload intent");
    let persisted_hash: Vec<u8> =
        sqlx::query_scalar("SELECT callback_token_hash FROM media.upload_intents WHERE id = $1")
            .bind(reservation.id)
            .fetch_one(&pool)
            .await
            .expect("persisted callback token hash");
    let expected_hash: [u8; 32] = Sha256::digest(callback_token.as_bytes()).into();
    assert_eq!(persisted_hash, expected_hash);
    let accepted_attempts: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.upload_credential_attempts WHERE account_id = $1",
    )
    .bind(persisted_owner)
    .fetch_one(&pool)
    .await
    .expect("accepted upload credential attempt");
    assert_eq!(accepted_attempts, 1);

    let active_owner = insert_account(&pool, &format!("{suffix}-active"), "user").await;
    for index in 0..10 {
        let token = format!("{suffix}-active-{index}");
        sqlx::query(
            "INSERT INTO media.upload_intents \
             (id, account_id, kind, oss_key, content_type, max_bytes, callback_token_hash, \
              expires_at) \
             VALUES ($1, $2, 'image', $3, 'image/png', 1, \
                     sha256(convert_to($4, 'UTF8')), now() + interval '15 minutes')",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(active_owner)
        .bind(format!("uploads/{active_owner}/image/{suffix}-active-{index}.png"))
        .bind(token)
        .execute(&pool)
        .await
        .expect("insert active upload intent quota fixture");
    }
    assert!(matches!(
        reserve_upload_intent(
            &pool,
            active_owner,
            "image",
            "image/png",
            None,
            &format!("{suffix}-active-overflow"),
        )
        .await,
        Err(shared::AppError::RateLimited)
    ));

    let daily_owner = insert_account(&pool, &format!("{suffix}-daily"), "user").await;
    sqlx::query(
        "INSERT INTO media.upload_credential_attempts (account_id, reserved_bytes, created_at) \
         SELECT $1, 1, now() FROM generate_series(1, 100)",
    )
    .bind(daily_owner)
    .execute(&pool)
    .await
    .expect("insert daily upload attempt quota fixture");
    assert!(matches!(
        reserve_upload_intent(
            &pool,
            daily_owner,
            "image",
            "image/png",
            None,
            &format!("{suffix}-daily-overflow"),
        )
        .await,
        Err(shared::AppError::RateLimited)
    ));

    let bytes_owner = insert_account(&pool, &format!("{suffix}-bytes"), "user").await;
    sqlx::query(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         SELECT $1, 'image', \
                'uploads/' || $1::text || '/image/' || $2 || '-bytes-' || value || '.png', \
                '', 20971520, 'image/png', repeat('b', 64), 'pending' \
         FROM generate_series(1, 25) AS value",
    )
    .bind(bytes_owner)
    .bind(&suffix)
    .execute(&pool)
    .await
    .expect("insert stored byte quota fixture");
    assert!(matches!(
        reserve_upload_intent(
            &pool,
            bytes_owner,
            "image",
            "image/png",
            None,
            &format!("{suffix}-bytes-overflow"),
        )
        .await,
        Err(shared::AppError::RateLimited)
    ));

    let object_owner = insert_account(&pool, &format!("{suffix}-objects"), "user").await;
    sqlx::query(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         SELECT $1, 'image', \
                'uploads/' || $1::text || '/image/' || $2 || '-object-' || value || '.png', \
                '', 1, 'image/png', repeat('c', 64), 'pending' \
         FROM generate_series(1, 500) AS value",
    )
    .bind(object_owner)
    .bind(&suffix)
    .execute(&pool)
    .await
    .expect("insert live object quota fixture");
    assert!(matches!(
        reserve_upload_intent(
            &pool,
            object_owner,
            "image",
            "image/png",
            None,
            &format!("{suffix}-object-overflow"),
        )
        .await,
        Err(shared::AppError::RateLimited)
    ));

    let suspended_owner = insert_account(&pool, &format!("{suffix}-suspended"), "user").await;
    let suspending_admin = insert_account(&pool, &format!("{suffix}-suspending"), "admin").await;
    let mut suspension = pool.begin().await.expect("begin suspension barrier");
    sqlx::query("SELECT id FROM identity.accounts WHERE id = $1 FOR UPDATE")
        .bind(suspended_owner)
        .execute(&mut *suspension)
        .await
        .expect("lock account for suspension");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by) \
         VALUES ($1, 'suspend', 'credential issuance suspension barrier', $2)",
    )
    .bind(suspended_owner)
    .bind(suspending_admin)
    .execute(&mut *suspension)
    .await
    .expect("stage account suspension");
    let barrier_pool = pool.clone();
    let callback_token = format!("{suffix}-suspension-callback");
    let issuance = tokio::spawn(async move {
        reserve_upload_intent(
            &barrier_pool,
            suspended_owner,
            "image",
            "image/png",
            None,
            &callback_token,
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!issuance.is_finished(), "credential issuance must wait for the account lock");
    suspension.commit().await.expect("commit account suspension");
    assert!(matches!(
        issuance.await.expect("join blocked upload issuance"),
        Err(shared::AppError::Forbidden)
    ));
}

#[tokio::test]
async fn privacy_housekeeping_purges_only_expired_ephemeral_media_facts() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let admin_id = insert_account(&pool, &suffix, "admin").await;
    let old_upload_id = insert_upload(&pool, owner_id, &suffix, "old-facts", "clean").await;
    let current_upload_id = insert_upload(&pool, owner_id, &suffix, "current-facts", "clean").await;
    let expired_preview_hash =
        format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let current_preview_hash =
        format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO media.moderation_preview_grants \
         (token_hash, upload_id, moderator_account_id, reason, created_at, expires_at) VALUES \
         ($4, $1, $3, 'expired preview credential', \
          now() - interval '3 days', now() - interval '2 days'), \
         ($5, $2, $3, 'current preview credential', \
          now() - interval '1 hour', now() + interval '1 hour')",
    )
    .bind(old_upload_id)
    .bind(current_upload_id)
    .bind(admin_id)
    .bind(expired_preview_hash)
    .bind(current_preview_hash)
    .execute(&pool)
    .await
    .expect("insert preview grant retention fixtures");
    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id, bound_at, detached_at, \
          detached_reason, gc_eligible_at) VALUES \
         ($1, $3, 'profile_avatar', $3, now() - interval '40 days', \
          now() - interval '31 days', 'cleared', now() - interval '1 day'), \
         ($2, $3, 'profile_banner', $3, now() - interval '1 day', \
          now(), 'cleared', now() + interval '30 days')",
    )
    .bind(old_upload_id)
    .bind(current_upload_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert detached binding retention fixtures");
    sqlx::query(
        "INSERT INTO media.upload_credential_attempts (account_id, reserved_bytes, created_at) \
         VALUES ($1, 1, now() - interval '3 days'), ($1, 1, now())",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert upload attempt retention fixtures");

    assert!(purge_expired_preview_grants(&pool, 100).await.expect("purge preview grants") >= 1);
    assert!(purge_expired_asset_bindings(&pool, 100).await.expect("purge bindings") >= 1);
    assert!(
        purge_upload_credential_attempts(&pool, 100).await.expect("purge upload attempts") >= 1
    );
    let remaining: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
           (SELECT count(*)::bigint FROM media.moderation_preview_grants \
            WHERE upload_id = ANY($1)), \
           (SELECT count(*)::bigint FROM media.asset_bindings \
            WHERE asset_id = ANY($1)), \
           (SELECT count(*)::bigint FROM media.upload_credential_attempts \
            WHERE account_id = $2)",
    )
    .bind(vec![old_upload_id, current_upload_id])
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("remaining ephemeral media facts");
    assert_eq!(remaining, (1, 1, 1));
}

#[tokio::test]
async fn account_purge_detaches_profile_media_but_preserves_shared_references_and_holds() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let admin_id = insert_account(&pool, &suffix, "admin").await;
    let avatar_id = insert_upload(&pool, owner_id, &suffix, "avatar", "clean").await;
    let unbound_id = insert_upload(&pool, owner_id, &suffix, "unbound", "pending").await;
    let promotion_id = insert_upload(&pool, owner_id, &suffix, "promotion", "clean").await;
    let forum_id = insert_upload(&pool, owner_id, &suffix, "forum", "clean").await;
    let held_id = insert_upload(&pool, owner_id, &suffix, "held-purge", "pending").await;
    let grace_id = insert_upload(&pool, owner_id, &suffix, "grace-purge", "clean").await;
    let draft_id = insert_upload(&pool, owner_id, &suffix, "draft-purge", "clean").await;
    sqlx::query("UPDATE media.uploads SET usage = 'forum_thread' WHERE id = $1")
        .bind(draft_id)
        .execute(&pool)
        .await
        .expect("mark account purge draft upload usage");

    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id) \
         VALUES ($1, $2, 'profile_avatar', $2), \
                ($3, $2, 'platform_promotion', $3)",
    )
    .bind(avatar_id)
    .bind(owner_id)
    .bind(promotion_id)
    .execute(&pool)
    .await
    .expect("insert account purge active bindings");
    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id, detached_at, detached_reason, \
          gc_eligible_at) \
         VALUES ($1, $2, 'profile_banner', $2, now(), 'cleared', now() + interval '30 days')",
    )
    .bind(grace_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert account purge grace binding");
    sqlx::query(
        "INSERT INTO media.asset_usages \
         (asset_id, owner_account_id, target_type, target_id, position, alt_text, \
          bound_content_version) \
         VALUES ($1, $2, 'forum_thread', $1, 0, 'retained forum image', 1)",
    )
    .bind(forum_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert account purge forum usage");
    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, expires_at) \
         VALUES ($1, 'security', 'preserve for account security investigation', $2, \
                 now() + interval '30 days')",
    )
    .bind(held_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("insert account purge hold");
    sqlx::query(
        "INSERT INTO forum.drafts (account_id, draft_key, payload) \
         VALUES ($1, 'thread:new', $2)",
    )
    .bind(owner_id)
    .bind(serde_json::json!({
        "kind": "thread",
        "boardId": null,
        "title": "private draft",
        "body": "draft image",
        "contentFormat": "markdown_v1",
        "tags": [],
        "pollQuestion": "",
        "pollOptions": [],
        "attachmentAssetIds": [draft_id.to_string()],
    }))
    .execute(&pool)
    .await
    .expect("insert account purge draft");
    let upload_intent_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, max_bytes, callback_token_hash, expires_at) \
         VALUES ($1, $2, 'image', $3, 'image/png', 10, \
                 sha256(convert_to($4, 'UTF8')), now() + interval '1 day')",
    )
    .bind(upload_intent_id)
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-intent.png"))
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .execute(&pool)
    .await
    .expect("insert account purge upload intent");

    let progress = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("prepare account media purge");
    assert_eq!(progress.scheduled, 6);
    assert!(!progress.has_more);
    assert_eq!(progress.pending_deletions, 5);
    assert_eq!(progress.dead_letter_deletions, 0);
    assert_eq!(progress.retained_assets, 3);
    assert_eq!(progress.missing_deletion_jobs, 0);
    let intent_cleanup_upload_id: i64 = sqlx::query_scalar(
        "SELECT upload_id FROM media.upload_intents WHERE id = $1 AND revoked_at IS NOT NULL",
    )
    .bind(upload_intent_id)
    .fetch_one(&pool)
    .await
    .expect("revoked upload intent cleanup target");
    let queued: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT upload.id, upload.status, job.request_source \
         FROM media.uploads upload JOIN media.object_deletion_jobs job \
           ON job.upload_id = upload.id \
         WHERE upload.id = ANY($1) ORDER BY upload.id",
    )
    .bind(vec![avatar_id, unbound_id, draft_id, grace_id, held_id, intent_cleanup_upload_id])
    .fetch_all(&pool)
    .await
    .expect("account purge media jobs");
    assert_eq!(queued.len(), 6);
    assert!(queued
        .iter()
        .all(|(_, status, source)| { status == "quarantined" && source == "account_purge" }));
    let preserved_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.uploads \
         WHERE id = ANY($1) AND status = 'clean'",
    )
    .bind(vec![promotion_id, forum_id])
    .fetch_one(&pool)
    .await
    .expect("preserved shared media count");
    assert_eq!(preserved_count, 2);
    let profile_binding: (String, bool) = sqlx::query_as(
        "SELECT detached_reason, gc_eligible_at <= now() FROM media.asset_bindings \
         WHERE asset_id = $1 AND target_type = 'profile_avatar'",
    )
    .bind(avatar_id)
    .fetch_one(&pool)
    .await
    .expect("purged profile binding");
    assert_eq!(profile_binding, ("account_purge".into(), true));
    let intent_state: (bool, i64) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL, upload_id FROM media.upload_intents WHERE id = $1",
    )
    .bind(upload_intent_id)
    .fetch_one(&pool)
    .await
    .expect("revoked upload intent state");
    assert_eq!(intent_state, (true, intent_cleanup_upload_id));
    let draft_reference_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.draft_asset_references WHERE account_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("remaining draft asset references");
    assert_eq!(draft_reference_count, 0);

    assert!(!process_upload_deletion_job(&pool, &SuccessfulObjectStore, intent_cleanup_upload_id,)
        .await
        .expect("active upload intent cleanup waits for credential expiry buffer"));
    sqlx::query("UPDATE media.object_deletion_jobs SET available_at = now() WHERE upload_id = $1")
        .bind(intent_cleanup_upload_id)
        .execute(&pool)
        .await
        .expect("advance upload intent cleanup after expiry buffer");
    for upload_id in [avatar_id, unbound_id, draft_id, grace_id, intent_cleanup_upload_id] {
        assert!(process_upload_deletion_job(&pool, &SuccessfulObjectStore, upload_id)
            .await
            .expect("process account purge deletion"));
    }
    let completed = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("recheck account media purge");
    assert_eq!(completed.scheduled, 0);
    assert!(!completed.has_more);
    assert_eq!(completed.pending_deletions, 0);
    assert_eq!(completed.dead_letter_deletions, 0);
    assert_eq!(completed.missing_deletion_jobs, 0);
    let remaining_intents: i64 =
        sqlx::query_scalar("SELECT count(*) FROM media.upload_intents WHERE account_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("remaining upload intents after provider cleanup");
    assert_eq!(remaining_intents, 0);
}

#[tokio::test]
async fn account_purge_progress_accounts_for_existing_jobs_retained_assets_and_anomalies() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let moderator_id = insert_account(&pool, &format!("{suffix}-mod"), "mod").await;
    let admin_id = insert_account(&pool, &format!("{suffix}-admin"), "admin").await;
    let retention_id = insert_upload(&pool, owner_id, &suffix, "retention-job", "clean").await;
    let held_job_id = insert_upload(&pool, owner_id, &suffix, "held-job", "clean").await;
    let moderation_id = insert_upload(&pool, owner_id, &suffix, "moderation-job", "clean").await;
    let missing_job_id = insert_upload(&pool, owner_id, &suffix, "missing-job", "clean").await;
    let held_missing_id =
        insert_upload(&pool, owner_id, &suffix, "held-missing-job", "clean").await;
    let eligible_id = insert_upload(&pool, owner_id, &suffix, "eligible", "clean").await;
    let shared_id = insert_upload(&pool, owner_id, &suffix, "shared", "clean").await;
    let grace_id = insert_upload(&pool, owner_id, &suffix, "grace", "clean").await;

    for upload_id in [retention_id, held_job_id, moderation_id, missing_job_id, held_missing_id] {
        sqlx::query("UPDATE media.uploads SET status = 'quarantined' WHERE id = $1")
            .bind(upload_id)
            .execute(&pool)
            .await
            .expect("quarantine progress fixture");
    }
    for upload_id in [retention_id, held_job_id] {
        sqlx::query(
            "INSERT INTO media.object_deletion_jobs \
             (upload_id, requested_by, requested_role, request_source, reason, previous_status) \
             VALUES ($1, NULL, NULL, 'retention_gc', 'retention fixture', 'clean')",
        )
        .bind(upload_id)
        .execute(&pool)
        .await
        .expect("insert retention progress job");
    }
    sqlx::query(
        "INSERT INTO media.object_deletion_jobs \
         (upload_id, requested_by, requested_role, request_source, reason, previous_status, \
          status, attempt_count, last_error_code) \
         VALUES ($1, $2, 'mod', 'moderation', 'moderation fixture', 'clean', \
                 'dead_letter', 8, 'provider_delete_failed')",
    )
    .bind(moderation_id)
    .bind(moderator_id)
    .execute(&pool)
    .await
    .expect("insert moderation dead-letter fixture");
    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, expires_at) \
         VALUES ($1, 'security', 'bounded account security evidence', $3, \
                 now() + interval '30 days'), \
                ($2, 'security', 'held missing job must remain observable', $3, \
                 now() + interval '30 days')",
    )
    .bind(held_job_id)
    .bind(held_missing_id)
    .bind(admin_id)
    .execute(&pool)
    .await
    .expect("hold queued deletion fixture");
    sqlx::query(
        "INSERT INTO media.asset_usages \
         (asset_id, owner_account_id, target_type, target_id, position, alt_text, \
          bound_content_version) \
         VALUES ($1, $2, 'forum_thread', $1, 0, 'shared content', 1)",
    )
    .bind(shared_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert retained shared usage");
    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id, detached_at, detached_reason, \
          gc_eligible_at) \
         VALUES ($1, $2, 'platform_promotion', $1, now(), 'archived', \
                 now() + interval '30 days')",
    )
    .bind(grace_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert retained binding grace");
    let rollout_intent_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, max_bytes, callback_token_hash, expires_at) \
         VALUES ($1, $2, 'image', $3, 'image/png', 10, \
                 sha256(convert_to($4, 'UTF8')), now() + interval '1 day')",
    )
    .bind(rollout_intent_id)
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-rollout-intent.png"))
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .execute(&pool)
    .await
    .expect("insert rollout-gated upload intent");

    let rollout_gated = prepare_account_media_purge(&pool, owner_id, false)
        .await
        .expect("account purge remains gated before writer cutover");
    assert_eq!(rollout_gated.scheduled, 0);
    assert!(rollout_gated.has_more);
    let eligible_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(eligible_id)
            .fetch_one(&pool)
            .await
            .expect("rollout-gated account media status");
    assert_eq!(eligible_status, "clean");
    let revoked_without_cleanup_target: (bool, Option<i64>) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL, upload_id FROM media.upload_intents WHERE id = $1",
    )
    .bind(rollout_intent_id)
    .fetch_one(&pool)
    .await
    .expect("rollout-gated intent revocation");
    assert_eq!(revoked_without_cleanup_target, (true, None));

    let progress = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("account purge progress across existing sources");
    assert_eq!(progress.scheduled, 2);
    assert!(!progress.has_more);
    assert_eq!(progress.pending_deletions, 3);
    assert_eq!(progress.dead_letter_deletions, 1);
    assert_eq!(progress.retained_assets, 4);
    assert_eq!(progress.missing_deletion_jobs, 2);
    let eligible_source: String = sqlx::query_scalar(
        "SELECT request_source FROM media.object_deletion_jobs WHERE upload_id = $1",
    )
    .bind(eligible_id)
    .fetch_one(&pool)
    .await
    .expect("new account purge job source");
    assert_eq!(eligible_source, "account_purge");
}

#[tokio::test]
async fn account_purge_batches_past_locked_rows_and_repeats_cleanup_idempotently() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id = insert_account(&pool, &suffix, "user").await;
    let mut upload_ids = Vec::new();
    for index in 0..52 {
        upload_ids.push(
            insert_upload(&pool, owner_id, &suffix, &format!("batch-{index}"), "clean").await,
        );
    }
    sqlx::query("UPDATE media.uploads SET usage = 'forum_thread' WHERE id = $1")
        .bind(upload_ids[1])
        .execute(&pool)
        .await
        .expect("mark batched draft upload usage");
    sqlx::query(
        "INSERT INTO media.asset_bindings \
         (asset_id, owner_account_id, target_type, target_id) \
         VALUES ($1, $2, 'profile_avatar', $2)",
    )
    .bind(upload_ids[0])
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("insert batched profile binding");
    sqlx::query(
        "INSERT INTO forum.drafts (account_id, draft_key, payload) VALUES ($1, 'thread:new', $2)",
    )
    .bind(owner_id)
    .bind(serde_json::json!({
        "kind": "thread",
        "attachmentAssetIds": [upload_ids[1].to_string()],
    }))
    .execute(&pool)
    .await
    .expect("insert batched draft");
    sqlx::query(
        "INSERT INTO media.upload_intents \
         (id, account_id, kind, oss_key, content_type, max_bytes, callback_token_hash, expires_at, \
          consumed_at, upload_id) \
         VALUES ($1, $2, 'image', $3, 'image/png', 10, \
                 sha256(convert_to($4, 'UTF8')), now() + interval '1 day', \
                 now(), $5)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-batch-intent.png"))
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(upload_ids[2])
    .execute(&pool)
    .await
    .expect("insert batched upload intent");

    let locked_upload_id = *upload_ids.last().expect("locked batched upload");
    let mut locked_transaction = pool.begin().await.expect("begin locked upload fixture");
    sqlx::query("SELECT id FROM media.uploads WHERE id = $1 FOR UPDATE")
        .bind(locked_upload_id)
        .execute(&mut *locked_transaction)
        .await
        .expect("lock one account purge candidate");

    let first = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("first bounded account purge batch");
    assert_eq!(first.scheduled, 50);
    assert!(first.has_more);
    let second = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("second bounded account purge batch");
    assert_eq!(second.scheduled, 1);
    assert!(second.has_more);
    locked_transaction.commit().await.expect("release skipped upload lock");
    let third = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("collect previously locked account media");
    assert_eq!(third.scheduled, 1);
    assert!(!third.has_more);
    assert_eq!(third.pending_deletions, 52);
    let repeated = prepare_account_media_purge(&pool, owner_id, true)
        .await
        .expect("repeat account media cleanup");
    assert_eq!(repeated.scheduled, 0);
    assert!(!repeated.has_more);
    assert_eq!(repeated.pending_deletions, 52);
    assert_eq!(repeated.missing_deletion_jobs, 0);

    let active_profile_bindings: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.asset_bindings \
         WHERE target_type = 'profile_avatar' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("active profile bindings after purge");
    assert_eq!(active_profile_bindings, 0);
    let draft_references: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.draft_asset_references WHERE account_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("draft references after repeated purge");
    assert_eq!(draft_references, 0);
    let upload_intents: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.upload_intents WHERE account_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("upload intents after repeated purge");
    assert_eq!(upload_intents, 0);
}

#[tokio::test]
async fn operations_can_inventory_and_retry_admin_owned_system_deletion_dead_letters() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media retention");
    let pool = PgPool::connect(&database_url).await.expect("media retention database");
    MIGRATOR.run(&pool).await.expect("media retention migrations");
    let _test_guard = serialize_retention_tests(&pool).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id = insert_account(&pool, &suffix, "admin").await;
    let moderator_id = insert_account(&pool, &format!("{suffix}-mod"), "mod").await;
    let upload_ids = [
        insert_upload(&pool, admin_id, &suffix, "system-dead-letter-a", "clean").await,
        insert_upload(&pool, admin_id, &suffix, "system-dead-letter-b", "clean").await,
    ];
    let progress = prepare_account_media_purge(&pool, admin_id, true)
        .await
        .expect("queue admin-owned system deletions");
    assert_eq!(progress.scheduled, 2);
    for upload_id in upload_ids {
        for _ in 0..8 {
            assert!(process_upload_deletion_job(&pool, &AlwaysFailingObjectStore, upload_id)
                .await
                .expect("process failing system deletion"));
            sqlx::query(
                "UPDATE media.object_deletion_jobs SET available_at = now() WHERE upload_id = $1",
            )
            .bind(upload_id)
            .execute(&pool)
            .await
            .expect("advance system deletion retry fixture");
        }
    }
    let dead_letter_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.object_deletion_jobs \
         WHERE upload_id = ANY($1) AND status = 'dead_letter' AND attempt_count = 8",
    )
    .bind(upload_ids.to_vec())
    .fetch_one(&pool)
    .await
    .expect("admin-owned dead-letter count");
    assert_eq!(dead_letter_count, 2);

    let user_id = insert_account(&pool, &format!("{suffix}-user"), "user").await;
    let user_upload_id =
        insert_upload(&pool, user_id, &suffix, "user-system-dead-letter", "clean").await;
    let user_progress = prepare_account_media_purge(&pool, user_id, true)
        .await
        .expect("queue user-owned system deletion");
    assert_eq!(user_progress.scheduled, 1);
    for _ in 0..8 {
        assert!(process_upload_deletion_job(&pool, &AlwaysFailingObjectStore, user_upload_id)
            .await
            .expect("process failing user-owned system deletion"));
        sqlx::query(
            "UPDATE media.object_deletion_jobs SET available_at = now() WHERE upload_id = $1",
        )
        .bind(user_upload_id)
        .execute(&pool)
        .await
        .expect("advance user-owned system deletion retry fixture");
    }

    let admin_token = session_token(&pool, admin_id, true).await;
    let stale_admin_token = session_token(&pool, admin_id, false).await;
    let moderator_token = session_token(&pool, moderator_id, true).await;
    let app = routes_with_object_store(test_state(pool.clone()), Arc::new(SuccessfulObjectStore));
    let moderation_queue = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/uploads?status=quarantined&limit=100".into(),
            &moderator_token,
            serde_json::json!({}),
        ))
        .await
        .expect("moderation queue excludes system deletion work");
    assert_eq!(moderation_queue.status(), StatusCode::OK);
    let moderation_queue = response_json(moderation_queue).await;
    let user_upload_id_text = user_upload_id.to_string();
    assert!(!moderation_queue["items"]
        .as_array()
        .expect("moderation queue items")
        .iter()
        .any(|item| item["id"].as_str() == Some(user_upload_id_text.as_str())));
    let moderation_retry = app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{user_upload_id}/block"),
            &moderator_token,
            serde_json::json!({ "reason": "must use the operations deletion workflow" }),
        ))
        .await
        .expect("moderator system deletion retry response");
    assert_eq!(moderation_retry.status(), StatusCode::CONFLICT);
    let preserved_system_job: (String, String, i32) = sqlx::query_as(
        "SELECT request_source, status, attempt_count FROM media.object_deletion_jobs \
         WHERE upload_id = $1",
    )
    .bind(user_upload_id)
    .fetch_one(&pool)
    .await
    .expect("preserved user-owned system deletion job");
    assert_eq!(preserved_system_job, ("account_purge".into(), "dead_letter".into(), 8));
    let denied = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/deletion-jobs?status=dead_letter&limit=1".into(),
            &moderator_token,
            serde_json::json!({}),
        ))
        .await
        .expect("moderator system deletion inventory response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let stale_inventory = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/deletion-jobs?status=dead_letter&limit=1".into(),
            &stale_admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("stale system deletion inventory response");
    assert_eq!(stale_inventory.status(), StatusCode::PRECONDITION_REQUIRED);
    let first_page = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/deletion-jobs?status=dead_letter&limit=1".into(),
            &admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("first system deletion inventory page");
    assert_eq!(first_page.status(), StatusCode::OK);
    assert_eq!(
        first_page.headers().get(header::CACHE_CONTROL).and_then(|value| value.to_str().ok()),
        Some("private, no-store")
    );
    let first_page = response_json(first_page).await;
    let first_item = &first_page["items"][0];
    assert_eq!(first_item["requestSource"], "account_purge");
    assert_eq!(first_item["status"], "dead_letter");
    for forbidden_field in ["ossKey", "sha256", "url"] {
        assert!(first_item.get(forbidden_field).is_none());
    }
    let job_id = first_item["id"].as_str().expect("system deletion job id").to_owned();
    let upload_id = first_item["uploadId"]
        .as_str()
        .expect("system deletion upload id")
        .parse::<i64>()
        .expect("numeric system deletion upload id");
    let cursor = first_page["nextCursor"].as_str().expect("system deletion next cursor");
    let second_page = app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/admin/media/deletion-jobs?status=dead_letter&limit=1&cursor={cursor}"),
            &admin_token,
            serde_json::json!({}),
        ))
        .await
        .expect("second system deletion inventory page");
    assert_eq!(second_page.status(), StatusCode::OK);
    assert_eq!(response_json(second_page).await["items"].as_array().map(Vec::len), Some(1));
    let inventory_audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'media.deletion_job_inventory.viewed'",
    )
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("system deletion inventory audit");
    assert_eq!(inventory_audit_count, 2);

    let retry_uri = format!("/api/v2/admin/media/deletion-jobs/{job_id}/retry");
    let retry_reason = "operations verified provider recovery and authorized a bounded retry";
    let stale_retry = app
        .clone()
        .oneshot(request(
            Method::POST,
            retry_uri.clone(),
            &stale_admin_token,
            serde_json::json!({ "reason": retry_reason }),
        ))
        .await
        .expect("stale system deletion retry response");
    assert_eq!(stale_retry.status(), StatusCode::PRECONDITION_REQUIRED);
    let retried = app
        .oneshot(request(
            Method::POST,
            retry_uri,
            &admin_token,
            serde_json::json!({ "reason": retry_reason }),
        ))
        .await
        .expect("fresh system deletion retry response");
    assert_eq!(retried.status(), StatusCode::ACCEPTED);
    let job_state: (String, i32, String) = sqlx::query_as(
        "SELECT status, attempt_count, reason FROM media.object_deletion_jobs WHERE id = $1",
    )
    .bind(job_id.parse::<i64>().expect("numeric system deletion job id"))
    .fetch_one(&pool)
    .await
    .expect("requeued system deletion state");
    assert_eq!(job_state, ("queued".into(), 0, "account media purge after recovery window".into()));
    let retry_event: (i64, String) = sqlx::query_as(
        "SELECT actor_id, reason FROM media.object_deletion_job_retry_events WHERE job_id = $1",
    )
    .bind(job_id.parse::<i64>().expect("numeric retry event job id"))
    .fetch_one(&pool)
    .await
    .expect("system deletion retry event");
    assert_eq!(retry_event, (admin_id, retry_reason.into()));
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'media.deletion_job.requeued' \
           AND target_id = $2",
    )
    .bind(admin_id)
    .bind(&job_id)
    .fetch_one(&pool)
    .await
    .expect("system deletion retry audit");
    assert_eq!(audit_count, 1);
    assert!(process_upload_deletion_job(&pool, &SuccessfulObjectStore, upload_id)
        .await
        .expect("complete requeued system deletion"));
    let final_upload_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("completed system deletion upload status");
    assert_eq!(final_upload_status, "blocked");
}

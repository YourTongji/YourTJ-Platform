//! Database integration coverage for upload quarantine ordering.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use media::{
    process_upload_deletion_job, routes_with_object_store, UploadObjectPreview, UploadObjectStore,
};
use sha2::{Digest, Sha256};
use shared::{AppResult, AppState};
use sqlx::PgPool;
use tokio::sync::Notify;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

struct FailingObjectStore;

#[async_trait::async_trait]
impl UploadObjectStore for FailingObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        Err(shared::AppError::BadRequest("simulated OSS failure".into()))
    }

    async fn read_image_for_moderation(
        &self,
        _oss_key: &str,
        expected_content_type: &str,
        expected_bytes: u64,
        max_bytes: u64,
    ) -> AppResult<UploadObjectPreview> {
        assert_eq!(expected_content_type, "image/png");
        assert_eq!(expected_bytes, 10);
        assert!(max_bytes >= expected_bytes);
        Ok(UploadObjectPreview {
            content_type: expected_content_type.into(),
            content_length: expected_bytes,
            image_width: 2,
            image_height: 3,
            body: Body::from(vec![0x89; expected_bytes as usize]),
        })
    }
}

struct SuccessfulObjectStore;

#[async_trait::async_trait]
impl UploadObjectStore for SuccessfulObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
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

struct PausingPreviewObjectStore {
    started: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait::async_trait]
impl UploadObjectStore for PausingPreviewObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        Ok(())
    }

    async fn read_image_for_moderation(
        &self,
        _oss_key: &str,
        expected_content_type: &str,
        expected_bytes: u64,
        _max_bytes: u64,
    ) -> AppResult<UploadObjectPreview> {
        self.started.notify_one();
        self.release.notified().await;
        Ok(UploadObjectPreview {
            content_type: expected_content_type.into(),
            content_length: expected_bytes,
            image_width: 2,
            image_height: 3,
            body: Body::from(vec![0x89; expected_bytes as usize]),
        })
    }
}

fn test_state(pool: PgPool) -> AppState {
    let mut config = shared::Config::from_env().expect("media test config");
    config.oss_region = "cn-shanghai".into();
    config.oss_bucket = "yourtj-test".into();
    config.oss_access_key_id = "test-ak".into();
    config.oss_access_key_secret = "test-secret".into();
    config.oss_role_arn = "acs:ram::1:role/upload".into();
    config.oss_callback_base_url = "https://api.example.test".into();
    AppState {
        db: pool,
        config,
        jwt_secret: "integration-test-secret-32bytes!".into(),
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

async fn admin_session_token(pool: &PgPool, account_id: i64, is_recent: bool) -> String {
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, user_agent, expires_at, \
          recent_authenticated_at, recent_auth_method, recent_auth_credential_version) \
         VALUES ($1, $2, $3, 'media-admin-self-review-test', now() + interval '1 day', \
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
    .expect("insert media admin session");
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await
            .expect("read media admin auth version");
    identity::auth::create_session_access_token(
        account_id,
        session_id,
        auth_version,
        "integration-test-secret-32bytes!",
        3_600,
    )
    .expect("create media admin session token")
}

async fn publish_synthetic_variants(pool: &PgPool, asset_id: i64, label: &str) {
    for (variant_kind, dimension) in
        [("thumb_256", 256_i32), ("display_1280", 1_280_i32), ("full_2048", 2_048_i32)]
    {
        let digest = hex::encode(Sha256::digest(format!("{label}-{variant_kind}").as_bytes()));
        sqlx::query(
            "INSERT INTO media.asset_variants \
             (asset_id, variant_kind, policy_version, object_key, content_sha256, mime, \
              bytes, width, height, status, published_at) \
             VALUES ($1, $2, 1, $3, $4, 'image/webp', 10, $5, $5, 'published', now())",
        )
        .bind(asset_id)
        .bind(variant_kind)
        .bind(format!("assets/{asset_id}/1/{variant_kind}-{digest}.webp"))
        .bind(digest)
        .bind(dimension)
        .execute(pool)
        .await
        .expect("insert synthetic published variant");
    }
    sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'published', published_at = now() WHERE asset_id = $1",
    )
    .bind(asset_id)
    .execute(pool)
    .await
    .expect("publish synthetic variant set");
}

fn request(method: Method, uri: String, token: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .expect("media request")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("media response body");
    serde_json::from_slice(&bytes).expect("media response JSON")
}

#[tokio::test]
async fn moderation_requires_evidence_and_deletion_remains_private_across_retries() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media integration");
    let pool = PgPool::connect(&database_url).await.expect("media test database");
    MIGRATOR.run(&pool).await.expect("media test migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("media-mod-{suffix}@tongji.edu.cn"))
    .bind(format!("media-mod-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed media moderator");
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("media-owner-{suffix}@tongji.edu.cn"))
    .bind(format!("media-owner-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed media owner");
    let upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256) \
         VALUES ($1, 'image', $2, $3, 10, 'image/png', $4) RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}.png"))
    .bind(format!("https://example.invalid/{suffix}.png"))
    .bind("a".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed pending upload");
    let state = test_state(pool.clone());
    let moderator_token =
        identity::auth::create_access_token(moderator_id, &state.jwt_secret, 3600)
            .expect("media moderator token");
    let owner_token = identity::auth::create_access_token(owner_id, &state.jwt_secret, 3600)
        .expect("media owner token");
    let upload_id_string = upload_id.to_string();
    let block_uri = format!("/api/v2/admin/media/uploads/{upload_id}/block");

    let failing_app = routes_with_object_store(state.clone(), Arc::new(FailingObjectStore));
    let queue_response = failing_app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/uploads".into(),
            &moderator_token,
            Body::empty(),
        ))
        .await
        .expect("moderation queue response");
    assert_eq!(queue_response.status(), StatusCode::OK);
    let queue_body = response_json(queue_response).await;
    let listed_upload = queue_body["items"]
        .as_array()
        .expect("moderation queue items")
        .iter()
        .find(|upload| upload["id"].as_str() == Some(&upload_id_string))
        .expect("seed upload in moderation queue");
    assert!(listed_upload.get("ossKey").is_none());
    assert!(listed_upload.get("url").is_none());
    assert!(listed_upload.get("sha256").is_none());

    let owner_preview = failing_app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/me/media/uploads/{upload_id}/preview"),
            &owner_token,
            Body::empty(),
        ))
        .await
        .expect("owner pending preview response");
    assert_eq!(owner_preview.status(), StatusCode::OK);
    assert_eq!(owner_preview.headers()[header::CACHE_CONTROL], "private, no-store, max-age=0");
    assert_eq!(owner_preview.headers()["cross-origin-resource-policy"], "same-origin");
    let cross_owner_preview = failing_app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/me/media/uploads/{upload_id}/preview"),
            &moderator_token,
            Body::empty(),
        ))
        .await
        .expect("cross-owner pending preview response");
    assert_eq!(cross_owner_preview.status(), StatusCode::NOT_FOUND);

    let own_grant_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/preview-grants"),
            &owner_token,
            Body::from(r#"{"reason":"review own upload"}"#),
        ))
        .await
        .expect("owner preview grant response");
    assert_eq!(own_grant_response.status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("promote upload owner for independent-review check");
    let self_review_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/preview-grants"),
            &owner_token,
            Body::from(r#"{"reason":"independent reviewer required"}"#),
        ))
        .await
        .expect("self-review grant response");
    assert_eq!(self_review_response.status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE identity.accounts SET role = 'user' WHERE id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("restore upload owner role");

    let premature_approval = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/approve"),
            &moderator_token,
            Body::from(r#"{"reason":"approve without trusted evidence"}"#),
        ))
        .await
        .expect("approval without preview response");
    assert_eq!(premature_approval.status(), StatusCode::CONFLICT);

    let grant_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/preview-grants"),
            &moderator_token,
            Body::from(r#"{"reason":"inspect image evidence"}"#),
        ))
        .await
        .expect("preview grant response");
    assert_eq!(grant_response.status(), StatusCode::OK);
    assert_eq!(grant_response.headers()[header::CACHE_CONTROL], "private, no-store");
    let grant_body = response_json(grant_response).await;
    let preview_token = grant_body["token"].as_str().expect("one-time preview token");
    assert_eq!(preview_token.len(), 43);
    assert!(grant_body.get("ossKey").is_none());
    assert!(grant_body.get("url").is_none());
    assert!(grant_body.get("sha256").is_none());
    let wrong_moderator_response = failing_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/admin/media/uploads/{upload_id}/preview"))
                .header(header::AUTHORIZATION, format!("Bearer {owner_token}"))
                .header("x-media-preview-token", preview_token)
                .body(Body::empty())
                .expect("wrong-moderator preview request"),
        )
        .await
        .expect("wrong-moderator preview response");
    assert_eq!(wrong_moderator_response.status(), StatusCode::FORBIDDEN);
    let preview_started = Arc::new(Notify::new());
    let preview_release = Arc::new(Notify::new());
    let pausing_preview_app = routes_with_object_store(
        state.clone(),
        Arc::new(PausingPreviewObjectStore {
            started: preview_started.clone(),
            release: preview_release.clone(),
        }),
    );
    let preview_uri = format!("/api/v2/admin/media/uploads/{upload_id}/preview");
    let preview_auth = format!("Bearer {moderator_token}");
    let preview_token_owned = preview_token.to_owned();
    let preview_task = tokio::spawn(async move {
        pausing_preview_app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(preview_uri)
                    .header(header::AUTHORIZATION, preview_auth)
                    .header("x-media-preview-token", preview_token_owned)
                    .body(Body::empty())
                    .expect("preview request"),
            )
            .await
            .expect("preview response")
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), preview_started.notified())
        .await
        .expect("provider preview started");
    let mut preview_lock_probe = pool.begin().await.expect("preview lock probe transaction");
    sqlx::query("SELECT id FROM media.uploads WHERE id = $1 FOR UPDATE NOWAIT")
        .bind(upload_id)
        .fetch_one(&mut *preview_lock_probe)
        .await
        .expect("upload is not locked across preview provider I/O");
    sqlx::query(
        "SELECT id FROM media.moderation_preview_grants \
         WHERE upload_id = $1 AND moderator_account_id = $2 \
         ORDER BY id DESC LIMIT 1 FOR UPDATE NOWAIT",
    )
    .bind(upload_id)
    .bind(moderator_id)
    .fetch_one(&mut *preview_lock_probe)
    .await
    .expect("preview grant is not locked across provider I/O");
    preview_lock_probe.commit().await.expect("release preview lock probe");
    preview_release.notify_one();
    let preview_response = preview_task.await.expect("preview task");
    assert_eq!(preview_response.status(), StatusCode::OK);
    assert_eq!(preview_response.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(preview_response.headers()[header::CACHE_CONTROL], "private, no-store, max-age=0");
    assert_eq!(preview_response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(
        to_bytes(preview_response.into_body(), usize::MAX).await.expect("preview bytes").len(),
        10
    );
    let replay_response = failing_app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/admin/media/uploads/{upload_id}/preview"))
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .header("x-media-preview-token", preview_token)
                .body(Body::empty())
                .expect("replay preview request"),
        )
        .await
        .expect("replay preview response");
    assert_eq!(replay_response.status(), StatusCode::NOT_FOUND);
    let (preview_reason, preview_metadata): (String, serde_json::Value) = sqlx::query_as(
        "SELECT reason, metadata FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'media.upload.previewed' \
           AND target_type = 'upload' AND target_id = $2",
    )
    .bind(moderator_id)
    .bind(&upload_id_string)
    .fetch_one(&pool)
    .await
    .expect("preview audit event");
    assert_eq!(preview_reason, "inspect image evidence");
    assert_eq!(preview_metadata["purpose"], "moderation_review");
    assert_eq!(preview_metadata["imageWidth"], 2);
    assert_eq!(preview_metadata["imageHeight"], 3);
    assert!(preview_metadata.get("ossKey").is_none());
    assert!(preview_metadata.get("url").is_none());
    let stored_dimensions: (i32, i32) =
        sqlx::query_as("SELECT image_width, image_height FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("trusted preview dimensions");
    assert_eq!(stored_dimensions, (2, 3));

    let approval_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/approve"),
            &moderator_token,
            Body::from(r#"{"reason":"reviewed image is acceptable"}"#),
        ))
        .await
        .expect("evidence-backed approval response");
    assert_eq!(approval_response.status(), StatusCode::OK);
    let approved_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("status after approval");
    assert_eq!(approved_status, "clean");

    let file_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256) \
         VALUES ($1, 'file', $2, $3, 20, 'application/pdf', $4) RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/file/{suffix}.pdf"))
    .bind(format!("https://example.invalid/{suffix}.pdf"))
    .bind("d".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed pending PDF");
    let file_approval_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{file_upload_id}/approve"),
            &moderator_token,
            Body::from(r#"{"reason":"attempt manual PDF approval"}"#),
        ))
        .await
        .expect("manual PDF approval response");
    assert_eq!(file_approval_response.status(), StatusCode::CONFLICT);

    let queued_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            block_uri.clone(),
            &moderator_token,
            Body::from(r#"{"reason":"confirmed malicious upload"}"#),
        ))
        .await
        .expect("quarantine response");
    assert_eq!(queued_response.status(), StatusCode::ACCEPTED);
    let quarantined_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("status after durable quarantine");
    assert_eq!(quarantined_status, "quarantined");
    let quarantined_url_response = failing_app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/media/{upload_id}/url"),
            &owner_token,
            Body::empty(),
        ))
        .await
        .expect("quarantined URL response");
    assert_eq!(quarantined_url_response.status(), StatusCode::NOT_FOUND);

    assert!(process_upload_deletion_job(&pool, &FailingObjectStore, upload_id)
        .await
        .expect("failed provider deletion is recorded"));
    let (status_after_failure, job_status): (String, String) = sqlx::query_as(
        "SELECT upload.status, job.status \
         FROM media.uploads upload \
         JOIN media.object_deletion_jobs job ON job.upload_id = upload.id \
         WHERE upload.id = $1",
    )
    .bind(upload_id)
    .fetch_one(&pool)
    .await
    .expect("durable deletion failure state");
    assert_eq!(status_after_failure, "quarantined");
    assert_eq!(job_status, "queued");
    sqlx::query(
        "UPDATE media.object_deletion_jobs \
         SET attempt_count = 7, available_at = now() \
         WHERE upload_id = $1",
    )
    .bind(upload_id)
    .execute(&pool)
    .await
    .expect("advance provider retry budget");
    sqlx::query(
        "UPDATE media.object_cleanup_steps SET available_at = now() \
         WHERE deletion_job_id = (SELECT id FROM media.object_deletion_jobs WHERE upload_id = $1)",
    )
    .bind(upload_id)
    .execute(&pool)
    .await
    .expect("advance cleanup-step retry window");
    assert!(process_upload_deletion_job(&pool, &FailingObjectStore, upload_id)
        .await
        .expect("final provider failure is recorded"));
    let (dead_letter_status, exhausted_attempts): (String, i32) = sqlx::query_as(
        "SELECT status, attempt_count FROM media.object_deletion_jobs WHERE upload_id = $1",
    )
    .bind(upload_id)
    .fetch_one(&pool)
    .await
    .expect("dead-letter deletion state");
    assert_eq!(dead_letter_status, "dead_letter");
    assert_eq!(exhausted_attempts, 8);
    let requeue_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            block_uri,
            &moderator_token,
            Body::from(r#"{"reason":"retry deletion after operator review"}"#),
        ))
        .await
        .expect("manual deletion requeue response");
    assert_eq!(requeue_response.status(), StatusCode::ACCEPTED);
    let (requeued_status, requeued_attempts): (String, i32) = sqlx::query_as(
        "SELECT status, attempt_count FROM media.object_deletion_jobs WHERE upload_id = $1",
    )
    .bind(upload_id)
    .fetch_one(&pool)
    .await
    .expect("manual deletion requeue state");
    assert_eq!(requeued_status, "queued");
    assert_eq!(requeued_attempts, 0);

    assert!(process_upload_deletion_job(&pool, &SuccessfulObjectStore, upload_id)
        .await
        .expect("provider deletion retry succeeds"));
    let blocked_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("status after successful quarantine");
    assert_eq!(blocked_status, "blocked");
    let blocked_url_response =
        routes_with_object_store(state.clone(), Arc::new(SuccessfulObjectStore))
            .oneshot(request(
                Method::GET,
                format!("/api/v2/media/{upload_id}/url"),
                &owner_token,
                Body::empty(),
            ))
            .await
            .expect("blocked URL response");
    assert_eq!(blocked_url_response.status(), StatusCode::NOT_FOUND);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'media.upload.blocked' \
           AND target_type = 'upload' AND target_id = $2",
    )
    .bind(moderator_id)
    .bind(upload_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("quarantine audit count");
    assert_eq!(audit_count, 1);
    let quarantined_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'media.upload.quarantined' \
           AND target_type = 'upload' AND target_id = $2",
    )
    .bind(moderator_id)
    .bind(upload_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("quarantine scheduling audit count");
    assert_eq!(quarantined_audit_count, 1);

    sqlx::query("DELETE FROM governance.audit_events WHERE actor_account_id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM media.uploads WHERE id = $1")
        .bind(upload_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM media.uploads WHERE id = $1")
        .bind(file_upload_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM identity.accounts WHERE id = ANY($1)")
        .bind(vec![moderator_id, owner_id])
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn admin_own_media_review_requires_recent_auth_confirmation_and_audited_evidence() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media integration");
    let pool = PgPool::connect(&database_url).await.expect("media test database");
    MIGRATOR.run(&pool).await.expect("media test migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("media-self-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("media-self-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert media self-review admin");
    let upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256) \
         VALUES ($1, 'image', $2, '', 10, 'image/png', $3) RETURNING id",
    )
    .bind(admin_id)
    .bind(format!("uploads/{admin_id}/image/{suffix}.png"))
    .bind("a".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("insert admin pending media");
    let recent_token = admin_session_token(&pool, admin_id, true).await;
    let stale_token = admin_session_token(&pool, admin_id, false).await;
    let app = routes_with_object_store(test_state(pool.clone()), Arc::new(FailingObjectStore));

    let queue = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/uploads".into(),
            &recent_token,
            Body::empty(),
        ))
        .await
        .expect("ADMIN own-media queue");
    assert_eq!(queue.status(), StatusCode::OK);
    let queue_body = response_json(queue).await;
    let own_item = queue_body["items"]
        .as_array()
        .expect("ADMIN moderation items")
        .iter()
        .find(|item| item["id"].as_str() == Some(&upload_id.to_string()))
        .expect("ADMIN own upload in queue");
    assert_eq!(own_item["isSelfReview"], true);

    let unconfirmed = app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/preview-grants"),
            &recent_token,
            Body::from(r#"{"reason":"inspect own media"}"#),
        ))
        .await
        .expect("unconfirmed ADMIN preview grant");
    assert_eq!(unconfirmed.status(), StatusCode::FORBIDDEN);
    let stale = app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/preview-grants"),
            &stale_token,
            Body::from(r#"{"reason":"inspect own media","selfReviewConfirmed":true}"#),
        ))
        .await
        .expect("stale ADMIN preview grant");
    assert_eq!(stale.status(), StatusCode::PRECONDITION_REQUIRED);
    for action in ["approve", "block"] {
        let stale_action = app
            .clone()
            .oneshot(request(
                Method::POST,
                format!("/api/v2/admin/media/uploads/{upload_id}/{action}"),
                &stale_token,
                Body::from(format!(
                    r#"{{"reason":"stale own media {action}","selfReviewConfirmed":true}}"#,
                )),
            ))
            .await
            .expect("stale ADMIN moderation action");
        assert_eq!(stale_action.status(), StatusCode::PRECONDITION_REQUIRED);
    }
    let grant = app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/preview-grants"),
            &recent_token,
            Body::from(r#"{"reason":"inspect own media","selfReviewConfirmed":true}"#),
        ))
        .await
        .expect("confirmed ADMIN preview grant");
    assert_eq!(grant.status(), StatusCode::OK);
    let grant_body = response_json(grant).await;
    let preview_token = grant_body["token"].as_str().expect("ADMIN preview token");
    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/admin/media/uploads/{upload_id}/preview"))
                .header(header::AUTHORIZATION, format!("Bearer {recent_token}"))
                .header("x-media-preview-token", preview_token)
                .body(Body::empty())
                .expect("ADMIN self-preview request"),
        )
        .await
        .expect("ADMIN self-preview response");
    assert_eq!(preview.status(), StatusCode::OK);
    let approval = app
        .clone()
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/approve"),
            &recent_token,
            Body::from(r#"{"reason":"approve own reviewed media","selfReviewConfirmed":true}"#),
        ))
        .await
        .expect("ADMIN own-media approval");
    assert_eq!(approval.status(), StatusCode::OK);
    let (grant_self_review, evidence_self_review): (bool, bool) = sqlx::query_as(
        "SELECT preview.self_review, evidence.self_review \
         FROM media.moderation_preview_grants preview \
         JOIN media.moderation_evidence evidence ON evidence.upload_id = preview.upload_id \
         WHERE preview.upload_id = $1 AND preview.moderator_account_id = $2 \
         ORDER BY preview.id DESC, evidence.id DESC LIMIT 1",
    )
    .bind(upload_id)
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("persisted self-review evidence");
    assert!(grant_self_review);
    assert!(evidence_self_review);
    let audit_flags: Vec<bool> = sqlx::query_scalar(
        "SELECT (metadata->>'selfReview')::boolean FROM governance.audit_events \
         WHERE actor_account_id = $1 AND target_type = 'upload' AND target_id = $2 \
           AND action IN ('media.upload.previewed', 'media.upload.approved') \
         ORDER BY id",
    )
    .bind(admin_id)
    .bind(upload_id.to_string())
    .fetch_all(&pool)
    .await
    .expect("ADMIN self-review audit events");
    assert_eq!(audit_flags, vec![true, true]);
}

#[tokio::test]
async fn operations_retry_requires_recent_auth_and_atomically_requeues_only_failed_delivery() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media integration");
    let pool = PgPool::connect(&database_url).await.expect("media test database");
    MIGRATOR.run(&pool).await.expect("media test migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("media-processing-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("media-processing-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert media processing administrator");
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("media-processing-owner-{suffix}@tongji.edu.cn"))
    .bind(format!("media-processing-owner-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert media processing owner");
    let upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, '', 10, 'image/png', repeat('a', 64), 'clean') \
         RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}.png"))
    .fetch_one(&pool)
    .await
    .expect("insert failed Delivery upload");
    sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'failed', last_error_code = 'delivery_write_failed' \
         WHERE asset_id = $1",
    )
    .bind(upload_id)
    .execute(&pool)
    .await
    .expect("mark Delivery publication failed");
    sqlx::query(
        "INSERT INTO media.variant_processing_jobs \
         (asset_id, policy_version, status, attempt_count, last_error_code) \
         VALUES ($1, 1, 'dead_letter', 8, 'delivery_write_failed')",
    )
    .bind(upload_id)
    .execute(&pool)
    .await
    .expect("insert dead-lettered Delivery job");
    let recent_admin_token = admin_session_token(&pool, admin_id, true).await;
    let stale_admin_token = admin_session_token(&pool, admin_id, false).await;
    let owner_token = admin_session_token(&pool, owner_id, true).await;
    let app = routes_with_object_store(test_state(pool.clone()), Arc::new(SuccessfulObjectStore));
    let endpoint = format!("/api/v2/admin/media/uploads/{upload_id}/processing/retry");

    let forbidden = app
        .clone()
        .oneshot(request(
            Method::POST,
            endpoint.clone(),
            &owner_token,
            Body::from(r#"{"reason":"retry reviewed Delivery failure"}"#),
        ))
        .await
        .expect("non-operations retry response");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let invalid_reason = app
        .clone()
        .oneshot(request(
            Method::POST,
            endpoint.clone(),
            &recent_admin_token,
            Body::from(r#"{"reason":""}"#),
        ))
        .await
        .expect("empty retry reason response");
    assert_eq!(invalid_reason.status(), StatusCode::BAD_REQUEST);
    let stale = app
        .clone()
        .oneshot(request(
            Method::POST,
            endpoint.clone(),
            &stale_admin_token,
            Body::from(r#"{"reason":"retry reviewed Delivery failure"}"#),
        ))
        .await
        .expect("stale operations retry response");
    assert_eq!(stale.status(), StatusCode::PRECONDITION_REQUIRED);

    let failed_queue = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/uploads?status=clean&limit=100".into(),
            &recent_admin_token,
            Body::empty(),
        ))
        .await
        .expect("failed Delivery moderation response");
    assert_eq!(failed_queue.status(), StatusCode::OK);
    let failed_body = response_json(failed_queue).await;
    let failed_item = failed_body["items"]
        .as_array()
        .expect("failed Delivery moderation items")
        .iter()
        .find(|item| item["id"].as_str() == Some(&upload_id.to_string()))
        .expect("failed Delivery upload in moderation queue");
    assert_eq!(failed_item["deliveryState"], "failed");
    assert_eq!(failed_item["deliveryErrorCode"], "delivery_write_failed");

    let accepted = app
        .clone()
        .oneshot(request(
            Method::POST,
            endpoint.clone(),
            &recent_admin_token,
            Body::from(r#"{"reason":"retry reviewed Delivery failure"}"#),
        ))
        .await
        .expect("accepted operations retry response");
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    let state: (String, String, i32, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT publication.status, job.status, job.attempt_count, \
                publication.last_error_code, job.last_error_code \
         FROM media.asset_publications publication \
         JOIN media.variant_processing_jobs job \
           ON job.asset_id = publication.asset_id \
          AND job.policy_version = publication.policy_version \
         WHERE publication.asset_id = $1",
    )
    .bind(upload_id)
    .fetch_one(&pool)
    .await
    .expect("atomically requeued Delivery state");
    assert_eq!(state, ("processing".into(), "queued".into(), 0, None, None));
    let audit: (String, i64) = sqlx::query_as(
        "SELECT reason, (metadata->>'previousAttemptCount')::bigint \
         FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'media.asset.processing_requeued' \
           AND target_type = 'upload' AND target_id = $2",
    )
    .bind(admin_id)
    .bind(upload_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("processing retry audit event");
    assert_eq!(audit, ("retry reviewed Delivery failure".into(), 8));

    let owner_state = app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/me/media/uploads/{upload_id}"),
            &owner_token,
            Body::empty(),
        ))
        .await
        .expect("owner Delivery state response");
    assert_eq!(owner_state.status(), StatusCode::OK);
    assert_eq!(response_json(owner_state).await["deliveryState"], "processing");
    let moderation_queue = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/admin/media/uploads?status=clean&limit=100".into(),
            &recent_admin_token,
            Body::empty(),
        ))
        .await
        .expect("moderation Delivery state response");
    assert_eq!(moderation_queue.status(), StatusCode::OK);
    let moderation_body = response_json(moderation_queue).await;
    let item = moderation_body["items"]
        .as_array()
        .expect("moderation Delivery items")
        .iter()
        .find(|item| item["id"].as_str() == Some(&upload_id.to_string()))
        .expect("failed Delivery upload in moderation queue");
    assert_eq!(item["deliveryState"], "processing");
    assert_eq!(item["deliveryErrorCode"], serde_json::Value::Null);

    let duplicate = app
        .oneshot(request(
            Method::POST,
            endpoint,
            &recent_admin_token,
            Body::from(r#"{"reason":"duplicate Delivery retry"}"#),
        ))
        .await
        .expect("duplicate retry response");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn moderation_queue_applies_role_hierarchy_before_pagination() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media integration");
    let pool = PgPool::connect(&database_url).await.expect("media test database");
    MIGRATOR.run(&pool).await.expect("media test migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let actor_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("media-role-actor-{suffix}@tongji.edu.cn"))
    .bind(format!("media-role-actor-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed moderator");
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'user') RETURNING id",
    )
    .bind(format!("media-role-user-{suffix}@tongji.edu.cn"))
    .bind(format!("media-role-user-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed user owner");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deactivated', deactivated_at = now() WHERE id = $1",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("preserve moderation of retained content for a deactivated owner");
    let peer_moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("media-role-peer-{suffix}@tongji.edu.cn"))
    .bind(format!("media-role-peer-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed peer moderator");
    let administrator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("media-role-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("media-role-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed administrator");
    let peer_administrator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("media-role-peer-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("media-role-peer-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed peer administrator");

    let insert_upload = |account_id: i64, label: &'static str| {
        let pool = pool.clone();
        let suffix = suffix.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "INSERT INTO media.uploads \
                 (account_id, kind, oss_key, url, bytes, mime, sha256) \
                 VALUES ($1, 'image', $2, $3, 10, 'image/png', $4) RETURNING id",
            )
            .bind(account_id)
            .bind(format!("uploads/{account_id}/image/{suffix}-{label}.png"))
            .bind(format!("https://example.invalid/{suffix}-{label}.png"))
            .bind("b".repeat(64))
            .fetch_one(&pool)
            .await
            .expect("seed hierarchy upload")
        }
    };
    let user_upload_id = insert_upload(user_id, "user").await;
    let older_user_upload_id = insert_upload(user_id, "older-user").await;
    let peer_upload_id = insert_upload(peer_moderator_id, "peer").await;
    let admin_upload_id = insert_upload(administrator_id, "admin").await;
    let self_upload_id = insert_upload(actor_id, "self").await;
    let peer_administrator_upload_ids: Vec<i64> = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, created_at) \
         SELECT $1, 'image', \
                'uploads/' || $1::text || '/image/' || $2 || '-admin-' || sequence::text || '.png', \
                'https://example.invalid/' || $2 || '-admin-' || sequence::text || '.png', \
                10, 'image/png', repeat('c', 64), \
                now() + interval '10 days' + sequence * interval '1 second' \
         FROM generate_series(1, 201) sequence RETURNING id",
    )
    .bind(peer_administrator_id)
    .bind(&suffix)
    .fetch_all(&pool)
    .await
    .expect("seed more than one candidate batch of higher-role media");
    sqlx::query(
        "UPDATE media.uploads SET created_at = CASE id \
           WHEN $1 THEN now() + interval '2 days' \
           WHEN $2 THEN now() + interval '1 day' \
           WHEN $3 THEN now() + interval '5 days' \
           WHEN $4 THEN now() + interval '4 days' \
           WHEN $5 THEN now() + interval '3 days' END \
         WHERE id = ANY($6)",
    )
    .bind(user_upload_id)
    .bind(older_user_upload_id)
    .bind(peer_upload_id)
    .bind(admin_upload_id)
    .bind(self_upload_id)
    .bind(vec![
        user_upload_id,
        older_user_upload_id,
        peer_upload_id,
        admin_upload_id,
        self_upload_id,
    ])
    .execute(&pool)
    .await
    .expect("isolate hierarchy pagination order from concurrent fixtures");

    let state = test_state(pool.clone());
    let moderator_token = identity::auth::create_access_token(actor_id, &state.jwt_secret, 3600)
        .expect("moderator token");
    let administrator_token =
        identity::auth::create_access_token(administrator_id, &state.jwt_secret, 3600)
            .expect("administrator token");
    let app = routes_with_object_store(state, Arc::new(FailingObjectStore));

    let mut moderator_cursor: Option<String> = None;
    let mut moderator_cursors = std::collections::HashSet::new();
    let mut moderator_ids = Vec::new();
    let mut empty_pages = 0;
    for _ in 0..8 {
        let uri = moderator_cursor.as_ref().map_or_else(
            || "/api/v2/admin/media/uploads?status=pending&limit=1".to_owned(),
            |cursor| format!("/api/v2/admin/media/uploads?status=pending&limit=1&cursor={cursor}"),
        );
        let moderator_queue = app
            .clone()
            .oneshot(request(Method::GET, uri, &moderator_token, Body::empty()))
            .await
            .expect("moderator queue response");
        assert_eq!(moderator_queue.status(), StatusCode::OK);
        let moderator_body = response_json(moderator_queue).await;
        let page_items = moderator_body["items"].as_array().expect("moderator queue items");
        if page_items.is_empty() {
            empty_pages += 1;
        }
        for item in page_items {
            let upload_id = item["id"].as_str().expect("moderator upload id").to_owned();
            assert_ne!(upload_id, peer_upload_id.to_string());
            assert_ne!(upload_id, admin_upload_id.to_string());
            assert_ne!(upload_id, self_upload_id.to_string());
            assert!(!peer_administrator_upload_ids
                .iter()
                .any(|peer_id| peer_id.to_string() == upload_id));
            moderator_ids.push(upload_id);
        }
        if moderator_ids.len() >= 2 {
            break;
        }
        moderator_cursor = moderator_body["nextCursor"].as_str().map(str::to_owned);
        assert!(moderator_cursor.is_some(), "bounded scan must advance until eligible media");
        assert!(
            moderator_cursors.insert(moderator_cursor.clone()),
            "bounded scan cursor must make forward progress"
        );
    }
    assert!(empty_pages >= 2, "the fixture must cross multiple bounded empty pages");
    assert_eq!(
        &moderator_ids[..2],
        &[user_upload_id.to_string(), older_user_upload_id.to_string()]
    );

    let mut administrator_cursor: Option<String> = None;
    let mut administrator_items = Vec::new();
    loop {
        let uri = administrator_cursor.as_ref().map_or_else(
            || "/api/v2/admin/media/uploads?status=pending&limit=100".to_owned(),
            |cursor| {
                format!("/api/v2/admin/media/uploads?status=pending&limit=100&cursor={cursor}")
            },
        );
        let administrator_queue = app
            .clone()
            .oneshot(request(Method::GET, uri, &administrator_token, Body::empty()))
            .await
            .expect("administrator queue response");
        assert_eq!(administrator_queue.status(), StatusCode::OK);
        let administrator_body = response_json(administrator_queue).await;
        administrator_items.extend(
            administrator_body["items"]
                .as_array()
                .expect("administrator queue items")
                .iter()
                .cloned(),
        );
        administrator_cursor = administrator_body["nextCursor"].as_str().map(str::to_owned);
        if administrator_cursor.is_none() {
            break;
        }
    }
    let administrator_ids = administrator_items
        .iter()
        .filter_map(|item| item["id"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    assert!(administrator_ids.contains(&user_upload_id.to_string()));
    assert!(administrator_ids.contains(&older_user_upload_id.to_string()));
    assert!(administrator_ids.contains(&peer_upload_id.to_string()));
    assert!(administrator_ids.contains(&admin_upload_id.to_string()));
    assert!(administrator_ids.contains(&self_upload_id.to_string()));
    assert!(peer_administrator_upload_ids
        .iter()
        .all(|peer_id| !administrator_ids.contains(&peer_id.to_string())));
    let admin_upload_id_text = admin_upload_id.to_string();
    assert_eq!(
        administrator_items
            .iter()
            .find(|item| item["id"].as_str() == Some(admin_upload_id_text.as_str()))
            .expect("ADMIN own media item")["isSelfReview"],
        true
    );

    let peer_preview = app
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{peer_upload_id}/preview-grants"),
            &moderator_token,
            Body::from(r#"{"reason":"peer moderation is forbidden"}"#),
        ))
        .await
        .expect("peer preview response");
    assert_eq!(peer_preview.status(), StatusCode::FORBIDDEN);

    sqlx::query("DELETE FROM media.uploads WHERE id = ANY($1)")
        .bind(vec![
            user_upload_id,
            older_user_upload_id,
            peer_upload_id,
            admin_upload_id,
            self_upload_id,
        ])
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM media.uploads WHERE id = ANY($1)")
        .bind(&peer_administrator_upload_ids)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM identity.accounts WHERE id = ANY($1)")
        .bind(vec![actor_id, user_id, peer_moderator_id, administrator_id, peer_administrator_id])
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn provider_deletion_does_not_hold_upload_or_job_locks() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media integration");
    let pool = PgPool::connect(&database_url).await.expect("media test database");
    MIGRATOR.run(&pool).await.expect("media test migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("media-lock-mod-{suffix}@tongji.edu.cn"))
    .bind(format!("media-lock-mod-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed lock test moderator");
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("media-lock-owner-{suffix}@tongji.edu.cn"))
    .bind(format!("media-lock-owner-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed lock test owner");
    let upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, $3, 10, 'image/png', $4, 'clean') RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}.png"))
    .bind(format!("https://example.invalid/{suffix}.png"))
    .bind("c".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed lock test upload");
    let state = test_state(pool.clone());
    let moderator_token =
        identity::auth::create_access_token(moderator_id, &state.jwt_secret, 3600)
            .expect("lock test moderator token");
    let app = routes_with_object_store(state, Arc::new(SuccessfulObjectStore));
    let quarantine_response = app
        .oneshot(request(
            Method::POST,
            format!("/api/v2/admin/media/uploads/{upload_id}/block"),
            &moderator_token,
            Body::from(r#"{"reason":"remove published unsafe image"}"#),
        ))
        .await
        .expect("lock test quarantine response");
    assert_eq!(quarantine_response.status(), StatusCode::ACCEPTED);

    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let object_store =
        Arc::new(PausingObjectStore { started: started.clone(), release: release.clone() });
    let worker_pool = pool.clone();
    let worker_store = object_store.clone();
    let worker = tokio::spawn(async move {
        process_upload_deletion_job(&worker_pool, worker_store.as_ref(), upload_id).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), started.notified())
        .await
        .expect("provider deletion started");

    let mut lock_probe = pool.begin().await.expect("lock probe transaction");
    let upload_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1 FOR UPDATE NOWAIT")
            .bind(upload_id)
            .fetch_one(&mut *lock_probe)
            .await
            .expect("upload is not locked across provider I/O");
    let job_status: String = sqlx::query_scalar(
        "SELECT status FROM media.object_deletion_jobs \
         WHERE upload_id = $1 FOR UPDATE NOWAIT",
    )
    .bind(upload_id)
    .fetch_one(&mut *lock_probe)
    .await
    .expect("deletion job is not locked across provider I/O");
    assert_eq!(upload_status, "quarantined");
    assert_eq!(job_status, "leased");
    lock_probe.commit().await.expect("release lock probe");
    let invalid_restore = sqlx::query("UPDATE media.uploads SET status = 'clean' WHERE id = $1")
        .bind(upload_id)
        .execute(&pool)
        .await
        .expect_err("quarantined upload cannot be republished while deletion is leased");
    assert_eq!(
        invalid_restore.as_database_error().and_then(|error| error.code()).as_deref(),
        Some("23514")
    );

    release.notify_one();
    assert!(worker.await.expect("deletion worker task").expect("deletion worker result"));
    let final_status: String = sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
        .bind(upload_id)
        .fetch_one(&pool)
        .await
        .expect("final upload status");
    assert_eq!(final_status, "blocked");

    sqlx::query("DELETE FROM governance.audit_events WHERE actor_account_id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM media.uploads WHERE id = $1")
        .bind(upload_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM identity.accounts WHERE id = ANY($1)")
        .bind(vec![moderator_id, owner_id])
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn profile_images_require_an_owned_clean_oss_asset() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL for media integration");
    let pool = PgPool::connect(&database_url).await.expect("media test database");
    MIGRATOR.run(&pool).await.expect("media test migrations");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let owner_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("profile-media-owner-{suffix}@tongji.edu.cn"))
    .bind(format!("profile-media-owner-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed profile media owner");
    let other_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("profile-media-other-{suffix}@tongji.edu.cn"))
    .bind(format!("profile-media-other-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed other media owner");
    let clean_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, usage) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4, 'clean', 'profile_avatar') \
         RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-clean.png"))
    .bind(format!("https://cdn.example.test/{suffix}-clean.png"))
    .bind("b".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed clean profile image");
    let replacement_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, usage) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4, 'clean', 'profile_avatar') \
         RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-replacement.png"))
    .bind(format!("https://cdn.example.test/{suffix}-replacement.png"))
    .bind("e".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed replacement profile image");
    let pending_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, usage) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4, 'profile_avatar') RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-pending.png"))
    .bind(format!("https://cdn.example.test/{suffix}-pending.png"))
    .bind("c".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed pending profile image");
    let other_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, usage) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4, 'clean', 'profile_avatar') \
         RETURNING id",
    )
    .bind(other_id)
    .bind(format!("uploads/{other_id}/image/{suffix}-other.png"))
    .bind(format!("https://cdn.example.test/{suffix}-other.png"))
    .bind("d".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed other profile image");
    publish_synthetic_variants(&pool, clean_upload_id, "clean-profile").await;
    publish_synthetic_variants(&pool, replacement_upload_id, "replacement-profile").await;
    publish_synthetic_variants(&pool, other_upload_id, "other-profile").await;
    let state = test_state(pool.clone());
    let token = identity::auth::create_access_token(owner_id, &state.jwt_secret, 3600)
        .expect("profile media token");
    let app = routes_with_object_store(state, Arc::new(SuccessfulObjectStore));

    let foreign_delivery_response = app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/media/{other_upload_id}/url"),
            &token,
            Body::empty(),
        ))
        .await
        .expect("foreign Delivery URL response");
    assert_eq!(foreign_delivery_response.status(), StatusCode::NOT_FOUND);

    let list_response = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/v2/me/media/uploads?usage=profile_avatar&limit=10".into(),
            &token,
            Body::empty(),
        ))
        .await
        .expect("owned upload list response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let items = list_json["items"].as_array().expect("owned upload items");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], pending_upload_id.to_string());
    assert_eq!(items[0]["status"], "pending");
    assert_eq!(items[0]["usage"], "profile_avatar");
    for sensitive_field in ["accountId", "ossKey", "url", "sha256"] {
        assert!(items[0].get(sensitive_field).is_none());
    }

    let status_response = app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/me/media/uploads/{pending_upload_id}"),
            &token,
            Body::empty(),
        ))
        .await
        .expect("owned upload status response");
    assert_eq!(status_response.status(), StatusCode::OK);
    assert_eq!(response_json(status_response).await["status"], "pending");

    let foreign_status_response = app
        .clone()
        .oneshot(request(
            Method::GET,
            format!("/api/v2/me/media/uploads/{other_upload_id}"),
            &token,
            Body::empty(),
        ))
        .await
        .expect("foreign upload status response");
    assert_eq!(foreign_status_response.status(), StatusCode::NOT_FOUND);

    for rejected_id in [pending_upload_id, other_upload_id] {
        let response = app
            .clone()
            .oneshot(request(
                Method::PUT,
                "/api/v2/me/profile/avatar".into(),
                &token,
                Body::from(format!(r#"{{"assetId":"{rejected_id}"}}"#)),
            ))
            .await
            .expect("rejected profile bind response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let bind_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/profile/avatar".into(),
            &token,
            Body::from(format!(r#"{{"assetId":"{clean_upload_id}"}}"#)),
        ))
        .await
        .expect("profile bind response");
    assert_eq!(bind_response.status(), StatusCode::NO_CONTENT);
    let stored_asset_id: Option<i64> =
        sqlx::query_scalar("SELECT avatar_asset_id FROM identity.profiles WHERE account_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("stored avatar asset");
    assert_eq!(stored_asset_id, Some(clean_upload_id));
    let active_binding: (i64, i64, String) = sqlx::query_as(
        "SELECT asset_id, owner_account_id, target_type FROM media.asset_bindings \
         WHERE target_type = 'profile_avatar' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("active profile media binding");
    assert_eq!(active_binding, (clean_upload_id, owner_id, "profile_avatar".into()));

    let replace_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/profile/avatar".into(),
            &token,
            Body::from(format!(r#"{{"assetId":"{replacement_upload_id}"}}"#)),
        ))
        .await
        .expect("profile replacement response");
    assert_eq!(replace_response.status(), StatusCode::NO_CONTENT);
    let binding_lifecycle: Vec<(i64, Option<String>, bool)> = sqlx::query_as(
        "SELECT asset_id, detached_reason, \
                detached_at IS NULL OR gc_eligible_at > detached_at \
         FROM media.asset_bindings WHERE target_type = 'profile_avatar' AND target_id = $1 \
         ORDER BY id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("profile replacement binding lifecycle");
    assert_eq!(
        binding_lifecycle,
        vec![(clean_upload_id, Some("replaced".into()), true), (replacement_upload_id, None, true)]
    );

    let clear_response = app
        .clone()
        .oneshot(request(Method::DELETE, "/api/v2/me/profile/avatar".into(), &token, Body::empty()))
        .await
        .expect("profile clear response");
    assert_eq!(clear_response.status(), StatusCode::NO_CONTENT);
    let cleared_asset_id: Option<i64> =
        sqlx::query_scalar("SELECT avatar_asset_id FROM identity.profiles WHERE account_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("cleared avatar asset");
    assert!(cleared_asset_id.is_none());
    let detached_binding: (String, bool) = sqlx::query_as(
        "SELECT detached_reason, gc_eligible_at > now() FROM media.asset_bindings \
         WHERE target_type = 'profile_avatar' AND target_id = $1 AND asset_id = $2",
    )
    .bind(owner_id)
    .bind(replacement_upload_id)
    .fetch_one(&pool)
    .await
    .expect("detached profile media binding");
    assert_eq!(detached_binding, ("cleared".into(), true));

    let mut deletion_transaction = pool.begin().await.expect("begin profile deletion race");
    sqlx::query(
        "UPDATE identity.accounts \
         SET status = 'deleted', deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 day', deleted_at = now(), \
             lifecycle_version = lifecycle_version + 1 \
         WHERE id = $1",
    )
    .bind(owner_id)
    .execute(&mut *deletion_transaction)
    .await
    .expect("stage profile owner deletion");
    let race_app = app.clone();
    let race_token = token.clone();
    let race_task = tokio::spawn(async move {
        race_app
            .oneshot(request(
                Method::PUT,
                "/api/v2/me/profile/avatar".into(),
                &race_token,
                Body::from(format!(r#"{{"assetId":"{clean_upload_id}"}}"#)),
            ))
            .await
            .expect("profile mutation waiting behind account deletion")
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!race_task.is_finished(), "profile mutation should wait on account lifecycle lock");
    deletion_transaction.commit().await.expect("commit profile owner deletion");
    let raced_response = tokio::time::timeout(std::time::Duration::from_secs(3), race_task)
        .await
        .expect("profile mutation completes after account deletion")
        .expect("join profile deletion race");
    assert_eq!(raced_response.status(), StatusCode::FORBIDDEN);
    let active_after_deletion: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.asset_bindings \
         WHERE target_type = 'profile_avatar' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("active profile binding after account deletion");
    assert_eq!(active_after_deletion, 0);

    sqlx::query(
        "DELETE FROM media.asset_bindings WHERE target_type = 'profile_avatar' AND target_id = $1",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .ok();
    sqlx::query("DELETE FROM media.uploads WHERE id = ANY($1)")
        .bind(vec![clean_upload_id, replacement_upload_id, pending_upload_id, other_upload_id])
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM identity.accounts WHERE id = ANY($1)")
        .bind(vec![owner_id, other_id])
        .execute(&pool)
        .await
        .ok();
}

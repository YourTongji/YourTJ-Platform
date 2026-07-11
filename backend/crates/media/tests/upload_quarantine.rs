//! Database integration coverage for upload quarantine ordering.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use media::{routes_with_object_store, UploadObjectStore};
use shared::{AppResult, AppState};
use sqlx::PgPool;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

struct FailingObjectStore;

#[async_trait::async_trait]
impl UploadObjectStore for FailingObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        Err(shared::AppError::BadRequest("simulated OSS failure".into()))
    }
}

struct SuccessfulObjectStore;

#[async_trait::async_trait]
impl UploadObjectStore for SuccessfulObjectStore {
    async fn delete_object(&self, _oss_key: &str) -> AppResult<()> {
        Ok(())
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

fn request(method: Method, uri: String, token: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .expect("media request")
}

#[tokio::test]
async fn failed_object_quarantine_leaves_upload_pending() {
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
    let block_uri = format!("/api/v2/admin/media/uploads/{upload_id}/block");

    let failing_app = routes_with_object_store(state.clone(), Arc::new(FailingObjectStore));
    let failed_response = failing_app
        .clone()
        .oneshot(request(
            Method::POST,
            block_uri.clone(),
            &moderator_token,
            Body::from(r#"{"reason":"confirmed malicious upload"}"#),
        ))
        .await
        .expect("failed block response");
    assert!(!failed_response.status().is_success());
    let pending_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("status after failed quarantine");
    assert_eq!(pending_status, "pending");
    let pending_url_response = failing_app
        .oneshot(request(
            Method::GET,
            format!("/api/v2/media/{upload_id}/url"),
            &owner_token,
            Body::empty(),
        ))
        .await
        .expect("pending URL response");
    assert_eq!(pending_url_response.status(), StatusCode::OK);

    let successful_app = routes_with_object_store(state.clone(), Arc::new(SuccessfulObjectStore));
    let successful_response = successful_app
        .clone()
        .oneshot(request(
            Method::POST,
            block_uri,
            &moderator_token,
            Body::from(r#"{"reason":"confirmed malicious upload"}"#),
        ))
        .await
        .expect("successful block response");
    assert_eq!(successful_response.status(), StatusCode::OK);
    let blocked_status: String =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1")
            .bind(upload_id)
            .fetch_one(&pool)
            .await
            .expect("status after successful quarantine");
    assert_eq!(blocked_status, "blocked");
    let blocked_url_response = successful_app
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
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4, 'clean') RETURNING id",
    )
    .bind(owner_id)
    .bind(format!("uploads/{owner_id}/image/{suffix}-clean.png"))
    .bind(format!("https://cdn.example.test/{suffix}-clean.png"))
    .bind("b".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed clean profile image");
    let pending_upload_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4) RETURNING id",
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
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, $3, 20, 'image/png', $4, 'clean') RETURNING id",
    )
    .bind(other_id)
    .bind(format!("uploads/{other_id}/image/{suffix}-other.png"))
    .bind(format!("https://cdn.example.test/{suffix}-other.png"))
    .bind("d".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("seed other profile image");
    let state = test_state(pool.clone());
    let token = identity::auth::create_access_token(owner_id, &state.jwt_secret, 3600)
        .expect("profile media token");
    let app = routes_with_object_store(state, Arc::new(SuccessfulObjectStore));

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

    let clear_response = app
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

    sqlx::query("DELETE FROM media.uploads WHERE id = ANY($1)")
        .bind(vec![clean_upload_id, pending_upload_id, other_upload_id])
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM identity.accounts WHERE id = ANY($1)")
        .bind(vec![owner_id, other_id])
        .execute(&pool)
        .await
        .ok();
}

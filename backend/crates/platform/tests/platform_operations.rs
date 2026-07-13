use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use shared::AppState;
use sqlx::PgPool;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
const JWT_SECRET: &str = "integration-test-secret-32bytes!";

fn configure_media_delivery() {
    static CONFIGURE: std::sync::Once = std::sync::Once::new();
    CONFIGURE.call_once(|| {
        for (name, value) in [
            ("OSS_REGION", "cn-shanghai"),
            ("MEDIA_DELIVERY_OSS_BUCKET", "yourtj-media-delivery-test"),
            ("MEDIA_DELIVERY_OSS_ACCESS_KEY_ID", "testdeliveryaccesskey"),
            ("MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET", "testdeliverysecret"),
            ("MEDIA_CDN_BASE_URL", "https://media-test.yourtj.de"),
            ("MEDIA_CDN_PRIMARY_KEY", "testprimarykey"),
            ("MEDIA_CDN_SECONDARY_KEY", "testsecondarykey"),
            ("MEDIA_CDN_SIGNING_KEY_SLOT", "primary"),
            ("MEDIA_CDN_URL_TTL_SECONDS", "300"),
            ("CDN_ACCESS_KEY_ID", "testcdnaccesskey"),
            ("CDN_ACCESS_KEY_SECRET", "testcdnsecret"),
        ] {
            std::env::set_var(name, value);
        }
    });
}

async fn test_pool() -> PgPool {
    configure_media_delivery();
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to platform test database");
    let has_identity: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(&pool)
    .await
    .expect("check identity schema");
    if !has_identity {
        MIGRATOR.run(&pool).await.expect("run platform test migrations");
    } else {
        let has_promotions: bool =
            sqlx::query_scalar("SELECT to_regclass('platform.promotions') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("check platform promotions schema");
        if !has_promotions {
            sqlx::raw_sql(include_str!(
                "../../../migrations/0035_platform_announcements_promotions.sql"
            ))
            .execute(&pool)
            .await
            .expect("run platform announcements and promotions migration");
        }
        let has_promotion_metrics: bool = sqlx::query_scalar(
            "SELECT to_regclass('platform.promotion_daily_metrics') IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .expect("check promotion metric schema");
        if !has_promotion_metrics {
            sqlx::raw_sql(include_str!("../../../migrations/0051_promotion_event_metrics.sql"))
                .execute(&pool)
                .await
                .expect("run promotion event metric migration");
        }
        let has_outbox: bool =
            sqlx::query_scalar("SELECT to_regclass('platform.outbox_events') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("check durable notification outbox schema");
        if !has_outbox {
            sqlx::raw_sql(include_str!("../../../migrations/0054_durable_notification_outbox.sql"))
                .execute(&pool)
                .await
                .expect("run durable notification outbox migration");
        }
    }
    pool
}

fn test_state(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        config: shared::Config::from_env().expect("load test config"),
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

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.expect("read response body");
    serde_json::from_slice(&bytes).expect("parse response JSON")
}

fn assert_private_no_store(response: &axum::response::Response) {
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).and_then(|value| value.to_str().ok()),
        Some("private, no-store")
    );
    assert_eq!(
        response.headers().get(header::PRAGMA).and_then(|value| value.to_str().ok()),
        Some("no-cache")
    );
}

async fn account(pool: &PgPool, suffix: &str, role: &str) -> (i64, String) {
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, $3::identity.account_role) RETURNING id",
    )
    .bind(format!("platform-{role}-{suffix}@tongji.edu.cn"))
    .bind(format!("platform-{role}-{suffix}"))
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("insert platform test account");
    let token = identity::auth::create_access_token(account_id, JWT_SECRET, 3600)
        .expect("create platform test token");
    (account_id, token)
}

async fn publish_synthetic_image_variants(pool: &PgPool, asset_id: i64) {
    for (index, (variant_kind, width, height)) in [
        ("thumb_256", 256_i32, 144_i32),
        ("display_1280", 1_280_i32, 720_i32),
        ("full_2048", 2_048_i32, 1_152_i32),
    ]
    .into_iter()
    .enumerate()
    {
        let digest = format!("{:064x}", asset_id * 10 + index as i64);
        sqlx::query(
            "INSERT INTO media.asset_variants \
             (asset_id, variant_kind, policy_version, object_key, content_sha256, mime, bytes, \
              width, height, status, published_at) \
             VALUES ($1, $2, 1, $3, $4, 'image/webp', 128, $5, $6, 'published', now())",
        )
        .bind(asset_id)
        .bind(variant_kind)
        .bind(format!("assets/{asset_id}/1/{variant_kind}-{digest}.webp"))
        .bind(digest)
        .bind(width)
        .bind(height)
        .execute(pool)
        .await
        .expect("insert synthetic promotion image variant");
    }
    sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'published', published_at = now(), updated_at = now() WHERE asset_id = $1",
    )
    .bind(asset_id)
    .execute(pool)
    .await
    .expect("publish synthetic promotion image variants");
}

fn json_request(method: Method, uri: String, token: Option<&str>, body: Value) -> Request<Body> {
    let mut request =
        Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    request.body(Body::from(body.to_string())).expect("build JSON request")
}

#[tokio::test]
async fn durable_outbox_leases_dead_letters_and_manual_retries_are_audited() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (admin_id, admin_token) = account(&pool, &suffix, "admin").await;
    let (_, moderator_token) = account(&pool, &format!("mod-{suffix}"), "mod").await;
    let (recipient_id, _) = account(&pool, &format!("user-{suffix}"), "user").await;
    let source_key = format!("outbox-lease-{suffix}");
    let second_source_key = format!("outbox-skip-locked-{suffix}");
    let abandoned_source_key = format!("outbox-abandoned-final-lease-{suffix}");
    let mut enqueue = pool.begin().await.expect("begin durable outbox enqueue");
    let first_id = platform::outbox::enqueue_notification_tx(
        &mut enqueue,
        &source_key,
        recipient_id,
        None,
        "system",
        &json!({ "title": "private integration payload" }),
        None,
        None,
    )
    .await
    .expect("enqueue first durable event");
    let second_id = platform::outbox::enqueue_notification_tx(
        &mut enqueue,
        &second_source_key,
        recipient_id,
        None,
        "system",
        &json!({ "title": "second private integration payload" }),
        None,
        None,
    )
    .await
    .expect("enqueue second durable event");
    let abandoned_id = platform::outbox::enqueue_notification_tx(
        &mut enqueue,
        &abandoned_source_key,
        recipient_id,
        None,
        "system",
        &json!({ "title": "recover final leased attempt" }),
        None,
        None,
    )
    .await
    .expect("enqueue abandoned final-attempt event");
    enqueue.commit().await.expect("commit durable events");
    let mut replay = pool.begin().await.expect("begin idempotent outbox replay");
    let replayed_id = platform::outbox::enqueue_notification_tx(
        &mut replay,
        &source_key,
        recipient_id,
        None,
        "system",
        &json!({ "title": "private integration payload" }),
        None,
        None,
    )
    .await
    .expect("replay identical durable event");
    assert_eq!(replayed_id, first_id);
    replay.commit().await.expect("commit idempotent outbox replay");
    let mut conflicting_replay = pool.begin().await.expect("begin conflicting outbox replay");
    let conflict = platform::outbox::enqueue_notification_tx(
        &mut conflicting_replay,
        &source_key,
        recipient_id,
        None,
        "system",
        &json!({ "title": "different private payload" }),
        None,
        None,
    )
    .await;
    assert!(matches!(conflict, Err(shared::AppError::Conflict(_))));
    conflicting_replay.rollback().await.expect("rollback conflicting outbox replay");
    sqlx::query(
        "UPDATE platform.outbox_events SET available_at = \
           CASE WHEN id = $1 THEN '1900-01-01'::timestamptz \
                WHEN id = $2 THEN '1900-01-02'::timestamptz \
                ELSE '1900-01-03'::timestamptz END \
         WHERE id = ANY($3)",
    )
    .bind(first_id)
    .bind(second_id)
    .bind(vec![first_id, second_id, abandoned_id])
    .execute(&pool)
    .await
    .expect("order durable lease candidates");

    let mut locked_candidate = pool.begin().await.expect("begin candidate lock");
    sqlx::query("SELECT id FROM platform.outbox_events WHERE id = $1 FOR UPDATE")
        .bind(first_id)
        .execute(&mut *locked_candidate)
        .await
        .expect("lock first outbox candidate");
    let second_worker = uuid::Uuid::new_v4();
    let claimed_while_locked = platform::outbox::claim_events(&pool, second_worker, 1)
        .await
        .expect("claim around locked candidate");
    assert_eq!(claimed_while_locked[0].id, second_id);
    locked_candidate.rollback().await.expect("release first candidate");

    let first_worker = uuid::Uuid::new_v4();
    let first_claim = platform::outbox::claim_events(&pool, first_worker, 1)
        .await
        .expect("claim first candidate");
    assert_eq!(first_claim[0].id, first_id);
    sqlx::query(
        "UPDATE platform.outbox_events SET lease_expires_at = now() - interval '1 second' \
         WHERE id = $1",
    )
    .bind(first_id)
    .execute(&pool)
    .await
    .expect("expire first worker lease");
    let replacement_worker = uuid::Uuid::new_v4();
    let replacement_claim = platform::outbox::claim_events(&pool, replacement_worker, 1)
        .await
        .expect("reclaim expired lease");
    assert_eq!(replacement_claim[0].id, first_id);
    assert_eq!(replacement_claim[0].attempts, 2);
    sqlx::query("UPDATE platform.outbox_events SET max_attempts = attempts WHERE id = $1")
        .bind(first_id)
        .execute(&pool)
        .await
        .expect("bound retry attempts for dead-letter test");
    assert_eq!(
        platform::outbox::record_failure(&pool, &replacement_claim[0], "database_unavailable")
            .await
            .expect("record terminal delivery failure"),
        Some("dead".into())
    );
    let mut complete_second = pool.begin().await.expect("begin second completion");
    assert!(platform::outbox::mark_succeeded_tx(&mut complete_second, second_id, second_worker,)
        .await
        .expect("complete second event"));
    complete_second.commit().await.expect("commit second completion");

    let abandoned_worker = uuid::Uuid::new_v4();
    let abandoned_claim = platform::outbox::claim_events(&pool, abandoned_worker, 1)
        .await
        .expect("claim event that will lose its final lease");
    assert_eq!(abandoned_claim[0].id, abandoned_id);
    sqlx::query(
        "UPDATE platform.outbox_events \
         SET max_attempts = attempts, lease_expires_at = now() - interval '1 second' \
         WHERE id = $1",
    )
    .bind(abandoned_id)
    .execute(&pool)
    .await
    .expect("expire final-attempt lease");
    let recovery_worker = uuid::Uuid::new_v4();
    let recovery_claim = platform::outbox::claim_events(&pool, recovery_worker, 1)
        .await
        .expect("reclaim final attempt without stranding running state");
    assert_eq!(recovery_claim[0].id, abandoned_id);
    assert_eq!(recovery_claim[0].attempts, recovery_claim[0].max_attempts);
    assert_eq!(
        platform::outbox::record_failure(&pool, &recovery_claim[0], "worker_failed")
            .await
            .expect("dead-letter recovered final attempt"),
        Some("dead".into())
    );

    let app = platform::routes(test_state(pool.clone()));
    let denied = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/notification-outbox?state=dead".into(),
            Some(&moderator_token),
            json!({}),
        ))
        .await
        .expect("moderator outbox response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let listed = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/notification-outbox?state=dead".into(),
            Some(&admin_token),
            json!({}),
        ))
        .await
        .expect("administrator outbox response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed = read_json(listed).await;
    let first_id_string = first_id.to_string();
    let item = listed["items"]
        .as_array()
        .expect("dead-letter items")
        .iter()
        .find(|item| item["id"].as_str() == Some(first_id_string.as_str()))
        .expect("dead-letter event visible");
    assert_eq!(item["lastErrorCode"], "database_unavailable");
    assert!(item.get("payload").is_none());
    assert!(item.get("sourceKey").is_none());

    let short_reason = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/notification-outbox/{first_id}/retry"),
            Some(&admin_token),
            json!({ "reason": "no" }),
        ))
        .await
        .expect("short retry reason response");
    assert_eq!(short_reason.status(), StatusCode::BAD_REQUEST);
    let retried = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/notification-outbox/{first_id}/retry"),
            Some(&admin_token),
            json!({ "reason": "database connectivity has recovered" }),
        ))
        .await
        .expect("manual retry response");
    assert_eq!(retried.status(), StatusCode::OK);
    let retried = read_json(retried).await;
    assert_eq!(retried["state"], "queued");
    assert_eq!(retried["attempts"], 0);
    assert_eq!(retried["manualRetryCount"], 1);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'platform.outbox.retried' \
           AND target_id = $2",
    )
    .bind(admin_id)
    .bind(first_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count manual retry audit events");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn announcement_revision_is_presented_once_and_privileged_writes_are_audited() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (admin_id, admin_token) = account(&pool, &suffix, "admin").await;
    let (_user_id, user_token) = account(&pool, &format!("user-{suffix}"), "user").await;
    let app = platform::routes(test_state(pool.clone()));

    let create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/announcements".into(),
            Some(&admin_token),
            json!({
                "title": format!("Revision notice {suffix}"),
                "body": "Initial announcement body",
                "status": "published",
                "presentation": "card",
                "severity": "warning",
                "priority": 50,
                "audience": "authenticated",
                "requiresAck": true,
                "startsAt": null,
                "endsAt": null,
                "reason": "publish integration announcement"
            }),
        ))
        .await
        .expect("create announcement response");
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = read_json(create).await;
    let announcement_id = created["id"].as_str().expect("announcement id");
    assert_eq!(created["revision"], 1);

    let unread = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/announcements/unread")
                .header(header::AUTHORIZATION, format!("Bearer {user_token}"))
                .body(Body::empty())
                .expect("build unread request"),
        )
        .await
        .expect("unread response");
    let unread_body = read_json(unread).await;
    assert!(unread_body
        .as_array()
        .expect("unread array")
        .iter()
        .any(|announcement| announcement["id"] == announcement_id));

    let seen = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/announcements/{announcement_id}/receipt"),
            Some(&user_token),
            json!({ "revision": 1, "action": "seen" }),
        ))
        .await
        .expect("seen response");
    assert_eq!(seen.status(), StatusCode::OK);

    let after_seen = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/announcements/unread")
                .header(header::AUTHORIZATION, format!("Bearer {user_token}"))
                .body(Body::empty())
                .expect("build second unread request"),
        )
        .await
        .expect("second unread response");
    let after_seen_body = read_json(after_seen).await;
    assert!(!after_seen_body
        .as_array()
        .expect("second unread array")
        .iter()
        .any(|announcement| announcement["id"] == announcement_id));

    let update = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            format!("/api/v2/admin/announcements/{announcement_id}"),
            Some(&admin_token),
            json!({
                "title": format!("Revision notice {suffix}"),
                "body": "Materially updated announcement body",
                "status": "published",
                "presentation": "banner",
                "severity": "critical",
                "priority": 75,
                "audience": "authenticated",
                "requiresAck": true,
                "startsAt": null,
                "endsAt": null,
                "expectedVersion": 1,
                "bumpRevision": true,
                "reason": "publish material announcement revision"
            }),
        ))
        .await
        .expect("update announcement response");
    assert_eq!(update.status(), StatusCode::OK);
    let updated = read_json(update).await;
    assert_eq!(updated["version"], 2);
    assert_eq!(updated["revision"], 2);

    let stale_update = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            format!("/api/v2/admin/announcements/{announcement_id}"),
            Some(&admin_token),
            json!({
                "title": format!("Revision notice {suffix}"),
                "body": "Stale operator copy",
                "status": "published",
                "presentation": "card",
                "severity": "info",
                "priority": 1,
                "audience": "authenticated",
                "requiresAck": true,
                "startsAt": null,
                "endsAt": null,
                "expectedVersion": 1,
                "bumpRevision": false,
                "reason": "attempt stale announcement overwrite"
            }),
        ))
        .await
        .expect("stale update response");
    assert_eq!(stale_update.status(), StatusCode::CONFLICT);

    let after_revision = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/announcements/unread")
                .header(header::AUTHORIZATION, format!("Bearer {user_token}"))
                .body(Body::empty())
                .expect("build revised unread request"),
        )
        .await
        .expect("revised unread response");
    let after_revision_body = read_json(after_revision).await;
    assert!(after_revision_body.as_array().expect("revised unread array").iter().any(
        |announcement| { announcement["id"] == announcement_id && announcement["revision"] == 2 }
    ));

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND target_type = 'announcement' AND target_id = $2",
    )
    .bind(admin_id)
    .bind(announcement_id)
    .fetch_one(&pool)
    .await
    .expect("read announcement audit events");
    assert_eq!(audit_count, 2);

    let immutable_revision = sqlx::query(
        "UPDATE platform.announcement_revisions SET title = title \
         WHERE announcement_id = $1 AND version = 1",
    )
    .bind(announcement_id.parse::<i64>().expect("numeric announcement id"))
    .execute(&pool)
    .await;
    assert!(immutable_revision.is_err());
}

#[tokio::test]
async fn promotions_require_admin_capability_safe_links_and_owned_clean_assets() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (admin_id, admin_token) = account(&pool, &suffix, "admin").await;
    let (second_admin_id, second_admin_token) =
        account(&pool, &format!("second-admin-{suffix}"), "admin").await;
    let (_moderator_id, moderator_token) = account(&pool, &format!("mod-{suffix}"), "mod").await;
    let asset_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, $3, 128, 'image/png', $4, 'clean') RETURNING id",
    )
    .bind(admin_id)
    .bind(format!("test/promotions/{suffix}.png"))
    .bind(format!("https://controlled.invalid/{suffix}.png"))
    .bind("a".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("insert clean promotion asset");
    publish_synthetic_image_variants(&pool, asset_id).await;
    let replacement_asset_id: i64 = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status) \
         VALUES ($1, 'image', $2, $3, 128, 'image/png', $4, 'clean') RETURNING id",
    )
    .bind(second_admin_id)
    .bind(format!("test/promotions/{suffix}-replacement.png"))
    .bind(format!("https://controlled.invalid/{suffix}-replacement.png"))
    .bind("b".repeat(64))
    .fetch_one(&pool)
    .await
    .expect("insert replacement promotion asset");
    publish_synthetic_image_variants(&pool, replacement_asset_id).await;
    let app = platform::routes(test_state(pool.clone()));

    let denied = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/promotions".into(),
            Some(&moderator_token),
            json!({
                "placement": "home-left-primary",
                "title": "Moderator promotion",
                "targetUrl": "/forum",
                "status": "published",
                "priority": 1,
                "audience": "all",
                "reason": "attempt without promotion capability"
            }),
        ))
        .await
        .expect("moderator promotion response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let unsafe_link = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/promotions".into(),
            Some(&admin_token),
            json!({
                "placement": "home-left-primary",
                "title": "Unsafe external promotion",
                "targetUrl": "https://example.com/track",
                "status": "published",
                "priority": 1,
                "audience": "all",
                "reason": "negative link validation test"
            }),
        ))
        .await
        .expect("unsafe promotion response");
    assert_eq!(unsafe_link.status(), StatusCode::BAD_REQUEST);

    let mut created_ids = Vec::new();
    for (priority, title) in [
        (1000, "Primary first-party promotion"),
        (999, "Secondary first-party promotion"),
        (998, "Midnight attribution promotion"),
    ] {
        let response = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v2/admin/promotions".into(),
                Some(&admin_token),
                json!({
                    "placement": "home-left-primary",
                    "title": format!("{title} {suffix}"),
                    "body": "A bounded first-party campus message",
                    "ctaLabel": "查看详情",
                    "targetUrl": "/forum",
                    "assetId": asset_id.to_string(),
                    "status": "published",
                    "priority": priority,
                    "audience": "all",
                    "reason": "publish first-party campus promotion"
                }),
            ))
            .await
            .expect("create promotion response");
        assert_eq!(response.status(), StatusCode::CREATED);
        created_ids
            .push(read_json(response).await["id"].as_str().expect("promotion id").to_owned());
    }
    let without_asset = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/promotions".into(),
            Some(&admin_token),
            json!({
                "placement": "home-left-primary",
                "title": format!("Text-only first-party promotion {suffix}"),
                "body": "This card deliberately has no media",
                "ctaLabel": "查看详情",
                "targetUrl": "/forum",
                "status": "published",
                "priority": 997,
                "audience": "all",
                "reason": "verify a promotion can intentionally omit media"
            }),
        ))
        .await
        .expect("create text-only promotion response");
    assert_eq!(without_asset.status(), StatusCode::CREATED);
    let without_asset_id =
        read_json(without_asset).await["id"].as_str().expect("text-only promotion id").to_owned();
    let active_bindings: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.asset_bindings \
         WHERE asset_id = $1 AND target_type = 'platform_promotion' AND detached_at IS NULL",
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("active promotion media bindings");
    assert_eq!(active_bindings, 3);

    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/promotions?placement=home-left-primary")
                .body(Body::empty())
                .expect("build public promotions request"),
        )
        .await
        .expect("public promotions response");
    assert_eq!(public.status(), StatusCode::OK);
    assert_private_no_store(&public);
    let items = read_json(public).await;
    let items = items.as_array().expect("promotion array");
    let first_position = items
        .iter()
        .position(|item| item["id"] == created_ids[0])
        .expect("first promotion returned");
    let second_position = items
        .iter()
        .position(|item| item["id"] == created_ids[1])
        .expect("second promotion returned");
    let third_position = items
        .iter()
        .position(|item| item["id"] == created_ids[2])
        .expect("third promotion returned");
    let without_asset_position = items
        .iter()
        .position(|item| item["id"] == without_asset_id)
        .expect("text-only promotion returned");
    assert!(first_position < second_position);
    let first_token = items[first_position]["trackingToken"]
        .as_str()
        .expect("anonymous promotion tracking token")
        .to_owned();
    let second_token = items[second_position]["trackingToken"]
        .as_str()
        .expect("second anonymous promotion tracking token")
        .to_owned();
    let third_token = items[third_position]["trackingToken"]
        .as_str()
        .expect("third anonymous promotion tracking token")
        .to_owned();
    assert!(items[first_position]["metrics"].is_null());
    let public_delivery = &items[first_position]["assetDelivery"];
    assert_eq!(public_delivery["assetId"], asset_id.to_string());
    assert_eq!(public_delivery["mime"], "image/webp");
    assert_eq!(public_delivery["width"], 1_280);
    assert_eq!(public_delivery["height"], 720);
    assert_eq!(public_delivery["variant"], "display_1280");
    assert!(public_delivery["expiresAt"].as_i64().is_some());
    assert!(public_delivery["url"]
        .as_str()
        .is_some_and(|url| url.starts_with("https://media-test.yourtj.de/assets/")
            && url.contains("?auth_key=")));
    assert!(items[without_asset_position]["assetDelivery"].is_null());
    let serialized_public = serde_json::to_string(&items[first_position])
        .expect("serialize public promotion projection");
    assert!(!serialized_public.contains("test/promotions/"));
    assert!(!serialized_public.contains("controlled.invalid"));
    assert!(!serialized_public.contains("aliyuncs.com"));
    assert!(!serialized_public.contains("oss-cn-"));

    for event_type in ["impression", "impression", "click", "click"] {
        let event = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                format!("/api/v2/promotions/{}/events", created_ids[0]),
                None,
                json!({ "eventType": event_type, "trackingToken": first_token }),
            ))
            .await
            .expect("promotion event response");
        assert_eq!(event.status(), StatusCode::NO_CONTENT);
    }
    let click_without_prior_delivery = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/promotions/{}/events", created_ids[1]),
            None,
            json!({ "eventType": "click", "trackingToken": second_token }),
        ))
        .await
        .expect("promotion click without prior impression response");
    assert_eq!(click_without_prior_delivery.status(), StatusCode::NO_CONTENT);
    let inferred_delivery: (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(impressions), 0)::bigint, COALESCE(SUM(clicks), 0)::bigint \
         FROM platform.promotion_daily_metrics WHERE promotion_id = $1",
    )
    .bind(created_ids[1].parse::<i64>().expect("numeric promotion id"))
    .fetch_one(&pool)
    .await
    .expect("click-derived promotion delivery");
    assert_eq!(inferred_delivery, (1, 1));

    let midnight_impression = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/promotions/{}/events", created_ids[2]),
            None,
            json!({ "eventType": "impression", "trackingToken": third_token }),
        ))
        .await
        .expect("promotion midnight impression response");
    assert_eq!(midnight_impression.status(), StatusCode::NO_CONTENT);
    let third_id = created_ids[2].parse::<i64>().expect("numeric third promotion id");
    sqlx::query(
        "UPDATE platform.promotion_event_receipts \
         SET recorded_at = date_trunc('day', now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC' \
                           - interval '1 second' \
         WHERE promotion_id = $1 AND event_type = 'impression'",
    )
    .bind(third_id)
    .execute(&pool)
    .await
    .expect("move receipt across UTC boundary");
    sqlx::query(
        "UPDATE platform.promotion_daily_metrics \
         SET metric_date = (timezone('UTC', now())::date - 1) \
         WHERE promotion_id = $1",
    )
    .bind(third_id)
    .execute(&pool)
    .await
    .expect("move aggregate across UTC boundary");
    let midnight_click = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/promotions/{}/events", created_ids[2]),
            None,
            json!({ "eventType": "click", "trackingToken": third_token }),
        ))
        .await
        .expect("promotion midnight click response");
    assert_eq!(midnight_click.status(), StatusCode::NO_CONTENT);
    let midnight_metrics: (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(impressions), 0)::bigint, \
                COALESCE(SUM(clicks), 0)::bigint, \
                COUNT(*) FILTER (WHERE metric_date = timezone('UTC', now())::date)::bigint \
         FROM platform.promotion_daily_metrics WHERE promotion_id = $1",
    )
    .bind(third_id)
    .fetch_one(&pool)
    .await
    .expect("midnight-attributed promotion metrics");
    assert_eq!(midnight_metrics, (1, 1, 0));
    let mismatched_token = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/promotions/{}/events", created_ids[1]),
            None,
            json!({ "eventType": "impression", "trackingToken": first_token }),
        ))
        .await
        .expect("mismatched promotion token response");
    assert_eq!(mismatched_token.status(), StatusCode::BAD_REQUEST);

    let metrics = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            format!("/api/v2/admin/promotions/{}/metrics", created_ids[0]),
            Some(&admin_token),
            json!({}),
        ))
        .await
        .expect("promotion metrics response");
    assert_eq!(metrics.status(), StatusCode::OK);
    let metrics = read_json(metrics).await;
    assert_eq!(metrics["summary"]["impressions"], 1);
    assert_eq!(metrics["summary"]["clicks"], 1);
    assert_eq!(metrics["days"].as_array().map(Vec::len), Some(30));

    let admin_list = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/promotions?limit=30".into(),
            Some(&admin_token),
            json!({}),
        ))
        .await
        .expect("promotion administration list response");
    assert_eq!(admin_list.status(), StatusCode::OK);
    assert_private_no_store(&admin_list);
    let admin_list = read_json(admin_list).await;
    let administered = admin_list["items"]
        .as_array()
        .expect("admin promotion items")
        .iter()
        .find(|item| item["id"] == created_ids[0])
        .expect("created promotion in administration list");
    assert!(administered["trackingToken"].is_null());
    assert_eq!(administered["metrics"]["impressions"], 1);
    assert_eq!(administered["metrics"]["clicks"], 1);
    assert_eq!(administered["assetDelivery"]["assetId"], asset_id.to_string());
    assert!(administered["assetDelivery"]["url"]
        .as_str()
        .is_some_and(|url| url.starts_with("https://media-test.yourtj.de/assets/")
            && url.contains("?auth_key=")));
    let administered_without_asset = admin_list["items"]
        .as_array()
        .expect("admin promotion items")
        .iter()
        .find(|item| item["id"] == without_asset_id)
        .expect("text-only promotion in administration list");
    assert!(administered_without_asset["assetDelivery"].is_null());

    sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'unpublished', published_at = NULL, updated_at = now() WHERE asset_id = $1",
    )
    .bind(asset_id)
    .execute(&pool)
    .await
    .expect("withdraw promotion asset publication");
    let public_after_withdrawal = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/promotions?placement=home-left-primary")
                .body(Body::empty())
                .expect("build public promotions request after media withdrawal"),
        )
        .await
        .expect("public promotions response after media withdrawal");
    assert_eq!(public_after_withdrawal.status(), StatusCode::OK);
    let public_after_withdrawal = read_json(public_after_withdrawal).await;
    let withdrawn_public = public_after_withdrawal
        .as_array()
        .expect("promotion array after media withdrawal")
        .iter()
        .find(|item| item["id"] == created_ids[0])
        .expect("promotion remains visible without withdrawn media");
    assert!(withdrawn_public["assetDelivery"].is_null());

    let admin_after_withdrawal = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/promotions?limit=30".into(),
            Some(&admin_token),
            json!({}),
        ))
        .await
        .expect("admin promotions response after media withdrawal");
    assert_eq!(admin_after_withdrawal.status(), StatusCode::OK);
    let admin_after_withdrawal = read_json(admin_after_withdrawal).await;
    let withdrawn_admin = admin_after_withdrawal["items"]
        .as_array()
        .expect("admin promotion items after media withdrawal")
        .iter()
        .find(|item| item["id"] == created_ids[0])
        .expect("withdrawn-media promotion remains in administration list");
    assert!(withdrawn_admin["assetDelivery"].is_null());
    sqlx::query(
        "UPDATE media.asset_publications \
         SET status = 'published', published_at = now(), updated_at = now() WHERE asset_id = $1",
    )
    .bind(asset_id)
    .execute(&pool)
    .await
    .expect("restore promotion asset publication for lifecycle assertions");

    let denied_metrics = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            format!("/api/v2/admin/promotions/{}/metrics", created_ids[0]),
            Some(&moderator_token),
            json!({}),
        ))
        .await
        .expect("unauthorized promotion metrics response");
    assert_eq!(denied_metrics.status(), StatusCode::FORBIDDEN);
    let invalid_range = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            format!(
                "/api/v2/admin/promotions/{}/metrics?from=2020-01-01&to=2021-01-01",
                created_ids[0]
            ),
            Some(&admin_token),
            json!({}),
        ))
        .await
        .expect("invalid promotion metric range response");
    assert_eq!(invalid_range.status(), StatusCode::BAD_REQUEST);

    let stored_receipts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.promotion_event_receipts WHERE promotion_id = $1",
    )
    .bind(created_ids[0].parse::<i64>().expect("numeric promotion id"))
    .fetch_one(&pool)
    .await
    .expect("promotion event receipts");
    assert_eq!(stored_receipts, 2);
    let stores_account_identifier: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'platform' AND table_name = 'promotion_event_receipts' \
           AND column_name IN ('account_id', 'ip_address', 'device_id'))",
    )
    .fetch_one(&pool)
    .await
    .expect("promotion receipt privacy columns");
    assert!(!stores_account_identifier);

    sqlx::query(
        "UPDATE platform.promotion_event_receipts SET recorded_at = now() - interval '49 hours' \
         WHERE promotion_id = $1",
    )
    .bind(created_ids[0].parse::<i64>().expect("numeric promotion id"))
    .execute(&pool)
    .await
    .expect("age promotion event receipts");
    assert_eq!(
        platform::purge_expired_promotion_event_receipts(&pool)
            .await
            .expect("purge promotion receipts"),
        2
    );
    let retained_metrics: (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(impressions), 0)::bigint, COALESCE(SUM(clicks), 0)::bigint \
         FROM platform.promotion_daily_metrics WHERE promotion_id = $1",
    )
    .bind(created_ids[0].parse::<i64>().expect("numeric promotion id"))
    .fetch_one(&pool)
    .await
    .expect("retained promotion aggregates");
    assert_eq!(retained_metrics, (1, 1));

    let promotion_id = created_ids[0].parse::<i64>().expect("numeric promotion id");
    let text_only = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            format!("/api/v2/admin/promotions/{promotion_id}"),
            Some(&second_admin_token),
            json!({
                "placement": "home-left-primary",
                "title": format!("Text-only update by another operator {suffix}"),
                "body": "The existing asset remains owned by the original operator",
                "ctaLabel": "查看详情",
                "targetUrl": "/forum",
                "assetId": asset_id.to_string(),
                "status": "published",
                "priority": 1000,
                "audience": "all",
                "reason": "verify text updates do not reauthorize an unchanged asset",
                "expectedVersion": 1
            }),
        ))
        .await
        .expect("cross-operator text-only promotion update");
    assert_eq!(text_only.status(), StatusCode::OK);
    let active_after_text_update: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.asset_bindings \
         WHERE target_type = 'platform_promotion' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(promotion_id)
    .fetch_one(&pool)
    .await
    .expect("promotion binding after text update");
    assert_eq!(active_after_text_update, 1);

    for (expected_version, next_asset_id, reason) in [
        (2, Some(replacement_asset_id), "replace promotion media with an owned clean asset"),
        (3, None, "clear promotion media while preserving a deletion grace"),
        (4, Some(replacement_asset_id), "restore promotion media before archival"),
    ] {
        let updated = app
            .clone()
            .oneshot(json_request(
                Method::PATCH,
                format!("/api/v2/admin/promotions/{promotion_id}"),
                Some(&second_admin_token),
                json!({
                    "placement": "home-left-primary",
                    "title": format!("Promotion media lifecycle {suffix}"),
                    "body": "A bounded first-party campus message",
                    "ctaLabel": "查看详情",
                    "targetUrl": "/forum",
                    "assetId": next_asset_id.map(|id| id.to_string()),
                    "status": "published",
                    "priority": 1000,
                    "audience": "all",
                    "reason": reason,
                    "expectedVersion": expected_version
                }),
            ))
            .await
            .expect("promotion media lifecycle update");
        assert_eq!(updated.status(), StatusCode::OK);
    }
    let archived = app
        .clone()
        .oneshot(json_request(
            Method::DELETE,
            format!("/api/v2/admin/promotions/{promotion_id}"),
            Some(&second_admin_token),
            json!({
                "expectedVersion": 5,
                "reason": "archive promotion and detach its final media binding"
            }),
        ))
        .await
        .expect("archive promotion media lifecycle fixture");
    assert_eq!(archived.status(), StatusCode::NO_CONTENT);
    let binding_history: Vec<(i64, i64, String, bool)> = sqlx::query_as(
        "SELECT asset_id, owner_account_id, detached_reason, gc_eligible_at > detached_at \
         FROM media.asset_bindings \
         WHERE target_type = 'platform_promotion' AND target_id = $1 ORDER BY id",
    )
    .bind(promotion_id)
    .fetch_all(&pool)
    .await
    .expect("promotion media binding history");
    assert_eq!(
        binding_history,
        vec![
            (asset_id, admin_id, "replaced".into(), true),
            (replacement_asset_id, second_admin_id, "cleared".into(), true),
            (replacement_asset_id, second_admin_id, "archived".into(), true),
        ]
    );

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'platform.promotion.created'",
    )
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("read promotion audit events");
    assert_eq!(audit_count, 4);
}

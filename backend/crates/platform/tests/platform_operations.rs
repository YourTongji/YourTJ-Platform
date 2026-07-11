use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use shared::AppState;
use sqlx::PgPool;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
const JWT_SECRET: &str = "integration-test-secret-32bytes!";

async fn test_pool() -> PgPool {
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

fn json_request(method: Method, uri: String, token: Option<&str>, body: Value) -> Request<Body> {
    let mut request =
        Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    request.body(Body::from(body.to_string())).expect("build JSON request")
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

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'platform.promotion.created'",
    )
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("read promotion audit events");
    assert_eq!(audit_count, 3);
}

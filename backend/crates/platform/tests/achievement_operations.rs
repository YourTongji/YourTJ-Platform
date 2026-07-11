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
    let pool = PgPool::connect(&url).await.expect("connect to achievement test database");
    let has_identity: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(&pool)
    .await
    .expect("check identity schema");
    if !has_identity {
        MIGRATOR.run(&pool).await.expect("run achievement test migrations");
    } else {
        let has_achievements: bool =
            sqlx::query_scalar("SELECT to_regclass('platform.achievement_events') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("check achievement schema");
        if !has_achievements {
            sqlx::raw_sql(include_str!("../../../migrations/0045_achievement_operations.sql"))
                .execute(&pool)
                .await
                .expect("run achievement operations migration");
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

async fn account(pool: &PgPool, suffix: &str, role: &str) -> (i64, String) {
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, $3::identity.account_role) RETURNING id",
    )
    .bind(format!("achievement-{role}-{suffix}@tongji.edu.cn"))
    .bind(format!("achievement-{role}-{suffix}"))
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("insert achievement test account");
    let token = identity::auth::create_access_token(account_id, JWT_SECRET, 3600)
        .expect("create achievement test token");
    (account_id, token)
}

fn json_request(method: Method, uri: String, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .expect("build achievement request")
}

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.expect("read response body");
    serde_json::from_slice(&bytes).expect("parse response JSON")
}

#[tokio::test]
async fn staff_definition_and_manual_award_lifecycle_is_versioned_audited_and_non_minting() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (admin_id, admin_token) = account(&pool, &suffix, "admin").await;
    let (peer_admin_id, _) = account(&pool, &format!("peer-{suffix}"), "admin").await;
    let (_, moderator_token) = account(&pool, &format!("mod-{suffix}"), "mod").await;
    let (user_id, _) = account(&pool, &format!("user-{suffix}"), "user").await;
    let app = platform::routes(test_state(pool.clone()));

    let unsafe_icon = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/achievements".into(),
            &admin_token,
            json!({
                "slug": format!("unsafe-{suffix}"),
                "name": "不安全图标",
                "description": "拒绝外链图标",
                "icon": "https://example.invalid/icon.svg",
                "mintAmount": 5,
                "reason": "reject unsafe achievement presentation"
            }),
        ))
        .await
        .expect("unsafe achievement icon response");
    assert_eq!(unsafe_icon.status(), StatusCode::BAD_REQUEST);
    let unsafe_text = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/achievements".into(),
            &admin_token,
            json!({
                "slug": format!("unsafe-text-{suffix}"),
                "name": "<img src=x>",
                "description": "unsafe visual payload",
                "icon": "award",
                "mintAmount": 5,
                "reason": "reject unsafe achievement presentation"
            }),
        ))
        .await
        .expect("unsafe achievement text response");
    assert_eq!(unsafe_text.status(), StatusCode::BAD_REQUEST);

    let slug = format!("community-helper-{suffix}");
    let create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/achievements".into(),
            &admin_token,
            json!({
                "slug": slug,
                "name": "社区助手",
                "description": "持续帮助社区成员",
                "icon": "award",
                "mintAmount": 20,
                "reason": "create a reviewed contribution achievement"
            }),
        ))
        .await
        .expect("create achievement response");
    assert_eq!(create.status(), StatusCode::CREATED);
    let definition = read_json(create).await;
    let achievement_id = definition["id"].as_str().expect("achievement id");
    assert_eq!(definition["version"], 1);

    let moderator_list = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/achievements".into(),
            &moderator_token,
            json!({}),
        ))
        .await
        .expect("moderator list response");
    assert_eq!(moderator_list.status(), StatusCode::FORBIDDEN);

    let update_body = json!({
        "expectedVersion": 1,
        "name": "社区贡献者",
        "description": "持续帮助社区成员",
        "icon": "star",
        "status": "active",
        "mintAmount": 20,
        "reason": "clarify the achievement display name"
    });
    let update = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            format!("/api/v2/admin/achievements/{achievement_id}"),
            &admin_token,
            update_body.clone(),
        ))
        .await
        .expect("update achievement response");
    assert_eq!(update.status(), StatusCode::OK);
    assert_eq!(read_json(update).await["version"], 2);
    let stale_update = app
        .clone()
        .oneshot(json_request(
            Method::PATCH,
            format!("/api/v2/admin/achievements/{achievement_id}"),
            &admin_token,
            update_body,
        ))
        .await
        .expect("stale update response");
    assert_eq!(stale_update.status(), StatusCode::CONFLICT);

    for protected_account_id in [admin_id, peer_admin_id] {
        let protected_grant = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                format!("/api/v2/admin/users/{protected_account_id}/achievements"),
                &admin_token,
                json!({
                    "achievementId": achievement_id,
                    "reason": "attempt to grant an achievement across a protected role boundary"
                }),
            ))
            .await
            .expect("protected target response");
        assert_eq!(protected_grant.status(), StatusCode::FORBIDDEN);
    }

    let grant = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/achievements"),
            &admin_token,
            json!({
                "achievementId": achievement_id,
                "reason": "manually recognize sustained community help"
            }),
        ))
        .await
        .expect("grant achievement response");
    assert_eq!(grant.status(), StatusCode::CREATED);
    assert_eq!(read_json(grant).await["status"], "active");

    let pending_mints: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.pending_mints WHERE account_id = $1 AND badge_slug = $2",
    )
    .bind(user_id)
    .bind(&slug)
    .fetch_one(&pool)
    .await
    .expect("count manual award mints");
    assert_eq!(pending_mints, 0, "manual recognition must never mint contribution points");
    let public = platform::achievements::list_public_account_achievements(&pool, user_id)
        .await
        .expect("list public achievements");
    assert!(public.iter().any(|achievement| achievement.slug == slug));

    let revoke = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/achievements/{achievement_id}/revoke"),
            &admin_token,
            json!({ "reason": "manual recognition was granted to the wrong account" }),
        ))
        .await
        .expect("revoke achievement response");
    assert_eq!(revoke.status(), StatusCode::OK);
    assert_eq!(read_json(revoke).await["status"], "revoked");
    assert!(platform::achievements::list_public_account_achievements(&pool, user_id)
        .await
        .expect("list revoked public achievements")
        .iter()
        .all(|achievement| achievement.slug != slug));

    let regrant = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/achievements"),
            &admin_token,
            json!({
                "achievementId": achievement_id,
                "reason": "review confirmed the contribution belongs to this account"
            }),
        ))
        .await
        .expect("regrant achievement response");
    assert_eq!(regrant.status(), StatusCode::CREATED);

    let events = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            format!("/api/v2/admin/users/{user_id}/achievement-events?limit=20"),
            &admin_token,
            json!({}),
        ))
        .await
        .expect("achievement events response");
    assert_eq!(events.status(), StatusCode::OK);
    let events = read_json(events).await;
    assert_eq!(events["items"].as_array().expect("achievement events").len(), 3);
    let admin_id = admin_id.to_string();
    assert!(events["items"]
        .as_array()
        .expect("achievement events")
        .iter()
        .all(|event| event["actorId"].as_str() == Some(admin_id.as_str())));
}

#[tokio::test]
async fn automatic_contribution_award_enqueues_exactly_one_idempotent_mint() {
    let pool = test_pool().await;
    platform::achievements::seed_achievements(&pool).await.expect("seed achievements");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (user_id, _) = account(&pool, &format!("automatic-{suffix}"), "user").await;

    let first = platform::achievements::award_achievement_by_slug(
        &pool,
        user_id,
        "first-thread",
        user_id,
        "published a first forum thread",
    )
    .await
    .expect("first automatic award")
    .expect("standard achievement exists");
    let repeated = platform::achievements::award_achievement_by_slug(
        &pool,
        user_id,
        "first-thread",
        user_id,
        "published a first forum thread",
    )
    .await
    .expect("repeated automatic award")
    .expect("standard achievement exists");
    assert!(first.newly_awarded);
    assert!(!repeated.newly_awarded);

    let pending_mints: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.pending_mints \
         WHERE account_id = $1 AND idempotency_key = $2",
    )
    .bind(user_id)
    .bind(format!("badge:first-thread:{user_id}"))
    .fetch_one(&pool)
    .await
    .expect("count automatic award mints");
    assert_eq!(pending_mints, 1);
    let award_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.achievement_events \
         WHERE account_id = $1 AND action = 'awarded' AND source = 'automatic'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count automatic award events");
    assert_eq!(award_events, 1);
}

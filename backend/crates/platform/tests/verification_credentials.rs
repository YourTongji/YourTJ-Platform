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
    let pool = PgPool::connect(&url).await.expect("connect to verification test database");
    let has_identity: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(&pool)
    .await
    .expect("check identity schema");
    if !has_identity {
        MIGRATOR.run(&pool).await.expect("run verification test migrations");
    } else {
        let has_verifications: bool =
            sqlx::query_scalar("SELECT to_regclass('platform.verification_grants') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("check verification schema");
        if !has_verifications {
            sqlx::raw_sql(include_str!("../../../migrations/0037_verification_credentials.sql"))
                .execute(&pool)
                .await
                .expect("run verification credential migration");
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
    .bind(format!("verification-{role}-{suffix}@tongji.edu.cn"))
    .bind(format!("verification-{role}-{suffix}"))
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("insert verification test account");
    let token = identity::auth::create_access_token(account_id, JWT_SECRET, 3600)
        .expect("create verification test token");
    (account_id, token)
}

fn json_request(method: Method, uri: String, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .expect("build verification request")
}

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.expect("read response body");
    serde_json::from_slice(&bytes).expect("parse response JSON")
}

#[tokio::test]
async fn grant_revoke_and_public_projection_preserve_private_evidence_and_audit_history() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (admin_id, admin_token) = account(&pool, &suffix, "admin").await;
    let (user_id, _user_token) = account(&pool, &format!("user-{suffix}"), "user").await;
    let app = platform::routes(test_state(pool.clone()));

    let create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/verifications/types".into(),
            &admin_token,
            json!({
                "slug": format!("official-{suffix}"),
                "category": "identity",
                "label": "官方学生组织",
                "description": "已核实的官方学生组织账号",
                "icon": "building-2",
                "badgeVariant": "default",
                "allowsPublicDisplay": true,
                "reason": "create official organization verification"
            }),
        ))
        .await
        .expect("create verification type response");
    assert_eq!(create.status(), StatusCode::CREATED);
    let definition = read_json(create).await;
    let definition_id = definition["id"].as_str().expect("verification type id");

    let grant = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &admin_token,
            json!({
                "verificationTypeId": definition_id,
                "displayOnProfile": true,
                "expiresAt": chrono::Utc::now().timestamp() + 3600,
                "evidenceReference": format!("case:{suffix}"),
                "reason": "verified organization ownership"
            }),
        ))
        .await
        .expect("grant verification response");
    let grant_status = grant.status();
    let grant = read_json(grant).await;
    assert_eq!(grant_status, StatusCode::CREATED, "grant response: {grant}");
    let grant_id = grant["id"].as_str().expect("verification grant id");
    assert_eq!(grant["status"], "active");
    assert_eq!(grant["hasEvidence"], true);
    assert!(grant.get("evidenceReference").is_none());

    let type_list = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/verifications/types?limit=50".into(),
            &admin_token,
            json!({}),
        ))
        .await
        .expect("list verification types response");
    assert_eq!(type_list.status(), StatusCode::OK);
    let type_list = read_json(type_list).await;
    assert!(type_list["items"]
        .as_array()
        .expect("verification type items")
        .iter()
        .any(|item| item["id"] == definition_id));

    let grant_list = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            format!("/api/v2/admin/users/{user_id}/verifications?limit=50"),
            &admin_token,
            json!({}),
        ))
        .await
        .expect("list verification grants response");
    assert_eq!(grant_list.status(), StatusCode::OK);
    let grant_list = read_json(grant_list).await;
    assert_eq!(grant_list["items"][0]["id"], grant_id);
    assert_eq!(grant_list["items"][0]["hasEvidence"], true);
    assert!(grant_list["items"][0].get("evidenceReference").is_none());

    let public = platform::verifications::list_public_account_verifications(&pool, user_id)
        .await
        .expect("read public verification projection");
    let public = serde_json::to_value(public).expect("serialize public projection");
    assert_eq!(public.as_array().expect("public verification array").len(), 1);
    assert_eq!(public[0]["slug"], format!("official-{suffix}"));
    assert!(public[0].get("issuedBy").is_none());
    assert!(public[0].get("issueReason").is_none());
    assert!(public[0].get("evidenceReference").is_none());

    let duplicate = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &admin_token,
            json!({
                "verificationTypeId": definition_id,
                "displayOnProfile": true,
                "reason": "attempt duplicate active verification"
            }),
        ))
        .await
        .expect("duplicate grant response");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let revoke = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/verifications/grants/{grant_id}/revoke"),
            &admin_token,
            json!({ "reason": "organization ownership could not be renewed" }),
        ))
        .await
        .expect("revoke verification response");
    assert_eq!(revoke.status(), StatusCode::OK);
    assert_eq!(read_json(revoke).await["status"], "revoked");
    let repeated_revoke = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/verifications/grants/{grant_id}/revoke"),
            &admin_token,
            json!({ "reason": "attempt to revoke the same credential twice" }),
        ))
        .await
        .expect("repeated revoke response");
    assert_eq!(repeated_revoke.status(), StatusCode::CONFLICT);
    assert!(platform::verifications::list_public_account_verifications(&pool, user_id)
        .await
        .expect("read revoked public projection")
        .is_empty());

    let concurrent_one = app.clone().oneshot(json_request(
        Method::POST,
        format!("/api/v2/admin/users/{user_id}/verifications"),
        &admin_token,
        json!({
            "verificationTypeId": definition_id,
            "displayOnProfile": false,
            "reason": "concurrent regrant attempt one"
        }),
    ));
    let concurrent_two = app.clone().oneshot(json_request(
        Method::POST,
        format!("/api/v2/admin/users/{user_id}/verifications"),
        &admin_token,
        json!({
            "verificationTypeId": definition_id,
            "displayOnProfile": false,
            "reason": "concurrent regrant attempt two"
        }),
    ));
    let (concurrent_one, concurrent_two) = tokio::join!(concurrent_one, concurrent_two);
    let mut statuses = [
        concurrent_one.expect("first concurrent grant response").status(),
        concurrent_two.expect("second concurrent grant response").status(),
    ];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);

    let persisted: (String, String, String) = sqlx::query_as(
        "SELECT issue_reason, evidence_reference, revoke_reason \
         FROM platform.verification_grants WHERE id = $1",
    )
    .bind(grant_id.parse::<i64>().expect("numeric grant id"))
    .fetch_one(&pool)
    .await
    .expect("read verification grant history");
    assert_eq!(persisted.0, "verified organization ownership");
    assert_eq!(persisted.1, format!("case:{suffix}"));
    assert_eq!(persisted.2, "organization ownership could not be renewed");

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action LIKE 'platform.verification%'",
    )
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("read verification audit events");
    assert_eq!(audit_count, 4);
}

#[tokio::test]
async fn capability_target_hierarchy_and_public_display_policy_are_enforced_by_handlers() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (admin_id, admin_token) = account(&pool, &suffix, "admin").await;
    let (other_admin_id, _other_admin_token) =
        account(&pool, &format!("other-admin-{suffix}"), "admin").await;
    let (_moderator_id, moderator_token) = account(&pool, &format!("mod-{suffix}"), "mod").await;
    let (user_id, _user_token) = account(&pool, &format!("user-{suffix}"), "user").await;
    let app = platform::routes(test_state(pool.clone()));

    let moderator_create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/verifications/types".into(),
            &moderator_token,
            json!({
                "slug": format!("mod-attempt-{suffix}"),
                "category": "special",
                "label": "不应创建",
                "icon": "sparkles",
                "badgeVariant": "outline",
                "allowsPublicDisplay": false,
                "reason": "attempt without verification capability"
            }),
        ))
        .await
        .expect("moderator create response");
    assert_eq!(moderator_create.status(), StatusCode::FORBIDDEN);
    let moderator_type_list = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            "/api/v2/admin/verifications/types".into(),
            &moderator_token,
            json!({}),
        ))
        .await
        .expect("moderator type list response");
    assert_eq!(moderator_type_list.status(), StatusCode::FORBIDDEN);

    let invalid_icon = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/verifications/types".into(),
            &admin_token,
            json!({
                "slug": format!("invalid-icon-{suffix}"),
                "category": "special",
                "label": "不安全图标",
                "icon": "https://example.invalid/badge.svg",
                "badgeVariant": "outline",
                "allowsPublicDisplay": false,
                "reason": "negative icon token validation"
            }),
        ))
        .await
        .expect("invalid icon response");
    assert_eq!(invalid_icon.status(), StatusCode::BAD_REQUEST);

    let invalid_variant = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/verifications/types".into(),
            &admin_token,
            json!({
                "slug": format!("invalid-variant-{suffix}"),
                "category": "special",
                "label": "不安全样式",
                "icon": "sparkles",
                "badgeVariant": "bg-red-500 custom-css",
                "allowsPublicDisplay": true,
                "reason": "negative badge style token validation"
            }),
        ))
        .await
        .expect("invalid badge variant response");
    assert_eq!(invalid_variant.status(), StatusCode::BAD_REQUEST);
    let database_rejects_uncontrolled_icon = sqlx::query(
        "INSERT INTO platform.verification_types \
         (slug, category, label, icon, badge_variant, allows_public_display, created_by) \
         VALUES ($1, 'special', '数据库约束测试', 'remote-image', 'outline', true, $2)",
    )
    .bind(format!("database-invalid-icon-{suffix}"))
    .bind(admin_id)
    .execute(&pool)
    .await;
    assert!(database_rejects_uncontrolled_icon.is_err());

    let markup_label = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/verifications/types".into(),
            &admin_token,
            json!({
                "slug": format!("markup-label-{suffix}"),
                "category": "identity",
                "label": "<b>伪造认证</b>",
                "icon": "badge-check",
                "badgeVariant": "default",
                "allowsPublicDisplay": true,
                "reason": "negative markup label validation"
            }),
        ))
        .await
        .expect("markup label response");
    assert_eq!(markup_label.status(), StatusCode::BAD_REQUEST);

    let create = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/admin/verifications/types".into(),
            &admin_token,
            json!({
                "slug": format!("private-{suffix}"),
                "category": "special",
                "label": "内部特殊认证",
                "icon": "shield-check",
                "badgeVariant": "secondary",
                "allowsPublicDisplay": false,
                "reason": "create private special verification"
            }),
        ))
        .await
        .expect("create private verification response");
    let definition_id = read_json(create).await["id"].as_str().expect("private type id").to_owned();

    let short_reason = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &admin_token,
            json!({
                "verificationTypeId": definition_id,
                "displayOnProfile": false,
                "reason": "x"
            }),
        ))
        .await
        .expect("short reason response");
    assert_eq!(short_reason.status(), StatusCode::BAD_REQUEST);

    let expired_grant_id: i64 = sqlx::query_scalar(
        "INSERT INTO platform.verification_grants \
         (account_id, verification_type_id, display_on_profile, issue_reason, issued_by, \
          issued_at, expires_at) \
         VALUES ($1, $2, false, 'expired grant fixture', $3, now() - interval '2 hours', \
                 now() - interval '1 hour') RETURNING id",
    )
    .bind(user_id)
    .bind(definition_id.parse::<i64>().expect("numeric private type id"))
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("insert expired verification grant");
    let expired_revoke = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/verifications/grants/{expired_grant_id}/revoke"),
            &admin_token,
            json!({ "reason": "attempt to revoke an already expired grant" }),
        ))
        .await
        .expect("expired revoke response");
    assert_eq!(expired_revoke.status(), StatusCode::CONFLICT);

    let moderator_list = app
        .clone()
        .oneshot(json_request(
            Method::GET,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &moderator_token,
            json!({}),
        ))
        .await
        .expect("moderator list response");
    assert_eq!(moderator_list.status(), StatusCode::FORBIDDEN);
    let moderator_grant = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &moderator_token,
            json!({
                "verificationTypeId": definition_id,
                "displayOnProfile": false,
                "reason": "attempt grant without verification capability"
            }),
        ))
        .await
        .expect("moderator grant response");
    assert_eq!(moderator_grant.status(), StatusCode::FORBIDDEN);
    let moderator_revoke = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/verifications/grants/{expired_grant_id}/revoke"),
            &moderator_token,
            json!({ "reason": "attempt revoke without verification capability" }),
        ))
        .await
        .expect("moderator revoke response");
    assert_eq!(moderator_revoke.status(), StatusCode::FORBIDDEN);

    for target_id in [admin_id, other_admin_id] {
        let denied_list = app
            .clone()
            .oneshot(json_request(
                Method::GET,
                format!("/api/v2/admin/users/{target_id}/verifications"),
                &admin_token,
                json!({}),
            ))
            .await
            .expect("hierarchy list denial response");
        assert_eq!(denied_list.status(), StatusCode::FORBIDDEN);
        let denied = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                format!("/api/v2/admin/users/{target_id}/verifications"),
                &admin_token,
                json!({
                    "verificationTypeId": definition_id,
                    "displayOnProfile": false,
                    "reason": "attempt self or equal role verification"
                }),
            ))
            .await
            .expect("hierarchy denial response");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    }

    let forbidden_public = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &admin_token,
            json!({
                "verificationTypeId": definition_id,
                "displayOnProfile": true,
                "reason": "attempt public display against definition policy"
            }),
        ))
        .await
        .expect("public display policy response");
    assert_eq!(forbidden_public.status(), StatusCode::BAD_REQUEST);

    let external_evidence = app
        .oneshot(json_request(
            Method::POST,
            format!("/api/v2/admin/users/{user_id}/verifications"),
            &admin_token,
            json!({
                "verificationTypeId": definition_id,
                "displayOnProfile": false,
                "evidenceReference": "https://example.invalid/private-evidence",
                "reason": "reject external evidence reference"
            }),
        ))
        .await
        .expect("external evidence response");
    assert_eq!(external_evidence.status(), StatusCode::BAD_REQUEST);

    let grant_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platform.verification_grants WHERE account_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read denied grant count");
    assert_eq!(grant_count, 1);
}

use activity::contributions::deactivate_contribution;
use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use chrono::Utc;
use shared::AppState;
use sqlx::PgPool;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to activity lifecycle test database");
    MIGRATOR.run(&pool).await.expect("run activity lifecycle test migrations");
    pool
}

fn test_state(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        config: shared::Config::from_env().expect("load test config"),
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

#[tokio::test]
async fn export_and_purge_cover_check_in_score_and_trust_history() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("activity-export-{suffix}@tongji.edu.cn"))
    .bind(format!("activity-export-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert activity lifecycle account");
    let token =
        identity::auth::create_access_token(account_id, "integration-test-secret-32bytes!", 3600)
            .expect("create activity lifecycle token");
    let app = activity::routes(test_state(pool.clone()));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/me/check-in")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build lifecycle check-in request"),
        )
        .await
        .expect("send lifecycle check-in request");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read lifecycle check-in response");
    let check_in: serde_json::Value =
        serde_json::from_slice(&body).expect("parse lifecycle check-in response");
    assert_eq!(check_in["newlyCheckedIn"], true);
    let activity_date = check_in["date"].as_str().expect("check-in activity date");

    let mut tx = pool.begin().await.expect("begin trust progress initialization");
    activity::trust::ensure_registered_progress(&mut tx, account_id)
        .await
        .expect("initialize exported trust progress");
    tx.commit().await.expect("commit trust progress initialization");

    let mut tx = pool.begin().await.expect("begin immutable check-in assertion");
    let source_key = format!("check_in:{account_id}:{activity_date}");
    deactivate_contribution(&mut tx, &source_key, Utc::now())
        .await
        .expect_err("check-in contribution must not be reversible");
    tx.commit().await.expect("commit immutable check-in assertion");

    let export = activity::data_export::snapshot(&pool, account_id)
        .await
        .expect("snapshot activity lifecycle data");
    let export = serde_json::to_value(export).expect("serialize activity lifecycle export");
    assert_eq!(export["days"].as_array().expect("exported activity days").len(), 1);
    assert_eq!(export["days"][0]["checkIns"], 1);
    assert_eq!(export["days"][0]["score"], export["scoreProjection"]["qualifyingScore"]);
    assert_eq!(export["checkIns"].as_array().expect("exported check-ins").len(), 1);
    assert!(export["scoreProjection"]["scorePolicyVersion"].is_number());
    assert!(export["scoreProjection"]["trustPolicyVersion"].is_number());
    assert_eq!(export["trustProgress"]["qualifyingScore"], export["days"][0]["score"]);
    assert!(export["trustEvents"]
        .as_array()
        .expect("exported trust history")
        .iter()
        .any(|event| event["eventKind"] == "registration"));

    activity::data_export::purge_account_data(&pool, account_id)
        .await
        .expect("purge activity lifecycle data");
    let remaining: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT \
           (SELECT COUNT(*) FROM activity.check_ins WHERE account_id = $1), \
           (SELECT COUNT(*) FROM activity.events WHERE account_id = $1), \
           (SELECT COUNT(*) FROM activity.daily_counts WHERE account_id = $1), \
           (SELECT COUNT(*) FROM activity.account_scores WHERE account_id = $1), \
           (SELECT COUNT(*) FROM activity.account_trust_progress WHERE account_id = $1), \
           (SELECT COUNT(*) FROM activity.trust_level_events WHERE account_id = $1)",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("verify purged activity lifecycle rows");
    assert_eq!(remaining, (0, 0, 0, 0, 0, 0));

    let after_purge = activity::data_export::snapshot(&pool, account_id)
        .await
        .expect("snapshot purged activity lifecycle data");
    let after_purge =
        serde_json::to_value(after_purge).expect("serialize purged activity lifecycle export");
    assert!(after_purge["days"].as_array().expect("purged activity days").is_empty());
    assert!(after_purge["checkIns"].as_array().expect("purged check-ins").is_empty());
    assert!(after_purge["scoreProjection"].is_null());
    assert!(after_purge["trustProgress"].is_null());
    assert!(after_purge["trustEvents"].as_array().expect("purged trust events").is_empty());

    sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("delete activity lifecycle account");
}

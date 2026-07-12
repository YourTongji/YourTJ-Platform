use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use shared::AppState;
use sqlx::PgPool;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to activity check-in database");
    MIGRATOR.run(&pool).await.expect("run check-in test migrations");
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

async fn request(app: axum::Router, token: &str, method: &str) -> serde_json::Value {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri("/api/v2/me/check-in")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build check-in request"),
        )
        .await
        .expect("send check-in request");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.expect("read check-in body");
    serde_json::from_slice(&bytes).expect("parse check-in body")
}

#[tokio::test]
async fn daily_check_in_is_idempotent_and_extends_the_streak_once() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("check-in-{suffix}@tongji.edu.cn"))
    .bind(format!("check-in-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert check-in account");
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(&pool)
            .await
            .expect("read canonical day");
    sqlx::query(
        "INSERT INTO activity.check_ins (account_id, activity_date, checked_in_at) \
         VALUES ($1, $2::date - 1, (($2::date - 1)::timestamp + interval '12 hours') \
           AT TIME ZONE 'Asia/Shanghai')",
    )
    .bind(account_id)
    .bind(today)
    .execute(&pool)
    .await
    .expect("insert previous check-in");
    let token =
        identity::auth::create_access_token(account_id, "integration-test-secret-32bytes!", 3600)
            .expect("create check-in access token");
    let app = activity::routes(test_state(pool.clone()));

    let first = request(app.clone(), &token, "POST").await;
    assert_eq!(first["checkedIn"], true);
    assert_eq!(first["newlyCheckedIn"], true);
    assert_eq!(first["currentStreak"], 2);
    assert_eq!(first["totalDays"], 2);

    let repeated = request(app.clone(), &token, "POST").await;
    assert_eq!(repeated["checkedIn"], true);
    assert_eq!(repeated["newlyCheckedIn"], false);
    assert_eq!(repeated["currentStreak"], 2);
    assert_eq!(repeated["totalDays"], 2);

    let fetched = request(app.clone(), &token, "GET").await;
    assert_eq!(fetched["newlyCheckedIn"], false);
    assert_eq!(fetched["date"], today.to_string());

    let (check_ins, event_count): (i32, i64) = sqlx::query_as(
        "SELECT counts.check_ins, \
                (SELECT COUNT(*) FROM activity.events \
                 WHERE account_id = $1 AND kind = 'check_in' AND delta = 1) \
         FROM activity.daily_counts counts \
         WHERE counts.account_id = $1 AND counts.activity_date = $2",
    )
    .bind(account_id)
    .bind(today)
    .fetch_one(&pool)
    .await
    .expect("read projected check-in counts");
    assert_eq!((check_ins, event_count), (1, 1));

    let silenced_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("silenced-check-in-{suffix}@tongji.edu.cn"))
    .bind(format!("silenced-check-in-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert silenced check-in account");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason) \
         VALUES ($1, 'silence', 'check-in write restriction test')",
    )
    .bind(silenced_id)
    .execute(&pool)
    .await
    .expect("silence check-in account");
    let silenced_token =
        identity::auth::create_access_token(silenced_id, "integration-test-secret-32bytes!", 3600)
            .expect("create silenced access token");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/me/check-in")
                .header(header::AUTHORIZATION, format!("Bearer {silenced_token}"))
                .body(Body::empty())
                .expect("build silenced check-in request"),
        )
        .await
        .expect("send silenced check-in request");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let silenced_check_ins: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM activity.check_ins WHERE account_id = $1")
            .bind(silenced_id)
            .fetch_one(&pool)
            .await
            .expect("read silenced check-ins");
    assert_eq!(silenced_check_ins, 0);
}

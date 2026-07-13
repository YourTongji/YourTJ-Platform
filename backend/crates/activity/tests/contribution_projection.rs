use activity::contributions::{activate_contribution, deactivate_contribution, ActivityKind};
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use chrono::{TimeZone, Utc};
use shared::AppState;
use sqlx::PgPool;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to activity test database");
    MIGRATOR.run(&pool).await.expect("run activity test migrations");
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
async fn duplicate_activation_and_reversal_are_idempotent() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin test transaction");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("activity-{suffix}@tongji.edu.cn"))
    .bind(format!("activity-{suffix}"))
    .fetch_one(&mut *tx)
    .await
    .expect("insert activity account");
    let source_key = format!("test_like:{suffix}");
    let occurred_at =
        Utc.with_ymd_and_hms(2026, 7, 10, 16, 30, 0).single().expect("valid timestamp");

    assert!(activate_contribution(
        &mut tx,
        account_id,
        ActivityKind::Like,
        &source_key,
        occurred_at,
    )
    .await
    .expect("activate contribution"));
    assert!(!activate_contribution(
        &mut tx,
        account_id,
        ActivityKind::Like,
        &source_key,
        occurred_at,
    )
    .await
    .expect("duplicate activation is a no-op"));

    let likes_after_activation: i32 = sqlx::query_scalar(
        "SELECT likes_given FROM activity.daily_counts \
         WHERE account_id = $1 AND activity_date = DATE '2026-07-11'",
    )
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await
    .expect("read activated count");
    assert_eq!(likes_after_activation, 1);

    assert!(deactivate_contribution(&mut tx, &source_key, Utc::now())
        .await
        .expect("deactivate contribution"));
    assert!(!deactivate_contribution(&mut tx, &source_key, Utc::now())
        .await
        .expect("duplicate deactivation is a no-op"));

    let (likes_after_reversal, event_count): (i32, i64) = sqlx::query_as(
        "SELECT counts.likes_given, COUNT(events.id) \
         FROM activity.daily_counts counts \
         JOIN activity.events events \
           ON events.account_id = counts.account_id \
          AND events.activity_date = counts.activity_date \
         WHERE counts.account_id = $1 AND counts.activity_date = DATE '2026-07-11' \
         GROUP BY counts.likes_given",
    )
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await
    .expect("read reversed projection");
    assert_eq!(likes_after_reversal, 0);
    assert_eq!(event_count, 2);

    tx.rollback().await.expect("rollback test transaction");
}

#[tokio::test]
async fn reactivation_moves_the_single_active_like_to_the_new_day() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin test transaction");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("reactivate-{suffix}@tongji.edu.cn"))
    .bind(format!("reactivate-{suffix}"))
    .fetch_one(&mut *tx)
    .await
    .expect("insert activity account");
    let source_key = format!("test_toggle:{suffix}");
    let first_day =
        Utc.with_ymd_and_hms(2026, 7, 10, 10, 0, 0).single().expect("valid first timestamp");
    let second_day =
        Utc.with_ymd_and_hms(2026, 7, 11, 10, 0, 0).single().expect("valid second timestamp");

    activate_contribution(&mut tx, account_id, ActivityKind::Like, &source_key, first_day)
        .await
        .expect("activate first day");
    deactivate_contribution(&mut tx, &source_key, second_day).await.expect("reverse first day");
    activate_contribution(&mut tx, account_id, ActivityKind::Like, &source_key, second_day)
        .await
        .expect("reactivate second day");

    let rows: Vec<(chrono::NaiveDate, i32)> = sqlx::query_as(
        "SELECT activity_date, likes_given FROM activity.daily_counts \
         WHERE account_id = $1 ORDER BY activity_date",
    )
    .bind(account_id)
    .fetch_all(&mut *tx)
    .await
    .expect("read toggle projection");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, 0);
    assert_eq!(rows[1].1, 1);

    tx.rollback().await.expect("rollback test transaction");
}

#[tokio::test]
async fn activity_endpoint_zero_fills_the_inclusive_range() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("calendar-{suffix}@tongji.edu.cn"))
    .bind(format!("calendar-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("insert calendar account");
    let token =
        identity::auth::create_access_token(account_id, "integration-test-secret-32bytes!", 3600)
            .expect("create calendar access token");
    let app = activity::routes(test_state(pool.clone()));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/me/activity?from=2026-07-01&to=2026-07-03")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build activity request"),
        )
        .await
        .expect("activity request succeeds");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.expect("read activity response");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("parse activity response");
    assert_eq!(body["timezone"], "Asia/Shanghai");
    assert_eq!(body["days"].as_array().expect("activity days").len(), 3);
    assert_eq!(body["days"][0]["date"], "2026-07-01");
    assert_eq!(body["days"][2]["date"], "2026-07-03");
    assert!(body["days"]
        .as_array()
        .expect("activity days")
        .iter()
        .all(|day| day["threads"] == 0 && day["comments"] == 0 && day["likes"] == 0));

    sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("delete calendar account");
}

#[tokio::test]
async fn existing_contribution_backfill_is_complete_and_idempotent() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin backfill transaction");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let author_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("backfill-author-{suffix}@tongji.edu.cn"))
    .bind(format!("backfill-author-{suffix}"))
    .fetch_one(&mut *tx)
    .await
    .expect("insert backfill author");
    let liker_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("backfill-liker-{suffix}@tongji.edu.cn"))
    .bind(format!("backfill-liker-{suffix}"))
    .fetch_one(&mut *tx)
    .await
    .expect("insert backfill liker");
    let course_id: i64 =
        sqlx::query_scalar("INSERT INTO courses.courses (code, name) VALUES ($1, $2) RETURNING id")
            .bind(format!("BACKFILL-{suffix}"))
            .bind("Backfill course")
            .fetch_one(&mut *tx)
            .await
            .expect("insert backfill course");
    let board_id: i64 =
        sqlx::query_scalar("INSERT INTO forum.boards (slug, name) VALUES ($1, $2) RETURNING id")
            .bind(format!("backfill-{suffix}"))
            .bind("Backfill board")
            .fetch_one(&mut *tx)
            .await
            .expect("insert backfill board");
    let occurred_at =
        Utc.with_ymd_and_hms(2026, 7, 10, 16, 30, 0).single().expect("backfill timestamp");
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body, created_at) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(board_id)
    .bind(author_id)
    .bind("Existing visible thread")
    .bind("body")
    .bind(occurred_at)
    .fetch_one(&mut *tx)
    .await
    .expect("insert existing thread");
    let comment_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body, created_at) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(thread_id)
    .bind(author_id)
    .bind("Existing visible comment")
    .bind(occurred_at)
    .fetch_one(&mut *tx)
    .await
    .expect("insert existing comment");
    let hidden_thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads \
         (board_id, author_id, title, body, created_at, hidden_at) \
         VALUES ($1, $2, $3, $4, $5, $5) RETURNING id",
    )
    .bind(board_id)
    .bind(author_id)
    .bind("Existing hidden thread")
    .bind("hidden body")
    .bind(occurred_at)
    .fetch_one(&mut *tx)
    .await
    .expect("insert hidden thread excluded from backfill");
    let review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
         VALUES ($1, $2, 4, 'Existing visible review', 'visible') RETURNING id",
    )
    .bind(course_id)
    .bind(author_id)
    .fetch_one(&mut *tx)
    .await
    .expect("insert existing review");
    let hidden_review_id: i64 = sqlx::query_scalar(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
         VALUES ($1, $2, 1, 'Existing hidden review', 'hidden') RETURNING id",
    )
    .bind(course_id)
    .bind(author_id)
    .fetch_one(&mut *tx)
    .await
    .expect("insert hidden review excluded from backfill");
    sqlx::query(
        "INSERT INTO forum.votes \
         (post_type, post_id, account_id, value, created_at, updated_at) \
         VALUES ('thread', $1, $2, 1, $3, $3)",
    )
    .bind(thread_id)
    .bind(liker_id)
    .bind(occurred_at)
    .execute(&mut *tx)
    .await
    .expect("insert existing forum vote");
    sqlx::query(
        "INSERT INTO forum.votes \
         (post_type, post_id, account_id, value, created_at, updated_at) \
         VALUES ('thread', $1, $2, 1, $3, $3)",
    )
    .bind(hidden_thread_id)
    .bind(liker_id)
    .bind(occurred_at)
    .execute(&mut *tx)
    .await
    .expect("insert hidden forum vote excluded from backfill");
    sqlx::query(
        "INSERT INTO reviews.review_likes (review_id, account_id, created_at) \
         VALUES ($1, $2, $3)",
    )
    .bind(review_id)
    .bind(liker_id)
    .bind(occurred_at)
    .execute(&mut *tx)
    .await
    .expect("insert existing review like");
    sqlx::query(
        "INSERT INTO reviews.review_likes (review_id, account_id, created_at) \
         VALUES ($1, $2, $3)",
    )
    .bind(hidden_review_id)
    .bind(liker_id)
    .bind(occurred_at)
    .execute(&mut *tx)
    .await
    .expect("insert hidden review like excluded from backfill");

    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../../../migrations/0027_activity_backfill.sql"))
            .execute(&mut *tx)
            .await
            .expect("run idempotent activity backfill");
    }

    let author_counts: (i32, i32, i32) = sqlx::query_as(
        "SELECT threads_created, comments_created, likes_given \
         FROM activity.daily_counts \
         WHERE account_id = $1 AND activity_date = DATE '2026-07-11'",
    )
    .bind(author_id)
    .fetch_one(&mut *tx)
    .await
    .expect("author backfill counts");
    assert_eq!(author_counts, (1, 1, 0));
    let liker_counts: (i32, i32, i32) = sqlx::query_as(
        "SELECT threads_created, comments_created, likes_given \
         FROM activity.daily_counts \
         WHERE account_id = $1 AND activity_date = DATE '2026-07-11'",
    )
    .bind(liker_id)
    .fetch_one(&mut *tx)
    .await
    .expect("liker backfill counts");
    assert_eq!(liker_counts, (0, 0, 2));
    let event_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM activity.events WHERE account_id = ANY($1)")
            .bind(vec![author_id, liker_id])
            .fetch_one(&mut *tx)
            .await
            .expect("backfilled events");
    assert_eq!(event_count, 4);

    let comment_event: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM activity.events WHERE source_key = $1)")
            .bind(format!("forum_comment:{comment_id}"))
            .fetch_one(&mut *tx)
            .await
            .expect("comment backfill event");
    assert!(comment_event);

    tx.rollback().await.expect("rollback backfill transaction");
}

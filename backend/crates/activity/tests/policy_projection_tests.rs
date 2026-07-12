use std::time::Duration;

use activity::contributions::{activate_contribution, ActivityKind};
use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use chrono::Utc;
use serde_json::{json, Value};
use shared::AppState;
use sqlx::{FromRow, PgPool};
use tokio::sync::oneshot;
use tower::ServiceExt;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

#[derive(Clone, Debug, FromRow)]
struct PolicySnapshot {
    score_version: i64,
    trust_version: i64,
    thread_weight: i32,
    comment_weight: i32,
    like_weight: i32,
    check_in_weight: i32,
    threshold_level_2: i32,
    threshold_level_3: i32,
    threshold_level_4: i32,
    threshold_level_5: i32,
    threshold_level_6: i32,
    like_daily_cap: i32,
    demotion_cooldown_days: i32,
}

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to activity policy test database");
    MIGRATOR.run(&pool).await.expect("run activity policy test migrations");
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

async fn insert_account(pool: &PgPool, label: &str, role: &str) -> i64 {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, $3::identity.account_role) RETURNING id",
    )
    .bind(format!("{label}-{suffix}@tongji.edu.cn"))
    .bind(format!("{label}-{suffix}"))
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("insert activity policy test account")
}

async fn current_policy_snapshot(pool: &PgPool) -> PolicySnapshot {
    sqlx::query_as(
        "SELECT score.version AS score_version, trust.version AS trust_version, \
                score.thread_weight, score.comment_weight, score.like_weight, \
                score.check_in_weight, trust.threshold_level_2, trust.threshold_level_3, \
                trust.threshold_level_4, trust.threshold_level_5, trust.threshold_level_6, \
                trust.like_daily_cap, trust.demotion_cooldown_days \
         FROM activity.trust_level_policies trust \
         INNER JOIN activity.score_policies score ON score.version = trust.score_policy_version \
         ORDER BY trust.version DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("load current activity and trust policy")
}

async fn request_json(
    app: axum::Router,
    token: &str,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let request_body = match body {
        Some(value) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&value).expect("serialize policy request"))
        }
        None => Body::empty(),
    };
    let response = app
        .oneshot(builder.body(request_body).expect("build activity policy request"))
        .await
        .expect("send activity policy request");
    let status = response.status();
    let bytes =
        to_bytes(response.into_body(), 1024 * 1024).await.expect("read activity policy response");
    let value = serde_json::from_slice(&bytes).expect("parse activity policy response");
    (status, value)
}

async fn update_activity_policy(
    app: axum::Router,
    token: &str,
    expected_version: i64,
    weights: (i32, i32, i32, i32),
    reason: &str,
) -> Value {
    let (status, body) = request_json(
        app,
        token,
        Method::PUT,
        "/api/v2/admin/activity-policy",
        Some(json!({
            "expectedVersion": expected_version,
            "weights": {
                "thread": weights.0,
                "comment": weights.1,
                "like": weights.2,
                "checkIn": weights.3,
            },
            "reason": reason,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "activity policy response: {body}");
    body
}

async fn update_trust_policy(
    app: axum::Router,
    token: &str,
    expected_version: i64,
    policy: &PolicySnapshot,
    like_daily_cap: i32,
    reason: &str,
) -> Value {
    let (status, body) = request_json(
        app,
        token,
        Method::PUT,
        "/api/v2/admin/trust-policy",
        Some(json!({
            "expectedVersion": expected_version,
            "thresholdLevel2": policy.threshold_level_2,
            "thresholdLevel3": policy.threshold_level_3,
            "thresholdLevel4": policy.threshold_level_4,
            "thresholdLevel5": policy.threshold_level_5,
            "thresholdLevel6": policy.threshold_level_6,
            "likeDailyCap": like_daily_cap,
            "demotionCooldownDays": policy.demotion_cooldown_days,
            "reason": reason,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "trust policy response: {body}");
    body
}

#[tokio::test]
async fn weight_and_like_cap_updates_reproject_scores_and_calendar_versions() {
    let pool = test_pool().await;
    let original = current_policy_snapshot(&pool).await;
    let admin_id = insert_account(&pool, "activity-policy-admin", "admin").await;
    let user_id = insert_account(&pool, "activity-policy-user", "user").await;
    let admin_token =
        identity::auth::create_access_token(admin_id, "integration-test-secret-32bytes!", 3600)
            .expect("create admin access token");
    let user_token =
        identity::auth::create_access_token(user_id, "integration-test-secret-32bytes!", 3600)
            .expect("create user access token");
    let app = activity::routes(test_state(pool.clone()));
    let occurred_at = Utc::now();
    let mut tx = pool.begin().await.expect("begin activity projection seed");
    activate_contribution(
        &mut tx,
        user_id,
        ActivityKind::Thread,
        &format!("policy-thread:{user_id}"),
        occurred_at,
    )
    .await
    .expect("activate policy thread");
    activate_contribution(
        &mut tx,
        user_id,
        ActivityKind::Comment,
        &format!("policy-comment:{user_id}"),
        occurred_at,
    )
    .await
    .expect("activate policy comment");
    for index in 0..3 {
        activate_contribution(
            &mut tx,
            user_id,
            ActivityKind::Like,
            &format!("policy-like:{user_id}:{index}"),
            occurred_at,
        )
        .await
        .expect("activate policy like");
    }
    tx.commit().await.expect("commit activity projection seed");
    let (check_in_status, check_in_body) =
        request_json(app.clone(), &user_token, Method::POST, "/api/v2/me/check-in", None).await;
    assert_eq!(check_in_status, StatusCode::OK, "check-in response: {check_in_body}");

    let changed_weights = (
        if original.thread_weight < 1000 { original.thread_weight + 1 } else { 999 },
        if original.comment_weight < 1000 { original.comment_weight + 1 } else { 999 },
        if original.like_weight < 1000 { original.like_weight + 1 } else { 999 },
        if original.check_in_weight < 1000 { original.check_in_weight + 1 } else { 999 },
    );
    let activity_policy = update_activity_policy(
        app.clone(),
        &admin_token,
        original.score_version,
        changed_weights,
        "verify score reproject after weight update",
    )
    .await;
    let score_policy_version = activity_policy["version"].as_i64().expect("score policy version");
    let after_weight = current_policy_snapshot(&pool).await;
    let expected_after_weight = i64::from(changed_weights.0)
        + i64::from(changed_weights.1)
        + (3_i64 * i64::from(changed_weights.2)).min(i64::from(original.like_daily_cap))
        + i64::from(changed_weights.3);
    let (daily_score, account_score, stored_score_version, stored_trust_version): (
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT counts.score, scores.qualifying_score, scores.score_policy_version, \
                scores.trust_policy_version \
         FROM activity.daily_counts counts \
         INNER JOIN activity.account_scores scores ON scores.account_id = counts.account_id \
         WHERE counts.account_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read score projection after weight update");
    assert_eq!((daily_score, account_score), (expected_after_weight, expected_after_weight));
    assert_eq!(stored_score_version, score_policy_version);
    assert_eq!(stored_trust_version, after_weight.trust_version);

    let changed_cap = if original.like_daily_cap == 5 { 6 } else { 5 };
    let trust_policy = update_trust_policy(
        app.clone(),
        &admin_token,
        after_weight.trust_version,
        &after_weight,
        changed_cap,
        "verify score reproject after like cap update",
    )
    .await;
    let trust_policy_version = trust_policy["version"].as_i64().expect("trust policy version");
    let expected_after_cap = i64::from(changed_weights.0)
        + i64::from(changed_weights.1)
        + (3_i64 * i64::from(changed_weights.2)).min(i64::from(changed_cap))
        + i64::from(changed_weights.3);
    let (daily_score, account_score, stored_score_version, stored_trust_version): (
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT counts.score, scores.qualifying_score, scores.score_policy_version, \
                scores.trust_policy_version \
         FROM activity.daily_counts counts \
         INNER JOIN activity.account_scores scores ON scores.account_id = counts.account_id \
         WHERE counts.account_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read score projection after cap update");
    assert_eq!((daily_score, account_score), (expected_after_cap, expected_after_cap));
    assert_eq!(stored_score_version, score_policy_version);
    assert_eq!(stored_trust_version, trust_policy_version);

    let activity_date: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(&pool)
            .await
            .expect("read calendar activity date");
    let (calendar_status, calendar) = request_json(
        app.clone(),
        &user_token,
        Method::GET,
        &format!("/api/v2/me/activity?from={activity_date}&to={activity_date}"),
        None,
    )
    .await;
    assert_eq!(calendar_status, StatusCode::OK, "calendar response: {calendar}");
    assert_eq!(calendar["policyVersion"], score_policy_version);
    assert_eq!(calendar["trustPolicyVersion"], trust_policy_version);
    assert_eq!(calendar["likeDailyCap"], changed_cap);
    assert_eq!(calendar["days"][0]["score"], expected_after_cap);
    assert_eq!(calendar["days"][0]["checkIns"], 1);

    restore_policy_with_pool(&pool, app, &admin_token, &original).await;
}

#[tokio::test]
async fn contribution_committed_while_policy_waits_finishes_on_the_new_policy_version() {
    let pool = test_pool().await;
    let original = current_policy_snapshot(&pool).await;
    let admin_id = insert_account(&pool, "concurrent-policy-admin", "admin").await;
    let user_id = insert_account(&pool, "concurrent-policy-user", "user").await;
    let admin_token =
        identity::auth::create_access_token(admin_id, "integration-test-secret-32bytes!", 3600)
            .expect("create concurrent policy admin token");
    let app = activity::routes(test_state(pool.clone()));
    let (ready_sender, ready_receiver) = oneshot::channel();
    let (release_sender, release_receiver) = oneshot::channel();
    let contribution_pool = pool.clone();
    let contribution_task = tokio::spawn(async move {
        let mut tx = contribution_pool.begin().await.expect("begin concurrent contribution");
        activate_contribution(
            &mut tx,
            user_id,
            ActivityKind::Thread,
            &format!("concurrent-policy-thread:{user_id}"),
            Utc::now(),
        )
        .await
        .expect("activate concurrent contribution");
        ready_sender.send(()).expect("signal contribution projection lock");
        release_receiver.await.expect("release concurrent contribution");
        tx.commit().await.expect("commit concurrent contribution");
    });
    ready_receiver.await.expect("wait for contribution projection lock");

    let changed_weights = (
        if original.thread_weight < 1000 { original.thread_weight + 1 } else { 999 },
        original.comment_weight,
        original.like_weight,
        original.check_in_weight,
    );
    let policy_app = app.clone();
    let policy_token = admin_token.clone();
    let expected_score_version = original.score_version;
    let policy_task = tokio::spawn(async move {
        update_activity_policy(
            policy_app,
            &policy_token,
            expected_score_version,
            changed_weights,
            "verify contribution and policy serialization",
        )
        .await
    });
    wait_for_projection_lock_wait(&pool).await;
    release_sender.send(()).expect("release contribution transaction");
    contribution_task.await.expect("join contribution task");
    let policy_response = policy_task.await.expect("join policy task");
    let score_policy_version =
        policy_response["version"].as_i64().expect("concurrent score policy version");
    let current = current_policy_snapshot(&pool).await;
    let (score, stored_score_version, stored_trust_version): (i64, i64, i64) = sqlx::query_as(
        "SELECT qualifying_score, score_policy_version, trust_policy_version \
         FROM activity.account_scores WHERE account_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read concurrent account score projection");
    assert_eq!(score, i64::from(changed_weights.0));
    assert_eq!(stored_score_version, score_policy_version);
    assert_eq!(stored_trust_version, current.trust_version);

    restore_policy_with_pool(&pool, app, &admin_token, &original).await;
}

#[tokio::test]
async fn concurrent_contributions_add_deltas_without_losing_an_account_score_update() {
    let pool = test_pool().await;
    let policy = current_policy_snapshot(&pool).await;
    assert!(policy.thread_weight > 0, "canonical activity policy must score threads");
    let user_id = insert_account(&pool, "concurrent-contribution-user", "user").await;
    let mut seed_tx = pool.begin().await.expect("begin account score seed");
    activate_contribution(
        &mut seed_tx,
        user_id,
        ActivityKind::Thread,
        &format!("concurrent-score-baseline:{user_id}"),
        Utc::now(),
    )
    .await
    .expect("activate account score baseline");
    seed_tx.commit().await.expect("commit account score baseline");

    let mut blocker = pool.begin().await.expect("begin account score blocker");
    sqlx::query_scalar::<_, i64>(
        "SELECT account_id FROM activity.account_scores WHERE account_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut *blocker)
    .await
    .expect("lock account score projection row");
    let first_pool = pool.clone();
    let first_task = tokio::spawn(async move {
        let mut tx = first_pool.begin().await.expect("begin first concurrent contribution");
        activate_contribution(
            &mut tx,
            user_id,
            ActivityKind::Thread,
            &format!("concurrent-score-first:{user_id}"),
            Utc::now() + chrono::Duration::days(1),
        )
        .await
        .expect("activate first concurrent contribution");
        tx.commit().await.expect("commit first concurrent contribution");
    });
    let second_pool = pool.clone();
    let second_task = tokio::spawn(async move {
        let mut tx = second_pool.begin().await.expect("begin second concurrent contribution");
        activate_contribution(
            &mut tx,
            user_id,
            ActivityKind::Thread,
            &format!("concurrent-score-second:{user_id}"),
            Utc::now() + chrono::Duration::days(2),
        )
        .await
        .expect("activate second concurrent contribution");
        tx.commit().await.expect("commit second concurrent contribution");
    });
    wait_for_account_score_writers(&pool).await;
    blocker.commit().await.expect("release account score projection row");
    first_task.await.expect("join first concurrent contribution");
    second_task.await.expect("join second concurrent contribution");

    let (qualifying_score, daily_score): (i64, i64) = sqlx::query_as(
        "SELECT scores.qualifying_score, SUM(counts.score)::bigint \
         FROM activity.account_scores scores \
         INNER JOIN activity.daily_counts counts ON counts.account_id = scores.account_id \
         WHERE scores.account_id = $1 \
         GROUP BY scores.qualifying_score",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read concurrent contribution projection");
    assert_eq!(qualifying_score, i64::from(policy.thread_weight) * 3);
    assert_eq!(qualifying_score, daily_score);

    sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("delete concurrent contribution account");
}

async fn restore_policy_with_pool(
    pool: &PgPool,
    app: axum::Router,
    token: &str,
    original: &PolicySnapshot,
) {
    let current = current_policy_snapshot(pool).await;
    update_activity_policy(
        app.clone(),
        token,
        current.score_version,
        (
            original.thread_weight,
            original.comment_weight,
            original.like_weight,
            original.check_in_weight,
        ),
        "restore activity policy after projection integration test",
    )
    .await;
    let current = current_policy_snapshot(pool).await;
    update_trust_policy(
        app,
        token,
        current.trust_version,
        original,
        original.like_daily_cap,
        "restore trust policy after projection integration test",
    )
    .await;
}

async fn wait_for_projection_lock_wait(pool: &PgPool) {
    for _ in 0..200 {
        let is_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE datname = current_database() \
                 AND pid <> pg_backend_pid() \
                 AND wait_event_type = 'Lock' \
                 AND wait_event = 'advisory' \
                 AND query LIKE '%activity.score_projection%' \
             )",
        )
        .fetch_one(pool)
        .await
        .expect("inspect projection advisory lock wait");
        if is_waiting {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("policy update did not wait for the contribution projection lock");
}

async fn wait_for_account_score_writers(pool: &PgPool) {
    for _ in 0..200 {
        let waiting_writers: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pg_stat_activity \
             WHERE datname = current_database() \
               AND pid <> pg_backend_pid() \
               AND wait_event_type = 'Lock' \
               AND query LIKE '%INSERT INTO activity.account_scores%'",
        )
        .fetch_one(pool)
        .await
        .expect("inspect account score projection writers");
        if waiting_writers >= 2 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("concurrent contributions did not both wait on the account score projection row");
}

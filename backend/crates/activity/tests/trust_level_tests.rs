//! Integration coverage for the unified activity-owned trust level system.
//!
//! Tests registration, one-step upgrades, demotion idempotency per governance
//! event, and override protection.

use activity::contributions::{activate_contribution, ActivityKind};
use activity::trust::{
    apply_governance_demotion_tx, ensure_registered_progress, run_trust_evaluation,
};
use chrono::{TimeZone, Utc};
use sqlx::PgPool;

use crate::helpers::{insert_test_account, set_manual_override};

mod helpers;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to activity trust test database");
    let has_identity: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'identity')",
    )
    .fetch_one(&pool)
    .await
    .expect("check identity schema");
    if !has_identity {
        MIGRATOR.run(&pool).await.expect("run trust test migrations");
    } else {
        let has_trust_policy: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
             WHERE table_schema = 'activity' AND table_name = 'trust_level_policies')",
        )
        .fetch_one(&pool)
        .await
        .expect("check trust policy table");
        if !has_trust_policy {
            sqlx::raw_sql(include_str!("../../../migrations/0059_activity_trust_levels.sql"))
                .execute(&pool)
                .await
                .expect("run migration 0059");
        }
    }
    pool
}

#[tokio::test]
async fn registration_grants_level_1_or_higher() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin tx");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = insert_test_account(
        &mut tx,
        &format!("reg-trust-{suffix}@tongji.edu.cn"),
        &format!("reg-trust-{suffix}"),
    )
    .await;

    let level = ensure_registered_progress(&mut tx, account_id).await.expect("register progress");
    assert!(level >= 1, "registered account must be at least Lv.1, got {level}");

    let projected: i16 =
        sqlx::query_scalar("SELECT trust_level FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&mut *tx)
            .await
            .expect("read projected level");
    assert_eq!(projected, level);

    // Idempotent: second call returns same level.
    let level2 =
        ensure_registered_progress(&mut tx, account_id).await.expect("register progress again");
    assert_eq!(level2, level);

    tx.rollback().await.expect("rollback");
}

#[tokio::test]
async fn upgrade_advances_at_most_one_level_per_evaluation() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin tx");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = insert_test_account(
        &mut tx,
        &format!("upgrade-{suffix}@tongji.edu.cn"),
        &format!("upgrade-{suffix}"),
    )
    .await;

    // 13 threads × 10 = 130 > threshold_level_3 (120). Qualifies for Lv.3
    // but one-step upgrade caps at Lv.2 after first evaluation.
    let occurred_at =
        Utc.with_ymd_and_hms(2026, 7, 10, 10, 0, 0).single().expect("valid timestamp");
    for day in 0..13 {
        let day_ts = occurred_at + chrono::Duration::days(day);
        let thread_key = format!("upgrade_thread:{suffix}:{day}");
        activate_contribution(&mut tx, account_id, ActivityKind::Thread, &thread_key, day_ts)
            .await
            .expect("activate thread contribution");
    }
    tx.commit().await.expect("commit seed");

    let (upgraded, _) = run_trust_evaluation(&pool).await;
    assert!(upgraded > 0);

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read progress after first eval");
    assert_eq!(level, 2, "first evaluation should advance from 1 → 2");

    let (upgraded2, _) = run_trust_evaluation(&pool).await;
    assert!(upgraded2 > 0);

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read progress after second eval");
    assert_eq!(level, 3, "second evaluation should advance from 2 → 3");
}

#[tokio::test]
async fn demotion_applies_once_per_governance_event() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin tx");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = insert_test_account(
        &mut tx,
        &format!("demote-{suffix}@tongji.edu.cn"),
        &format!("demote-{suffix}"),
    )
    .await;
    tx.commit().await.expect("commit account");

    // Seed the account at Lv.4 via direct SQL override so we can test demotion.
    set_manual_override(&pool, account_id, 4, "test: seed Lv.4 for demotion test").await;

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read level after override");
    assert_eq!(level, 4);

    // First demotion with governance event id 1.
    let mut tx2 = pool.begin().await.expect("begin demotion tx");
    let demoted = apply_governance_demotion_tx(&mut tx2, account_id, 1, "test demotion")
        .await
        .expect("first demotion");
    assert!(demoted);
    tx2.commit().await.expect("commit demotion");

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read after first demotion");
    assert_eq!(level, 3, "demotion should lower by exactly one level");

    // Second call with same governance event id is a no-op.
    let mut tx3 = pool.begin().await.expect("begin idempotency tx");
    let demoted_again = apply_governance_demotion_tx(&mut tx3, account_id, 1, "duplicate demotion")
        .await
        .expect("duplicate demotion check");
    assert!(!demoted_again, "duplicate governance event must not demote again");
    tx3.commit().await.expect("commit idempotency check");

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read after duplicate");
    assert_eq!(level, 3, "level unchanged after duplicate governance event");
}

#[tokio::test]
async fn evaluation_does_not_overwrite_manual_override() {
    let pool = test_pool().await;
    let mut tx = pool.begin().await.expect("begin tx");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account_id = insert_test_account(
        &mut tx,
        &format!("override-{suffix}@tongji.edu.cn"),
        &format!("override-{suffix}"),
    )
    .await;

    // Seed contributions so auto eval *would* promote.
    let occurred_at =
        Utc.with_ymd_and_hms(2026, 7, 10, 10, 0, 0).single().expect("valid timestamp");
    for day in 0..15 {
        let day_ts = occurred_at + chrono::Duration::days(day);
        let thread_key = format!("override_thread:{suffix}:{day}");
        activate_contribution(&mut tx, account_id, ActivityKind::Thread, &thread_key, day_ts)
            .await
            .expect("activate thread");
    }
    tx.commit().await.expect("commit seed");

    // Set manual override to Lv.2 via direct SQL.
    set_manual_override(&pool, account_id, 2, "test: pin at Lv.2").await;

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read after override");
    assert_eq!(level, 2);

    let has_override: bool = sqlx::query_scalar(
        "SELECT override_level IS NOT NULL \
         FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("check override flag");
    assert!(has_override);

    // Run evaluation — override must persist.
    run_trust_evaluation(&pool).await;
    run_trust_evaluation(&pool).await;

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read after evals");
    assert_eq!(level, 2, "manual override must not be overwritten by evaluation");

    let override_level: Option<i16> = sqlx::query_scalar(
        "SELECT override_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read override after evals");
    assert_eq!(override_level, Some(2), "override_level field must survive evaluation");
}

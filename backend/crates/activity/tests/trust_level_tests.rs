//! Integration coverage for the unified activity-owned trust level system.
//!
//! Tests registration, one-step upgrades, demotion idempotency per governance
//! event, and override protection.

use activity::contributions::{activate_contribution, ActivityKind};
use activity::trust::{
    adjust_trust_level, apply_governance_demotion_tx, ensure_registered_progress,
    run_scheduled_trust_evaluation, run_trust_evaluation,
};
use activity::TrustLevelAdjustInput;
use chrono::{TimeZone, Utc};
use sqlx::{PgConnection, PgPool};

use crate::helpers::{insert_test_account, set_manual_override};

mod helpers;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/yourtj_test".into());
    let pool = PgPool::connect(&url).await.expect("connect to activity trust test database");
    MIGRATOR.run(&pool).await.expect("run trust test migrations");
    pool
}

fn unique_governance_event_id() -> i64 {
    let value = uuid::Uuid::new_v4().as_u128() % (i64::MAX as u128 - 10_000);
    i64::try_from(value).expect("governance event fixture fits i64") + 10_000
}

async fn seed_level_two_score(connection: &mut PgConnection, account_id: i64, source_prefix: &str) {
    let (threshold, thread_weight): (i32, i32) = sqlx::query_as(
        "SELECT trust.threshold_level_2, score.thread_weight \
         FROM activity.trust_level_policies trust \
         INNER JOIN activity.score_policies score ON score.version = trust.score_policy_version \
         ORDER BY trust.version DESC LIMIT 1",
    )
    .fetch_one(&mut *connection)
    .await
    .expect("read trust threshold fixture");
    assert!(thread_weight > 0, "canonical activity policy must score threads");
    let contribution_count =
        (i64::from(threshold) + i64::from(thread_weight) - 1) / i64::from(thread_weight);
    for index in 0..contribution_count {
        activate_contribution(
            connection,
            account_id,
            ActivityKind::Thread,
            &format!("{source_prefix}:{index}"),
            Utc::now(),
        )
        .await
        .expect("seed level-two activity score");
    }
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
    let first_notification: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.outbox_events \
         WHERE recipient_account_id = $1 AND event_type = 'trust_level_upgraded'",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read first trust notification");
    assert_eq!(first_notification, 1);

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
async fn scheduled_workers_upgrade_each_account_at_most_once_per_day() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut tx = pool.begin().await.expect("begin scheduled evaluation seed");
    let account_id = insert_test_account(
        &mut tx,
        &format!("scheduled-trust-{suffix}@tongji.edu.cn"),
        &format!("scheduled-trust-{suffix}"),
    )
    .await;
    ensure_registered_progress(&mut tx, account_id).await.expect("initialize scheduled progress");
    for index in 0..13 {
        activate_contribution(
            &mut tx,
            account_id,
            ActivityKind::Thread,
            &format!("scheduled_thread:{suffix}:{index}"),
            Utc::now(),
        )
        .await
        .expect("seed scheduled contribution");
    }
    tx.commit().await.expect("commit scheduled seed");
    sqlx::query(
        "DELETE FROM activity.trust_evaluation_runs \
         WHERE activity_date = (now() AT TIME ZONE 'Asia/Shanghai')::date",
    )
    .execute(&pool)
    .await
    .expect("clear scheduled run fixture");
    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET last_scheduled_evaluation_date = (now() AT TIME ZONE 'Asia/Shanghai')::date \
         WHERE account_id <> $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("isolate scheduled account fixture");

    let (first, second) =
        tokio::join!(run_scheduled_trust_evaluation(&pool), run_scheduled_trust_evaluation(&pool));
    assert_eq!(first.0 + second.0, 1);
    assert_eq!(run_scheduled_trust_evaluation(&pool).await.0, 0);

    let level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read scheduled trust level");
    assert_eq!(level, 2);

    sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("delete scheduled account");
}

#[tokio::test]
async fn scheduled_worker_isolates_one_account_failure_and_recovers_it_on_retry() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut tx = pool.begin().await.expect("begin isolated failure seed");
    let failing_account_id = insert_test_account(
        &mut tx,
        &format!("trust-failure-{suffix}@tongji.edu.cn"),
        &format!("trust-failure-{suffix}"),
    )
    .await;
    let later_account_id = insert_test_account(
        &mut tx,
        &format!("trust-later-{suffix}@tongji.edu.cn"),
        &format!("trust-later-{suffix}"),
    )
    .await;
    ensure_registered_progress(&mut tx, failing_account_id)
        .await
        .expect("initialize failing account progress");
    ensure_registered_progress(&mut tx, later_account_id)
        .await
        .expect("initialize later account progress");
    seed_level_two_score(&mut tx, failing_account_id, &format!("trust-failure-thread:{suffix}"))
        .await;
    seed_level_two_score(&mut tx, later_account_id, &format!("trust-later-thread:{suffix}")).await;
    tx.commit().await.expect("commit isolated failure seed");
    let activity_date: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(&pool)
            .await
            .expect("read current activity date");
    sqlx::query("DELETE FROM activity.trust_evaluation_runs WHERE activity_date = $1")
        .bind(activity_date)
        .execute(&pool)
        .await
        .expect("reset scheduled run");
    sqlx::query(
        "UPDATE activity.account_trust_progress SET last_scheduled_evaluation_date = $2 \
         WHERE account_id NOT IN ($1, $3)",
    )
    .bind(failing_account_id)
    .bind(activity_date)
    .bind(later_account_id)
    .execute(&pool)
    .await
    .expect("isolate scheduled failure accounts");
    sqlx::raw_sql(&format!(
        "CREATE FUNCTION activity.test_reject_trust_projection() RETURNS TRIGGER \
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic trust projection failure'; END $$; \
         CREATE TRIGGER test_reject_trust_projection BEFORE UPDATE OF trust_level \
         ON identity.accounts FOR EACH ROW WHEN (OLD.id = {failing_account_id}) \
         EXECUTE FUNCTION activity.test_reject_trust_projection();"
    ))
    .execute(&pool)
    .await
    .expect("install account-specific projection failure");

    run_scheduled_trust_evaluation(&pool).await;
    let first_run: (String, i32, i64) = sqlx::query_as(
        "SELECT status, failed_count, \
                (SELECT COUNT(*) FROM activity.trust_evaluation_failures \
                 WHERE activity_date = run.activity_date) \
         FROM activity.trust_evaluation_runs run WHERE activity_date = $1",
    )
    .bind(activity_date)
    .fetch_one(&pool)
    .await
    .expect("read isolated failure run");
    assert_eq!(first_run, ("failed".into(), 1, 1));
    let later_level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(later_account_id)
    .fetch_one(&pool)
    .await
    .expect("read later account trust level");
    assert_eq!(later_level, 2, "one failure must not block a later account");

    sqlx::raw_sql(
        "DROP TRIGGER test_reject_trust_projection ON identity.accounts; \
         DROP FUNCTION activity.test_reject_trust_projection();",
    )
    .execute(&pool)
    .await
    .expect("remove projection failure");
    sqlx::query(
        "UPDATE activity.trust_evaluation_runs SET next_attempt_at = now() \
         WHERE activity_date = $1",
    )
    .bind(activity_date)
    .execute(&pool)
    .await
    .expect("make failed run retryable");
    run_scheduled_trust_evaluation(&pool).await;

    let recovered: (String, i64, i32, bool, i64) = sqlx::query_as(
        "SELECT status, \
                (SELECT COUNT(*) FROM activity.trust_evaluation_failures \
                 WHERE activity_date = run.activity_date), \
                attempts, lease_token IS NULL, cursor_account_id \
         FROM activity.trust_evaluation_runs run WHERE activity_date = $1",
    )
    .bind(activity_date)
    .fetch_one(&pool)
    .await
    .expect("read recovered run");
    assert_eq!(recovered.0, "completed");
    assert_eq!(recovered.1, 0);
    assert_eq!(recovered.2, 2);
    assert!(recovered.3);
    assert!(recovered.4 >= later_account_id);
    let recovered_level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(failing_account_id)
    .fetch_one(&pool)
    .await
    .expect("read recovered account trust level");
    assert_eq!(recovered_level, 2);
}

#[tokio::test]
async fn scheduled_worker_dead_letters_after_eight_failed_attempts() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut tx = pool.begin().await.expect("begin dead-letter seed");
    let account_id = insert_test_account(
        &mut tx,
        &format!("trust-dead-{suffix}@tongji.edu.cn"),
        &format!("trust-dead-{suffix}"),
    )
    .await;
    ensure_registered_progress(&mut tx, account_id).await.expect("initialize dead-letter progress");
    seed_level_two_score(&mut tx, account_id, &format!("trust-dead-thread:{suffix}")).await;
    tx.commit().await.expect("commit dead-letter seed");
    let activity_date: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(&pool)
            .await
            .expect("read dead-letter activity date");
    sqlx::query("DELETE FROM activity.trust_evaluation_runs WHERE activity_date = $1")
        .bind(activity_date)
        .execute(&pool)
        .await
        .expect("reset dead-letter run");
    sqlx::query(
        "UPDATE activity.account_trust_progress SET last_scheduled_evaluation_date = $2 \
         WHERE account_id <> $1",
    )
    .bind(account_id)
    .bind(activity_date)
    .execute(&pool)
    .await
    .expect("isolate dead-letter account");
    sqlx::raw_sql(&format!(
        "CREATE FUNCTION activity.test_reject_dead_trust_projection() RETURNS TRIGGER \
         LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic persistent trust failure'; END $$; \
         CREATE TRIGGER test_reject_dead_trust_projection BEFORE UPDATE OF trust_level \
         ON identity.accounts FOR EACH ROW WHEN (OLD.id = {account_id}) \
         EXECUTE FUNCTION activity.test_reject_dead_trust_projection();"
    ))
    .execute(&pool)
    .await
    .expect("install persistent projection failure");

    for attempt in 1..=8 {
        run_scheduled_trust_evaluation(&pool).await;
        if attempt < 8 {
            sqlx::query(
                "UPDATE activity.trust_evaluation_runs SET next_attempt_at = now() \
                 WHERE activity_date = $1",
            )
            .bind(activity_date)
            .execute(&pool)
            .await
            .expect("make next dead-letter attempt due");
        }
    }
    sqlx::raw_sql(
        "DROP TRIGGER test_reject_dead_trust_projection ON identity.accounts; \
         DROP FUNCTION activity.test_reject_dead_trust_projection();",
    )
    .execute(&pool)
    .await
    .expect("remove persistent projection failure");

    let run: (String, i32, i32, bool) = sqlx::query_as(
        "SELECT status, attempts, failed_count, lease_token IS NULL \
         FROM activity.trust_evaluation_runs WHERE activity_date = $1",
    )
    .bind(activity_date)
    .fetch_one(&pool)
    .await
    .expect("read dead-letter run");
    assert_eq!(run, ("dead".into(), 8, 1, true));
    let failure_attempts: i16 = sqlx::query_scalar(
        "SELECT attempts FROM activity.trust_evaluation_failures \
         WHERE activity_date = $1 AND account_id = $2",
    )
    .bind(activity_date)
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read dead-letter account failure");
    assert_eq!(failure_attempts, 8);
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

    let governance_event_id = unique_governance_event_id();
    let mut tx2 = pool.begin().await.expect("begin demotion tx");
    let demoted =
        apply_governance_demotion_tx(&mut tx2, account_id, governance_event_id, "test demotion")
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
    let demoted_again = apply_governance_demotion_tx(
        &mut tx3,
        account_id,
        governance_event_id,
        "duplicate demotion",
    )
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
async fn demotion_requires_elapsed_cooldown_and_new_activity_before_repromotion() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut tx = pool.begin().await.expect("begin cooldown trust seed");
    let account_id = insert_test_account(
        &mut tx,
        &format!("cooldown-trust-{suffix}@tongji.edu.cn"),
        &format!("cooldown-trust-{suffix}"),
    )
    .await;
    ensure_registered_progress(&mut tx, account_id).await.expect("initialize cooldown progress");
    seed_level_two_score(&mut tx, account_id, &format!("cooldown-thread:{suffix}")).await;
    tx.commit().await.expect("commit cooldown trust seed");

    run_trust_evaluation(&pool).await;
    let initial_level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read initial promoted level");
    assert_eq!(initial_level, 2);

    let mut tx = pool.begin().await.expect("begin cooldown demotion");
    assert!(apply_governance_demotion_tx(
        &mut tx,
        account_id,
        unique_governance_event_id(),
        "verified governance demotion cooldown test",
    )
    .await
    .expect("apply cooldown demotion"));
    tx.commit().await.expect("commit cooldown demotion");
    let (demoted_level, score_floor, qualifying_score, blocked_until): (
        i16,
        Option<i64>,
        i64,
        Option<chrono::DateTime<Utc>>,
    ) = sqlx::query_as(
        "SELECT trust_level, promotion_score_floor, qualifying_score, promotion_blocked_until \
         FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read demotion promotion gates");
    assert_eq!(demoted_level, 1);
    assert_eq!(score_floor, Some(qualifying_score));
    assert!(blocked_until.is_some_and(|blocked| blocked > Utc::now()));

    run_trust_evaluation(&pool).await;
    let level_during_cooldown: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read level during cooldown");
    assert_eq!(level_during_cooldown, 1);

    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET promotion_blocked_until = now() - interval '1 second' \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("advance cooldown test clock");
    run_trust_evaluation(&pool).await;
    let level_without_new_activity: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read level without new activity");
    assert_eq!(level_without_new_activity, 1);

    let mut tx = pool.begin().await.expect("begin post-demotion contribution");
    activate_contribution(
        &mut tx,
        account_id,
        ActivityKind::Thread,
        &format!("cooldown-new-thread:{suffix}"),
        Utc::now(),
    )
    .await
    .expect("activate post-demotion contribution");
    tx.commit().await.expect("commit post-demotion contribution");
    run_trust_evaluation(&pool).await;
    let (repromoted_level, blocked_until, score_floor): (
        i16,
        Option<chrono::DateTime<Utc>>,
        Option<i64>,
    ) = sqlx::query_as(
        "SELECT trust_level, promotion_blocked_until, promotion_score_floor \
         FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read repromoted trust progress");
    assert_eq!(repromoted_level, 2);
    assert!(blocked_until.is_none());
    assert!(score_floor.is_none());
    let upgrade_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity.trust_level_events \
         WHERE account_id = $1 AND event_kind = 'upgrade'",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count cooldown upgrade events");
    assert_eq!(upgrade_count, 2);
}

#[tokio::test]
async fn same_day_same_score_repromotion_uses_a_distinct_audit_key() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut tx = pool.begin().await.expect("begin repeat promotion seed");
    let account_id = insert_test_account(
        &mut tx,
        &format!("repeat-promotion-{suffix}@tongji.edu.cn"),
        &format!("repeat-promotion-{suffix}"),
    )
    .await;
    ensure_registered_progress(&mut tx, account_id)
        .await
        .expect("initialize repeat promotion progress");
    seed_level_two_score(&mut tx, account_id, &format!("repeat-promotion-thread:{suffix}")).await;
    tx.commit().await.expect("commit repeat promotion seed");
    let activity_date: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(&pool)
            .await
            .expect("read repeat promotion date");
    sqlx::query("DELETE FROM activity.trust_evaluation_runs WHERE activity_date = $1")
        .bind(activity_date)
        .execute(&pool)
        .await
        .expect("reset first scheduled run");

    run_scheduled_trust_evaluation(&pool).await;
    let first_level: i16 = sqlx::query_scalar(
        "SELECT trust_level FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("read first scheduled promotion");
    assert_eq!(first_level, 2);

    let mut tx = pool.begin().await.expect("begin repeat promotion demotion");
    apply_governance_demotion_tx(
        &mut tx,
        account_id,
        unique_governance_event_id(),
        "verified same-day repeat promotion test",
    )
    .await
    .expect("demote before repeat promotion");
    tx.commit().await.expect("commit repeat promotion demotion");
    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET promotion_blocked_until = now() - interval '1 second', \
             promotion_score_floor = NULL, last_scheduled_evaluation_date = NULL \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("simulate approved promotion-gate reset");
    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET last_scheduled_evaluation_date = $2 \
         WHERE account_id <> $1",
    )
    .bind(account_id)
    .bind(activity_date)
    .execute(&pool)
    .await
    .expect("isolate repeat scheduled promotion account");
    sqlx::query("DELETE FROM activity.trust_evaluation_runs WHERE activity_date = $1")
        .bind(activity_date)
        .execute(&pool)
        .await
        .expect("reset second scheduled run");

    run_scheduled_trust_evaluation(&pool).await;
    let events: Vec<(String, i64, chrono::NaiveDate)> = sqlx::query_as(
        "SELECT event_key, qualifying_score, \
                (created_at AT TIME ZONE 'Asia/Shanghai')::date \
         FROM activity.trust_level_events \
         WHERE account_id = $1 AND event_kind = 'upgrade' \
         ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(&pool)
    .await
    .expect("read repeat promotion audit events");
    assert_eq!(events.len(), 2);
    assert_ne!(events[0].0, events[1].0);
    assert_eq!(events[0].1, events[1].1);
    assert_eq!(events[0].2, activity_date);
    assert_eq!(events[1].2, activity_date);
    let notification_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform.outbox_events \
         WHERE recipient_account_id = $1 AND event_type = 'trust_level_upgraded'",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("count repeat promotion notifications");
    assert_eq!(notification_count, 2);
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

#[tokio::test]
async fn manual_adjustment_requires_an_active_strictly_lower_role_target() {
    let pool = test_pool().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mut tx = pool.begin().await.expect("begin hierarchy seed");
    let actor_id = insert_test_account(
        &mut tx,
        &format!("trust-admin-{suffix}@tongji.edu.cn"),
        &format!("trust-admin-{suffix}"),
    )
    .await;
    let user_id = insert_test_account(
        &mut tx,
        &format!("trust-user-{suffix}@tongji.edu.cn"),
        &format!("trust-user-{suffix}"),
    )
    .await;
    let peer_id = insert_test_account(
        &mut tx,
        &format!("trust-peer-{suffix}@tongji.edu.cn"),
        &format!("trust-peer-{suffix}"),
    )
    .await;
    let closed_id = insert_test_account(
        &mut tx,
        &format!("trust-closed-{suffix}@tongji.edu.cn"),
        &format!("trust-closed-{suffix}"),
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = ANY($1)")
        .bind(vec![actor_id, peer_id])
        .execute(&mut *tx)
        .await
        .expect("seed hierarchy roles");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deactivated', deactivated_at = now() \
         WHERE id = $1",
    )
    .bind(closed_id)
    .execute(&mut *tx)
    .await
    .expect("seed closed hierarchy target");
    tx.commit().await.expect("commit hierarchy seed");
    let input = TrustLevelAdjustInput {
        trust_level: Some(2),
        clear_override: false,
        reason: "verified community support adjustment".into(),
    };

    let adjusted = adjust_trust_level(&pool, user_id, &input, actor_id, "admin")
        .await
        .expect("adjust lower-role active target");
    assert_eq!(adjusted.trust_level, 2);
    assert!(adjust_trust_level(&pool, actor_id, &input, actor_id, "admin").await.is_err());
    assert!(adjust_trust_level(&pool, peer_id, &input, actor_id, "admin").await.is_err());
    assert!(adjust_trust_level(&pool, closed_id, &input, actor_id, "admin").await.is_err());
}

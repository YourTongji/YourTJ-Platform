//! Integration coverage for deterministic one-step trust transitions.

mod helpers;

use helpers::{create_test_account, create_test_app};

#[tokio::test]
async fn trust_scan_uses_active_days_and_never_skips_a_level() {
    let (pool, _) = create_test_app().await;
    let (account_id, _) =
        create_test_account(&pool, "trust-scan@tongji.edu.cn", "trust-scan").await;
    sqlx::query(
        "UPDATE identity.accounts SET created_at = now() - interval '100 days' WHERE id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("age trust account");
    sqlx::query(
        "INSERT INTO forum.user_stats (account_id, threads_created, comments_created, \
                                       votes_received, flags_upheld, flagged_upheld) \
         VALUES ($1, 3, 0, 50, 3, 0)",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("seed trust metrics");
    sqlx::query(
        "INSERT INTO forum.threads (board_id, author_id, title, created_at) \
         SELECT 1, $1, 'trust topic ' || day::text, now() - make_interval(days => day) \
         FROM generate_series(1, 20) day",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("seed trust topics");
    sqlx::query(
        "INSERT INTO forum.thread_reads (account_id, thread_id, updated_at) \
         SELECT $1, id, created_at FROM forum.threads WHERE author_id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("seed active read days");

    assert_eq!(forum::trust_levels::run_daily_tl_promotion(&pool).await, (1, 0));
    let level: i16 = sqlx::query_scalar("SELECT trust_level FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("trust level after first scan");
    assert_eq!(level, 1);

    assert_eq!(forum::trust_levels::run_daily_tl_promotion(&pool).await, (1, 0));
    assert_eq!(forum::trust_levels::run_daily_tl_promotion(&pool).await, (1, 0));
    let level: i16 = sqlx::query_scalar("SELECT trust_level FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("trust level after promotions");
    assert_eq!(level, 3);

    sqlx::query("UPDATE forum.user_stats SET flagged_upheld = 1 WHERE account_id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("seed upheld report");
    assert_eq!(forum::trust_levels::run_daily_tl_promotion(&pool).await, (0, 1));
    let level: i16 = sqlx::query_scalar("SELECT trust_level FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("trust level after demotion");
    assert_eq!(level, 2);
}

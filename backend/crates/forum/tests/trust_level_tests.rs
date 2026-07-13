//! Integration coverage for forum-level trust-level helpers.
//!
//! The activity domain owns trust evaluation; forum only reads and delegates.
//! This suite verifies flag_weight mapping and that the forum delegation layer
//! correctly calls activity trusted evaluation.

mod helpers;

use helpers::{create_test_account, create_test_app};

#[tokio::test]
async fn flag_weight_reads_from_activity_trust_delegate() {
    let (pool, _) = create_test_app().await;
    let (account_id, _) =
        create_test_account(&pool, "flag-weight@tongji.edu.cn", "flag-weight").await;

    // Seed the trust progress row at Lv.3 (governance demotion when flags upheld).
    sqlx::query(
        "INSERT INTO activity.account_trust_progress \
         (account_id, trust_level, qualifying_score, policy_version) \
         SELECT $1, 3, 130, version \
         FROM activity.trust_level_policies ORDER BY version DESC LIMIT 1 \
         ON CONFLICT (account_id) DO UPDATE \
         SET trust_level = 3, qualifying_score = 130, updated_at = now()",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("seed trust progress to Lv.3");
    sqlx::query("UPDATE identity.accounts SET trust_level = 3 WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("project trust level");

    let level = forum::trust_levels::get_trust_level(&pool, account_id)
        .await
        .expect("read trust level via forum delegate");
    assert_eq!(level, 3);
    assert!((forum::trust_levels::flag_weight(level) - 1.5_f32).abs() < f32::EPSILON);
}

#[tokio::test]
async fn trust_delegate_returns_zero_for_nonexistent_account() {
    let (pool, _) = create_test_app().await;
    let level = forum::trust_levels::get_trust_level(&pool, 999_999_999)
        .await
        .expect("read trust for missing account");
    assert_eq!(level, 0);
}

//! Integration coverage for identity APIs consumed by other domains.

#[path = "helpers/mod.rs"]
mod helpers;

use std::time::{Duration, Instant};

use sqlx::PgPool;

async fn wait_for_lock_wait(pool: &PgPool, query_prefix: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    let query_pattern = format!("{query_prefix}%");
    loop {
        let is_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE datname = current_database() AND pid <> pg_backend_pid() \
                 AND wait_event_type = 'Lock' AND ltrim(query) LIKE $1 \
             )",
        )
        .bind(&query_pattern)
        .fetch_one(pool)
        .await
        .expect("inspect account eligibility lock wait");
        if is_waiting {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn public_lookup_is_privacy_safe_and_system_silence_owns_identity_writes() {
    let (pool, _) = helpers::create_test_app().await;
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts ( \
           email, email_ciphertext, email_key_version, email_blind_index, handle \
         ) VALUES (NULL, 'opaque-ciphertext', 1, 'public-domain-blind-index', 'BoundaryUser') \
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert encrypted-only account");

    let account = identity::public_accounts::find_public_account_by_handle(&pool, "boundaryuser")
        .await
        .expect("public lookup")
        .expect("public account");
    assert_eq!(account.id, account_id);
    assert_eq!(account.handle, "BoundaryUser");
    assert_eq!(account.role, "user");

    sqlx::query(
        "UPDATE identity.accounts SET status = 'deleted', \
             deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 day', deleted_at = now() \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("mark account deleted");
    let deleted = identity::public_accounts::find_public_account_by_handle(&pool, "BoundaryUser")
        .await
        .expect("deleted lookup");
    assert!(deleted.is_none());

    let target_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) \
         VALUES ('system-silence-target@tongji.edu.cn', 'system-silence-target') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert sanction target");
    let metadata = serde_json::json!({ "source": "forum_threshold", "autoHideCount": 2 });
    let mut tx = pool.begin().await.expect("begin sanction transaction");
    let created = identity::sanctions::issue_system_silence_tx(
        &mut tx,
        target_id,
        "automated forum safety threshold",
        chrono::Utc::now() + chrono::Duration::hours(24),
        Some(&metadata),
    )
    .await
    .expect("issue system silence");
    assert!(created);
    tx.commit().await.expect("commit sanction transaction");

    let sanction: (Option<i64>, String) = sqlx::query_as(
        "SELECT issued_by, reason FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'silence' AND revoked_at IS NULL",
    )
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("system sanction row");
    assert!(sanction.0.is_none());
    assert_eq!(sanction.1, "automated forum safety threshold");

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE actor_account_id IS NULL AND action = 'identity.sanction.auto_silence' \
           AND target_type = 'account' AND target_id = $1",
    )
    .bind(target_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("system sanction audit");
    assert_eq!(audit_count, 1);

    let mut duplicate_tx = pool.begin().await.expect("begin duplicate transaction");
    let duplicate = identity::sanctions::issue_system_silence_tx(
        &mut duplicate_tx,
        target_id,
        "automated forum safety threshold",
        chrono::Utc::now() + chrono::Duration::hours(24),
        Some(&metadata),
    )
    .await
    .expect("check duplicate silence");
    assert!(!duplicate);
    duplicate_tx.rollback().await.expect("rollback duplicate transaction");

    let moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ('protected-mod@tongji.edu.cn', 'protected-mod', 'mod') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert moderator");
    let mut moderator_tx = pool.begin().await.expect("begin moderator transaction");
    let moderator_silenced = identity::sanctions::issue_system_silence_tx(
        &mut moderator_tx,
        moderator_id,
        "automated forum safety threshold",
        chrono::Utc::now() + chrono::Duration::hours(24),
        Some(&metadata),
    )
    .await
    .expect("protect moderator");
    assert!(!moderator_silenced);
    moderator_tx.rollback().await.expect("rollback moderator transaction");
}

#[tokio::test]
async fn credit_recipient_eligibility_ignores_future_suspend_until_it_starts() {
    let (pool, _) = helpers::create_test_app().await;
    let issuer_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) \
         VALUES ('future-suspend-issuer@tongji.edu.cn', 'future-suspend-issuer') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert future suspend issuer");
    let recipient_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) \
         VALUES ('future-suspend-recipient@tongji.edu.cn', 'future-suspend-recipient') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert future suspend recipient");
    let sanction_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sanctions \
         (account_id, kind, reason, issued_by, starts_at, ends_at) \
         VALUES ($1, 'suspend', 'scheduled suspension', $2, \
                 now() + interval '1 hour', now() + interval '2 hours') RETURNING id",
    )
    .bind(recipient_id)
    .bind(issuer_id)
    .fetch_one(&pool)
    .await
    .expect("insert future suspend sanction");

    let mut future_tx = pool.begin().await.expect("begin future eligibility transaction");
    let is_eligible =
        identity::public_accounts::is_credit_recipient_eligible(&mut future_tx, recipient_id)
            .await
            .expect("check future sanction eligibility");
    future_tx.rollback().await.expect("rollback future eligibility transaction");
    assert!(is_eligible);
    assert!(!identity::sanctions::is_suspended(None, &pool, recipient_id)
        .await
        .expect("check future authentication suspension"));

    sqlx::query(
        "UPDATE identity.sanctions SET starts_at = now() - interval '1 hour' WHERE id = $1",
    )
    .bind(sanction_id)
    .execute(&pool)
    .await
    .expect("activate suspend sanction");
    let mut active_tx = pool.begin().await.expect("begin active eligibility transaction");
    let is_eligible =
        identity::public_accounts::is_credit_recipient_eligible(&mut active_tx, recipient_id)
            .await
            .expect("check active sanction eligibility");
    active_tx.rollback().await.expect("rollback active eligibility transaction");
    assert!(!is_eligible);
    assert!(identity::sanctions::is_suspended(None, &pool, recipient_id)
        .await
        .expect("check active authentication suspension"));
}

#[tokio::test]
async fn credit_eligibility_rechecks_wall_clock_after_waiting_for_concurrent_suspend() {
    let (pool, _) = helpers::create_test_app().await;
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) \
         VALUES ('concurrent-suspend@tongji.edu.cn', 'concurrent-suspend') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert concurrent suspend account");

    let mut eligibility_tx = pool.begin().await.expect("begin eligibility transaction");
    let eligibility_started_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT transaction_timestamp()")
            .fetch_one(&mut *eligibility_tx)
            .await
            .expect("capture eligibility transaction timestamp");

    let mut sanction_tx = pool.begin().await.expect("begin concurrent suspension");
    sqlx::query("SELECT id FROM identity.accounts WHERE id = $1 FOR UPDATE")
        .bind(account_id)
        .execute(&mut *sanction_tx)
        .await
        .expect("lock concurrent suspend account");
    let sanction_started_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO identity.sanctions (account_id, kind, reason, starts_at) \
         VALUES ($1, 'suspend', 'concurrent eligibility regression', clock_timestamp()) \
         RETURNING starts_at",
    )
    .bind(account_id)
    .fetch_one(&mut *sanction_tx)
    .await
    .expect("stage concurrent suspension");
    assert!(sanction_started_at > eligibility_started_at);
    sqlx::query("UPDATE identity.accounts SET auth_version = auth_version + 1 WHERE id = $1")
        .bind(account_id)
        .execute(&mut *sanction_tx)
        .await
        .expect("stage suspension credential revocation");

    let eligibility_handle = tokio::spawn(async move {
        let is_eligible = identity::public_accounts::lock_active_interaction_accounts(
            &mut eligibility_tx,
            &[account_id],
        )
        .await
        .expect("recheck eligibility after suspension");
        eligibility_tx.rollback().await.expect("rollback eligibility transaction");
        is_eligible
    });
    assert!(
        wait_for_lock_wait(
            &pool,
            "SELECT account.id FROM identity.accounts AS account WHERE account.id = ANY",
        )
        .await,
        "credit eligibility did not wait for the concurrent account suspension"
    );

    sanction_tx.commit().await.expect("commit concurrent suspension");
    let is_eligible = tokio::time::timeout(Duration::from_secs(5), eligibility_handle)
        .await
        .expect("eligibility recheck timed out")
        .expect("join eligibility recheck");
    assert!(!is_eligible);
}

//! Integration coverage for identity APIs consumed by other domains.

#[path = "helpers/mod.rs"]
mod helpers;

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

    sqlx::query("UPDATE identity.accounts SET status = 'deleted' WHERE id = $1")
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

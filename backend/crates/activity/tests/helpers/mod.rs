use sqlx::PgConnection;
use sqlx::PgPool;

pub(crate) async fn insert_test_account(
    connection: &mut PgConnection,
    email: &str,
    handle: &str,
) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(email)
    .bind(handle)
    .fetch_one(connection)
    .await
    .expect("insert test account");
    id
}

pub(crate) async fn set_manual_override(pool: &PgPool, account_id: i64, level: i16, reason: &str) {
    // Ensure progress row exists.
    sqlx::query(
        "INSERT INTO activity.account_trust_progress \
         (account_id, trust_level, qualifying_score, policy_version) \
         SELECT $1, 1, 0, version \
         FROM activity.trust_level_policies \
         ORDER BY version DESC LIMIT 1 \
         ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .execute(pool)
    .await
    .expect("ensure trust progress row");
    // Apply override and update projected identity level.
    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET override_level = $2, override_reason = $3, override_by = $1, \
             override_at = now(), trust_level = $2, updated_at = now() \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .bind(level)
    .bind(reason)
    .execute(pool)
    .await
    .expect("set manual override");
    sqlx::query(
        "UPDATE identity.accounts SET trust_level = $2, updated_at = now() \
         WHERE id = $1",
    )
    .bind(account_id)
    .bind(level)
    .execute(pool)
    .await
    .expect("project override to identity");
}

//! CRUD for forum.user_ignores (blocking / ignoring other users).
//!
//! All functions take `&PgPool` and return `AppResult` so callers can use `?`.

use shared::AppResult;
use sqlx::PgPool;

/// Insert an ignore relationship. Silently succeeds on duplicate (ON CONFLICT DO NOTHING).
pub async fn insert_ignore(
    pool: &PgPool,
    account_id: i64,
    ignored_account_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO forum.user_ignores (account_id, ignored_account_id) \
         VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(account_id)
    .bind(ignored_account_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete an ignore relationship. Succeeds even if the row did not exist.
pub async fn delete_ignore(
    pool: &PgPool,
    account_id: i64,
    ignored_account_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "DELETE FROM forum.user_ignores \
         WHERE account_id = $1 AND ignored_account_id = $2",
    )
    .bind(account_id)
    .bind(ignored_account_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Return the list of account_ids this account has ignored.
pub async fn list_ignored_ids(pool: &PgPool, account_id: i64) -> AppResult<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT ignored_account_id FROM forum.user_ignores \
         WHERE account_id = $1 \
         ORDER BY created_at DESC",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Convenience alias — same as `list_ignored_ids` but with a name that makes
//  its filtering purpose obvious.
pub async fn batch_ignored_ids(pool: &PgPool, account_id: i64) -> AppResult<Vec<i64>> {
    list_ignored_ids(pool, account_id).await
}

/// Check whether `account_id` has ignored `target_account_id`.
pub async fn is_ignored(pool: &PgPool, account_id: i64, target_account_id: i64) -> AppResult<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM forum.user_ignores \
         WHERE account_id = $1 AND ignored_account_id = $2 \
         LIMIT 1",
    )
    .bind(account_id)
    .bind(target_account_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

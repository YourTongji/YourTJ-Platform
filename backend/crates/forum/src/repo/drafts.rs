//! Draft CRUD for forum posts and comments.
//!
//! Each account may have up to 50 drafts. The limit is enforced at the
//! application layer in the upsert handler.

use serde_json::Value;
use shared::AppResult;
use sqlx::PgPool;

/// Row returned when listing drafts.
#[derive(Debug, sqlx::FromRow)]
pub struct DraftRow {
    pub draft_key: String,
    pub payload: Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Upsert a draft (create or update payload).
///
/// The caller is responsible for enforcing the 50-draft-per-account limit
/// before calling this function with a *new* draft key. Updating an existing
/// key does not increase the count.
pub async fn upsert_draft(
    pool: &PgPool,
    account_id: i64,
    draft_key: &str,
    payload: &Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO forum.drafts (account_id, draft_key, payload) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (account_id, draft_key) \
         DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()",
    )
    .bind(account_id)
    .bind(draft_key)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

/// List all drafts belonging to an account, newest first.
pub async fn list_drafts(pool: &PgPool, account_id: i64) -> AppResult<Vec<DraftRow>> {
    let rows: Vec<DraftRow> = sqlx::query_as(
        "SELECT draft_key, payload, updated_at \
         FROM forum.drafts \
         WHERE account_id = $1 \
         ORDER BY updated_at DESC",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Count drafts belonging to an account.
pub async fn count_drafts(pool: &PgPool, account_id: i64) -> AppResult<i64> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*)::bigint FROM forum.drafts WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await?;
    Ok(count.0)
}

/// Check whether a specific draft key already exists for this account.
pub async fn draft_exists(pool: &PgPool, account_id: i64, draft_key: &str) -> AppResult<bool> {
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM forum.drafts WHERE account_id = $1 AND draft_key = $2)",
    )
    .bind(account_id)
    .bind(draft_key)
    .fetch_one(pool)
    .await?;
    Ok(exists.0)
}

/// Get a single draft's payload by key.
pub async fn get_draft(
    pool: &PgPool,
    account_id: i64,
    draft_key: &str,
) -> AppResult<Option<Value>> {
    let row: Option<(Value,)> =
        sqlx::query_as("SELECT payload FROM forum.drafts WHERE account_id = $1 AND draft_key = $2")
            .bind(account_id)
            .bind(draft_key)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

/// Delete a single draft by key.
pub async fn delete_draft(pool: &PgPool, account_id: i64, draft_key: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM forum.drafts WHERE account_id = $1 AND draft_key = $2")
        .bind(account_id)
        .bind(draft_key)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete all drafts belonging to an account.
pub async fn delete_drafts_for_account(pool: &PgPool, account_id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM forum.drafts WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await?;
    Ok(())
}

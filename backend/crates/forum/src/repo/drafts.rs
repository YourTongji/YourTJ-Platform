//! Versioned draft persistence for unpublished forum content.

use shared::{AppError, AppResult};
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};

use crate::dto::DraftPayload;

/// Row returned by draft reads and mutations.
#[derive(Debug, sqlx::FromRow)]
pub struct DraftRow {
    pub draft_key: String,
    pub payload: Json<DraftPayload>,
    pub version: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Serialize an account's draft mutations with account lifecycle writes.
pub async fn lock_draft_owner(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
) -> AppResult<()> {
    identity::public_accounts::lock_active_account_for_owned_mutation(transaction, account_id).await
}

/// Count drafts belonging to an account inside the current transaction.
pub async fn count_drafts(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
) -> AppResult<i64> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM forum.drafts WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&mut **transaction)
            .await?;
    Ok(count)
}

/// Check for one account-owned draft inside the current serialized mutation.
pub async fn draft_exists(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    draft_key: &str,
) -> AppResult<bool> {
    let exists = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.drafts WHERE account_id = $1 AND draft_key = $2)",
    )
    .bind(account_id)
    .bind(draft_key)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(exists)
}

/// Create version 1 when `expected_version` is zero, otherwise compare-and-swap an existing draft.
pub async fn save_draft(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    draft_key: &str,
    payload: &DraftPayload,
    expected_version: i64,
) -> AppResult<DraftRow> {
    let row = if expected_version == 0 {
        sqlx::query_as::<_, DraftRow>(
            "INSERT INTO forum.drafts (account_id, draft_key, payload) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (account_id, draft_key) DO NOTHING \
             RETURNING draft_key, payload, version, updated_at",
        )
        .bind(account_id)
        .bind(draft_key)
        .bind(Json(payload))
        .fetch_optional(&mut **transaction)
        .await?
    } else {
        sqlx::query_as::<_, DraftRow>(
            "UPDATE forum.drafts \
             SET payload = $3, version = version + 1, updated_at = now() \
             WHERE account_id = $1 AND draft_key = $2 AND version = $4 \
             RETURNING draft_key, payload, version, updated_at",
        )
        .bind(account_id)
        .bind(draft_key)
        .bind(Json(payload))
        .bind(expected_version)
        .fetch_optional(&mut **transaction)
        .await?
    };

    row.ok_or_else(|| AppError::Conflict("draft changed in another session".into()))
}

/// List all drafts belonging to an account, newest first.
pub async fn list_drafts(pool: &PgPool, account_id: i64) -> AppResult<Vec<DraftRow>> {
    let rows = sqlx::query_as::<_, DraftRow>(
        "SELECT draft_key, payload, version, updated_at \
         FROM forum.drafts \
         WHERE account_id = $1 \
         ORDER BY updated_at DESC, draft_key ASC",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get one account-owned draft by its stable key.
pub async fn get_draft(
    pool: &PgPool,
    account_id: i64,
    draft_key: &str,
) -> AppResult<Option<DraftRow>> {
    let row = sqlx::query_as::<_, DraftRow>(
        "SELECT draft_key, payload, version, updated_at \
         FROM forum.drafts WHERE account_id = $1 AND draft_key = $2",
    )
    .bind(account_id)
    .bind(draft_key)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Delete a single account-owned draft.
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

//! Owner-domain reads for account-bound wallet verification keys.

use std::collections::HashMap;

use shared::AppResult;
use sqlx::{PgConnection, PgPool};

/// Return the account's sole active Ed25519 public key, if one has been enrolled.
pub async fn active_public_key(pool: &PgPool, account_id: i64) -> AppResult<Option<String>> {
    let mut conn = pool.acquire().await?;
    active_public_key_on(&mut conn, account_id).await
}

/// Return the active key on the caller's connection so key checks share its transaction snapshot.
pub async fn active_public_key_on(
    conn: &mut PgConnection,
    account_id: i64,
) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT public_key FROM identity.account_keys \
         WHERE account_id = $1 AND revoked_at IS NULL FOR SHARE",
    )
    .bind(account_id)
    .fetch_optional(conn)
    .await?)
}

/// Return every retained verification key for the requested ledger signer accounts.
pub async fn verification_public_keys_on(
    conn: &mut PgConnection,
    account_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<String>>> {
    if account_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT account_id, public_key FROM identity.account_keys \
         WHERE account_id = ANY($1)",
    )
    .bind(account_ids)
    .fetch_all(conn)
    .await?;
    let mut keys = HashMap::new();
    for (account_id, public_key) in rows {
        keys.entry(account_id).or_insert_with(Vec::new).push(public_key);
    }
    Ok(keys)
}

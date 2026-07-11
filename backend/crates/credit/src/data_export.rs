//! Privacy-minimized owner projection of the append-only credit ledger.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditExport {
    balance: i64,
    ledger: Vec<ExportLedgerEntry>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportLedgerEntry {
    seq: i64,
    tx_id: String,
    entry_type: String,
    from_account: Option<i64>,
    to_account: Option<i64>,
    amount: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<CreditExport> {
    let balance = sqlx::query_scalar(
        "SELECT COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $1), 0)",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    let ledger = sqlx::query_as::<_, ExportLedgerEntry>(
        "SELECT seq, tx_id, type AS entry_type, from_account, to_account, amount, created_at \
         FROM credit.ledger WHERE from_account = $1 OR to_account = $1 ORDER BY seq",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(CreditExport { balance, ledger })
}

/// Remove unused signing authority while preserving consumed intents required to verify ledger rows.
pub async fn purge_account_private_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM credit.signing_intents WHERE account_id = $1 AND consumed_at IS NULL")
        .bind(account_id)
        .execute(pool)
        .await?;
    Ok(())
}

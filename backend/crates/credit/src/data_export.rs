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
    created_tasks: Vec<ExportTask>,
    accepted_tasks: Vec<ExportTask>,
    created_products: Vec<ExportProduct>,
    purchases: Vec<ExportPurchase>,
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

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportTask {
    id: i64,
    acceptor_id: Option<i64>,
    title: String,
    description: Option<String>,
    reward_amount: i64,
    contact_info: Option<String>,
    status: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportProduct {
    id: i64,
    title: String,
    description: Option<String>,
    price: i64,
    stock: i32,
    delivery_info: Option<String>,
    status: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportPurchase {
    id: i64,
    product_id: i64,
    buyer_id: i64,
    seller_id: i64,
    amount: i64,
    status: String,
    delivery_info: Option<String>,
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
    let created_tasks = sqlx::query_as::<_, ExportTask>(
        "SELECT id, acceptor_id, title, description, reward_amount, contact_info, \
                status::text AS status, created_at \
         FROM credit.tasks WHERE creator_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let accepted_tasks = sqlx::query_as::<_, ExportTask>(
        "SELECT id, acceptor_id, title, description, reward_amount, contact_info, \
                status::text AS status, created_at \
         FROM credit.tasks WHERE acceptor_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let created_products = sqlx::query_as::<_, ExportProduct>(
        "SELECT id, title, description, price, stock, delivery_info, status::text AS status, \
                created_at \
         FROM credit.products WHERE seller_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let purchases = sqlx::query_as::<_, ExportPurchase>(
        "SELECT purchase.id, purchase.product_id, purchase.buyer_id, purchase.seller_id, \
                purchase.amount, purchase.status::text AS status, product.delivery_info, \
                purchase.created_at \
         FROM credit.purchases purchase \
         JOIN credit.products product ON product.id = purchase.product_id \
         WHERE purchase.buyer_id = $1 OR purchase.seller_id = $1 \
         ORDER BY purchase.id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(CreditExport { balance, ledger, created_tasks, accepted_tasks, created_products, purchases })
}

/// Remove owner-private escrow fields and unused signing authority while preserving transaction facts.
pub async fn purge_account_private_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE credit.tasks SET contact_info = NULL \
         WHERE creator_id = $1 AND contact_info IS NOT NULL",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE credit.products SET delivery_info = NULL \
         WHERE seller_id = $1 AND delivery_info IS NOT NULL",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM credit.signing_intents WHERE account_id = $1 AND consumed_at IS NULL")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

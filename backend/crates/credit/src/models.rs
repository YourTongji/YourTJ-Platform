//! Database row types mapped from the credit schema via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

/// A row from `credit.ledger` — append-only, hash-chain linked.
#[derive(Debug, Clone, FromRow)]
pub struct LedgerEntryRow {
    pub seq: i64,
    pub tx_id: String,
    #[sqlx(rename = "type")]
    pub type_: String,
    pub from_account: Option<i64>,
    pub to_account: Option<i64>,
    pub amount: i64,
    pub nonce: String,
    pub metadata: Option<serde_json::Value>,
    pub signer: String,
    pub signature: String,
    pub prev_hash: String,
    pub hash: String,
    pub created_at: DateTime<Utc>,
}

/// A row from `credit.wallets` — derived balance cache.
#[derive(Debug, Clone, FromRow)]
pub struct WalletRow {
    pub account_id: i64,
    pub balance: i64,
    pub last_seq: i64,
}

/// A row from `credit.tasks`.
#[derive(Debug, Clone, FromRow)]
pub struct TaskRow {
    pub id: i64,
    pub creator_id: i64,
    pub acceptor_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub reward_amount: i64,
    pub contact_info: Option<String>,
    pub status: String,
    pub hold_tx_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A row from `credit.products`.
#[derive(Debug, Clone, FromRow)]
pub struct ProductRow {
    pub id: i64,
    pub seller_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub price: i64,
    pub stock: i32,
    pub delivery_info: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// A row from `credit.purchases`.
#[derive(Debug, Clone, FromRow, Deserialize)]
pub struct PurchaseRow {
    pub id: i64,
    pub product_id: i64,
    pub buyer_id: i64,
    pub seller_id: i64,
    pub amount: i64,
    pub status: String,
    pub hold_tx_id: Option<String>,
}

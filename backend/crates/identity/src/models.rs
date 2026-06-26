//! Database row types mapped from the identity and credit schemas via `sqlx::FromRow`.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// A row from `identity.accounts`.
#[derive(Debug, Clone, FromRow)]
pub struct AccountRow {
    pub id: i64,
    #[allow(dead_code)]
    pub email: String,
    pub handle: String,
    pub avatar_url: Option<String>,
    pub role: String,
    #[allow(dead_code)]
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// A row from `identity.email_codes`.
#[derive(Debug, Clone, FromRow)]
pub struct EmailCodeRow {
    #[allow(dead_code)]
    pub email: String,
    pub code_hash: String,
    #[allow(dead_code)]
    pub expires_at: DateTime<Utc>,
    pub attempts: i32,
}

/// A row from `identity.sessions`.
#[derive(Debug, Clone, FromRow)]
pub struct SessionRow {
    pub id: i64,
    pub account_id: i64,
    pub refresh_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A row from `identity.account_keys`.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AccountKeyRow {
    pub account_id: i64,
    pub public_key: String,
}

/// A row from `credit.wallets`.
#[derive(Debug, Clone, FromRow)]
pub struct WalletRow {
    pub account_id: i64,
    pub balance: i64,
}

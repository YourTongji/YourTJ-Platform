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
    pub trust_level: i16,
    #[allow(dead_code)]
    #[sqlx(default)]
    pub password_hash: Option<String>,
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
    #[allow(dead_code)]
    pub purpose: Option<String>,
    #[allow(dead_code)]
    pub used_at: Option<DateTime<Utc>>,
}

/// A row from `identity.sessions`.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct SessionRow {
    pub id: i64,
    pub account_id: i64,
    pub refresh_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub family_id: Option<uuid::Uuid>,
    pub replaced_by_session_id: Option<i64>,
    pub device_name: Option<String>,
    pub user_agent: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A row from `identity.account_keys`.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AccountKeyRow {
    pub account_id: i64,
    pub public_key: String,
}

/// A row from `identity.wallet_claim_challenges`.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct WalletClaimChallengeRow {
    pub id: String,
    pub account_id: i64,
    pub nonce: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A row from `identity.legacy_wallet_links`.
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct LegacyWalletLinkRow {
    pub legacy_user_hash: String,
    pub account_id: Option<i64>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub legacy_public_key: Option<String>,
    pub legacy_balance: i64,
    pub imported_metadata: Option<serde_json::Value>,
}

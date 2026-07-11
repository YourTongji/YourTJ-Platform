//! Credential-epoch guarded password mutations.
//!
//! Expensive password verification happens outside PostgreSQL. Callers capture the credential
//! version with the hash and these functions re-check it under the account lock, preventing a
//! verified-but-stale password from changing credentials or elevating recent-auth afterwards.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::PgPool;

pub async fn mark_password_recent_auth_if_current(
    pool: &PgPool,
    account_id: i64,
    session_id: i64,
    expected_credential_version: i64,
) -> AppResult<DateTime<Utc>> {
    crate::repo::mark_recent_auth_password(
        pool,
        account_id,
        session_id,
        expected_credential_version,
    )
    .await
}

pub async fn replace_password_if_current(
    pool: &PgPool,
    account_id: i64,
    current_session_id: Option<i64>,
    expected_credential_version: i64,
    password_hash: &str,
) -> AppResult<()> {
    crate::repo::change_password_preserving_session(
        pool,
        account_id,
        current_session_id,
        expected_credential_version,
        password_hash,
    )
    .await
}

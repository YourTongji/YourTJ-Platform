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

/// Result of an epoch-guarded password replacement and refresh-family rotation.
pub struct ReplacedCredentialSession {
    pub session_id: i64,
    pub auth_version: i64,
}

pub async fn replace_password_if_current(
    pool: &PgPool,
    account_id: i64,
    current_session_id: i64,
    expected_credential_version: i64,
    password_hash: &str,
    successor_refresh_hash: &str,
    successor_expires_at: DateTime<Utc>,
) -> AppResult<ReplacedCredentialSession> {
    let mutation = crate::repo::change_password_and_replace_sessions(
        pool,
        account_id,
        current_session_id,
        expected_credential_version,
        password_hash,
        successor_refresh_hash,
        successor_expires_at,
        None,
        None,
    )
    .await?;
    Ok(ReplacedCredentialSession {
        session_id: mutation.session_id,
        auth_version: mutation.auth_version,
    })
}

//! Database access layer for the identity domain.
//!
//! Every function takes `&PgPool` and returns `Result` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::PgPool;

use crate::models::{AccountRow, EmailCodeRow, SessionRow};

// ---------------------------------------------------------------------------
// email_codes
// ---------------------------------------------------------------------------

/// Invalidate existing active codes for `email`, then insert a new row.
pub async fn insert_email_code(
    pool: &PgPool,
    email: &str,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    // Mark previous codes as exhausted.
    sqlx::query(
        "UPDATE identity.email_codes SET attempts = 99 WHERE email = $1 AND expires_at > now()",
    )
    .bind(email)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO identity.email_codes (email, code_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(email)
    .bind(code_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Look up the most recent live code for `email` (not expired & not exhausted).
pub async fn find_email_code(pool: &PgPool, email: &str) -> AppResult<Option<EmailCodeRow>> {
    let row = sqlx::query_as::<_, EmailCodeRow>(
        "SELECT email, code_hash, expires_at, attempts \
         FROM identity.email_codes \
         WHERE email = $1 AND expires_at > now() AND attempts < 5 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Bump the attempt counter for the active code of `email`.
pub async fn increment_code_attempts(pool: &PgPool, email: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.email_codes \
         SET attempts = attempts + 1 \
         WHERE email = $1 AND expires_at > now() AND attempts < 5",
    )
    .bind(email)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// accounts
// ---------------------------------------------------------------------------

/// Insert a new account row.  If `handle` is `None` it is derived from the
/// email prefix; if the handle collides a random 4-digit suffix is appended
/// (up to 3 retries).
pub async fn insert_account(
    pool: &PgPool,
    email: &str,
    handle: Option<&str>,
) -> AppResult<AccountRow> {
    let base = handle
        .map(|h| h.to_string())
        .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());

    let mut attempts = 0;
    loop {
        let h = if attempts == 0 {
            base.clone()
        } else {
            let suffix: u16 = {
                use ring::rand::{SecureRandom, SystemRandom};
                let rng = SystemRandom::new();
                let mut buf = [0u8; 2];
                rng.fill(&mut buf).expect("CSPRNG");
                u16::from_be_bytes(buf) % 10000
            };
            format!("{base}{suffix:04}")
        };

        let result = sqlx::query_as::<_, AccountRow>(
            "INSERT INTO identity.accounts (email, handle) \
             VALUES ($1, $2) \
             ON CONFLICT (handle) DO NOTHING \
             RETURNING id, email::text, handle, avatar_url, role::text, status::text, trust_level, created_at",
        )
        .bind(email)
        .bind(&h)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = result {
            return Ok(row);
        }

        attempts += 1;
        if attempts > 3 {
            return Err(crate::error::IdentityError::HandleTaken.into());
        }
    }
}

/// Look up an account by its email (case-insensitive via CITEXT).
pub async fn find_account_by_email(pool: &PgPool, email: &str) -> AppResult<Option<AccountRow>> {
    let row = sqlx::query_as::<_, AccountRow>(
        "SELECT id, email::text, handle, avatar_url, role::text, status::text, trust_level, created_at \
         FROM identity.accounts WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Look up an account by primary-key id.
pub async fn find_account_by_id(pool: &PgPool, id: i64) -> AppResult<Option<AccountRow>> {
    let row = sqlx::query_as::<_, AccountRow>(
        "SELECT id, email::text, handle, avatar_url, role::text, status::text, trust_level, created_at \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Partial update: handle and/or avatar_url. Returns the updated row.
pub async fn update_account(
    pool: &PgPool,
    id: i64,
    handle: Option<&str>,
    avatar_url: Option<&str>,
) -> AppResult<AccountRow> {
    // We build a minimal dynamic UPDATE so unused fields stay untouched.
    let mut set = Vec::new();
    let mut idx = 0u32;

    if let Some(h) = handle {
        idx += 1;
        set.push((format!("handle = ${idx}"), h.to_string()));
    }
    if let Some(a) = avatar_url {
        idx += 1;
        set.push((format!("avatar_url = ${idx}"), a.to_string()));
    }
    if set.is_empty() {
        return find_account_by_id(pool, id).await?.ok_or(shared::AppError::NotFound);
    }
    // Always bump updated_at.
    idx += 1;
    set.push((format!("updated_at = ${idx}"), "now()".to_string()));

    // Positional: parameters come from set, then id as last.
    let mut sql = String::from("UPDATE identity.accounts SET ");
    let parts: Vec<&str> = set.iter().map(|(c, _)| c.as_str()).collect();
    sql.push_str(&parts.join(", "));
    idx += 1;
    sql.push_str(&format!(" WHERE id = ${idx} RETURNING id, email::text, handle, avatar_url, role::text, status::text, trust_level, created_at"));

    let mut q = sqlx::query_as::<_, AccountRow>(&sql);
    for (_, val) in &set {
        q = q.bind(val);
    }
    let row = q.bind(id).fetch_one(pool).await?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// sessions
// ---------------------------------------------------------------------------

/// Create a new session row. Returns the auto-generated session id.
pub async fn insert_session(
    pool: &PgPool,
    account_id: i64,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO identity.sessions (account_id, refresh_hash, expires_at) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(account_id)
    .bind(refresh_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Look up a non-revoked, non-expired session by id + refresh hash.
pub async fn find_session(
    pool: &PgPool,
    session_id: i64,
    refresh_hash: &str,
) -> AppResult<Option<SessionRow>> {
    let row = sqlx::query_as::<_, SessionRow>(
        "SELECT id, account_id, refresh_hash, expires_at, revoked_at \
         FROM identity.sessions \
         WHERE id = $1 AND refresh_hash = $2 \
           AND expires_at > now() AND revoked_at IS NULL",
    )
    .bind(session_id)
    .bind(refresh_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Soft-revoke a single session.
pub async fn revoke_session(pool: &PgPool, session_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE identity.sessions SET revoked_at = now() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Soft-revoke every session belonging to an account.
pub async fn revoke_all_sessions(pool: &PgPool, account_id: i64) -> AppResult<()> {
    sqlx::query("UPDATE identity.sessions SET revoked_at = now() WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// account_keys
// ---------------------------------------------------------------------------

/// Bind an Ed25519 public key to an account. Returns `KeyAlreadyBound` if
/// the key is already bound to this account.
pub async fn insert_account_key(pool: &PgPool, account_id: i64, public_key: &str) -> AppResult<()> {
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity.account_keys \
         WHERE account_id = $1 AND public_key = $2)",
    )
    .bind(account_id)
    .bind(public_key)
    .fetch_one(pool)
    .await?;

    if already {
        return Err(crate::error::IdentityError::KeyAlreadyBound.into());
    }

    sqlx::query(
        "INSERT INTO identity.account_keys (account_id, public_key) \
         VALUES ($1, $2)",
    )
    .bind(account_id)
    .bind(public_key)
    .execute(pool)
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// wallet claim challenges
// ---------------------------------------------------------------------------

use crate::models::{LegacyWalletLinkRow, WalletClaimChallengeRow};

/// Insert a new wallet claim challenge for the given account.
pub async fn insert_claim_challenge(
    pool: &PgPool,
    id: &str,
    account_id: i64,
    nonce: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO identity.wallet_claim_challenges (id, account_id, nonce, expires_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(account_id)
    .bind(nonce)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch a claim challenge by id.
#[allow(dead_code)]
pub async fn find_claim_challenge(
    pool: &PgPool,
    id: &str,
) -> AppResult<Option<WalletClaimChallengeRow>> {
    let row = sqlx::query_as::<_, WalletClaimChallengeRow>(
        "SELECT id, account_id, nonce, expires_at, used_at, created_at \
         FROM identity.wallet_claim_challenges WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Mark a claim challenge as used.
#[allow(dead_code)]
pub async fn mark_challenge_used(pool: &PgPool, id: &str) -> AppResult<()> {
    sqlx::query("UPDATE identity.wallet_claim_challenges SET used_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Look up a legacy wallet link by hash.
#[allow(dead_code)]
pub async fn find_legacy_wallet_link(
    pool: &PgPool,
    legacy_user_hash: &str,
) -> AppResult<Option<LegacyWalletLinkRow>> {
    let row = sqlx::query_as::<_, LegacyWalletLinkRow>(
        "SELECT legacy_user_hash, account_id, claimed_at, legacy_public_key, \
                legacy_balance, imported_metadata \
         FROM identity.legacy_wallet_links WHERE legacy_user_hash = $1",
    )
    .bind(legacy_user_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Claim a legacy wallet link: set account_id and claimed_at.
#[allow(dead_code)]
pub async fn claim_legacy_wallet_link(
    pool: &PgPool,
    legacy_user_hash: &str,
    account_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.legacy_wallet_links \
         SET account_id = $2, claimed_at = now() \
         WHERE legacy_user_hash = $1 AND account_id IS NULL",
    )
    .bind(legacy_user_hash)
    .bind(account_id)
    .execute(pool)
    .await?;
    Ok(())
}

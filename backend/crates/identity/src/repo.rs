//! Database access layer for the identity domain.
//!
//! Every function takes `&PgPool` and returns `Result` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use chrono::{DateTime, Utc};
use shared::email_crypto::EmailEncryption;
use shared::{AppError, AppResult};
use sqlx::PgPool;

use crate::email_code::{verify_code, CodePurpose};
use crate::models::{AccountRow, EmailCodeRow, SessionRow};

#[derive(sqlx::FromRow)]
struct StoredAccountRow {
    id: i64,
    email: Option<String>,
    email_ciphertext: Option<String>,
    handle: String,
    avatar_url: Option<String>,
    role: String,
    status: String,
    trust_level: i16,
    #[sqlx(default)]
    password_hash: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct AdminUserRow {
    pub id: i64,
    pub handle: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub status: String,
    pub trust_level: i16,
    pub last_active_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct DeviceSessionRow {
    pub id: i64,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub(crate) struct RotatedSession {
    pub account_id: i64,
    pub session_id: i64,
    pub auth_version: i64,
}

impl StoredAccountRow {
    fn decrypt(self, encryption: Option<&EmailEncryption>) -> AppResult<AccountRow> {
        let email = match (self.email, self.email_ciphertext) {
            (Some(email), _) => email,
            (None, Some(ciphertext)) => encryption
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(
                        "encrypted account email cannot be read without configured keys"
                    ))
                })?
                .decrypt(&ciphertext)
                .map_err(AppError::Internal)?,
            (None, None) => {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "account has no stored email representation"
                )))
            }
        };
        Ok(AccountRow {
            id: self.id,
            email,
            handle: self.handle,
            avatar_url: self.avatar_url,
            role: self.role,
            status: self.status,
            trust_level: self.trust_level,
            password_hash: self.password_hash,
            created_at: self.created_at,
        })
    }
}

// ---------------------------------------------------------------------------
// email_codes
// ---------------------------------------------------------------------------

/// Invalidate active codes for the same purpose, then insert an undelivered code.
pub async fn insert_email_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    purpose: CodePurpose,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<uuid::Uuid> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let request_id = uuid::Uuid::new_v4();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE identity.email_codes SET used_at = now(), attempts = 99 \
         WHERE (email = $1 OR email_blind_index = $2) AND purpose = $3 \
           AND used_at IS NULL",
    )
    .bind(email)
    .bind(&blind_index)
    .bind(purpose.as_str())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO identity.email_codes \
         (email, email_blind_index, email_key_version, purpose, request_id, code_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(encryption.is_none().then_some(email))
    .bind(&blind_index)
    .bind(encryption.map(|encryption| i16::from(encryption.active_version())))
    .bind(purpose.as_str())
    .bind(request_id)
    .bind(code_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(request_id)
}

/// Mark a code usable only after the mail provider accepted its envelope.
pub async fn mark_email_code_delivered(pool: &PgPool, request_id: uuid::Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.email_codes SET delivery_accepted_at = now() \
         WHERE request_id = $1 AND used_at IS NULL",
    )
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Exhaust one generated code after outbound delivery fails.
pub async fn invalidate_email_code(pool: &PgPool, request_id: uuid::Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.email_codes SET used_at = now(), attempts = 99 \
         WHERE request_id = $1 AND used_at IS NULL",
    )
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Lock and consume one purpose-compatible code exactly once.
pub async fn consume_email_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    purpose: Option<CodePurpose>,
    attempted_code: &str,
) -> AppResult<CodePurpose> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let mut tx = pool.begin().await?;
    let purpose_filter = purpose.map(CodePurpose::as_str);
    let row = sqlx::query_as::<_, EmailCodeRow>(
        "SELECT id, purpose, code_hash, attempts \
         FROM identity.email_codes \
         WHERE (email = $1 OR email_blind_index = $2) \
           AND expires_at > now() AND used_at IS NULL \
           AND delivery_accepted_at IS NOT NULL \
           AND (($3::text IS NULL AND purpose IN ('login', 'registration')) OR purpose = $3) \
         ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE",
    )
    .bind(email)
    .bind(&blind_index)
    .bind(purpose_filter)
    .fetch_optional(&mut *tx)
    .await?;
    let row = row.ok_or(crate::error::IdentityError::CodeExpired)?;
    if row.attempts >= 5 {
        return Err(crate::error::IdentityError::CodeExhausted.into());
    }
    if !verify_code(attempted_code, &row.code_hash) {
        sqlx::query("UPDATE identity.email_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(crate::error::IdentityError::InvalidCode.into());
    }
    sqlx::query(
        "UPDATE identity.email_codes SET attempts = attempts + 1, used_at = now() WHERE id = $1",
    )
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    CodePurpose::from_stored(&row.purpose)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("invalid persisted email code purpose")))
}

// ---------------------------------------------------------------------------
// accounts
// ---------------------------------------------------------------------------

/// Insert a new account row.  If `handle` is `None` it is derived from the
/// email prefix; if the handle collides a random 4-digit suffix is appended
/// (up to 3 retries).
pub async fn insert_account(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
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

        let ciphertext = encryption
            .map(|encryption| encryption.encrypt(email))
            .transpose()
            .map_err(AppError::Internal)?;
        let blind_index = encryption.map(|encryption| encryption.blind_index(email));
        let result = sqlx::query_as::<_, StoredAccountRow>(
            "INSERT INTO identity.accounts \
             (email, email_ciphertext, email_key_version, email_blind_index, \
              password_email_blind, handle, email_verified_at) \
             VALUES ($1, $2, $3, $4, $4, $5, now()) \
             ON CONFLICT (handle) DO NOTHING \
             RETURNING id, email::text AS email, email_ciphertext, \
                       handle, avatar_url, role::text, status::text, trust_level, created_at",
        )
        .bind(encryption.is_none().then_some(email))
        .bind(ciphertext)
        .bind(encryption.map(|encryption| i16::from(encryption.active_version())))
        .bind(blind_index)
        .bind(&h)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = result {
            return row.decrypt(encryption);
        }

        attempts += 1;
        if attempts > 3 {
            return Err(crate::error::IdentityError::HandleTaken.into());
        }
    }
}

/// Provision an unverified account that must still prove campus mailbox ownership.
#[allow(clippy::too_many_arguments)] // reason: encrypted identity storage fields are explicit to avoid accidental plaintext fallbacks
pub async fn insert_invited_account(
    tx: &mut sqlx::PgConnection,
    encryption: Option<&EmailEncryption>,
    email: &str,
    handle: &str,
    role: &str,
    invited_by: i64,
) -> AppResult<AccountRow> {
    let ciphertext = encryption
        .map(|encryption| encryption.encrypt(email))
        .transpose()
        .map_err(AppError::Internal)?;
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let row = sqlx::query_as::<_, StoredAccountRow>(
        "INSERT INTO identity.accounts \
         (email, email_ciphertext, email_key_version, email_blind_index, password_email_blind, \
          handle, role, invited_by, invited_at, invitation_expires_at) \
         VALUES ($1, $2, $3, $4, $4, $5, $6::identity.account_role, $7, now(), \
                 now() + interval '7 days') ON CONFLICT DO NOTHING \
         RETURNING id, email::text AS email, email_ciphertext, handle, avatar_url, \
                   role::text, status::text, trust_level, created_at",
    )
    .bind(encryption.is_none().then_some(email))
    .bind(ciphertext)
    .bind(encryption.map(|encryption| i16::from(encryption.active_version())))
    .bind(blind_index)
    .bind(handle)
    .bind(role)
    .bind(invited_by)
    .fetch_optional(tx)
    .await?
    .ok_or_else(|| AppError::Conflict("email or handle is already registered".into()))?;
    row.decrypt(encryption)
}

/// Mark mailbox ownership proven after successful email-code verification.
pub async fn mark_email_verified(pool: &PgPool, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.accounts \
         SET email_verified_at = COALESCE(email_verified_at, now()), \
             invitation_accepted_at = CASE WHEN invited_at IS NOT NULL \
                                          THEN COALESCE(invitation_accepted_at, now()) \
                                          ELSE invitation_accepted_at END \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Reject an unverified staff invitation after its bounded acceptance window.
pub async fn ensure_invitation_valid(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let is_valid: bool = sqlx::query_scalar(
        "SELECT invited_at IS NULL OR email_verified_at IS NOT NULL \
                OR invitation_expires_at > now() \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    if !is_valid {
        return Err(crate::error::IdentityError::InvitationExpired.into());
    }
    Ok(())
}

/// Return a bounded page from the privacy-safe staff user directory.
#[allow(clippy::too_many_arguments)] // reason: each optional directory filter is independently bound and intentionally explicit
pub async fn list_admin_users(
    pool: &PgPool,
    cursor: Option<i64>,
    limit: i64,
    query: Option<&str>,
    role: Option<&str>,
    status: Option<&str>,
) -> AppResult<Vec<AdminUserRow>> {
    let rows = sqlx::query_as::<_, AdminUserRow>(
        "SELECT accounts.id, accounts.handle::text, accounts.avatar_url, accounts.role::text, \
                CASE WHEN accounts.status = 'active' AND EXISTS ( \
                    SELECT 1 FROM identity.sanctions sanctions \
                    WHERE sanctions.account_id = accounts.id AND sanctions.kind = 'suspend' \
                      AND sanctions.revoked_at IS NULL \
                      AND (sanctions.ends_at IS NULL OR sanctions.ends_at > now()) \
                ) THEN 'suspended' ELSE accounts.status::text END AS status, \
                accounts.trust_level, accounts.last_active_at, accounts.created_at \
         FROM identity.accounts accounts \
         WHERE ($1::bigint IS NULL OR accounts.id < $1) \
           AND ($2::text IS NULL OR accounts.handle ILIKE '%' || $2 || '%' \
                OR accounts.id::text = $2) \
           AND ($3::text IS NULL OR accounts.role::text = $3) \
           AND ($4::text IS NULL OR \
                CASE WHEN accounts.status = 'active' AND EXISTS ( \
                    SELECT 1 FROM identity.sanctions sanctions \
                    WHERE sanctions.account_id = accounts.id AND sanctions.kind = 'suspend' \
                      AND sanctions.revoked_at IS NULL \
                      AND (sanctions.ends_at IS NULL OR sanctions.ends_at > now()) \
                ) THEN 'suspended' ELSE accounts.status::text END = $4) \
         ORDER BY accounts.id DESC LIMIT $5",
    )
    .bind(cursor)
    .bind(query)
    .bind(role)
    .bind(status)
    .bind(limit.clamp(1, 100) + 1)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Return one privacy-safe account record for staff actions.
pub async fn find_admin_user(pool: &PgPool, account_id: i64) -> AppResult<Option<AdminUserRow>> {
    let row = sqlx::query_as::<_, AdminUserRow>(
        "SELECT accounts.id, accounts.handle::text, accounts.avatar_url, accounts.role::text, \
                CASE WHEN accounts.status = 'active' AND EXISTS ( \
                    SELECT 1 FROM identity.sanctions sanctions \
                    WHERE sanctions.account_id = accounts.id AND sanctions.kind = 'suspend' \
                      AND sanctions.revoked_at IS NULL \
                      AND (sanctions.ends_at IS NULL OR sanctions.ends_at > now()) \
                ) THEN 'suspended' ELSE accounts.status::text END AS status, \
                accounts.trust_level, accounts.last_active_at, accounts.created_at \
         FROM identity.accounts accounts WHERE accounts.id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Look up an account by its email (case-insensitive via CITEXT).
pub async fn find_account_by_email(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
) -> AppResult<Option<AccountRow>> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let row = sqlx::query_as::<_, StoredAccountRow>(
        "SELECT id, email::text AS email, email_ciphertext, handle, \
                avatar_url, role::text, status::text, trust_level, password_hash, created_at \
         FROM identity.accounts WHERE email = $1 OR email_blind_index = $2",
    )
    .bind(email)
    .bind(blind_index)
    .fetch_optional(pool)
    .await?;
    row.map(|row| row.decrypt(encryption)).transpose()
}

/// Look up an account by primary-key id.
pub async fn find_account_by_id(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    id: i64,
) -> AppResult<Option<AccountRow>> {
    let row = sqlx::query_as::<_, StoredAccountRow>(
        "SELECT id, email::text AS email, email_ciphertext, handle, \
                avatar_url, role::text, status::text, trust_level, password_hash, created_at \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| row.decrypt(encryption)).transpose()
}

/// Partial update: handle and/or avatar_url. Returns the updated row.
pub async fn update_account(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
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
        return find_account_by_id(pool, encryption, id).await?.ok_or(shared::AppError::NotFound);
    }

    // Positional: parameters come from set, then id as last. `updated_at` is set
    // to the SQL `now()` function directly — it must not be a bound parameter,
    // as the text "now()" is not a valid timestamptz literal.
    let mut sql = String::from("UPDATE identity.accounts SET ");
    let parts: Vec<&str> = set.iter().map(|(c, _)| c.as_str()).collect();
    sql.push_str(&parts.join(", "));
    sql.push_str(", updated_at = now()");
    idx += 1;
    sql.push_str(&format!(" WHERE id = ${idx} RETURNING id, email::text AS email, email_ciphertext, handle, avatar_url, role::text, status::text, trust_level, created_at"));

    let mut q = sqlx::query_as::<_, StoredAccountRow>(&sql);
    for (_, val) in &set {
        q = q.bind(val);
    }
    let row = q.bind(id).fetch_one(pool).await?;
    row.decrypt(encryption)
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
    user_agent: Option<&str>,
) -> AppResult<(i64, i64)> {
    let mut tx = pool.begin().await?;
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_one(&mut *tx)
            .await?;
    let family_id = uuid::Uuid::new_v4();
    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, user_agent, expires_at) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(account_id)
    .bind(refresh_hash)
    .bind(family_id)
    .bind(user_agent)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((session_id, auth_version))
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left_byte, right_byte) in left.iter().zip(right) {
        difference |= left_byte ^ right_byte;
    }
    difference == 0
}

async fn invalidate_account_access(tx: &mut sqlx::PgConnection, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.accounts \
         SET auth_version = auth_version + 1, legacy_access_revoked_before = now() \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Atomically rotate a refresh token and detect replay of a consumed token.
pub async fn rotate_session(
    pool: &PgPool,
    session_id: i64,
    presented_hash: &str,
    successor_hash: &str,
    successor_expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
) -> AppResult<RotatedSession> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, SessionRow>(
        "SELECT id, account_id, refresh_hash, expires_at, revoked_at, family_id, \
                replaced_by_id, user_agent, created_at, last_used_at \
         FROM identity.sessions WHERE id = $1 FOR UPDATE",
    )
    .bind(session_id)
    .fetch_optional(&mut *tx)
    .await?;
    let row = row.ok_or(AppError::Unauthorized)?;

    if !constant_time_equal(row.refresh_hash.as_bytes(), presented_hash.as_bytes()) {
        return Err(AppError::Unauthorized);
    }
    if row.revoked_at.is_some() || row.replaced_by_id.is_some() {
        sqlx::query(
            "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
             WHERE family_id = $1",
        )
        .bind(row.family_id)
        .execute(&mut *tx)
        .await?;
        invalidate_account_access(&mut tx, row.account_id).await?;
        tx.commit().await?;
        return Err(AppError::Unauthorized);
    }
    if row.expires_at <= Utc::now() {
        sqlx::query("UPDATE identity.sessions SET revoked_at = now() WHERE id = $1")
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(AppError::Unauthorized);
    }

    let successor_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, rotated_from_id, user_agent, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(row.account_id)
    .bind(successor_hash)
    .bind(row.family_id)
    .bind(row.id)
    .bind(user_agent.or(row.user_agent.as_deref()))
    .bind(successor_expires_at)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.sessions \
         SET revoked_at = now(), replaced_by_id = $2, last_used_at = now() WHERE id = $1",
    )
    .bind(row.id)
    .bind(successor_id)
    .execute(&mut *tx)
    .await?;
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(row.account_id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(RotatedSession { account_id: row.account_id, session_id: successor_id, auth_version })
}

/// Revoke a named session owned by an account.
pub async fn revoke_account_session(
    pool: &PgPool,
    account_id: i64,
    session_id: i64,
) -> AppResult<bool> {
    let mut tx = pool.begin().await?;
    let family_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT family_id FROM identity.sessions \
         WHERE id = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(session_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(family_id) = family_id else {
        tx.rollback().await?;
        return Ok(false);
    };
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE family_id = $1",
    )
    .bind(family_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

/// Revoke all sessions and invalidate every outstanding access token.
pub async fn revoke_all_sessions(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    invalidate_account_access(&mut tx, account_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Revoke all sessions except the current one while preserving its access token.
pub async fn revoke_other_sessions(
    pool: &PgPool,
    account_id: i64,
    current_session_id: i64,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let current_family_id: uuid::Uuid = sqlx::query_scalar(
        "SELECT family_id FROM identity.sessions \
         WHERE id = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(current_session_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::Unauthorized)?;
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1 AND family_id <> $2",
    )
    .bind(account_id)
    .bind(current_family_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE identity.accounts SET legacy_access_revoked_before = now() WHERE id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// List live device sessions for an account, newest activity first.
pub async fn list_sessions(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Vec<DeviceSessionRow>> {
    let rows = sqlx::query_as::<_, DeviceSessionRow>(
        "SELECT id, user_agent, created_at, last_used_at, expires_at \
         FROM identity.sessions WHERE account_id = $1 AND revoked_at IS NULL \
           AND expires_at > now() AND ($2::bigint IS NULL OR id < $2) \
         ORDER BY id DESC LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(limit.clamp(1, 100) + 1)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Atomically update a password and revoke all access, including legacy JWTs.
pub async fn reset_password_and_revoke_all(
    pool: &PgPool,
    account_id: i64,
    password_hash: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE identity.accounts SET password_hash = $1, auth_version = auth_version + 1, \
                legacy_access_revoked_before = now() WHERE id = $2",
    )
    .bind(password_hash)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Update a password and preserve only the identified current session.
pub async fn change_password_preserving_session(
    pool: &PgPool,
    account_id: i64,
    current_session_id: Option<i64>,
    password_hash: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    if let Some(session_id) = current_session_id {
        let current_family_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT family_id FROM identity.sessions \
             WHERE id = $1 AND account_id = $2 FOR UPDATE",
        )
        .bind(session_id)
        .bind(account_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::Unauthorized)?;
        sqlx::query(
            "UPDATE identity.accounts SET password_hash = $1, \
                    legacy_access_revoked_before = now() WHERE id = $2",
        )
        .bind(password_hash)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
             WHERE account_id = $1 AND family_id <> $2",
        )
        .bind(account_id)
        .bind(current_family_id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE identity.accounts SET password_hash = $1, \
                    auth_version = auth_version + 1, legacy_access_revoked_before = now() \
             WHERE id = $2",
        )
        .bind(password_hash)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
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
// password_hash
// ---------------------------------------------------------------------------

/// Update (or set) the password hash for an account.
pub async fn update_password_hash(pool: &PgPool, account_id: i64, hash: &str) -> AppResult<()> {
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(hash)
        .bind(account_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Look up the password hash for an account by email, returning None if the
/// account has no password set (email-code-only user).
pub async fn find_password_hash(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
) -> AppResult<Option<String>> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let hash: Option<String> = sqlx::query_scalar(
        "SELECT password_hash FROM identity.accounts \
         WHERE email = $1 OR password_email_blind = $2",
    )
    .bind(email)
    .bind(blind_index)
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(hash)
}

/// Look up the password hash by authenticated account id.
pub async fn find_password_hash_by_account_id(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<Option<String>> {
    let hash = sqlx::query_scalar("SELECT password_hash FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_optional(pool)
        .await?
        .flatten();
    Ok(hash)
}

/// Encrypt legacy plaintext emails and clear their plaintext columns before serving traffic.
pub async fn backfill_email_encryption(
    pool: &PgPool,
    encryption: &EmailEncryption,
) -> AppResult<()> {
    let account_rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, email::text FROM identity.accounts \
         WHERE email IS NOT NULL AND email_ciphertext IS NULL",
    )
    .fetch_all(pool)
    .await?;
    let code_emails: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT email::text FROM identity.email_codes WHERE email IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    let encryption_for_work = encryption.clone();
    let prepared = tokio::task::spawn_blocking(move || -> Result<_, anyhow::Error> {
        let accounts = account_rows
            .into_iter()
            .map(|(account_id, email)| {
                Ok((
                    account_id,
                    encryption_for_work.encrypt(&email)?,
                    encryption_for_work.blind_index(&email),
                ))
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        let codes = code_emails
            .into_iter()
            .map(|email| {
                let blind_index = encryption_for_work.blind_index(&email);
                (email, blind_index)
            })
            .collect::<Vec<_>>();
        Ok((accounts, codes))
    })
    .await
    .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?
    .map_err(AppError::Internal)?;

    let mut tx = pool.begin().await?;
    for (account_id, ciphertext, blind_index) in prepared.0 {
        sqlx::query(
            "UPDATE identity.accounts \
             SET email_ciphertext = $1, email_key_version = $2, email_blind_index = $3, \
                 password_email_blind = $3, email = NULL \
             WHERE id = $4",
        )
        .bind(ciphertext)
        .bind(i16::from(encryption.active_version()))
        .bind(blind_index)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    }
    for (email, blind_index) in prepared.1 {
        sqlx::query(
            "UPDATE identity.email_codes \
             SET email_blind_index = $1, email_key_version = $2, email = NULL \
             WHERE email = $3",
        )
        .bind(blind_index)
        .bind(i16::from(encryption.active_version()))
        .bind(email)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Return whether any identity email remains in plaintext or lacks encrypted storage.
pub async fn has_unencrypted_email_rows(pool: &PgPool) -> AppResult<bool> {
    let has_unencrypted: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM identity.accounts \
           WHERE email IS NOT NULL OR email_ciphertext IS NULL OR email_blind_index IS NULL \
         ) OR EXISTS( \
           SELECT 1 FROM identity.email_codes \
           WHERE email IS NOT NULL OR email_blind_index IS NULL \
         )",
    )
    .fetch_one(pool)
    .await?;
    Ok(has_unencrypted)
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

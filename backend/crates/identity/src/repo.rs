//! Database access layer for the identity domain.
//!
//! Every function takes `&PgPool` and returns `Result` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use chrono::{DateTime, Utc};
use shared::email_crypto::EmailEncryption;
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::email_code::{hash_code, verify_code, CodePurpose};
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

#[derive(sqlx::FromRow)]
struct CredentialAccountRow {
    id: i64,
    email: Option<String>,
    email_ciphertext: Option<String>,
    handle: String,
    avatar_url: Option<String>,
    role: String,
    status: String,
    trust_level: i16,
    password_hash: Option<String>,
    credential_version: i64,
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

pub(crate) struct CredentialMutation {
    pub account: AccountRow,
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

impl CredentialAccountRow {
    fn decrypt(self, encryption: Option<&EmailEncryption>) -> AppResult<AccountRow> {
        StoredAccountRow {
            id: self.id,
            email: self.email,
            email_ciphertext: self.email_ciphertext,
            handle: self.handle,
            avatar_url: self.avatar_url,
            role: self.role,
            status: self.status,
            trust_level: self.trust_level,
            password_hash: self.password_hash,
            created_at: self.created_at,
        }
        .decrypt(encryption)
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
    credential_version: Option<i64>,
) -> AppResult<uuid::Uuid> {
    if (purpose == CodePurpose::PasswordReset) != credential_version.is_some() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "password-reset code credential binding is invalid"
        )));
    }
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
         (email, email_blind_index, email_key_version, purpose, request_id, code_hash, expires_at, \
          credential_version) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(encryption.is_none().then_some(email))
    .bind(&blind_index)
    .bind(encryption.map(|encryption| i16::from(encryption.active_version())))
    .bind(purpose.as_str())
    .bind(request_id)
    .bind(code_hash)
    .bind(expires_at)
    .bind(credential_version)
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

/// Lock the newest live code matching one email and optional purpose.
async fn lock_email_code_tx(
    connection: &mut PgConnection,
    encryption: Option<&EmailEncryption>,
    email: &str,
    purpose: Option<CodePurpose>,
) -> AppResult<EmailCodeRow> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let purpose_filter = purpose.map(CodePurpose::as_str);
    sqlx::query_as::<_, EmailCodeRow>(
        "SELECT id, purpose, code_hash, attempts, credential_version \
         FROM identity.email_codes \
         WHERE (email = $1 OR email_blind_index = $2) \
           AND expires_at > now() AND used_at IS NULL \
           AND delivery_accepted_at IS NOT NULL \
           AND (($3::text IS NULL AND purpose IN ('login', 'registration')) OR purpose = $3) \
         ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE",
    )
    .bind(email)
    .bind(blind_index)
    .bind(purpose_filter)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| crate::error::IdentityError::CodeExpired.into())
}

/// Lock and consume one purpose-compatible code exactly once.
pub async fn consume_email_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    purpose: Option<CodePurpose>,
    attempted_code: &str,
) -> AppResult<CodePurpose> {
    let mut tx = pool.begin().await?;
    let row = lock_email_code_tx(&mut tx, encryption, email, purpose).await?;
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

/// Verify a live code before expensive credential work without consuming it.
///
/// The final credential transaction must lock, verify, and consume the same purpose again. Invalid
/// attempts are still counted here, while a successful preflight only releases the row lock.
pub async fn preflight_email_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    purpose: Option<CodePurpose>,
    attempted_code: &str,
) -> AppResult<CodePurpose> {
    let mut tx = pool.begin().await?;
    let row = lock_email_code_tx(&mut tx, encryption, email, purpose).await?;
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
    let code_purpose = CodePurpose::from_stored(&row.purpose).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("invalid persisted email code purpose"))
    })?;
    tx.commit().await?;
    Ok(code_purpose)
}

/// Perform a non-consuming, version-bound reset-code preflight with a neutral failure surface.
pub async fn preflight_password_reset_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    attempted_code: &str,
) -> AppResult<()> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let mut tx = pool.begin().await?;
    let code = sqlx::query_as::<_, EmailCodeRow>(
        "SELECT id, purpose, code_hash, attempts, credential_version \
         FROM identity.email_codes \
         WHERE (email = $1 OR email_blind_index = $2) AND purpose = 'password_reset' \
           AND expires_at > now() AND used_at IS NULL AND delivery_accepted_at IS NOT NULL \
         ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE",
    )
    .bind(email)
    .bind(&blind_index)
    .fetch_optional(&mut *tx)
    .await?;
    let account: Option<(i64, bool)> = sqlx::query_as(
        "SELECT credential_version, password_hash IS NOT NULL AND status = 'active' AS eligible \
         FROM identity.accounts WHERE email = $1 OR password_email_blind = $2 FOR SHARE",
    )
    .bind(email)
    .bind(&blind_index)
    .fetch_optional(&mut *tx)
    .await?;
    let dummy_hash = hash_code("000000");
    let code_matches = verify_code(
        attempted_code,
        code.as_ref().map_or(dummy_hash.as_str(), |row| row.code_hash.as_str()),
    );
    let is_eligible = match (code.as_ref(), account) {
        (Some(code), Some((credential_version, true))) => {
            code.attempts < 5 && code_matches && code.credential_version == Some(credential_version)
        }
        _ => false,
    };
    if is_eligible {
        tx.commit().await?;
        return Ok(());
    }
    if let Some(code) = code {
        if code_matches {
            sqlx::query(
                "UPDATE identity.email_codes SET used_at = now(), attempts = 99 WHERE id = $1",
            )
            .bind(code.id)
            .execute(&mut *tx)
            .await?;
        } else if code.attempts < 5 {
            sqlx::query("UPDATE identity.email_codes SET attempts = attempts + 1 WHERE id = $1")
                .bind(code.id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Err(crate::error::IdentityError::InvalidCode.into())
}

/// Consume a recent-auth code and mark the same active session in one transaction.
pub async fn consume_recent_auth_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    account_id: i64,
    session_id: i64,
    email: &str,
    attempted_code: &str,
) -> AppResult<DateTime<Utc>> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let mut tx = pool.begin().await?;
    let session_exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM identity.sessions \
         WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL AND expires_at > now() \
         FOR UPDATE",
    )
    .bind(session_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    if session_exists.is_none() {
        return Err(AppError::RecentAuthRequired);
    }
    let row = sqlx::query_as::<_, EmailCodeRow>(
        "SELECT id, purpose, code_hash, attempts, credential_version \
         FROM identity.email_codes \
         WHERE (email = $1 OR email_blind_index = $2) AND purpose = 'recent_auth' \
           AND expires_at > now() AND used_at IS NULL \
           AND delivery_accepted_at IS NOT NULL \
         ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE",
    )
    .bind(email)
    .bind(&blind_index)
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
    let authenticated_at: DateTime<Utc> = sqlx::query_scalar(
        "UPDATE identity.sessions SET recent_authenticated_at = now(), \
                recent_auth_method = 'email_code', recent_auth_credential_version = NULL \
         WHERE id = $1 RETURNING recent_authenticated_at",
    )
    .bind(session_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(authenticated_at)
}

// ---------------------------------------------------------------------------
// accounts
// ---------------------------------------------------------------------------

async fn lock_credential_account_by_id_tx(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<CredentialAccountRow> {
    sqlx::query_as::<_, CredentialAccountRow>(
        "SELECT id, email::text AS email, email_ciphertext, handle, avatar_url, role::text, \
                status::text, trust_level, password_hash, credential_version, created_at \
         FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn lock_credential_account_by_email_tx(
    connection: &mut PgConnection,
    encryption: Option<&EmailEncryption>,
    email: &str,
) -> AppResult<Option<CredentialAccountRow>> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    sqlx::query_as::<_, CredentialAccountRow>(
        "SELECT id, email::text AS email, email_ciphertext, handle, avatar_url, role::text, \
                status::text, trust_level, password_hash, credential_version, created_at \
         FROM identity.accounts WHERE email = $1 OR email_blind_index = $2 FOR UPDATE",
    )
    .bind(email)
    .bind(blind_index)
    .fetch_optional(connection)
    .await
    .map_err(Into::into)
}

/// Atomically consume a registration code and persist exactly the selected public handle.
#[allow(clippy::too_many_arguments)] // reason: registration binds proof, encrypted identity fields, chosen handle, and optional password in one transaction
pub async fn register_account_with_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    purpose: Option<CodePurpose>,
    attempted_code: &str,
    handle: &str,
    password_hash: Option<&str>,
) -> AppResult<AccountRow> {
    let ciphertext = encryption
        .map(|encryption| encryption.encrypt(email))
        .transpose()
        .map_err(AppError::Internal)?;
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let mut tx = pool.begin().await?;
    let code = lock_email_code_tx(&mut tx, encryption, email, purpose).await?;
    if code.attempts >= 5 {
        return Err(crate::error::IdentityError::CodeExhausted.into());
    }
    if !verify_code(attempted_code, &code.code_hash) {
        sqlx::query("UPDATE identity.email_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(code.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(crate::error::IdentityError::InvalidCode.into());
    }
    let code_purpose = CodePurpose::from_stored(&code.purpose).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("invalid persisted email code purpose"))
    })?;
    if code_purpose != CodePurpose::Registration {
        sqlx::query(
            "UPDATE identity.email_codes SET attempts = attempts + 1, used_at = now() \
             WHERE id = $1",
        )
        .bind(code.id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return if code_purpose == CodePurpose::Login {
            Err(AppError::Conflict("account state changed; request a new code".into()))
        } else {
            Err(crate::error::IdentityError::InvalidCode.into())
        };
    }
    let result = sqlx::query_as::<_, StoredAccountRow>(
        "INSERT INTO identity.accounts \
         (email, email_ciphertext, email_key_version, email_blind_index, \
          password_email_blind, handle, password_hash, email_verified_at) \
         VALUES ($1, $2, $3, $4, $4, $5, $6, now()) \
         ON CONFLICT (handle) DO NOTHING \
         RETURNING id, email::text AS email, email_ciphertext, \
                   handle, avatar_url, role::text, status::text, trust_level, password_hash, \
                   created_at",
    )
    .bind(encryption.is_none().then_some(email))
    .bind(ciphertext)
    .bind(encryption.map(|encryption| i16::from(encryption.active_version())))
    .bind(blind_index)
    .bind(handle)
    .bind(password_hash)
    .fetch_optional(&mut *tx)
    .await;
    let result = match result {
        Ok(result) => result,
        Err(error)
            if error.as_database_error().is_some_and(|database| database.is_unique_violation()) =>
        {
            return Err(AppError::Conflict("account is already registered".into()));
        }
        Err(error) => return Err(error.into()),
    };
    let row = result.ok_or(crate::error::IdentityError::HandleTaken)?;
    sqlx::query(
        "UPDATE identity.email_codes SET attempts = attempts + 1, used_at = now() WHERE id = $1",
    )
    .bind(code.id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.account_onboarding SET accepted_terms_version = NULL, \
             accepted_at = NULL, completed_at = NULL, updated_at = now() \
         WHERE account_id = $1",
    )
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
    if password_hash.is_some() {
        crate::security_events::record_tx(
            &mut tx,
            row.id,
            crate::security_events::SecurityEventKind::PasswordSet,
            None,
        )
        .await?;
        crate::email_delivery::enqueue_tx(
            &mut tx,
            row.id,
            crate::email_delivery::EmailDeliveryKind::PasswordSet,
        )
        .await?;
    }
    let row = row.decrypt(encryption)?;
    tx.commit().await?;
    Ok(row)
}

/// Consume a login code and set a code-only account's first password in the same transaction.
#[allow(clippy::too_many_arguments)] // reason: mailbox proof, credential mutation, and replacement session must share one transaction
pub(crate) async fn set_password_with_login_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    account_id: i64,
    email: &str,
    attempted_code: &str,
    password_hash: &str,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
) -> AppResult<CredentialMutation> {
    let mut tx = pool.begin().await?;
    let mut account = lock_credential_account_by_id_tx(&mut tx, account_id).await?;
    if account.status != "active" {
        return Err(AppError::Forbidden);
    }
    if account.password_hash.is_some() {
        return Err(crate::error::IdentityError::AlreadyHasPassword.into());
    }
    let invitation_valid: bool = sqlx::query_scalar(
        "SELECT invited_at IS NULL OR email_verified_at IS NOT NULL \
                OR invitation_expires_at > now() \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await?;
    if !invitation_valid {
        return Err(crate::error::IdentityError::InvitationExpired.into());
    }
    let code = lock_email_code_tx(&mut tx, encryption, email, Some(CodePurpose::Login)).await?;
    if code.attempts >= 5 {
        return Err(crate::error::IdentityError::CodeExhausted.into());
    }
    if !verify_code(attempted_code, &code.code_hash) {
        sqlx::query("UPDATE identity.email_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(code.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(crate::error::IdentityError::InvalidCode.into());
    }
    sqlx::query(
        "UPDATE identity.email_codes SET attempts = attempts + 1, used_at = now() WHERE id = $1",
    )
    .bind(code.id)
    .execute(&mut *tx)
    .await?;
    let auth_version: i64 = sqlx::query_scalar(
        "UPDATE identity.accounts SET password_hash = $1, email_verified_at = now(), \
                invitation_accepted_at = CASE WHEN invited_at IS NOT NULL \
                                             THEN COALESCE(invitation_accepted_at, now()) \
                                             ELSE invitation_accepted_at END, \
                credential_version = credential_version + 1, auth_version = auth_version + 1, \
                legacy_access_revoked_before = now(), updated_at = now() \
         WHERE id = $2 AND password_hash IS NULL RETURNING auth_version",
    )
    .bind(password_hash)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(crate::error::IdentityError::AlreadyHasPassword)?;
    invalidate_password_reset_codes_tx(&mut tx, account_id).await?;
    let replacement_session_id = replace_all_sessions_tx(
        &mut tx,
        account_id,
        refresh_hash,
        expires_at,
        user_agent,
        auth_version,
    )
    .await?;
    crate::security_events::record_tx(
        &mut tx,
        account_id,
        crate::security_events::SecurityEventKind::PasswordSet,
        Some(replacement_session_id),
    )
    .await?;
    crate::email_delivery::enqueue_tx(
        &mut tx,
        account_id,
        crate::email_delivery::EmailDeliveryKind::PasswordSet,
    )
    .await?;
    account.password_hash = Some(password_hash.to_owned());
    let account = account.decrypt(encryption)?;
    tx.commit().await?;
    Ok(CredentialMutation { account, session_id: replacement_session_id, auth_version })
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
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("email or handle is already registered".into()))?;
    sqlx::query(
        "UPDATE identity.account_onboarding SET accepted_terms_version = NULL, \
             accepted_at = NULL, completed_at = NULL, updated_at = now() WHERE account_id = $1",
    )
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
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
         FROM identity.accounts WHERE id = $1 AND status <> 'purged'",
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
         FROM identity.accounts WHERE id = $1 AND status <> 'purged'",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| row.decrypt(encryption)).transpose()
}

/// Update the public handle while profile images remain media-owned assets.
pub async fn update_account(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    id: i64,
    handle: Option<&str>,
) -> AppResult<AccountRow> {
    let Some(handle) = handle else {
        return find_account_by_id(pool, encryption, id).await?.ok_or(shared::AppError::NotFound);
    };
    let row = sqlx::query_as::<_, StoredAccountRow>(
        "UPDATE identity.accounts SET handle = $1, updated_at = now() WHERE id = $2 \
         RETURNING id, email::text AS email, email_ciphertext, handle, avatar_url, \
                   role::text, status::text, trust_level, created_at",
    )
    .bind(handle)
    .bind(id)
    .fetch_one(pool)
    .await?;
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
         (account_id, refresh_hash, family_id, user_agent, expires_at, issued_auth_version) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(account_id)
    .bind(refresh_hash)
    .bind(family_id)
    .bind(user_agent)
    .bind(expires_at)
    .bind(auth_version)
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
                replaced_by_id, user_agent, created_at, last_used_at, \
                recent_authenticated_at, recent_auth_method, recent_auth_credential_version, \
                issued_auth_version \
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
        let current_auth_version: i64 =
            sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
                .bind(row.account_id)
                .fetch_one(&mut *tx)
                .await?;
        if row.issued_auth_version.is_none()
            || row.issued_auth_version == Some(current_auth_version)
        {
            invalidate_account_access(&mut tx, row.account_id).await?;
        }
        crate::security_events::record_tx(
            &mut tx,
            row.account_id,
            crate::security_events::SecurityEventKind::RefreshReplayDetected,
            Some(row.id),
        )
        .await?;
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
         (account_id, refresh_hash, family_id, rotated_from_id, user_agent, expires_at, \
          recent_authenticated_at, recent_auth_method, recent_auth_credential_version, \
          issued_auth_version) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(row.account_id)
    .bind(successor_hash)
    .bind(row.family_id)
    .bind(row.id)
    .bind(user_agent.or(row.user_agent.as_deref()))
    .bind(successor_expires_at)
    .bind(row.recent_authenticated_at)
    .bind(row.recent_auth_method)
    .bind(row.recent_auth_credential_version)
    .bind(row.issued_auth_version)
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

/// Mark a password-verified active session as recently authenticated.
pub async fn mark_recent_auth_password(
    pool: &PgPool,
    account_id: i64,
    session_id: i64,
    expected_credential_version: i64,
) -> AppResult<DateTime<Utc>> {
    let mut tx = pool.begin().await?;
    let current_version: Option<i64> = sqlx::query_scalar(
        "SELECT credential_version FROM identity.accounts \
         WHERE id = $1 AND status = 'active' FOR SHARE",
    )
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    if current_version != Some(expected_credential_version) {
        return Err(crate::error::IdentityError::RecentAuthFailed.into());
    }
    let authenticated_at = sqlx::query_scalar(
        "UPDATE identity.sessions SET recent_authenticated_at = now(), \
                recent_auth_method = 'password', recent_auth_credential_version = $3 \
         WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL AND expires_at > now() \
         RETURNING recent_authenticated_at",
    )
    .bind(session_id)
    .bind(account_id)
    .bind(expected_credential_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::RecentAuthRequired)?;
    tx.commit().await?;
    Ok(authenticated_at)
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

async fn invalidate_password_reset_codes_tx(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.email_codes code \
         SET used_at = COALESCE(code.used_at, now()), attempts = GREATEST(code.attempts, 99) \
         FROM identity.accounts account \
         WHERE account.id = $1 AND code.purpose = 'password_reset' AND code.used_at IS NULL \
           AND (code.email = account.email \
                OR code.email_blind_index = account.email_blind_index)",
    )
    .bind(account_id)
    .execute(connection)
    .await?;
    Ok(())
}

async fn replace_all_sessions_tx(
    connection: &mut PgConnection,
    account_id: i64,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
    auth_version: i64,
) -> AppResult<i64> {
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&mut *connection)
    .await?;
    let session_id = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, user_agent, expires_at, issued_auth_version) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(account_id)
    .bind(refresh_hash)
    .bind(uuid::Uuid::new_v4())
    .bind(user_agent)
    .bind(expires_at)
    .bind(auth_version)
    .fetch_one(connection)
    .await?;
    Ok(session_id)
}

/// Set the first password under an account lock and replace every prior session.
#[allow(clippy::too_many_arguments)] // reason: the credential mutation and replacement session are committed as one security boundary
pub(crate) async fn set_password_and_replace_sessions(
    pool: &PgPool,
    context: &crate::auth_middleware::AuthenticatedContext,
    password_hash: &str,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
    encryption: Option<&EmailEncryption>,
) -> AppResult<CredentialMutation> {
    let mut tx = pool.begin().await?;
    let mut account = lock_credential_account_by_id_tx(&mut tx, context.account.id).await?;
    if account.status != "active" {
        return Err(AppError::Forbidden);
    }
    if account.password_hash.is_some() {
        return Err(crate::error::IdentityError::AlreadyHasPassword.into());
    }
    crate::auth_middleware::require_recent_auth_tx(context, &mut tx).await?;
    let session_id = context.session_id.ok_or(AppError::RecentAuthRequired)?;
    let auth_version: i64 = sqlx::query_scalar(
        "UPDATE identity.accounts SET password_hash = $1, \
                credential_version = credential_version + 1, auth_version = auth_version + 1, \
                legacy_access_revoked_before = now(), updated_at = now() \
         WHERE id = $2 AND password_hash IS NULL RETURNING auth_version",
    )
    .bind(password_hash)
    .bind(account.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(crate::error::IdentityError::AlreadyHasPassword)?;
    invalidate_password_reset_codes_tx(&mut tx, account.id).await?;
    let replacement_session_id = replace_all_sessions_tx(
        &mut tx,
        account.id,
        refresh_hash,
        expires_at,
        user_agent,
        auth_version,
    )
    .await?;
    crate::security_events::record_tx(
        &mut tx,
        account.id,
        crate::security_events::SecurityEventKind::PasswordSet,
        Some(replacement_session_id),
    )
    .await?;
    crate::email_delivery::enqueue_tx(
        &mut tx,
        account.id,
        crate::email_delivery::EmailDeliveryKind::PasswordSet,
    )
    .await?;
    account.password_hash = Some(password_hash.to_owned());
    let account = account.decrypt(encryption)?;
    tx.commit().await?;
    tracing::info!(
        account_id = account.id,
        prior_session_id = session_id,
        "password was set and prior sessions were replaced"
    );
    Ok(CredentialMutation { account, session_id: replacement_session_id, auth_version })
}

/// Change a password only if the proof's credential epoch is still current.
#[allow(clippy::too_many_arguments)] // reason: the credential mutation and replacement session are committed as one security boundary
pub(crate) async fn change_password_and_replace_sessions(
    pool: &PgPool,
    account_id: i64,
    current_session_id: i64,
    expected_credential_version: i64,
    password_hash: &str,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
    encryption: Option<&EmailEncryption>,
) -> AppResult<CredentialMutation> {
    let mut tx = pool.begin().await?;
    let mut account = lock_credential_account_by_id_tx(&mut tx, account_id).await?;
    if account.status != "active" {
        return Err(AppError::Forbidden);
    }
    if account.credential_version != expected_credential_version {
        return Err(AppError::Conflict(
            "credentials changed; verify the current password again".into(),
        ));
    }
    if account.password_hash.is_none() {
        return Err(crate::error::IdentityError::NoPasswordSet.into());
    }
    let current_session_exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM identity.sessions \
         WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL AND expires_at > now() \
         FOR UPDATE",
    )
    .bind(current_session_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    if current_session_exists.is_none() {
        return Err(AppError::Unauthorized);
    }
    let auth_version: i64 = sqlx::query_scalar(
        "UPDATE identity.accounts SET password_hash = $1, \
                credential_version = credential_version + 1, auth_version = auth_version + 1, \
                legacy_access_revoked_before = now(), updated_at = now() \
         WHERE id = $2 AND credential_version = $3 RETURNING auth_version",
    )
    .bind(password_hash)
    .bind(account_id)
    .bind(expected_credential_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("credentials changed; verify again".into()))?;
    invalidate_password_reset_codes_tx(&mut tx, account_id).await?;
    let replacement_session_id = replace_all_sessions_tx(
        &mut tx,
        account_id,
        refresh_hash,
        expires_at,
        user_agent,
        auth_version,
    )
    .await?;
    crate::security_events::record_tx(
        &mut tx,
        account_id,
        crate::security_events::SecurityEventKind::PasswordChanged,
        Some(replacement_session_id),
    )
    .await?;
    crate::email_delivery::enqueue_tx(
        &mut tx,
        account_id,
        crate::email_delivery::EmailDeliveryKind::PasswordChanged,
    )
    .await?;
    account.password_hash = Some(password_hash.to_owned());
    let account = account.decrypt(encryption)?;
    tx.commit().await?;
    Ok(CredentialMutation { account, session_id: replacement_session_id, auth_version })
}

/// Consume a version-bound reset code and replace the password and all sessions atomically.
#[allow(clippy::too_many_arguments)] // reason: code proof, encrypted identity lookup, credential mutation, and replacement session form one transaction
pub(crate) async fn reset_password_with_code(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
    attempted_code: &str,
    password_hash: &str,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
    user_agent: Option<&str>,
) -> AppResult<CredentialMutation> {
    let mut tx = pool.begin().await?;
    let mut account = lock_credential_account_by_email_tx(&mut tx, encryption, email)
        .await?
        .ok_or(crate::error::IdentityError::InvalidCode)?;
    if account.status != "active" || account.password_hash.is_none() {
        return Err(crate::error::IdentityError::InvalidCode.into());
    }
    let code =
        lock_email_code_tx(&mut tx, encryption, email, Some(CodePurpose::PasswordReset)).await?;
    if code.attempts >= 5 {
        return Err(crate::error::IdentityError::CodeExhausted.into());
    }
    if code.credential_version != Some(account.credential_version) {
        sqlx::query("UPDATE identity.email_codes SET used_at = now(), attempts = 99 WHERE id = $1")
            .bind(code.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(crate::error::IdentityError::InvalidCode.into());
    }
    if !verify_code(attempted_code, &code.code_hash) {
        sqlx::query("UPDATE identity.email_codes SET attempts = attempts + 1 WHERE id = $1")
            .bind(code.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Err(crate::error::IdentityError::InvalidCode.into());
    }
    sqlx::query(
        "UPDATE identity.email_codes SET attempts = attempts + 1, used_at = now() WHERE id = $1",
    )
    .bind(code.id)
    .execute(&mut *tx)
    .await?;
    let auth_version: i64 = sqlx::query_scalar(
        "UPDATE identity.accounts SET password_hash = $1, \
                credential_version = credential_version + 1, auth_version = auth_version + 1, \
                legacy_access_revoked_before = now(), updated_at = now() \
         WHERE id = $2 AND credential_version = $3 RETURNING auth_version",
    )
    .bind(password_hash)
    .bind(account.id)
    .bind(account.credential_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("credentials changed; request a new reset code".into()))?;
    invalidate_password_reset_codes_tx(&mut tx, account.id).await?;
    let replacement_session_id = replace_all_sessions_tx(
        &mut tx,
        account.id,
        refresh_hash,
        expires_at,
        user_agent,
        auth_version,
    )
    .await?;
    crate::security_events::record_tx(
        &mut tx,
        account.id,
        crate::security_events::SecurityEventKind::PasswordReset,
        Some(replacement_session_id),
    )
    .await?;
    crate::email_delivery::enqueue_tx(
        &mut tx,
        account.id,
        crate::email_delivery::EmailDeliveryKind::PasswordReset,
    )
    .await?;
    account.password_hash = Some(password_hash.to_owned());
    let account = account.decrypt(encryption)?;
    tx.commit().await?;
    Ok(CredentialMutation { account, session_id: replacement_session_id, auth_version })
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

/// Return the credential epoch that a newly-issued reset code must bind to.
pub async fn find_password_reset_version(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    email: &str,
) -> AppResult<Option<i64>> {
    let blind_index = encryption.map(|encryption| encryption.blind_index(email));
    let credential_version = sqlx::query_scalar(
        "SELECT credential_version FROM identity.accounts \
         WHERE (email = $1 OR password_email_blind = $2) \
           AND password_hash IS NOT NULL AND status = 'active'",
    )
    .bind(email)
    .bind(blind_index)
    .fetch_optional(pool)
    .await?;
    Ok(credential_version)
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

/// Return the password hash and credential epoch captured before expensive verification.
pub async fn find_password_state_by_account_id(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<Option<(String, i64)>> {
    let state = sqlx::query_as(
        "SELECT password_hash, credential_version FROM identity.accounts \
         WHERE id = $1 AND password_hash IS NOT NULL",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    Ok(state)
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

//! Durable owner-data export jobs and the identity-owned export projection.

use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Serialize;
use sha2::{Digest, Sha256};
use shared::email_crypto::EmailEncryption;
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection, PgPool};

use crate::auth_middleware::AuthenticatedContext;

#[derive(Debug, Clone, FromRow)]
pub struct ExportJobRecord {
    pub id: uuid::Uuid,
    pub account_id: i64,
    pub status: String,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ClaimedExportJob {
    pub id: uuid::Uuid,
    pub account_id: i64,
}

#[derive(Debug, Clone)]
pub struct DownloadGrant {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityExport {
    account: IdentityAccountExport,
    profile: IdentityProfileExport,
    privacy: IdentityPrivacyExport,
    onboarding: IdentityOnboardingExport,
    lifecycle: crate::lifecycle::LifecycleRecord,
    sessions: Vec<IdentitySessionExport>,
    sanctions: Vec<IdentitySanctionExport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityAccountExport {
    id: String,
    handle: String,
    role: String,
    status: String,
    trust_level: i16,
    email_verified_at: Option<i64>,
    created_at: i64,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityProfileExport {
    display_name: Option<String>,
    bio: Option<String>,
    website: Option<String>,
    avatar_asset_id: Option<i64>,
    banner_asset_id: Option<i64>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityPrivacyExport {
    profile_visibility: String,
    activity_visibility: String,
    followers_visibility: String,
    following_visibility: String,
    discoverable: bool,
    dm_policy: String,
    mention_policy: String,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityOnboardingExport {
    required_terms_version: String,
    accepted_terms_version: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    accepted_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentitySessionExport {
    device_label: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    last_used_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    expires_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentitySanctionExport {
    kind: String,
    reason: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    starts_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    ends_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    revoked_at: Option<DateTime<Utc>>,
}

fn hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn generate_token() -> AppResult<String> {
    let mut bytes = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("system random source failed")))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

fn validate_idempotency_key(value: &str) -> AppResult<&str> {
    let value = value.trim();
    if !(8..=128).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(AppError::BadRequest("invalid Idempotency-Key".into()));
    }
    Ok(value)
}

async fn fetch_job(
    connection: &mut PgConnection,
    account_id: i64,
    export_id: uuid::Uuid,
) -> AppResult<ExportJobRecord> {
    sqlx::query_as::<_, ExportJobRecord>(
        "SELECT id, account_id, status, error_code, created_at, updated_at, expires_at \
         FROM identity.account_export_jobs WHERE id = $1 AND account_id = $2",
    )
    .bind(export_id)
    .bind(account_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn create_job(
    pool: &PgPool,
    context: &AuthenticatedContext,
    idempotency_key: &str,
) -> AppResult<ExportJobRecord> {
    let idempotency_key = validate_idempotency_key(idempotency_key)?;
    let idempotency_hash = hash(idempotency_key);
    let mut tx = pool.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(context, &mut tx).await?;
    if let Some(existing) = sqlx::query_as::<_, ExportJobRecord>(
        "SELECT id, account_id, status, error_code, created_at, updated_at, expires_at \
         FROM identity.account_export_jobs \
         WHERE account_id = $1 AND idempotency_hash = $2 FOR SHARE",
    )
    .bind(context.account.id)
    .bind(&idempotency_hash)
    .fetch_optional(&mut *tx)
    .await?
    {
        tx.commit().await?;
        return Ok(existing);
    }
    let export_id = uuid::Uuid::new_v4();
    let expires_at = Utc::now() + Duration::hours(24);
    sqlx::query(
        "INSERT INTO identity.account_export_jobs \
         (id, account_id, idempotency_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(export_id)
    .bind(context.account.id)
    .bind(idempotency_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    let job = fetch_job(&mut tx, context.account.id, export_id).await?;
    tx.commit().await?;
    Ok(job)
}

pub async fn get_job(
    pool: &PgPool,
    account_id: i64,
    export_id: uuid::Uuid,
) -> AppResult<ExportJobRecord> {
    sqlx::query_as::<_, ExportJobRecord>(
        "SELECT id, account_id, \
                CASE WHEN expires_at <= now() THEN 'expired' ELSE status END AS status, \
                error_code, created_at, updated_at, expires_at \
         FROM identity.account_export_jobs WHERE id = $1 AND account_id = $2",
    )
    .bind(export_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn list_jobs(pool: &PgPool, account_id: i64) -> AppResult<Vec<ExportJobRecord>> {
    Ok(sqlx::query_as::<_, ExportJobRecord>(
        "SELECT id, account_id, \
                CASE WHEN expires_at <= now() THEN 'expired' ELSE status END AS status, \
                error_code, created_at, updated_at, expires_at \
         FROM identity.account_export_jobs WHERE account_id = $1 \
         ORDER BY created_at DESC, id DESC LIMIT 20",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?)
}

pub async fn claim_job(pool: &PgPool) -> AppResult<Option<ClaimedExportJob>> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM identity.account_export_download_grants WHERE expires_at <= now()")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE identity.account_export_jobs SET status = 'expired', updated_at = now(), \
             artifact = NULL, locked_at = NULL WHERE expires_at <= now() AND status <> 'expired'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.account_export_jobs SET status = 'failed', locked_at = NULL, \
             next_attempt_at = now(), error_code = 'worker_lease_expired', updated_at = now() \
         WHERE status = 'running' AND locked_at < now() - interval '10 minutes'",
    )
    .execute(&mut *tx)
    .await?;
    let job = sqlx::query_as::<_, ClaimedExportJob>(
        "SELECT id, account_id FROM identity.account_export_jobs \
         WHERE status IN ('queued', 'failed') AND next_attempt_at <= now() \
           AND expires_at > now() AND attempts < 10 \
         ORDER BY next_attempt_at, created_at, id LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(job) = &job {
        sqlx::query(
            "UPDATE identity.account_export_jobs SET status = 'running', locked_at = now(), \
                 attempts = attempts + 1, error_code = NULL, updated_at = now() WHERE id = $1",
        )
        .bind(job.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(job)
}

pub async fn complete_job<T: Serialize>(
    pool: &PgPool,
    export_id: uuid::Uuid,
    artifact: &T,
) -> AppResult<()> {
    let artifact = serde_json::to_value(artifact)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    let changed = sqlx::query(
        "UPDATE identity.account_export_jobs SET status = 'ready', artifact = $2, \
             locked_at = NULL, error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'running' AND expires_at > now()",
    )
    .bind(export_id)
    .bind(artifact)
    .execute(pool)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::Conflict("export job is no longer running".into()));
    }
    Ok(())
}

pub async fn fail_job(pool: &PgPool, export_id: uuid::Uuid, error_code: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.account_export_jobs SET status = 'failed', locked_at = NULL, \
             next_attempt_at = now() + LEAST(attempts, 10) * interval '1 minute', \
             error_code = left($2, 80), updated_at = now() WHERE id = $1 AND status = 'running'",
    )
    .bind(export_id)
    .bind(error_code)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn issue_download_grant(
    pool: &PgPool,
    context: &AuthenticatedContext,
    export_id: uuid::Uuid,
) -> AppResult<DownloadGrant> {
    let mut tx = pool.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(context, &mut tx).await?;
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM identity.account_export_jobs \
         WHERE id = $1 AND account_id = $2 AND expires_at > now() FOR SHARE",
    )
    .bind(export_id)
    .bind(context.account.id)
    .fetch_optional(&mut *tx)
    .await?;
    match status.as_deref() {
        Some("ready") => {}
        Some(_) => return Err(AppError::Conflict("export is not ready".into())),
        None => return Err(AppError::NotFound),
    }
    let token = generate_token()?;
    let expires_at = Utc::now() + Duration::minutes(5);
    sqlx::query(
        "INSERT INTO identity.account_export_download_grants \
         (export_id, account_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(export_id)
    .bind(context.account.id)
    .bind(hash(&token))
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(DownloadGrant { token, expires_at })
}

pub async fn consume_download_grant(
    pool: &PgPool,
    account_id: i64,
    export_id: uuid::Uuid,
    token: &str,
) -> AppResult<serde_json::Value> {
    if token.len() != 43 {
        return Err(AppError::Unauthorized);
    }
    let mut tx = pool.begin().await?;
    let grant_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM identity.account_export_download_grants \
         WHERE export_id = $1 AND account_id = $2 AND token_hash = $3 \
           AND expires_at > now() AND consumed_at IS NULL FOR UPDATE",
    )
    .bind(export_id)
    .bind(account_id)
    .bind(hash(token))
    .fetch_optional(&mut *tx)
    .await?;
    let grant_id = grant_id.ok_or(AppError::Unauthorized)?;
    let artifact: serde_json::Value = sqlx::query_scalar(
        "SELECT artifact FROM identity.account_export_jobs \
         WHERE id = $1 AND account_id = $2 AND status = 'ready' AND expires_at > now()",
    )
    .bind(export_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    sqlx::query(
        "UPDATE identity.account_export_download_grants SET consumed_at = now() WHERE id = $1",
    )
    .bind(grant_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.account_export_jobs SET downloaded_at = now(), updated_at = now() \
         WHERE id = $1 AND account_id = $2",
    )
    .bind(export_id)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(artifact)
}

pub async fn snapshot(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    account_id: i64,
) -> AppResult<IdentityExport> {
    let account = crate::repo::find_account_by_id(pool, encryption, account_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let email_verified_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT email_verified_at FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await?;
    let profile = sqlx::query_as::<_, IdentityProfileExport>(
        "SELECT display_name, bio, website, avatar_asset_id, banner_asset_id \
         FROM identity.profiles WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(IdentityProfileExport {
        display_name: None,
        bio: None,
        website: None,
        avatar_asset_id: None,
        banner_asset_id: None,
    });
    let privacy = sqlx::query_as::<_, IdentityPrivacyExport>(
        "SELECT profile_visibility, activity_visibility, followers_visibility, \
                following_visibility, discoverable, dm_policy, mention_policy \
         FROM identity.profile_privacy WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    let onboarding = sqlx::query_as::<_, IdentityOnboardingExport>(
        "SELECT required_terms_version, accepted_terms_version, accepted_at, completed_at \
         FROM identity.account_onboarding WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    let sessions = sqlx::query_as::<_, IdentitySessionExport>(
        "SELECT user_agent AS device_label, created_at, last_used_at, expires_at, revoked_at \
         FROM identity.sessions WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let sanctions = sqlx::query_as::<_, IdentitySanctionExport>(
        "SELECT kind, reason, starts_at, ends_at, revoked_at \
         FROM identity.sanctions WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let lifecycle = crate::lifecycle::get(pool, account_id).await?;
    Ok(IdentityExport {
        account: IdentityAccountExport {
            id: account.id.to_string(),
            handle: account.handle,
            role: account.role,
            status: account.status,
            trust_level: account.trust_level,
            email_verified_at: email_verified_at.map(|value: DateTime<Utc>| value.timestamp()),
            created_at: account.created_at.timestamp(),
        },
        profile,
        privacy,
        onboarding,
        lifecycle,
        sessions,
        sanctions,
    })
}

/// Resolve the owner's encrypted-at-rest email only for immediate download response assembly.
pub async fn owner_email(
    pool: &PgPool,
    encryption: Option<&EmailEncryption>,
    account_id: i64,
) -> AppResult<String> {
    crate::repo::find_account_by_id(pool, encryption, account_id)
        .await?
        .map(|account| account.email)
        .ok_or(AppError::NotFound)
}

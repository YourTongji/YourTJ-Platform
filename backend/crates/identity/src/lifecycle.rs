//! Account lifecycle transitions and purpose-bound recovery credentials.

use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Serialize;
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection, PgPool};

use crate::auth_middleware::AuthenticatedContext;

const RECOVERY_CREDENTIAL_MINUTES: i64 = 15;
const DELETION_RECOVERY_DAYS: i64 = 30;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleRecord {
    pub state: String,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub deactivated_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub deletion_requested_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub recover_until: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub purge_started_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub purged_at: Option<DateTime<Utc>>,
    pub lifecycle_version: i64,
}

#[derive(Debug, Clone)]
pub struct IssuedRecoveryCredential {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub lifecycle: LifecycleRecord,
}

#[derive(Debug, Clone, FromRow)]
pub struct LifecycleJob {
    pub id: i64,
    pub account_id: i64,
    pub job_type: String,
    pub lease_token: uuid::Uuid,
}

#[derive(Debug, FromRow)]
struct LifecycleJobCandidate {
    id: i64,
    account_id: i64,
    job_type: String,
}

#[derive(Debug, FromRow)]
struct RecoveryCredentialRow {
    lifecycle_version: i64,
    consumed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct PurgeCandidateRow {
    state: String,
    recover_until: Option<DateTime<Utc>>,
    purge_started_at: Option<DateTime<Utc>>,
    recovery_expired: bool,
}

#[derive(Debug, FromRow)]
struct PurgeFinalizationRow {
    state: String,
    recover_until: Option<DateTime<Utc>>,
    email: Option<String>,
    email_blind_index: Option<String>,
    purge_started_at: Option<DateTime<Utc>>,
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn generate_token() -> AppResult<String> {
    let mut bytes = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("system random source failed")))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

async fn read_lifecycle_tx(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<LifecycleRecord> {
    sqlx::query_as::<_, LifecycleRecord>(
        "SELECT status::text AS state, deactivated_at, deletion_requested_at, \
                deletion_recover_until AS recover_until, deleted_at, purge_started_at, purged_at, \
                lifecycle_version \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn get(pool: &PgPool, account_id: i64) -> AppResult<LifecycleRecord> {
    sqlx::query_as::<_, LifecycleRecord>(
        "SELECT status::text AS state, deactivated_at, deletion_requested_at, \
                deletion_recover_until AS recover_until, deleted_at, purge_started_at, purged_at, \
                lifecycle_version \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

fn is_recoverable(lifecycle: &LifecycleRecord, now: DateTime<Utc>) -> bool {
    if lifecycle.purge_started_at.is_some() {
        return false;
    }
    match lifecycle.state.as_str() {
        "deactivated" => true,
        "deletion_requested" | "deleted" => {
            lifecycle.recover_until.is_some_and(|deadline| deadline > now)
        }
        _ => false,
    }
}

pub async fn can_recover(pool: &PgPool, account_id: i64) -> AppResult<bool> {
    let lifecycle = get(pool, account_id).await?;
    let purge_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM identity.account_lifecycle_jobs \
         WHERE account_id = $1 AND job_type = 'purge'",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    Ok(!matches!(purge_status.as_deref(), Some("running" | "failed"))
        && is_recoverable(&lifecycle, Utc::now()))
}

async fn issue_recovery_credential_tx(
    connection: &mut PgConnection,
    account_id: i64,
    lifecycle: LifecycleRecord,
    proof_method: &str,
) -> AppResult<IssuedRecoveryCredential> {
    if !is_recoverable(&lifecycle, Utc::now()) {
        return Err(AppError::Forbidden);
    }
    let token = generate_token()?;
    let expires_at = Utc::now() + Duration::minutes(RECOVERY_CREDENTIAL_MINUTES);
    sqlx::query(
        "INSERT INTO identity.account_recovery_credentials \
         (account_id, token_hash, proof_method, lifecycle_version, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(account_id)
    .bind(token_hash(&token))
    .bind(proof_method)
    .bind(lifecycle.lifecycle_version)
    .bind(expires_at)
    .execute(connection)
    .await?;
    Ok(IssuedRecoveryCredential { token, expires_at, lifecycle })
}

pub async fn issue_recovery_credential(
    pool: &PgPool,
    account_id: i64,
    proof_method: &str,
) -> AppResult<IssuedRecoveryCredential> {
    let mut tx = pool.begin().await?;
    let purge_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM identity.account_lifecycle_jobs \
         WHERE account_id = $1 AND job_type = 'purge' FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    if matches!(purge_status.as_deref(), Some("running" | "failed")) {
        return Err(AppError::Forbidden);
    }
    let lifecycle = sqlx::query_as::<_, LifecycleRecord>(
        "SELECT status::text AS state, deactivated_at, deletion_requested_at, \
                deletion_recover_until AS recover_until, deleted_at, purge_started_at, purged_at, \
                lifecycle_version \
         FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let credential =
        issue_recovery_credential_tx(&mut tx, account_id, lifecycle, proof_method).await?;
    tx.commit().await?;
    Ok(credential)
}

async fn revoke_all_sessions_tx(connection: &mut PgConnection, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE identity.accounts \
         SET auth_version = auth_version + 1, legacy_access_revoked_before = now() \
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(connection)
    .await?;
    Ok(())
}

fn validate_idempotency_key(value: &str) -> AppResult<&str> {
    let value = value.trim();
    if !(8..=128).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(AppError::BadRequest("invalid Idempotency-Key".into()));
    }
    Ok(value)
}

async fn existing_transition(
    connection: &mut PgConnection,
    account_id: i64,
    idempotency_key: &str,
    request_hash: &str,
) -> AppResult<Option<LifecycleRecord>> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT request_hash FROM identity.account_lifecycle_events \
         WHERE account_id = $1 AND idempotency_key = $2",
    )
    .bind(account_id)
    .bind(idempotency_key)
    .fetch_optional(&mut *connection)
    .await?;
    match existing {
        Some(existing_hash) if existing_hash == request_hash => {
            read_lifecycle_tx(connection, account_id).await.map(Some)
        }
        Some(_) => Err(AppError::Conflict(
            "Idempotency-Key was already used for another lifecycle request".into(),
        )),
        None => Ok(None),
    }
}

pub async fn deactivate(
    pool: &PgPool,
    context: &AuthenticatedContext,
    idempotency_key: &str,
) -> AppResult<IssuedRecoveryCredential> {
    let idempotency_key = validate_idempotency_key(idempotency_key)?;
    let request_hash = hex::encode(Sha256::digest(b"deactivate"));
    let mut tx = pool.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(context, &mut tx).await?;
    if let Some(lifecycle) =
        existing_transition(&mut tx, context.account.id, idempotency_key, &request_hash).await?
    {
        let credential =
            issue_recovery_credential_tx(&mut tx, context.account.id, lifecycle, "session").await?;
        tx.commit().await?;
        return Ok(credential);
    }
    let previous_state: String =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(context.account.id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    if previous_state != "active" {
        return Err(AppError::Conflict("account is not active".into()));
    }
    sqlx::query(
        "UPDATE identity.accounts \
         SET status = 'deactivated', deactivated_at = now(), lifecycle_version = lifecycle_version + 1, \
             updated_at = now() WHERE id = $1",
    )
    .bind(context.account.id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity.account_lifecycle_events \
         (account_id, actor_kind, from_state, to_state, idempotency_key, request_hash) \
         VALUES ($1, 'account', 'active', 'deactivated', $2, $3)",
    )
    .bind(context.account.id)
    .bind(idempotency_key)
    .bind(&request_hash)
    .execute(&mut *tx)
    .await?;
    revoke_all_sessions_tx(&mut tx, context.account.id).await?;
    let lifecycle = read_lifecycle_tx(&mut tx, context.account.id).await?;
    let credential =
        issue_recovery_credential_tx(&mut tx, context.account.id, lifecycle, "session").await?;
    tx.commit().await?;
    Ok(credential)
}

pub async fn request_deletion(
    pool: &PgPool,
    context: &AuthenticatedContext,
    idempotency_key: &str,
) -> AppResult<IssuedRecoveryCredential> {
    let idempotency_key = validate_idempotency_key(idempotency_key)?;
    let request_hash = hex::encode(Sha256::digest(b"delete"));
    let mut tx = pool.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(context, &mut tx).await?;
    if let Some(lifecycle) =
        existing_transition(&mut tx, context.account.id, idempotency_key, &request_hash).await?
    {
        let credential =
            issue_recovery_credential_tx(&mut tx, context.account.id, lifecycle, "session").await?;
        tx.commit().await?;
        return Ok(credential);
    }
    let previous_state: String =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(context.account.id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    if previous_state != "active" {
        return Err(AppError::Conflict("account is not active".into()));
    }
    let requested_at = Utc::now();
    let recover_until = requested_at + Duration::days(DELETION_RECOVERY_DAYS);
    sqlx::query(
        "UPDATE identity.accounts \
         SET status = 'deletion_requested', deactivated_at = NULL, \
             deletion_requested_at = $2, deletion_recover_until = $3, \
             lifecycle_version = lifecycle_version + 1, updated_at = now() \
         WHERE id = $1",
    )
    .bind(context.account.id)
    .bind(requested_at)
    .bind(recover_until)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity.account_lifecycle_events \
         (account_id, actor_kind, from_state, to_state, idempotency_key, request_hash) \
         VALUES ($1, 'account', 'active', 'deletion_requested', $2, $3)",
    )
    .bind(context.account.id)
    .bind(idempotency_key)
    .bind(&request_hash)
    .execute(&mut *tx)
    .await?;
    let scheduled_job_types: Vec<String> = sqlx::query_scalar(
        "INSERT INTO identity.account_lifecycle_jobs (account_id, job_type, next_attempt_at) \
         VALUES ($1, 'mark_deleted', now()), ($1, 'purge', $2) \
         ON CONFLICT (account_id, job_type) DO UPDATE \
         SET status = 'queued', attempts = 0, next_attempt_at = EXCLUDED.next_attempt_at, \
             locked_at = NULL, lease_token = NULL, last_error_code = NULL, updated_at = now() \
         WHERE account_lifecycle_jobs.status = 'succeeded' \
         RETURNING job_type",
    )
    .bind(context.account.id)
    .bind(recover_until)
    .fetch_all(&mut *tx)
    .await?;
    if scheduled_job_types.len() != 2
        || !scheduled_job_types.iter().any(|job_type| job_type == "mark_deleted")
        || !scheduled_job_types.iter().any(|job_type| job_type == "purge")
    {
        return Err(AppError::Conflict(
            "account lifecycle jobs are not ready for a new deletion request".into(),
        ));
    }
    revoke_all_sessions_tx(&mut tx, context.account.id).await?;
    let lifecycle = read_lifecycle_tx(&mut tx, context.account.id).await?;
    let credential =
        issue_recovery_credential_tx(&mut tx, context.account.id, lifecycle, "session").await?;
    tx.commit().await?;
    Ok(credential)
}

pub async fn inspect_recovery(pool: &PgPool, token: &str) -> AppResult<LifecycleRecord> {
    if token.len() != 43 {
        return Err(AppError::Unauthorized);
    }
    let row = sqlx::query_as::<_, LifecycleRecord>(
        "SELECT account.status::text AS state, account.deactivated_at, \
                account.deletion_requested_at, account.deletion_recover_until AS recover_until, \
                account.deleted_at, account.purge_started_at, account.purged_at, \
                account.lifecycle_version \
         FROM identity.account_recovery_credentials credential \
         JOIN identity.accounts account ON account.id = credential.account_id \
         WHERE credential.token_hash = $1 AND credential.expires_at > now() \
           AND (credential.consumed_at IS NULL OR account.status = 'active') \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.account_lifecycle_jobs job \
             WHERE job.account_id = account.id AND job.job_type = 'purge' \
               AND job.status IN ('running', 'failed') \
           )",
    )
    .bind(token_hash(token))
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if row.state != "active" && !is_recoverable(&row, Utc::now()) {
        return Err(AppError::Forbidden);
    }
    Ok(row)
}

pub async fn reactivate(pool: &PgPool, token: &str) -> AppResult<LifecycleRecord> {
    if token.len() != 43 {
        return Err(AppError::Unauthorized);
    }
    let hashed_token = token_hash(token);
    let account_id: i64 = sqlx::query_scalar(
        "SELECT account_id FROM identity.account_recovery_credentials WHERE token_hash = $1",
    )
    .bind(&hashed_token)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    let mut tx = pool.begin().await?;
    let lifecycle_jobs: Vec<(String, String)> = sqlx::query_as(
        "SELECT job_type, status FROM identity.account_lifecycle_jobs \
         WHERE account_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(account_id)
    .fetch_all(&mut *tx)
    .await?;
    let lifecycle = sqlx::query_as::<_, LifecycleRecord>(
        "SELECT status::text AS state, deactivated_at, deletion_requested_at, \
                deletion_recover_until AS recover_until, deleted_at, purge_started_at, purged_at, \
                lifecycle_version \
         FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await?;
    let credential = sqlx::query_as::<_, RecoveryCredentialRow>(
        "SELECT lifecycle_version, consumed_at, expires_at \
         FROM identity.account_recovery_credentials \
         WHERE token_hash = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(&hashed_token)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if credential.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }
    let purge_status = lifecycle_jobs
        .iter()
        .find_map(|(job_type, status)| (job_type == "purge").then_some(status.as_str()));
    if matches!(purge_status, Some("running" | "failed")) {
        return Err(AppError::Forbidden);
    }
    if credential.consumed_at.is_some() && lifecycle.state == "active" {
        tx.commit().await?;
        return Ok(lifecycle);
    }
    if credential.consumed_at.is_some()
        || credential.lifecycle_version != lifecycle.lifecycle_version
        || !is_recoverable(&lifecycle, Utc::now())
    {
        return Err(AppError::Forbidden);
    }
    sqlx::query(
        "UPDATE identity.accounts SET status = 'active', deactivated_at = NULL, \
             deletion_requested_at = NULL, deletion_recover_until = NULL, deleted_at = NULL, \
             purge_started_at = NULL, \
             lifecycle_version = lifecycle_version + 1, updated_at = now() WHERE id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.account_recovery_credentials SET consumed_at = now() WHERE token_hash = $1",
    )
    .bind(&hashed_token)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'succeeded', locked_at = NULL, lease_token = NULL, \
             updated_at = now(), last_error_code = NULL \
         WHERE account_id = $1 AND status <> 'succeeded'",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity.account_lifecycle_events \
         (account_id, actor_kind, from_state, to_state) VALUES ($1, 'account', $2, 'active')",
    )
    .bind(account_id)
    .bind(&lifecycle.state)
    .execute(&mut *tx)
    .await?;
    revoke_all_sessions_tx(&mut tx, account_id).await?;
    let active = read_lifecycle_tx(&mut tx, account_id).await?;
    tx.commit().await?;
    Ok(active)
}

pub async fn claim_due_job(pool: &PgPool) -> AppResult<Option<LifecycleJob>> {
    sqlx::query("DELETE FROM identity.account_recovery_credentials WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'failed', locked_at = NULL, lease_token = NULL, next_attempt_at = now(), \
             last_error_code = 'worker_lease_expired', updated_at = now() \
         WHERE status = 'running' AND locked_at < now() - interval '10 minutes'",
    )
    .execute(&mut *tx)
    .await?;
    let candidate = sqlx::query_as::<_, LifecycleJobCandidate>(
        "SELECT id, account_id, job_type FROM identity.account_lifecycle_jobs \
         WHERE status IN ('queued', 'failed') AND next_attempt_at <= now() AND attempts < 20 \
         ORDER BY next_attempt_at, id LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(candidate) = candidate else {
        tx.commit().await?;
        return Ok(None);
    };
    if candidate.job_type == "purge" {
        let account = sqlx::query_as::<_, PurgeCandidateRow>(
            "SELECT status::text AS state, deletion_recover_until AS recover_until, \
                        purge_started_at, \
                        deletion_recover_until IS NOT NULL \
                            AND deletion_recover_until <= now() AS recovery_expired \
                 FROM identity.accounts WHERE id = $1 FOR UPDATE",
        )
        .bind(candidate.account_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(account) = account else {
            return Err(AppError::NotFound);
        };
        if account.state == "purged"
            || !matches!(account.state.as_str(), "deletion_requested" | "deleted")
        {
            finish_unleased_job_tx(&mut tx, candidate.id).await?;
            tx.commit().await?;
            return Ok(None);
        }
        if account.purge_started_at.is_none() && !account.recovery_expired {
            let recover_until = account.recover_until.ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("purge candidate is missing recovery deadline"))
            })?;
            sqlx::query(
                "UPDATE identity.account_lifecycle_jobs \
                 SET status = 'queued', locked_at = NULL, lease_token = NULL, \
                     next_attempt_at = $2, updated_at = now(), last_error_code = NULL \
                 WHERE id = $1 AND status IN ('queued', 'failed') AND lease_token IS NULL",
            )
            .bind(candidate.id)
            .bind(recover_until)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(None);
        }
        if account.purge_started_at.is_none() {
            if account.state == "deletion_requested" {
                sqlx::query(
                    "UPDATE identity.accounts SET status = 'deleted', deleted_at = now(), \
                         purge_started_at = now(), lifecycle_version = lifecycle_version + 1, \
                         updated_at = now() WHERE id = $1",
                )
                .bind(candidate.account_id)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO identity.account_lifecycle_events \
                     (account_id, actor_kind, from_state, to_state) \
                     VALUES ($1, 'system', 'deletion_requested', 'deleted')",
                )
                .bind(candidate.account_id)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE identity.accounts SET purge_started_at = now(), updated_at = now() \
                     WHERE id = $1",
                )
                .bind(candidate.account_id)
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    let lease_token = uuid::Uuid::new_v4();
    let job = sqlx::query_as::<_, LifecycleJob>(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'running', locked_at = now(), lease_token = $2, attempts = attempts + 1, \
             updated_at = now(), last_error_code = NULL \
         WHERE id = $1 AND status IN ('queued', 'failed') AND lease_token IS NULL \
         RETURNING id, account_id, job_type, lease_token",
    )
    .bind(candidate.id)
    .bind(lease_token)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("lifecycle job is no longer claimable".into()))?;
    tx.commit().await?;
    Ok(Some(job))
}

pub async fn complete_mark_deleted(pool: &PgPool, job: &LifecycleJob) -> AppResult<Option<i64>> {
    let mut tx = pool.begin().await?;
    lock_leased_job_tx(&mut tx, job).await?;
    let state: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(job.account_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(state) = state else {
        return Err(AppError::NotFound);
    };
    let changed = if state == "deletion_requested" {
        sqlx::query(
            "UPDATE identity.accounts SET status = 'deleted', deleted_at = now(), \
                 lifecycle_version = lifecycle_version + 1, updated_at = now() WHERE id = $1",
        )
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO identity.account_lifecycle_events \
             (account_id, actor_kind, from_state, to_state) \
             VALUES ($1, 'system', 'deletion_requested', 'deleted')",
        )
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
        Some(job.account_id)
    } else {
        None
    };
    finish_leased_job_tx(&mut tx, job).await?;
    tx.commit().await?;
    Ok(changed)
}

pub async fn complete_purge(pool: &PgPool, job: &LifecycleJob) -> AppResult<Option<i64>> {
    let mut tx = pool.begin().await?;
    lock_leased_job_tx(&mut tx, job).await?;
    let row = sqlx::query_as::<_, PurgeFinalizationRow>(
        "SELECT status::text AS state, deletion_recover_until AS recover_until, \
                    email::text, email_blind_index, purge_started_at \
         FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(job.account_id)
    .fetch_optional(&mut *tx)
    .await?;
    let row = row.ok_or(AppError::NotFound)?;
    if row.state == "purged" {
        finish_leased_job_tx(&mut tx, job).await?;
        tx.commit().await?;
        return Ok(None);
    }
    if row.state != "deleted" || row.purge_started_at.is_none() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "purge finalization attempted before irreversible purge start"
        )));
    }
    if row.recover_until.is_some_and(|deadline| deadline > Utc::now()) {
        return Err(AppError::Internal(anyhow::anyhow!(
            "purge finalization attempted before recovery deadline"
        )));
    }
    sqlx::query(
        "DELETE FROM identity.email_codes \
         WHERE ($1::text IS NOT NULL AND email = $1::citext) \
            OR ($2::text IS NOT NULL AND email_blind_index = $2)",
    )
    .bind(row.email)
    .bind(row.email_blind_index)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM identity.account_export_jobs WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.account_recovery_credentials WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.sessions WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.legacy_wallet_links WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.wallet_claim_challenges WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.profiles WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.profile_privacy WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM identity.account_onboarding WHERE account_id = $1")
        .bind(job.account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE identity.account_keys SET revoked_at = COALESCE(revoked_at, now()) \
         WHERE account_id = $1",
    )
    .bind(job.account_id)
    .execute(&mut *tx)
    .await?;
    let tombstone_id = uuid::Uuid::new_v4();
    sqlx::query(
        "UPDATE identity.accounts SET status = 'purged', email = NULL, email_ciphertext = NULL, \
             email_key_version = NULL, email_blind_index = NULL, password_email_blind = NULL, \
             password_hash = NULL, email_verified_at = NULL, handle = $2, avatar_url = NULL, \
             role = 'user', trust_level = 0, invited_by = NULL, invited_at = NULL, \
             invitation_expires_at = NULL, invitation_accepted_at = NULL, purged_at = now(), \
             tombstone_id = $3, lifecycle_version = lifecycle_version + 1, \
             credential_version = credential_version + 1, auth_version = auth_version + 1, \
             legacy_access_revoked_before = now(), last_active_at = now(), updated_at = now() \
         WHERE id = $1",
    )
    .bind(job.account_id)
    .bind(format!("deleted-{}", &tombstone_id.simple().to_string()[..12]))
    .bind(tombstone_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity.account_lifecycle_events \
         (account_id, actor_kind, from_state, to_state) VALUES ($1, 'system', 'deleted', 'purged')",
    )
    .bind(job.account_id)
    .execute(&mut *tx)
    .await?;
    finish_leased_job_tx(&mut tx, job).await?;
    tx.commit().await?;
    Ok(Some(job.account_id))
}

async fn lock_leased_job_tx(connection: &mut PgConnection, job: &LifecycleJob) -> AppResult<()> {
    let leased_job_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM identity.account_lifecycle_jobs \
         WHERE id = $1 AND account_id = $2 AND job_type = $3 \
           AND status = 'running' AND lease_token = $4 \
         FOR UPDATE",
    )
    .bind(job.id)
    .bind(job.account_id)
    .bind(&job.job_type)
    .bind(job.lease_token)
    .fetch_optional(connection)
    .await?;
    if leased_job_id.is_none() {
        return Err(AppError::Conflict("lifecycle job lease was lost".into()));
    }
    Ok(())
}

async fn finish_unleased_job_tx(connection: &mut PgConnection, job_id: i64) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'succeeded', locked_at = NULL, lease_token = NULL, \
             updated_at = now(), last_error_code = NULL \
         WHERE id = $1 AND status IN ('queued', 'failed') AND lease_token IS NULL",
    )
    .bind(job_id)
    .execute(connection)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("lifecycle job is no longer claimable".into()));
    }
    Ok(())
}

async fn finish_leased_job_tx(connection: &mut PgConnection, job: &LifecycleJob) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'succeeded', locked_at = NULL, lease_token = NULL, \
             updated_at = now(), last_error_code = NULL \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .execute(connection)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("lifecycle job lease was lost".into()));
    }
    Ok(())
}

pub async fn fail_job(pool: &PgPool, job: &LifecycleJob, error_code: &str) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'failed', locked_at = NULL, lease_token = NULL, \
             next_attempt_at = now() + LEAST(attempts, 10) * interval '1 minute', \
             last_error_code = left($3, 80), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(error_code)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("lifecycle job lease was lost".into()));
    }
    Ok(())
}

/// Return a running job to the queue without consuming a retry attempt.
pub async fn defer_running_job(
    pool: &PgPool,
    job: &LifecycleJob,
    retry_delay: Duration,
    error_code: &str,
) -> AppResult<()> {
    if retry_delay < Duration::seconds(1) {
        return Err(AppError::Internal(anyhow::anyhow!(
            "lifecycle job retry delay must be at least one second"
        )));
    }
    let next_attempt_at = Utc::now().checked_add_signed(retry_delay).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("lifecycle job retry delay is out of range"))
    })?;
    let affected = sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'queued', locked_at = NULL, lease_token = NULL, \
             attempts = GREATEST(attempts - 1, 0), next_attempt_at = $3, \
             last_error_code = left($4, 80), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(next_attempt_at)
    .bind(error_code)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("lifecycle job lease was lost".into()));
    }
    Ok(())
}

/// Stop automatic retries for a running job until an administrator explicitly requeues it.
pub async fn block_running_job(
    pool: &PgPool,
    job: &LifecycleJob,
    error_code: &str,
) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'failed', attempts = 20, locked_at = NULL, lease_token = NULL, \
             next_attempt_at = now(), last_error_code = left($3, 80), updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(error_code)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("lifecycle job lease was lost".into()));
    }
    Ok(())
}

//! Durable delivery for non-code identity email.
//!
//! Jobs persist an account id and a bounded template kind only. The worker resolves the current
//! encrypted-at-rest mailbox and renders content after claiming a lease, keeping recipient and body
//! plaintext outside PostgreSQL.

use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgConnection, PgPool};

const MAX_ATTEMPTS: i16 = 8;
const LEASE_MINUTES: i64 = 5;

#[derive(Clone, Copy)]
pub(crate) enum EmailDeliveryKind {
    PasswordSet,
    PasswordChanged,
    PasswordReset,
    AdminInvitation,
}

impl EmailDeliveryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::PasswordSet => "password_set",
            Self::PasswordChanged => "password_changed",
            Self::PasswordReset => "password_reset",
            Self::AdminInvitation => "admin_invitation",
        }
    }
}

#[derive(Debug, FromRow)]
struct EmailDeliveryJob {
    id: i64,
    account_id: i64,
    kind: String,
    attempts: i16,
    lease_token: uuid::Uuid,
}

pub(crate) async fn enqueue_tx(
    connection: &mut PgConnection,
    account_id: i64,
    kind: EmailDeliveryKind,
) -> AppResult<()> {
    sqlx::query("INSERT INTO identity.email_delivery_jobs (account_id, kind) VALUES ($1, $2)")
        .bind(account_id)
        .bind(kind.as_str())
        .execute(connection)
        .await?;
    Ok(())
}

async fn claim_due_job(pool: &PgPool) -> AppResult<Option<EmailDeliveryJob>> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE identity.email_delivery_jobs \
         SET status = CASE WHEN attempts >= $1 THEN 'dead' ELSE 'queued' END, \
             next_attempt_at = now(), locked_at = NULL, lease_token = NULL, \
             last_error_code = 'worker_lease_expired', updated_at = now() \
         WHERE status = 'running' AND locked_at < now() - ($2::bigint * interval '1 minute')",
    )
    .bind(MAX_ATTEMPTS)
    .bind(LEASE_MINUTES)
    .execute(&mut *tx)
    .await?;

    let candidate_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM identity.email_delivery_jobs \
         WHERE status = 'queued' AND attempts < $1 AND next_attempt_at <= now() \
         ORDER BY next_attempt_at, id FOR UPDATE SKIP LOCKED LIMIT 1",
    )
    .bind(MAX_ATTEMPTS)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(candidate_id) = candidate_id else {
        tx.commit().await?;
        return Ok(None);
    };
    let lease_token = uuid::Uuid::new_v4();
    let job = sqlx::query_as::<_, EmailDeliveryJob>(
        "UPDATE identity.email_delivery_jobs \
         SET status = 'running', attempts = attempts + 1, locked_at = now(), \
             lease_token = $2, last_error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'queued' \
         RETURNING id, account_id, kind, attempts, lease_token",
    )
    .bind(candidate_id)
    .bind(lease_token)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(job)
}

async fn complete_job(pool: &PgPool, job: &EmailDeliveryJob) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE identity.email_delivery_jobs \
         SET status = 'succeeded', accepted_at = now(), locked_at = NULL, lease_token = NULL, \
             last_error_code = NULL, updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("email delivery job lease was lost".into()));
    }
    Ok(())
}

async fn retry_job(
    pool: &PgPool,
    job: &EmailDeliveryJob,
    error_code: &'static str,
) -> AppResult<()> {
    let exponent = u32::try_from(job.attempts.clamp(0, 7)).unwrap_or(0);
    let next_attempt_at = Utc::now() + Duration::seconds((30_i64 * 2_i64.pow(exponent)).min(3600));
    let next_status = if job.attempts >= MAX_ATTEMPTS { "dead" } else { "queued" };
    let affected = sqlx::query(
        "UPDATE identity.email_delivery_jobs \
         SET status = $3, next_attempt_at = $4, locked_at = NULL, lease_token = NULL, \
             last_error_code = $5, updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(next_status)
    .bind(next_attempt_at)
    .bind(error_code)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("email delivery job lease was lost".into()));
    }
    Ok(())
}

async fn dead_letter_job(
    pool: &PgPool,
    job: &EmailDeliveryJob,
    error_code: &'static str,
) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE identity.email_delivery_jobs \
         SET status = 'dead', locked_at = NULL, lease_token = NULL, \
             last_error_code = $3, updated_at = now() \
         WHERE id = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(job.id)
    .bind(job.lease_token)
    .bind(error_code)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("email delivery job lease was lost".into()));
    }
    Ok(())
}

fn render_template(kind: &str) -> Option<crate::email_templates::EmailContent> {
    match kind {
        "password_set" => Some(crate::email_templates::password_set_notice()),
        "password_changed" => Some(crate::email_templates::password_changed_notice()),
        "password_reset" => Some(crate::email_templates::password_reset_notice()),
        "admin_invitation" => Some(crate::email_templates::community_invitation()),
        _ => None,
    }
}

/// Claim and process at most one due notification without holding a database lock during I/O.
pub async fn deliver_one_due_email(state: &AppState) -> AppResult<bool> {
    let Some(job) = claim_due_job(&state.db).await? else {
        return Ok(false);
    };
    let recipient_status: Result<Option<String>, sqlx::Error> =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
            .bind(job.account_id)
            .fetch_optional(&state.db)
            .await;
    match recipient_status {
        Ok(Some(status)) if status != "purged" => {}
        Ok(_) => {
            tracing::warn!(
                job_id = job.id,
                kind = %job.kind,
                attempt = job.attempts,
                error_code = "recipient_unavailable",
                "identity email delivery moved to dead letter"
            );
            dead_letter_job(&state.db, &job, "recipient_unavailable").await?;
            return Ok(true);
        }
        Err(_) => {
            tracing::warn!(
                job_id = job.id,
                kind = %job.kind,
                attempt = job.attempts,
                error_code = "identity_unavailable",
                "identity email delivery will retry"
            );
            retry_job(&state.db, &job, "identity_unavailable").await?;
            return Ok(true);
        }
    }
    let account = match crate::repo::find_account_by_id(
        &state.db,
        state.email_encryption.as_ref(),
        job.account_id,
    )
    .await
    {
        Ok(Some(account)) => account,
        Ok(_) => {
            tracing::warn!(
                job_id = job.id,
                kind = %job.kind,
                attempt = job.attempts,
                error_code = "recipient_unavailable",
                "identity email delivery moved to dead letter"
            );
            dead_letter_job(&state.db, &job, "recipient_unavailable").await?;
            return Ok(true);
        }
        Err(_) => {
            tracing::warn!(
                job_id = job.id,
                kind = %job.kind,
                attempt = job.attempts,
                error_code = "identity_unavailable",
                "identity email delivery will retry"
            );
            retry_job(&state.db, &job, "identity_unavailable").await?;
            return Ok(true);
        }
    };
    let Some(content) = render_template(&job.kind) else {
        tracing::warn!(
            job_id = job.id,
            kind = %job.kind,
            attempt = job.attempts,
            error_code = "template_unavailable",
            "identity email delivery moved to dead letter"
        );
        dead_letter_job(&state.db, &job, "template_unavailable").await?;
        return Ok(true);
    };
    match shared::email::send_email(
        &state.config,
        &account.email,
        content.subject,
        &content.text,
        Some(&content.html),
    )
    .await
    {
        Ok(()) => complete_job(&state.db, &job).await?,
        Err(_) => {
            tracing::warn!(
                job_id = job.id,
                kind = %job.kind,
                attempt = job.attempts,
                error_code = "provider_unavailable",
                "identity email delivery will retry"
            );
            retry_job(&state.db, &job, "provider_unavailable").await?;
        }
    }
    Ok(true)
}

/// Purge terminal delivery metadata and expired high-value security facts.
pub async fn purge_expired_email_delivery_data(pool: &PgPool) -> AppResult<u64> {
    let jobs = sqlx::query(
        "DELETE FROM identity.email_delivery_jobs \
         WHERE (status = 'succeeded' AND updated_at < now() - interval '30 days') \
            OR (status = 'dead' AND updated_at < now() - interval '90 days')",
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(jobs + crate::security_events::purge_expired(pool).await?)
}

/// Run the durable identity-email worker until process shutdown.
pub async fn run_email_delivery_worker(state: AppState) {
    let mut next_retention = Utc::now();
    loop {
        if Utc::now() >= next_retention {
            if let Err(error) = purge_expired_email_delivery_data(&state.db).await {
                tracing::warn!(?error, "identity email retention failed");
            }
            next_retention = Utc::now() + Duration::hours(1);
        }
        match deliver_one_due_email(&state).await {
            Ok(true) => continue,
            Ok(false) => tokio::time::sleep(StdDuration::from_millis(500)).await,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    error_code = "worker_iteration_failed",
                    "identity email delivery worker failed"
                );
                tokio::time::sleep(StdDuration::from_secs(1)).await;
            }
        }
    }
}

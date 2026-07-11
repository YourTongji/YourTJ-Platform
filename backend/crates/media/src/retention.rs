//! Purpose-limited administrative holds that pause provider deletion.

use std::time::Duration as StdDuration;

use base64::Engine;
use chrono::{Duration, Utc};
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgConnection, PgPool};

use crate::dto::{RetentionHoldDto, RetentionHoldInput};

const MIN_HOLD_MINUTES: i64 = 5;
const MAX_HOLD_DAYS: i64 = 365;
const HOLD_HISTORY_DAYS: i64 = 365;
const CONSUMED_INTENT_HISTORY_DAYS: i64 = 30;
const PREVIEW_GRANT_HISTORY_DAYS: i64 = 1;
const UPLOAD_CREDENTIAL_ATTEMPT_HISTORY_DAYS: i64 = 2;
const CLEANUP_TOMBSTONE_HISTORY_DAYS: i64 = 30;
const HOUSEKEEPING_INTERVAL_SECONDS: u64 = 60 * 60;

#[derive(Debug, FromRow)]
struct RetentionHoldRow {
    id: i64,
    upload_id: i64,
    account_id: i64,
    upload_status: String,
    hold_kind: String,
    reason: String,
    placed_by: i64,
    expires_at: chrono::DateTime<Utc>,
    created_at: chrono::DateTime<Utc>,
}

fn encode_cursor(expires_at: chrono::DateTime<Utc>, id: i64) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(format!("{}:{id}", expires_at.timestamp_micros()))
}

fn decode_cursor(cursor: &str) -> AppResult<(chrono::DateTime<Utc>, i64)> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| AppError::BadRequest("invalid retention hold cursor".into()))?;
    let decoded = String::from_utf8(bytes)
        .map_err(|_| AppError::BadRequest("invalid retention hold cursor".into()))?;
    let (timestamp, id) = decoded
        .rsplit_once(':')
        .ok_or_else(|| AppError::BadRequest("invalid retention hold cursor".into()))?;
    let timestamp = timestamp
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid retention hold cursor".into()))?;
    let id = id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid retention hold cursor".into()))?;
    let expires_at = chrono::DateTime::from_timestamp_micros(timestamp)
        .ok_or_else(|| AppError::BadRequest("invalid retention hold cursor".into()))?;
    Ok((expires_at, id))
}

/// List unreleased holds by expiry for the operations inventory.
pub async fn list_holds(
    connection: &mut PgConnection,
    state: &str,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<Page<RetentionHoldDto>> {
    if !matches!(state, "active" | "expired") {
        return Err(AppError::BadRequest("invalid retention hold state".into()));
    }
    let (expires_at_bound, id_bound) = cursor.map(decode_cursor).transpose()?.unzip();
    let rows = sqlx::query_as::<_, RetentionHoldRow>(
        "SELECT hold.id, hold.asset_id AS upload_id, upload.account_id, \
                upload.status AS upload_status, hold.hold_kind, hold.reason, hold.placed_by, \
                hold.expires_at, hold.created_at \
         FROM media.asset_retention_holds hold \
         JOIN media.uploads upload ON upload.id = hold.asset_id \
         WHERE hold.released_at IS NULL \
           AND (($1 = 'active' AND hold.expires_at > now()) \
                OR ($1 = 'expired' AND hold.expires_at <= now())) \
           AND ($2::timestamptz IS NULL OR hold.expires_at > $2 \
                OR (hold.expires_at = $2 AND hold.id > $3::bigint)) \
         ORDER BY hold.expires_at, hold.id LIMIT $4",
    )
    .bind(state)
    .bind(expires_at_bound)
    .bind(id_bound)
    .bind(limit + 1)
    .fetch_all(connection)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let visible = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = if has_more {
        visible.last().map(|row| encode_cursor(row.expires_at, row.id))
    } else {
        None
    };
    let now = Utc::now();
    let items = visible
        .into_iter()
        .map(|row| RetentionHoldDto {
            id: row.id.to_string(),
            upload_id: row.upload_id.to_string(),
            account_id: row.account_id.to_string(),
            upload_status: row.upload_status,
            hold_kind: row.hold_kind,
            reason: row.reason,
            placed_by: row.placed_by.to_string(),
            expires_at: row.expires_at.timestamp(),
            created_at: row.created_at.timestamp(),
            is_expired: row.expires_at <= now,
        })
        .collect();
    Ok(Page::new(items, next_cursor))
}

/// Delete exact operational purpose/retry text and completed work records after bounded windows.
pub async fn purge_media_operations_history(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let limit = limit.clamp(1, 500);
    let mut transaction = pool.begin().await?;
    let deleted_holds = sqlx::query_scalar::<_, i64>(
        "WITH candidates AS ( \
           SELECT id FROM media.asset_retention_holds \
           WHERE COALESCE(released_at, expires_at) <= \
                 now() - ($1 * interval '1 day') \
           ORDER BY COALESCE(released_at, expires_at), id \
           FOR UPDATE SKIP LOCKED LIMIT $2 \
         ), removed AS ( \
           DELETE FROM media.asset_retention_holds hold USING candidates \
           WHERE hold.id = candidates.id RETURNING hold.id \
         ) SELECT count(*)::bigint FROM removed",
    )
    .bind(HOLD_HISTORY_DAYS)
    .bind(limit)
    .fetch_one(&mut *transaction)
    .await?;
    let remaining_limit = limit.saturating_sub(deleted_holds);
    let deleted_retry_events = if remaining_limit > 0 {
        sqlx::query_scalar::<_, i64>(
            "WITH candidates AS ( \
               SELECT id FROM media.object_deletion_job_retry_events \
               WHERE created_at <= now() - ($1 * interval '1 day') \
               ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT $2 \
             ), removed AS ( \
               DELETE FROM media.object_deletion_job_retry_events event USING candidates \
               WHERE event.id = candidates.id RETURNING event.id \
             ) SELECT count(*)::bigint FROM removed",
        )
        .bind(HOLD_HISTORY_DAYS)
        .bind(remaining_limit)
        .fetch_one(&mut *transaction)
        .await?
    } else {
        0
    };
    let remaining_limit = remaining_limit.saturating_sub(deleted_retry_events);
    let deleted_jobs = if remaining_limit > 0 {
        sqlx::query_scalar::<_, i64>(
            "WITH candidates AS ( \
               SELECT id FROM media.object_deletion_jobs \
               WHERE status = 'succeeded' \
                 AND completed_at <= now() - ($1 * interval '1 day') \
               ORDER BY completed_at, id FOR UPDATE SKIP LOCKED LIMIT $2 \
             ), removed AS ( \
               DELETE FROM media.object_deletion_jobs job USING candidates \
               WHERE job.id = candidates.id RETURNING job.id \
             ) SELECT count(*)::bigint FROM removed",
        )
        .bind(HOLD_HISTORY_DAYS)
        .bind(remaining_limit)
        .fetch_one(&mut *transaction)
        .await?
    } else {
        0
    };
    let remaining_limit = remaining_limit.saturating_sub(deleted_jobs);
    let deleted_evidence = if remaining_limit > 0 {
        sqlx::query_scalar::<_, i64>(
            "WITH candidates AS ( \
               SELECT evidence.id FROM media.moderation_evidence evidence \
               JOIN media.uploads upload ON upload.id = evidence.upload_id \
               WHERE upload.redacted_at IS NOT NULL \
                 AND evidence.created_at <= now() - ($1 * interval '1 day') \
               ORDER BY evidence.created_at, evidence.id \
               FOR UPDATE OF evidence SKIP LOCKED LIMIT $2 \
             ), removed AS ( \
               DELETE FROM media.moderation_evidence evidence USING candidates \
               WHERE evidence.id = candidates.id RETURNING evidence.id \
             ) SELECT count(*)::bigint FROM removed",
        )
        .bind(HOLD_HISTORY_DAYS)
        .bind(remaining_limit)
        .fetch_one(&mut *transaction)
        .await?
    } else {
        0
    };
    transaction.commit().await?;
    let deleted = deleted_holds
        .saturating_add(deleted_retry_events)
        .saturating_add(deleted_jobs)
        .saturating_add(deleted_evidence);
    Ok(u64::try_from(deleted).unwrap_or_default())
}

async fn purge_consumed_upload_intents(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "WITH candidates AS ( \
           SELECT id FROM media.upload_intents \
           WHERE revoked_at IS NULL AND upload_id IS NOT NULL \
             AND consumed_at <= now() - ($1 * interval '1 day') \
           ORDER BY consumed_at, id FOR UPDATE SKIP LOCKED LIMIT $2 \
         ), removed AS ( \
           DELETE FROM media.upload_intents intent USING candidates \
           WHERE intent.id = candidates.id RETURNING intent.id \
         ) SELECT count(*)::bigint FROM removed",
    )
    .bind(CONSUMED_INTENT_HISTORY_DAYS)
    .bind(limit.clamp(1, 500))
    .fetch_one(pool)
    .await?;
    Ok(u64::try_from(deleted).unwrap_or_default())
}

/// Purge expired one-time preview credentials after their bounded audit window.
pub async fn purge_expired_preview_grants(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "WITH candidates AS ( \
           SELECT id FROM media.moderation_preview_grants \
           WHERE expires_at <= now() - ($1 * interval '1 day') \
           ORDER BY expires_at, id FOR UPDATE SKIP LOCKED LIMIT $2 \
         ), removed AS ( \
           DELETE FROM media.moderation_preview_grants preview_grant USING candidates \
           WHERE preview_grant.id = candidates.id RETURNING preview_grant.id \
         ) SELECT count(*)::bigint FROM removed",
    )
    .bind(PREVIEW_GRANT_HISTORY_DAYS)
    .bind(limit.clamp(1, 500))
    .fetch_one(pool)
    .await?;
    Ok(u64::try_from(deleted).unwrap_or_default())
}

/// Remove detached binding facts once their only purpose, GC grace, has elapsed.
pub async fn purge_expired_asset_bindings(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "WITH candidates AS ( \
           SELECT id FROM media.asset_bindings \
           WHERE detached_at IS NOT NULL AND gc_eligible_at <= now() \
           ORDER BY gc_eligible_at, id FOR UPDATE SKIP LOCKED LIMIT $1 \
         ), removed AS ( \
           DELETE FROM media.asset_bindings binding USING candidates \
           WHERE binding.id = candidates.id RETURNING binding.id \
         ) SELECT count(*)::bigint FROM removed",
    )
    .bind(limit.clamp(1, 500))
    .fetch_one(pool)
    .await?;
    Ok(u64::try_from(deleted).unwrap_or_default())
}

/// Purge credential-attempt counters after the rolling quota can no longer consult them.
pub async fn purge_upload_credential_attempts(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "WITH candidates AS ( \
           SELECT id FROM media.upload_credential_attempts \
           WHERE created_at <= now() - ($1 * interval '1 day') \
           ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT $2 \
         ), removed AS ( \
           DELETE FROM media.upload_credential_attempts attempt USING candidates \
           WHERE attempt.id = candidates.id RETURNING attempt.id \
         ) SELECT count(*)::bigint FROM removed",
    )
    .bind(UPLOAD_CREDENTIAL_ATTEMPT_HISTORY_DAYS)
    .bind(limit.clamp(1, 500))
    .fetch_one(pool)
    .await?;
    Ok(u64::try_from(deleted).unwrap_or_default())
}

/// Purge redacted internal cleanup rows after their reconciliation window and hold history end.
pub async fn purge_completed_cleanup_tombstones(pool: &PgPool, limit: i64) -> AppResult<u64> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "WITH candidates AS ( \
           SELECT upload.id FROM media.uploads upload \
           WHERE upload.is_cleanup_tombstone AND upload.status = 'blocked' \
             AND upload.redacted_at <= now() - ($1 * interval '1 day') \
             AND NOT EXISTS ( \
               SELECT 1 FROM media.object_deletion_jobs job \
               WHERE job.upload_id = upload.id AND job.status <> 'succeeded' \
             ) \
             AND NOT EXISTS ( \
               SELECT 1 FROM media.object_deletion_jobs job \
               JOIN media.object_deletion_job_retry_events retry ON retry.job_id = job.id \
               WHERE job.upload_id = upload.id \
             ) \
             AND NOT EXISTS (SELECT 1 FROM media.asset_retention_holds hold \
                             WHERE hold.asset_id = upload.id) \
             AND NOT EXISTS (SELECT 1 FROM media.asset_usages usage \
                             WHERE usage.asset_id = upload.id) \
             AND NOT EXISTS (SELECT 1 FROM media.asset_bindings binding \
                             WHERE binding.asset_id = upload.id) \
             AND NOT EXISTS (SELECT 1 FROM media.draft_asset_references draft_reference \
                             WHERE draft_reference.asset_id = upload.id) \
             AND NOT EXISTS (SELECT 1 FROM media.upload_intents intent \
                             WHERE intent.upload_id = upload.id) \
           ORDER BY upload.redacted_at, upload.id \
           FOR UPDATE OF upload SKIP LOCKED LIMIT $2 \
         ), removed AS ( \
           DELETE FROM media.uploads upload USING candidates \
           WHERE upload.id = candidates.id RETURNING upload.id \
         ) SELECT count(*)::bigint FROM removed",
    )
    .bind(CLEANUP_TOMBSTONE_HISTORY_DAYS)
    .bind(limit.clamp(1, 500))
    .fetch_one(pool)
    .await?;
    Ok(u64::try_from(deleted).unwrap_or_default())
}

/// Run bounded retention-metadata cleanup independently from object-GC rollout activation.
pub async fn run_retention_housekeeping_worker(state: AppState) {
    loop {
        let mut did_work = false;
        match crate::gc::schedule_expired_upload_intent_cleanup_batch(&state.db, 100).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "expired upload intent cleanup queued");
                did_work = true;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, "expired upload intent cleanup scheduling failed");
            }
        }
        match purge_consumed_upload_intents(&state.db, 200).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "consumed upload intent credentials purged");
                did_work = true;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, "consumed upload intent housekeeping failed");
            }
        }
        match purge_expired_preview_grants(&state.db, 200).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "expired moderation preview grants purged");
                did_work = true;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, "moderation preview grant housekeeping failed");
            }
        }
        match purge_expired_asset_bindings(&state.db, 200).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "expired media binding facts purged");
                did_work = true;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, "media binding housekeeping failed");
            }
        }
        match purge_upload_credential_attempts(&state.db, 200).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "expired media credential quota facts purged");
                did_work = true;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, "media credential quota housekeeping failed");
            }
        }
        match purge_completed_cleanup_tombstones(&state.db, 200).await {
            Ok(count) if count > 0 => {
                tracing::info!(count, "expired upload cleanup tombstones purged");
                did_work = true;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, "upload cleanup tombstone housekeeping failed");
            }
        }
        if state.config.media_operations_history_purge_enabled {
            match purge_media_operations_history(&state.db, 200).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "expired media operations history purged");
                    did_work = true;
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(?error, "media operations history housekeeping failed");
                }
            }
        }
        if !did_work {
            tokio::time::sleep(StdDuration::from_secs(HOUSEKEEPING_INTERVAL_SECONDS)).await;
        }
    }
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let trimmed = reason.trim();
    if trimmed != reason || !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be trimmed and 3-500 characters".into()));
    }
    Ok(reason)
}

fn parse_hold_id(id: &str) -> AppResult<i64> {
    id.parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::BadRequest("expectedHoldId is invalid".into()))
}

/// Place one hold only while no provider deletion lease can be in flight.
pub async fn place_hold(
    pool: &PgPool,
    auth_context: &identity::auth_middleware::AuthenticatedContext,
    upload_id: i64,
    input: RetentionHoldInput,
) -> AppResult<()> {
    let reason = validate_reason(&input.reason)?;
    let expires_at = chrono::DateTime::from_timestamp(input.expires_at, 0)
        .ok_or_else(|| AppError::BadRequest("expiresAt is invalid".into()))?;
    let now = Utc::now();
    if expires_at < now + Duration::minutes(MIN_HOLD_MINUTES)
        || expires_at > now + Duration::days(MAX_HOLD_DAYS)
    {
        return Err(AppError::BadRequest(
            "expiresAt must be between 5 minutes and 365 days from now".into(),
        ));
    }

    let mut transaction = pool.begin().await?;
    identity::auth_middleware::require_recent_auth_tx(auth_context, &mut transaction).await?;
    let upload_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(upload_id)
            .fetch_optional(&mut *transaction)
            .await?;
    let upload_status = upload_status.ok_or(AppError::NotFound)?;
    if upload_status == "blocked" {
        return Err(AppError::Conflict("deleted media cannot be retained".into()));
    }
    let deletion_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM media.object_deletion_jobs WHERE upload_id = $1 FOR UPDATE",
    )
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    if deletion_status.as_deref().is_some_and(|status| matches!(status, "leased" | "succeeded")) {
        return Err(AppError::Conflict("provider deletion is already in progress".into()));
    }

    let prior_hold: Option<(i64, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, expires_at FROM media.asset_retention_holds \
         WHERE asset_id = $1 AND released_at IS NULL FOR UPDATE",
    )
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let expected_hold_id = input.expected_hold_id.as_deref().map(parse_hold_id).transpose()?;
    if prior_hold.as_ref().map(|(hold_id, _)| *hold_id) != expected_hold_id {
        return Err(AppError::Conflict("retention hold changed; reload before updating".into()));
    }
    if let Some((hold_id, prior_expiry)) = prior_hold {
        sqlx::query(
            "UPDATE media.asset_retention_holds \
             SET released_at = now(), released_by = $2, \
                 release_reason = 'superseded by a new authorized hold' \
             WHERE id = $1 AND released_at IS NULL",
        )
        .bind(hold_id)
        .bind(auth_context.account.id)
        .execute(&mut *transaction)
        .await?;
        governance::record_account_event_tx(
            &mut transaction,
            governance::AccountActor {
                account_id: auth_context.account.id,
                role: &auth_context.account.role,
            },
            "media.upload.retention_hold_superseded",
            "upload",
            &upload_id.to_string(),
            "authorized media retention hold superseded",
            Some(&serde_json::json!({ "scheduledExpiryAt": prior_expiry.timestamp() })),
        )
        .await?;
    }

    sqlx::query(
        "INSERT INTO media.asset_retention_holds \
         (asset_id, hold_kind, reason, placed_by, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(upload_id)
    .bind(input.hold_kind.as_str())
    .bind(reason)
    .bind(auth_context.account.id)
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        "media.upload.retention_hold_placed",
        "upload",
        &upload_id.to_string(),
        "authorized media retention hold placed",
        Some(&serde_json::json!({
            "expiresAt": expires_at.timestamp(),
        })),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// Release the current hold and allow an existing queued deletion job to resume.
pub async fn release_hold(
    pool: &PgPool,
    auth_context: &identity::auth_middleware::AuthenticatedContext,
    upload_id: i64,
    expected_hold_id: &str,
    reason: &str,
) -> AppResult<()> {
    let reason = validate_reason(reason)?;
    let expected_hold_id = parse_hold_id(expected_hold_id)?;
    let mut transaction = pool.begin().await?;
    identity::auth_middleware::require_recent_auth_tx(auth_context, &mut transaction).await?;
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT id FROM media.uploads WHERE id = $1 FOR UPDATE")
            .bind(upload_id)
            .fetch_optional(&mut *transaction)
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }
    let hold: Option<(i64, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, expires_at FROM media.asset_retention_holds \
         WHERE asset_id = $1 AND released_at IS NULL FOR UPDATE",
    )
    .bind(upload_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let (hold_id, expires_at) =
        hold.ok_or_else(|| AppError::Conflict("upload has no unreleased retention hold".into()))?;
    if hold_id != expected_hold_id {
        return Err(AppError::Conflict("retention hold changed; reload before releasing".into()));
    }
    sqlx::query(
        "UPDATE media.asset_retention_holds \
         SET released_at = now(), released_by = $2, release_reason = $3 \
         WHERE id = $1 AND released_at IS NULL",
    )
    .bind(hold_id)
    .bind(auth_context.account.id)
    .bind(reason)
    .execute(&mut *transaction)
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor {
            account_id: auth_context.account.id,
            role: &auth_context.account.role,
        },
        "media.upload.retention_hold_released",
        "upload",
        &upload_id.to_string(),
        "authorized media retention hold released",
        Some(&serde_json::json!({
            "scheduledExpiryAt": expires_at.timestamp(),
        })),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

//! Durable cross-domain side-effect outbox.
//!
//! Producers append an event in the same PostgreSQL transaction as their business fact. Workers
//! claim with a lease and `SKIP LOCKED`; consumers must complete the event in the transaction that
//! records their idempotent delivery receipt.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, Page};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::auth::staff_account;
use crate::validation::{parse_id, reason};

const DEFAULT_MAX_ATTEMPTS: i16 = 8;
const MAX_PAYLOAD_BYTES: usize = 8 * 1024;

/// One claimed outbox event. Its payload is internal and must never be exposed by admin DTOs.
#[derive(Debug, Clone, FromRow)]
pub struct OutboxEvent {
    pub id: i64,
    pub topic: String,
    pub source_key: String,
    pub recipient_account_id: i64,
    pub actor_account_id: Option<i64>,
    pub event_type: String,
    pub payload: Value,
    pub aggregation_key: Option<String>,
    pub attempts: i16,
    pub max_attempts: i16,
    pub available_at: DateTime<Utc>,
    pub claimed_by: Uuid,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct OutboxAdminRecord {
    id: i64,
    topic: String,
    recipient_account_id: i64,
    event_type: String,
    state: String,
    attempts: i16,
    max_attempts: i16,
    manual_retry_count: i16,
    available_at: DateTime<Utc>,
    last_error_code: Option<String>,
    completed_at: Option<DateTime<Utc>>,
    dead_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutboxAdminDto {
    id: String,
    topic: String,
    recipient_account_id: String,
    event_type: String,
    state: String,
    attempts: i16,
    max_attempts: i16,
    manual_retry_count: i16,
    available_at: i64,
    last_error_code: Option<String>,
    completed_at: Option<i64>,
    dead_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutboxListQuery {
    state: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutboxRetryInput {
    reason: String,
}

#[derive(Debug, FromRow)]
struct ExistingEvent {
    topic: String,
    recipient_account_id: i64,
    actor_account_id: Option<i64>,
    event_type: String,
    payload: Value,
    aggregation_key: Option<String>,
    available_at: DateTime<Utc>,
}

fn admin_dto(record: OutboxAdminRecord) -> OutboxAdminDto {
    OutboxAdminDto {
        id: record.id.to_string(),
        topic: record.topic,
        recipient_account_id: record.recipient_account_id.to_string(),
        event_type: record.event_type,
        state: record.state,
        attempts: record.attempts,
        max_attempts: record.max_attempts,
        manual_retry_count: record.manual_retry_count,
        available_at: record.available_at.timestamp(),
        last_error_code: record.last_error_code,
        completed_at: record.completed_at.map(|value| value.timestamp()),
        dead_at: record.dead_at.map(|value| value.timestamp()),
        created_at: record.created_at.timestamp(),
        updated_at: record.updated_at.timestamp(),
    }
}

fn validate_event_fields(
    topic: &str,
    source_key: &str,
    event_type: &str,
    payload: &Value,
    aggregation_key: Option<&str>,
) -> AppResult<()> {
    if !matches!(topic, "notification" | "achievement_award") {
        return Err(AppError::BadRequest("invalid outbox topic".into()));
    }
    if source_key.is_empty() || source_key.len() > 200 {
        return Err(AppError::BadRequest("invalid outbox source key".into()));
    }
    if event_type.is_empty() || event_type.len() > 80 {
        return Err(AppError::BadRequest("invalid outbox event type".into()));
    }
    if !payload.is_object()
        || serde_json::to_vec(payload).map_err(|error| AppError::Internal(error.into()))?.len()
            > MAX_PAYLOAD_BYTES
    {
        return Err(AppError::BadRequest("outbox payload must be a bounded object".into()));
    }
    if aggregation_key.is_some_and(|value| value.is_empty() || value.len() > 160) {
        return Err(AppError::BadRequest("invalid outbox aggregation key".into()));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // reason: idempotency comparison requires every durable event field to remain explicit
async fn enqueue_tx(
    connection: &mut PgConnection,
    topic: &str,
    source_key: &str,
    recipient_account_id: i64,
    actor_account_id: Option<i64>,
    event_type: &str,
    payload: &Value,
    aggregation_key: Option<&str>,
    available_at: DateTime<Utc>,
    require_schedule_match: bool,
) -> AppResult<i64> {
    validate_event_fields(topic, source_key, event_type, payload, aggregation_key)?;
    let inserted_id: Option<i64> = sqlx::query_scalar(
        "INSERT INTO platform.outbox_events \
         (topic, source_key, recipient_account_id, actor_account_id, event_type, payload, \
          aggregation_key, available_at, max_attempts) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (source_key) DO NOTHING RETURNING id",
    )
    .bind(topic)
    .bind(source_key)
    .bind(recipient_account_id)
    .bind(actor_account_id)
    .bind(event_type)
    .bind(payload)
    .bind(aggregation_key)
    .bind(available_at)
    .bind(DEFAULT_MAX_ATTEMPTS)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(event_id) = inserted_id {
        return Ok(event_id);
    }

    let existing = sqlx::query_as::<_, ExistingEvent>(
        "SELECT topic, recipient_account_id, actor_account_id, event_type, payload, \
                aggregation_key, available_at \
         FROM platform.outbox_events WHERE source_key = $1 FOR SHARE",
    )
    .bind(source_key)
    .fetch_one(&mut *connection)
    .await?;
    if existing.topic != topic
        || existing.recipient_account_id != recipient_account_id
        || existing.actor_account_id != actor_account_id
        || existing.event_type != event_type
        || existing.payload != *payload
        || existing.aggregation_key.as_deref() != aggregation_key
        || (require_schedule_match && existing.available_at != available_at)
    {
        return Err(AppError::Conflict("outbox source key was reused with different data".into()));
    }
    let event_id =
        sqlx::query_scalar("SELECT id FROM platform.outbox_events WHERE source_key = $1")
            .bind(source_key)
            .fetch_one(&mut *connection)
            .await?;
    Ok(event_id)
}

/// Enqueue one user notification in the caller's transaction.
#[allow(clippy::too_many_arguments)] // reason: notification policy fields must remain explicit at the producer boundary
pub async fn enqueue_notification_tx(
    connection: &mut PgConnection,
    source_key: &str,
    recipient_account_id: i64,
    actor_account_id: Option<i64>,
    event_type: &str,
    payload: &Value,
    aggregation_key: Option<&str>,
    available_at: Option<DateTime<Utc>>,
) -> AppResult<i64> {
    let require_schedule_match = available_at.is_some();
    enqueue_tx(
        connection,
        "notification",
        source_key,
        recipient_account_id,
        actor_account_id,
        event_type,
        payload,
        aggregation_key,
        available_at.unwrap_or_else(Utc::now),
        require_schedule_match,
    )
    .await
}

/// Enqueue an automatic achievement award from a committed contribution fact.
pub async fn enqueue_achievement_award_tx(
    connection: &mut PgConnection,
    source_key: &str,
    account_id: i64,
    awarded_by: i64,
    slug: &str,
    award_reason: &str,
) -> AppResult<i64> {
    enqueue_tx(
        connection,
        "achievement_award",
        source_key,
        account_id,
        Some(awarded_by),
        slug,
        &serde_json::json!({ "awardReason": award_reason }),
        None,
        Utc::now(),
        false,
    )
    .await
}

/// Cancel a still-queued scheduled event in the caller's transaction.
pub async fn cancel_queued_event_tx(
    connection: &mut PgConnection,
    source_key: &str,
) -> AppResult<bool> {
    let changed = sqlx::query(
        "UPDATE platform.outbox_events \
         SET state = 'cancelled', completed_at = now(), updated_at = now(), \
             claimed_by = NULL, lease_expires_at = NULL \
         WHERE source_key = $1 AND state = 'queued'",
    )
    .bind(source_key)
    .execute(connection)
    .await?;
    Ok(changed.rows_affected() == 1)
}

/// Claim due or expired-lease events for one worker without blocking other instances.
pub async fn claim_events(
    pool: &PgPool,
    worker_id: Uuid,
    limit: i64,
) -> AppResult<Vec<OutboxEvent>> {
    let bounded_limit = limit.clamp(1, 100);
    let events = sqlx::query_as::<_, OutboxEvent>(
        "WITH candidates AS ( \
           SELECT id FROM platform.outbox_events \
           WHERE (state = 'queued' AND attempts < max_attempts AND available_at <= now()) \
              OR (state = 'running' AND lease_expires_at <= now()) \
           ORDER BY available_at, id \
           FOR UPDATE SKIP LOCKED LIMIT $1 \
         ) \
         UPDATE platform.outbox_events AS event \
         SET state = 'running', attempts = LEAST(event.attempts + 1, event.max_attempts), \
             claimed_by = $2, \
             lease_expires_at = now() + interval '30 seconds', updated_at = now(), \
             last_error_code = NULL \
         FROM candidates WHERE event.id = candidates.id \
         RETURNING event.id, event.topic, event.source_key, event.recipient_account_id, \
                   event.actor_account_id, event.event_type, event.payload, \
                   event.aggregation_key, event.attempts, event.max_attempts, \
                   event.available_at, event.claimed_by, event.lease_expires_at",
    )
    .bind(bounded_limit)
    .bind(worker_id)
    .fetch_all(pool)
    .await?;
    Ok(events)
}

/// Fence a consumer transaction to the worker that currently owns the row lease.
pub async fn lock_claim_tx(
    connection: &mut PgConnection,
    event_id: i64,
    worker_id: Uuid,
) -> AppResult<bool> {
    let owns_claim: Option<bool> = sqlx::query_scalar(
        "SELECT claimed_by = $2 FROM platform.outbox_events \
         WHERE id = $1 AND state = 'running' FOR UPDATE",
    )
    .bind(event_id)
    .bind(worker_id)
    .fetch_optional(connection)
    .await?;
    Ok(owns_claim.unwrap_or(false))
}

/// Complete a claimed event inside the consumer's idempotent delivery transaction.
pub async fn mark_succeeded_tx(
    connection: &mut PgConnection,
    event_id: i64,
    worker_id: Uuid,
) -> AppResult<bool> {
    let changed = sqlx::query(
        "UPDATE platform.outbox_events \
         SET state = 'succeeded', completed_at = now(), updated_at = now(), \
             claimed_by = NULL, lease_expires_at = NULL, last_error_code = NULL \
         WHERE id = $1 AND state = 'running' AND claimed_by = $2",
    )
    .bind(event_id)
    .bind(worker_id)
    .execute(connection)
    .await?;
    Ok(changed.rows_affected() == 1)
}

fn retry_delay_seconds(attempts: i16) -> i32 {
    let exponent = u32::from(attempts.saturating_sub(1).clamp(0, 9) as u16);
    (1_i32 << exponent).min(900)
}

/// Return a failed claim to the queue with bounded exponential backoff, or dead-letter it.
pub async fn record_failure(
    pool: &PgPool,
    event: &OutboxEvent,
    error_code: &str,
) -> AppResult<Option<String>> {
    let error_code =
        if error_code.is_empty() || error_code.len() > 80 { "delivery_failed" } else { error_code };
    let state = sqlx::query_scalar(
        "UPDATE platform.outbox_events \
         SET state = CASE WHEN attempts >= max_attempts THEN 'dead' ELSE 'queued' END, \
             available_at = CASE WHEN attempts >= max_attempts THEN available_at \
                                 ELSE now() + make_interval(secs => $4::int) END, \
             dead_at = CASE WHEN attempts >= max_attempts THEN now() ELSE NULL END, \
             claimed_by = NULL, lease_expires_at = NULL, last_error_code = $3, updated_at = now() \
         WHERE id = $1 AND state = 'running' AND claimed_by = $2 \
         RETURNING state",
    )
    .bind(event.id)
    .bind(event.claimed_by)
    .bind(error_code)
    .bind(retry_delay_seconds(event.attempts))
    .fetch_optional(pool)
    .await?;
    Ok(state)
}

/// Delete terminal integration records after their documented operational retention windows.
pub async fn purge_terminal_events(pool: &PgPool) -> AppResult<u64> {
    let removed = sqlx::query(
        "DELETE FROM platform.outbox_events \
         WHERE (state IN ('succeeded', 'cancelled') \
                AND completed_at < now() - interval '30 days') \
            OR (state = 'dead' AND dead_at < now() - interval '90 days')",
    )
    .execute(pool)
    .await?;
    Ok(removed.rows_affected())
}

async fn admin_list_outbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OutboxListQuery>,
) -> AppResult<Json<Page<OutboxAdminDto>>> {
    staff_account(&headers, &state, Capability::RunOperations).await?;
    let state_filter = query.state.as_deref().unwrap_or("dead");
    if !matches!(state_filter, "queued" | "running" | "succeeded" | "dead" | "cancelled") {
        return Err(AppError::BadRequest("invalid outbox state".into()));
    }
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30);
    if !(1..=100).contains(&limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let records = sqlx::query_as::<_, OutboxAdminRecord>(
        "SELECT id, topic, recipient_account_id, event_type, state, attempts, max_attempts, \
                manual_retry_count, available_at, last_error_code, completed_at, dead_at, \
                created_at, updated_at \
         FROM platform.outbox_events \
         WHERE state = $1 AND ($2::bigint IS NULL OR id < $2) \
         ORDER BY id DESC LIMIT $3",
    )
    .bind(state_filter)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = records.len() > limit as usize;
    let visible = records.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor =
        has_more.then(|| visible.last().map(|record| record.id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(admin_dto).collect(), next_cursor)))
}

async fn admin_retry_outbox(
    State(state): State<AppState>,
    Path(event_id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<OutboxRetryInput>,
) -> AppResult<Json<OutboxAdminDto>> {
    let actor = staff_account(&headers, &state, Capability::RunOperations).await?;
    let event_id = parse_id(&event_id, "outbox event id")?;
    let retry_reason = reason(&input.reason)?;
    let mut tx = state.db.begin().await?;
    let current: (String, String, i64) = sqlx::query_as(
        "SELECT topic, event_type, recipient_account_id \
         FROM platform.outbox_events WHERE id = $1 AND state = 'dead' FOR UPDATE",
    )
    .bind(event_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("only dead-letter events can be retried".into()))?;
    let record = sqlx::query_as::<_, OutboxAdminRecord>(
        "UPDATE platform.outbox_events \
         SET state = 'queued', attempts = 0, manual_retry_count = manual_retry_count + 1, \
             available_at = now(), claimed_by = NULL, lease_expires_at = NULL, \
             last_error_code = NULL, completed_at = NULL, dead_at = NULL, updated_at = now() \
         WHERE id = $1 \
         RETURNING id, topic, recipient_account_id, event_type, state, attempts, max_attempts, \
                   manual_retry_count, available_at, last_error_code, completed_at, dead_at, \
                   created_at, updated_at",
    )
    .bind(event_id)
    .fetch_one(&mut *tx)
    .await?;
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.outbox.retried",
        "outbox_event",
        &event_id.to_string(),
        retry_reason,
        Some(&serde_json::json!({
            "topic": current.0,
            "eventType": current.1,
            "recipientAccountId": current.2.to_string(),
        })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(admin_dto(record)))
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v2/admin/notification-outbox", get(admin_list_outbox))
        .route("/api/v2/admin/notification-outbox/{event_id}/retry", post(admin_retry_outbox))
}

#[cfg(test)]
mod tests {
    use super::retry_delay_seconds;

    #[test]
    fn retry_backoff_is_bounded() {
        assert_eq!(retry_delay_seconds(1), 1);
        assert_eq!(retry_delay_seconds(4), 8);
        assert_eq!(retry_delay_seconds(20), 512);
    }
}

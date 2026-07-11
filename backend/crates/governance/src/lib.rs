//! Cross-domain, append-only audit records for privileged platform actions.
//!
//! Domain mutations call the transaction-aware writer so the protected change
//! and its audit record either both commit or both roll back. Audit metadata
//! must never contain secrets, raw email addresses, or private message bodies.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::{AppResult, Page};
use sqlx::{FromRow, PgConnection, PgPool};

#[derive(Debug, Clone, FromRow)]
struct AuditEventRow {
    id: i64,
    actor_kind: String,
    actor_account_id: Option<i64>,
    actor_role: Option<String>,
    actor_handle: Option<String>,
    action: String,
    target_type: String,
    target_id: String,
    reason: Option<String>,
    metadata: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventDto {
    pub id: String,
    pub actor_kind: String,
    pub actor_id: Option<String>,
    pub actor_handle: Option<String>,
    pub actor_role: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct AccountActor<'a> {
    pub account_id: i64,
    pub role: &'a str,
}

/// Append an account-authored audit event inside an existing transaction.
#[allow(clippy::too_many_arguments)] // reason: audit fields are intentionally explicit to prevent opaque request-body logging
pub async fn record_account_event_tx(
    tx: &mut PgConnection,
    actor: AccountActor<'_>,
    action: &str,
    target_type: &str,
    target_id: &str,
    reason: &str,
    metadata: Option<&serde_json::Value>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO governance.audit_events \
         (actor_kind, actor_account_id, actor_role, action, target_type, target_id, reason, metadata) \
         VALUES ('account', $1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(actor.account_id)
    .bind(actor.role)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .bind(metadata)
    .execute(tx)
    .await?;
    Ok(())
}

/// Append an account-authored audit event as its own transaction.
#[allow(clippy::too_many_arguments)] // reason: convenience wrapper mirrors the transaction-aware writer's explicit fields
pub async fn record_account_event(
    pool: &PgPool,
    actor: AccountActor<'_>,
    action: &str,
    target_type: &str,
    target_id: &str,
    reason: &str,
    metadata: Option<&serde_json::Value>,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    record_account_event_tx(&mut tx, actor, action, target_type, target_id, reason, metadata)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Append a system-authored audit event inside an existing transaction.
#[allow(clippy::too_many_arguments)] // reason: audit fields are intentionally explicit to prevent opaque request-body logging
pub async fn record_system_event_tx(
    tx: &mut PgConnection,
    action: &str,
    target_type: &str,
    target_id: &str,
    reason: &str,
    metadata: Option<&serde_json::Value>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO governance.audit_events \
         (actor_kind, action, target_type, target_id, reason, metadata) \
         VALUES ('system', $1, $2, $3, $4, $5)",
    )
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .bind(metadata)
    .execute(tx)
    .await?;
    Ok(())
}

/// Return newest audit events with bounded cursor pagination.
pub async fn list_events(
    pool: &PgPool,
    cursor: Option<i64>,
    limit: i64,
    actor_id: Option<i64>,
    action: Option<&str>,
    target_type: Option<&str>,
) -> AppResult<Page<AuditEventDto>> {
    let page_size = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, AuditEventRow>(
        "SELECT events.id, events.actor_kind, events.actor_account_id, events.actor_role, \
                accounts.handle::text AS actor_handle, events.action, events.target_type, \
                events.target_id, events.reason, events.metadata, events.created_at \
         FROM governance.audit_events events \
         LEFT JOIN identity.accounts accounts ON accounts.id = events.actor_account_id \
         WHERE ($1::bigint IS NULL OR events.id < $1) \
           AND ($2::bigint IS NULL OR events.actor_account_id = $2) \
           AND ($3::text IS NULL OR events.action = $3) \
           AND ($4::text IS NULL OR events.target_type = $4) \
         ORDER BY events.id DESC LIMIT $5",
    )
    .bind(cursor)
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    let visible_rows = if has_more { &rows[..page_size as usize] } else { &rows };
    let items = visible_rows
        .iter()
        .map(|row| AuditEventDto {
            id: row.id.to_string(),
            actor_kind: row.actor_kind.clone(),
            actor_id: row.actor_account_id.map(|id| id.to_string()),
            actor_handle: row.actor_handle.clone(),
            actor_role: row.actor_role.clone(),
            action: row.action.clone(),
            target_type: row.target_type.clone(),
            target_id: row.target_id.clone(),
            reason: row.reason.clone(),
            metadata: row.metadata.clone(),
            created_at: row.created_at.timestamp(),
        })
        .collect();
    let next_cursor = has_more.then(|| visible_rows.last().map(|row| row.id.to_string())).flatten();
    Ok(Page::new(items, next_cursor))
}

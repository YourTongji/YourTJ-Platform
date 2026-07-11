//! Appeal cases and append-only transition history.
//!
//! This domain owns review state and evidence metadata. The account, forum, and reviews
//! domains remain responsible for validating and reversing their own dispositions.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use shared::{AppError, AppResult, Page};
use sqlx::{FromRow, PgConnection, PgPool};

use crate::notices::create_notice_tx;
use crate::{record_account_event_tx, AccountActor};

const APPEAL_WINDOW_DAYS: i64 = 30;

#[derive(Debug, Clone, FromRow)]
pub struct AppealableAuditEvent {
    pub id: i64,
    pub actor_account_id: Option<i64>,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VerifiedAppealTarget {
    pub target_kind: String,
    pub target_id: String,
    pub disposition_kind: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct AppealRecord {
    pub id: i64,
    pub original_event_id: i64,
    pub appellant_account_id: i64,
    pub original_actor_id: Option<i64>,
    pub original_action: String,
    pub original_reason: Option<String>,
    pub target_kind: String,
    pub target_id: String,
    pub disposition_kind: String,
    pub status: String,
    pub submission_reason: String,
    pub submitted_at: DateTime<Utc>,
    pub appealable_until: DateTime<Utc>,
    pub reviewer_account_id: Option<i64>,
    pub review_started_at: Option<DateTime<Utc>>,
    pub decision_reason: Option<String>,
    pub amendment: Option<serde_json::Value>,
    pub decided_at: Option<DateTime<Utc>>,
    pub version: i64,
}

#[derive(Debug, Clone, FromRow)]
struct AppealEventRecord {
    id: i64,
    appeal_id: i64,
    from_status: Option<String>,
    to_status: String,
    reason: String,
    metadata: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealHistoryDto {
    pub id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub reason: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealDto {
    pub id: String,
    pub governance_event_id: String,
    pub original_action: String,
    pub original_reason: Option<String>,
    pub target_kind: String,
    pub target_id: String,
    pub disposition_kind: String,
    pub status: String,
    pub submission_reason: String,
    pub submitted_at: i64,
    pub appealable_until: i64,
    pub review_started_at: Option<i64>,
    pub decision_reason: Option<String>,
    pub amendment: Option<serde_json::Value>,
    pub decided_at: Option<i64>,
    pub version: i64,
    pub history: Vec<AppealHistoryDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAppealDto {
    #[serde(flatten)]
    pub appeal: AppealDto,
    pub appellant_account_id: String,
    pub reviewer_account_id: Option<String>,
}

pub struct SubmitAppeal<'a> {
    pub actor: AccountActor<'a>,
    pub governance_event: AppealableAuditEvent,
    pub target: VerifiedAppealTarget,
    pub reason: &'a str,
    pub idempotency_key: &'a str,
    pub request_hash: &'a str,
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=1000).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–1000 characters".into()));
    }
    Ok(reason)
}

fn validate_idempotency_key(value: &str) -> AppResult<&str> {
    let value = value.trim();
    if !(8..=128).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AppError::BadRequest("invalid Idempotency-Key".into()));
    }
    Ok(value)
}

fn row_query(where_clause: &str) -> String {
    format!(
        "SELECT appeal.id, appeal.original_event_id, appeal.appellant_account_id, \
                appeal.original_actor_id, appeal.original_action, audit.reason AS original_reason, \
                appeal.target_kind, appeal.target_id, appeal.disposition_kind, appeal.status, \
                appeal.submission_reason, appeal.submitted_at, appeal.appealable_until, \
                appeal.reviewer_account_id, appeal.review_started_at, appeal.decision_reason, \
                appeal.amendment, appeal.decided_at, appeal.version \
         FROM governance.appeals appeal \
         JOIN governance.audit_events audit ON audit.id = appeal.original_event_id \
         {where_clause}"
    )
}

/// Lock an immutable governance event before an owner domain validates its target.
pub async fn find_appealable_event_tx(
    connection: &mut PgConnection,
    event_id: i64,
) -> AppResult<AppealableAuditEvent> {
    sqlx::query_as::<_, AppealableAuditEvent>(
        "SELECT id, actor_account_id, action, target_type, target_id, reason, metadata, created_at \
         FROM governance.audit_events WHERE id = $1 FOR SHARE",
    )
    .bind(event_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn load_history(
    pool: &PgPool,
    appeal_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<AppealHistoryDto>>> {
    if appeal_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, AppealEventRecord>(
        "SELECT id, appeal_id, from_status, to_status, reason, metadata, created_at \
         FROM governance.appeal_events WHERE appeal_id = ANY($1) ORDER BY id",
    )
    .bind(appeal_ids)
    .fetch_all(pool)
    .await?;
    let mut grouped: HashMap<i64, Vec<AppealHistoryDto>> = HashMap::new();
    for row in rows {
        grouped.entry(row.appeal_id).or_default().push(AppealHistoryDto {
            id: row.id.to_string(),
            from_status: row.from_status,
            to_status: row.to_status,
            reason: row.reason,
            metadata: row.metadata,
            created_at: row.created_at.timestamp(),
        });
    }
    Ok(grouped)
}

fn public_disposition_summary(record: &AppealRecord) -> String {
    match (record.target_kind.as_str(), record.disposition_kind.as_str()) {
        ("sanction", "silence") => "账号禁言处置".into(),
        ("sanction", "suspend") => "账号封禁处置".into(),
        ("forum_thread", "hide") => "主题隐藏处置".into(),
        ("forum_thread", "delete") => "主题软移除处置".into(),
        ("forum_comment", "hide") => "评论隐藏处置".into(),
        ("forum_comment", "delete") => "评论软移除处置".into(),
        ("review", "hide") => "课评隐藏处置".into(),
        _ => "社区治理处置".into(),
    }
}

fn appeal_dto(
    record: AppealRecord,
    history: Vec<AppealHistoryDto>,
    include_internal_reason: bool,
) -> AppealDto {
    let original_reason = if include_internal_reason {
        record.original_reason.clone()
    } else {
        Some(public_disposition_summary(&record))
    };
    AppealDto {
        id: record.id.to_string(),
        governance_event_id: record.original_event_id.to_string(),
        original_action: record.original_action,
        original_reason,
        target_kind: record.target_kind,
        target_id: record.target_id,
        disposition_kind: record.disposition_kind,
        status: record.status,
        submission_reason: record.submission_reason,
        submitted_at: record.submitted_at.timestamp(),
        appealable_until: record.appealable_until.timestamp(),
        review_started_at: record.review_started_at.map(|value| value.timestamp()),
        decision_reason: record.decision_reason,
        amendment: record.amendment,
        decided_at: record.decided_at.map(|value| value.timestamp()),
        version: record.version,
        history,
    }
}

/// Create one appeal. Retries with the same key and payload return the existing case.
pub async fn submit_appeal_tx(
    connection: &mut PgConnection,
    input: SubmitAppeal<'_>,
) -> AppResult<(AppealRecord, bool)> {
    let reason = validate_reason(input.reason)?;
    let idempotency_key = validate_idempotency_key(input.idempotency_key)?;
    if input.request_hash.len() != 64
        || !input.request_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::BadRequest("invalid request hash".into()));
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("governance:appeal:{}:{idempotency_key}", input.actor.account_id))
        .execute(&mut *connection)
        .await?;

    let existing: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, request_hash FROM governance.appeals \
         WHERE appellant_account_id = $1 AND idempotency_key = $2",
    )
    .bind(input.actor.account_id)
    .bind(idempotency_key)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some((appeal_id, stored_hash)) = existing {
        if stored_hash != input.request_hash {
            return Err(AppError::Conflict(
                "idempotency key was already used for another appeal request".into(),
            ));
        }
        let record = sqlx::query_as::<_, AppealRecord>(&row_query("WHERE appeal.id = $1"))
            .bind(appeal_id)
            .fetch_one(connection)
            .await?;
        return Ok((record, true));
    }

    let appealable_until = input.governance_event.created_at + Duration::days(APPEAL_WINDOW_DAYS);
    if appealable_until <= Utc::now() {
        return Err(AppError::Conflict("the appeal window has closed".into()));
    }
    let appeal_id: i64 = sqlx::query_scalar(
        "INSERT INTO governance.appeals \
         (original_event_id, appellant_account_id, original_actor_id, original_action, \
          target_kind, target_id, disposition_kind, submission_reason, idempotency_key, \
          request_hash, appealable_until) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(input.governance_event.id)
    .bind(input.actor.account_id)
    .bind(input.governance_event.actor_account_id)
    .bind(&input.governance_event.action)
    .bind(&input.target.target_kind)
    .bind(&input.target.target_id)
    .bind(&input.target.disposition_kind)
    .bind(reason)
    .bind(idempotency_key)
    .bind(input.request_hash)
    .bind(appealable_until)
    .fetch_one(&mut *connection)
    .await
    .map_err(|error| {
        if error.as_database_error().is_some_and(|database| database.is_unique_violation()) {
            AppError::Conflict("this disposition already has an appeal".into())
        } else {
            error.into()
        }
    })?;
    sqlx::query(
        "INSERT INTO governance.appeal_events \
         (appeal_id, actor_kind, actor_account_id, to_status, reason) \
         VALUES ($1, 'account', $2, 'submitted', $3)",
    )
    .bind(appeal_id)
    .bind(input.actor.account_id)
    .bind(reason)
    .execute(&mut *connection)
    .await?;
    record_account_event_tx(
        connection,
        input.actor,
        "governance.appeal.submitted",
        "appeal",
        &appeal_id.to_string(),
        reason,
        Some(&serde_json::json!({ "governanceEventId": input.governance_event.id })),
    )
    .await?;
    create_notice_tx(
        connection,
        input.actor.account_id,
        "appeal_submitted",
        &format!("appeal:{appeal_id}:submitted"),
        None,
        Some(appeal_id),
        "appeal",
        &appeal_id.to_string(),
        "申诉已提交，将由未参与原处置的工作人员复核。",
    )
    .await?;
    let record = sqlx::query_as::<_, AppealRecord>(&row_query("WHERE appeal.id = $1"))
        .bind(appeal_id)
        .fetch_one(connection)
        .await?;
    Ok((record, false))
}

/// List appeal cases belonging to one account.
pub async fn list_my_appeals(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<AppealDto>> {
    list_appeals(pool, Some(account_id), None, cursor, limit).await.map(|page| {
        Page::new(page.items.into_iter().map(|item| item.appeal).collect(), page.next_cursor)
    })
}

/// Return one appeal to its owner, including append-only status history.
pub async fn get_my_appeal(pool: &PgPool, account_id: i64, appeal_id: i64) -> AppResult<AppealDto> {
    let record = sqlx::query_as::<_, AppealRecord>(&row_query(
        "WHERE appeal.id = $1 AND appeal.appellant_account_id = $2",
    ))
    .bind(appeal_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let mut histories = load_history(pool, &[appeal_id]).await?;
    Ok(appeal_dto(record, histories.remove(&appeal_id).unwrap_or_default(), false))
}

/// Return one appeal to authorized staff, including the appellant and reviewer ids.
pub async fn get_admin_appeal(pool: &PgPool, appeal_id: i64) -> AppResult<AdminAppealDto> {
    let record = sqlx::query_as::<_, AppealRecord>(&row_query("WHERE appeal.id = $1"))
        .bind(appeal_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut histories = load_history(pool, &[appeal_id]).await?;
    let appellant_account_id = record.appellant_account_id.to_string();
    let reviewer_account_id = record.reviewer_account_id.map(|value| value.to_string());
    Ok(AdminAppealDto {
        appeal: appeal_dto(record, histories.remove(&appeal_id).unwrap_or_default(), true),
        appellant_account_id,
        reviewer_account_id,
    })
}

/// List staff appeal queue cases with bounded status/cursor filters.
pub async fn list_admin_appeals(
    pool: &PgPool,
    status: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<AdminAppealDto>> {
    if status.is_some_and(|value| {
        !matches!(
            value,
            "submitted" | "in_review" | "upheld" | "overturned" | "amended" | "withdrawn"
        )
    }) {
        return Err(AppError::BadRequest("invalid appeal status".into()));
    }
    list_appeals(pool, None, status, cursor, limit).await
}

async fn list_appeals(
    pool: &PgPool,
    appellant_account_id: Option<i64>,
    status: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<AdminAppealDto>> {
    if !(1..=100).contains(&limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let rows = sqlx::query_as::<_, AppealRecord>(&row_query(
        "WHERE ($1::bigint IS NULL OR appeal.appellant_account_id = $1) \
           AND ($2::text IS NULL OR appeal.status = $2) \
           AND ($3::bigint IS NULL OR appeal.id < $3) \
         ORDER BY appeal.id DESC LIMIT $4",
    ))
    .bind(appellant_account_id)
    .bind(status)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible_count = if has_more { limit as usize } else { rows.len() };
    let rows = rows.into_iter().take(visible_count).collect::<Vec<_>>();
    let ids = rows.iter().map(|record| record.id).collect::<Vec<_>>();
    let mut histories = load_history(pool, &ids).await?;
    let include_internal_reason = appellant_account_id.is_none();
    let items = rows
        .into_iter()
        .map(|record| {
            let appellant_account_id = record.appellant_account_id.to_string();
            let reviewer_account_id = record.reviewer_account_id.map(|value| value.to_string());
            let history = histories.remove(&record.id).unwrap_or_default();
            AdminAppealDto {
                appeal: appeal_dto(record, history, include_internal_reason),
                appellant_account_id,
                reviewer_account_id,
            }
        })
        .collect::<Vec<_>>();
    let next_cursor = has_more.then(|| items.last().map(|item| item.appeal.id.clone())).flatten();
    Ok(Page::new(items, next_cursor))
}

/// Lock an appeal for a staff transition or owner withdrawal.
pub async fn find_appeal_for_update_tx(
    connection: &mut PgConnection,
    appeal_id: i64,
) -> AppResult<AppealRecord> {
    sqlx::query_as::<_, AppealRecord>(&format!(
        "{} FOR UPDATE OF appeal",
        row_query("WHERE appeal.id = $1")
    ))
    .bind(appeal_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

/// Move a submitted case to independent review using optimistic versioning.
pub async fn start_review_tx(
    connection: &mut PgConnection,
    appeal: &AppealRecord,
    reviewer: AccountActor<'_>,
    expected_version: i64,
    reason: &str,
) -> AppResult<()> {
    let reason = validate_reason(reason)?;
    if appeal.version != expected_version {
        return Err(AppError::Conflict("appeal version changed".into()));
    }
    if appeal.status != "submitted" {
        return Err(AppError::Conflict("appeal is not awaiting review".into()));
    }
    if appeal.appellant_account_id == reviewer.account_id
        || appeal.original_actor_id == Some(reviewer.account_id)
    {
        return Err(AppError::Forbidden);
    }
    let changed = sqlx::query(
        "UPDATE governance.appeals \
         SET status = 'in_review', reviewer_account_id = $1, review_started_at = now(), \
             version = version + 1 \
         WHERE id = $2 AND status = 'submitted' AND version = $3",
    )
    .bind(reviewer.account_id)
    .bind(appeal.id)
    .bind(expected_version)
    .execute(&mut *connection)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::Conflict("appeal version changed".into()));
    }
    append_transition(
        connection,
        appeal.id,
        reviewer,
        Some("submitted"),
        "in_review",
        "申诉已由独立复核员领取。",
        None,
    )
    .await?;
    create_notice_tx(
        connection,
        appeal.appellant_account_id,
        "appeal_in_review",
        &format!("appeal:{}:in_review", appeal.id),
        None,
        Some(appeal.id),
        "appeal",
        &appeal.id.to_string(),
        "申诉已进入独立复核。",
    )
    .await?;
    record_account_event_tx(
        connection,
        reviewer,
        "governance.appeal.review_started",
        "appeal",
        &appeal.id.to_string(),
        reason,
        None,
    )
    .await
}

/// Persist a terminal appeal decision after the owner domain has applied any reversal/amendment.
#[allow(clippy::too_many_arguments)] // reason: transition and audit inputs stay explicit at the transaction boundary
pub async fn decide_appeal_tx(
    connection: &mut PgConnection,
    appeal: &AppealRecord,
    reviewer: AccountActor<'_>,
    expected_version: i64,
    outcome: &str,
    reason: &str,
    amendment: Option<&serde_json::Value>,
) -> AppResult<()> {
    let reason = validate_reason(reason)?;
    if appeal.version != expected_version {
        return Err(AppError::Conflict("appeal version changed".into()));
    }
    if appeal.status != "in_review" || appeal.reviewer_account_id != Some(reviewer.account_id) {
        return Err(AppError::Forbidden);
    }
    if !matches!(outcome, "upheld" | "overturned" | "amended") {
        return Err(AppError::BadRequest("invalid appeal outcome".into()));
    }
    if (outcome == "amended") != amendment.is_some() {
        return Err(AppError::BadRequest("amended decisions require amendment details".into()));
    }
    let changed = sqlx::query(
        "UPDATE governance.appeals \
         SET status = $1, decision_reason = $2, amendment = $3, decided_at = now(), \
             version = version + 1 \
         WHERE id = $4 AND status = 'in_review' AND reviewer_account_id = $5 AND version = $6",
    )
    .bind(outcome)
    .bind(reason)
    .bind(amendment)
    .bind(appeal.id)
    .bind(reviewer.account_id)
    .bind(expected_version)
    .execute(&mut *connection)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::Conflict("appeal version changed".into()));
    }
    append_transition(
        connection,
        appeal.id,
        reviewer,
        Some("in_review"),
        outcome,
        reason,
        amendment,
    )
    .await?;
    create_notice_tx(
        connection,
        appeal.appellant_account_id,
        &format!("appeal_{outcome}"),
        &format!("appeal:{}:{outcome}", appeal.id),
        None,
        Some(appeal.id),
        "appeal",
        &appeal.id.to_string(),
        match outcome {
            "upheld" => "申诉复核完成：原处置维持。",
            "overturned" => "申诉复核完成：原处置已撤销。",
            "amended" => "申诉复核完成：原处置已调整。",
            _ => return Err(AppError::BadRequest("invalid appeal outcome".into())),
        },
    )
    .await?;
    record_account_event_tx(
        connection,
        reviewer,
        &format!("governance.appeal.{outcome}"),
        "appeal",
        &appeal.id.to_string(),
        reason,
        amendment,
    )
    .await
}

/// Withdraw an unclaimed appeal. History remains immutable.
pub async fn withdraw_appeal_tx(
    connection: &mut PgConnection,
    appeal: &AppealRecord,
    actor: AccountActor<'_>,
    expected_version: i64,
    reason: &str,
) -> AppResult<()> {
    let reason = validate_reason(reason)?;
    if appeal.appellant_account_id != actor.account_id {
        return Err(AppError::NotFound);
    }
    if appeal.version != expected_version {
        return Err(AppError::Conflict("appeal version changed".into()));
    }
    if appeal.status != "submitted" {
        return Err(AppError::Conflict("only an unclaimed appeal can be withdrawn".into()));
    }
    let changed = sqlx::query(
        "UPDATE governance.appeals \
         SET status = 'withdrawn', decision_reason = $1, decided_at = now(), version = version + 1 \
         WHERE id = $2 AND appellant_account_id = $3 AND status = 'submitted' AND version = $4",
    )
    .bind(reason)
    .bind(appeal.id)
    .bind(actor.account_id)
    .bind(expected_version)
    .execute(&mut *connection)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(AppError::Conflict("appeal version changed".into()));
    }
    append_transition(connection, appeal.id, actor, Some("submitted"), "withdrawn", reason, None)
        .await?;
    create_notice_tx(
        connection,
        actor.account_id,
        "appeal_withdrawn",
        &format!("appeal:{}:withdrawn", appeal.id),
        None,
        Some(appeal.id),
        "appeal",
        &appeal.id.to_string(),
        "申诉已由你撤回，原处置不变。",
    )
    .await?;
    record_account_event_tx(
        connection,
        actor,
        "governance.appeal.withdrawn",
        "appeal",
        &appeal.id.to_string(),
        reason,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)] // reason: append-only transition fields remain explicit
async fn append_transition(
    connection: &mut PgConnection,
    appeal_id: i64,
    actor: AccountActor<'_>,
    from_status: Option<&str>,
    to_status: &str,
    reason: &str,
    metadata: Option<&serde_json::Value>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO governance.appeal_events \
         (appeal_id, actor_kind, actor_account_id, from_status, to_status, reason, metadata) \
         VALUES ($1, 'account', $2, $3, $4, $5, $6)",
    )
    .bind(appeal_id)
    .bind(actor.account_id)
    .bind(from_status)
    .bind(to_status)
    .bind(reason)
    .bind(metadata)
    .execute(connection)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_idempotency_key, validate_reason};

    #[test]
    fn bounded_plain_reasons_and_idempotency_keys_are_required() {
        assert!(validate_reason("valid reason").is_ok());
        assert!(validate_reason("x").is_err());
        assert!(validate_idempotency_key("appeal-1234").is_ok());
        assert!(validate_idempotency_key("bad key").is_err());
    }
}

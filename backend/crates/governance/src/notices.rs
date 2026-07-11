//! Purpose-limited governance notices visible only to the affected account.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::{AppError, AppResult, Page};
use sqlx::{FromRow, PgConnection, PgPool};

#[derive(Debug, Clone, FromRow)]
struct GovernanceNoticeRow {
    id: i64,
    notice_type: String,
    subject_kind: String,
    subject_id: String,
    summary: String,
    governance_event_id: Option<i64>,
    appeal_id: Option<i64>,
    read_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceNoticeDto {
    pub id: String,
    pub notice_type: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub summary: String,
    pub appeal_id: Option<String>,
    pub target_url: String,
    pub read: bool,
    pub read_at: Option<i64>,
    pub created_at: i64,
}

fn notice_dto(row: GovernanceNoticeRow) -> GovernanceNoticeDto {
    let target_url = row
        .appeal_id
        .map(|appeal_id| format!("/appeals?appeal={appeal_id}"))
        .or_else(|| row.governance_event_id.map(|event_id| format!("/appeals?event={event_id}")))
        .unwrap_or_else(|| "/appeals".into());
    GovernanceNoticeDto {
        id: row.id.to_string(),
        notice_type: row.notice_type,
        subject_kind: row.subject_kind,
        subject_id: row.subject_id,
        summary: row.summary,
        appeal_id: row.appeal_id.map(|value| value.to_string()),
        target_url,
        read: row.read_at.is_some(),
        read_at: row.read_at.map(|value| value.timestamp()),
        created_at: row.created_at.timestamp(),
    }
}

/// Append one deduplicated, account-private governance notice in the caller's transaction.
#[allow(clippy::too_many_arguments)] // reason: notice fields remain explicit so private evidence cannot be passed accidentally
pub async fn create_notice_tx(
    connection: &mut PgConnection,
    account_id: i64,
    notice_type: &str,
    dedupe_key: &str,
    governance_event_id: Option<i64>,
    appeal_id: Option<i64>,
    subject_kind: &str,
    subject_id: &str,
    summary: &str,
) -> AppResult<()> {
    let summary = summary.trim();
    if summary.is_empty() {
        return Err(AppError::BadRequest("notice summary must not be empty".into()));
    }
    let summary = summary.chars().take(500).collect::<String>();
    sqlx::query(
        "INSERT INTO governance.notices \
         (account_id, notice_type, dedupe_key, governance_event_id, appeal_id, \
          subject_kind, subject_id, summary) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (account_id, dedupe_key) DO NOTHING",
    )
    .bind(account_id)
    .bind(notice_type)
    .bind(dedupe_key)
    .bind(governance_event_id)
    .bind(appeal_id)
    .bind(subject_kind)
    .bind(subject_id)
    .bind(summary)
    .execute(connection)
    .await?;
    Ok(())
}

/// List one account's notices with an exclusive id cursor.
pub async fn list_notices(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    unread_only: bool,
    limit: i64,
) -> AppResult<Page<GovernanceNoticeDto>> {
    if !(1..=100).contains(&limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let rows = sqlx::query_as::<_, GovernanceNoticeRow>(
        "SELECT id, notice_type, subject_kind, subject_id, summary, governance_event_id, \
                appeal_id, read_at, created_at \
         FROM governance.notices \
         WHERE account_id = $1 AND ($2::bigint IS NULL OR id < $2) \
           AND (NOT $3 OR read_at IS NULL) \
         ORDER BY id DESC LIMIT $4",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(unread_only)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible_count = if has_more { limit as usize } else { rows.len() };
    let items = rows.into_iter().take(visible_count).map(notice_dto).collect::<Vec<_>>();
    let next_cursor = has_more.then(|| items.last().map(|item| item.id.clone())).flatten();
    Ok(Page::new(items, next_cursor))
}

pub async fn unread_count(pool: &PgPool, account_id: i64) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.notices WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?)
}

/// Mark selected notices, or all notices when `notice_ids` is `None`, for one owner only.
pub async fn mark_read(
    pool: &PgPool,
    account_id: i64,
    notice_ids: Option<&[i64]>,
) -> AppResult<()> {
    match notice_ids {
        Some(ids) => {
            if ids.is_empty() || ids.len() > 100 || ids.iter().any(|id| *id <= 0) {
                return Err(AppError::BadRequest("ids must contain 1–100 positive ids".into()));
            }
            sqlx::query(
                "UPDATE governance.notices SET read_at = COALESCE(read_at, now()) \
                 WHERE account_id = $1 AND id = ANY($2)",
            )
            .bind(account_id)
            .bind(ids)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query(
                "UPDATE governance.notices SET read_at = now() \
                 WHERE account_id = $1 AND read_at IS NULL",
            )
            .bind(account_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

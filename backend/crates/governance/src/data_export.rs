//! Account-visible governance export without reporter, reviewer, staff, or evidence metadata.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceExport {
    notices: Vec<ExportNotice>,
    appeals: Vec<ExportAppeal>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportNotice {
    id: i64,
    notice_type: String,
    subject_kind: String,
    subject_id: String,
    summary: String,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    read_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportAppeal {
    id: i64,
    original_action: String,
    target_kind: String,
    target_id: String,
    disposition_kind: String,
    status: String,
    submission_reason: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    submitted_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    appealable_until: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    review_started_at: Option<DateTime<Utc>>,
    decision_reason: Option<String>,
    amendment: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    decided_at: Option<DateTime<Utc>>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<GovernanceExport> {
    let notices = sqlx::query_as::<_, ExportNotice>(
        "SELECT id, notice_type, subject_kind, subject_id, summary, read_at, created_at \
         FROM governance.notices WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let appeals = sqlx::query_as::<_, ExportAppeal>(
        "SELECT id, original_action, target_kind, target_id, disposition_kind, status, \
                submission_reason, submitted_at, appealable_until, review_started_at, \
                decision_reason, amendment, decided_at \
         FROM governance.appeals WHERE appellant_account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(GovernanceExport { notices, appeals })
}

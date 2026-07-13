//! Activity-owned daily-count export and purge operations.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportActivityDay {
    activity_date: NaiveDate,
    threads_created: i32,
    comments_created: i32,
    likes_given: i32,
    check_ins: i32,
    score: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCheckIn {
    activity_date: NaiveDate,
    #[serde(with = "chrono::serde::ts_seconds")]
    checked_in_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTrustProgress {
    trust_level: i16,
    qualifying_score: i64,
    policy_version: i64,
    override_active: bool,
    #[serde(with = "chrono::serde::ts_seconds")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScoreProjection {
    qualifying_score: i64,
    score_policy_version: i64,
    trust_policy_version: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTrustEvent {
    event_kind: String,
    from_level: i16,
    to_level: i16,
    qualifying_score: i64,
    policy_version: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityExport {
    days: Vec<ExportActivityDay>,
    check_ins: Vec<ExportCheckIn>,
    score_projection: Option<ExportScoreProjection>,
    trust_progress: Option<ExportTrustProgress>,
    trust_events: Vec<ExportTrustEvent>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<ActivityExport> {
    let (days, check_ins, score_projection, trust_progress, trust_events) = tokio::try_join!(
        sqlx::query_as::<_, ExportActivityDay>(
            "SELECT activity_date, threads_created, comments_created, likes_given, check_ins, \
                    score, updated_at \
             FROM activity.daily_counts WHERE account_id = $1 ORDER BY activity_date",
        )
        .bind(account_id)
        .fetch_all(pool),
        sqlx::query_as::<_, ExportCheckIn>(
            "SELECT activity_date, checked_in_at FROM activity.check_ins \
             WHERE account_id = $1 ORDER BY activity_date",
        )
        .bind(account_id)
        .fetch_all(pool),
        sqlx::query_as::<_, ExportScoreProjection>(
            "SELECT qualifying_score, score_policy_version, trust_policy_version, updated_at \
             FROM activity.account_scores WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_optional(pool),
        sqlx::query_as::<_, ExportTrustProgress>(
            "SELECT trust_level, qualifying_score, policy_version, \
                    override_level IS NOT NULL AS override_active, updated_at \
             FROM activity.account_trust_progress WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_optional(pool),
        sqlx::query_as::<_, ExportTrustEvent>(
            "SELECT event_kind, from_level, to_level, qualifying_score, policy_version, created_at \
             FROM activity.trust_level_events WHERE account_id = $1 ORDER BY id",
        )
        .bind(account_id)
        .fetch_all(pool),
    )?;
    Ok(ActivityExport { days, check_ins, score_projection, trust_progress, trust_events })
}

pub async fn purge_account_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM activity.account_trust_progress WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM activity.check_ins WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM activity.account_scores WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM activity.events WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM activity.daily_counts WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM activity.trust_level_events WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

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
    #[serde(with = "chrono::serde::ts_seconds")]
    updated_at: DateTime<Utc>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<Vec<ExportActivityDay>> {
    Ok(sqlx::query_as::<_, ExportActivityDay>(
        "SELECT activity_date, threads_created, comments_created, likes_given, updated_at \
         FROM activity.daily_counts WHERE account_id = $1 ORDER BY activity_date",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?)
}

pub async fn purge_account_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM activity.events WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM activity.daily_counts WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

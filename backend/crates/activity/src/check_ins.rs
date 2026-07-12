//! Idempotent daily check-ins on the canonical activity calendar.

use chrono::{DateTime, NaiveDate, Utc};
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::contributions::activate_check_in_contribution;
use crate::dto::CheckInStatusDto;
use crate::models::CheckInStatusRow;

/// Read today's check-in status without changing account state.
pub async fn current_status(pool: &PgPool, account_id: i64) -> AppResult<CheckInStatusDto> {
    let mut connection = pool.acquire().await?;
    let activity_date = current_activity_date(&mut connection).await?;
    status(&mut connection, account_id, activity_date, false).await
}

/// Create today's check-in once and return its current state.
pub async fn check_in(pool: &PgPool, account_id: i64) -> AppResult<CheckInStatusDto> {
    let mut tx = pool.begin().await?;
    let (checked_in_at, activity_date): (DateTime<Utc>, NaiveDate) =
        sqlx::query_as("SELECT now(), (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(&mut *tx)
            .await?;
    let inserted = sqlx::query_scalar::<_, bool>(
        "INSERT INTO activity.check_ins (account_id, activity_date, checked_in_at) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (account_id, activity_date) DO NOTHING \
         RETURNING true",
    )
    .bind(account_id)
    .bind(activity_date)
    .bind(checked_in_at)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(false);

    if inserted {
        let source_key = format!("check_in:{account_id}:{activity_date}");
        if !activate_check_in_contribution(&mut tx, account_id, &source_key, checked_in_at).await? {
            return Err(AppError::Internal(anyhow::anyhow!(
                "new check-in did not create its activity projection"
            )));
        }
    }

    let result = status(&mut tx, account_id, activity_date, inserted).await?;
    tx.commit().await?;
    Ok(result)
}

async fn current_activity_date(connection: &mut PgConnection) -> AppResult<NaiveDate> {
    Ok(sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
        .fetch_one(connection)
        .await?)
}

async fn status(
    connection: &mut PgConnection,
    account_id: i64,
    activity_date: NaiveDate,
    newly_checked_in: bool,
) -> AppResult<CheckInStatusDto> {
    let row = sqlx::query_as::<_, CheckInStatusRow>(
        "WITH today AS ( \
           SELECT checked_in_at \
           FROM activity.check_ins \
           WHERE account_id = $1 AND activity_date = $2 \
         ), ordered AS ( \
           SELECT activity_date, row_number() OVER (ORDER BY activity_date DESC)::int AS position \
           FROM activity.check_ins \
           WHERE account_id = $1 \
             AND activity_date <= CASE \
               WHEN EXISTS (SELECT 1 FROM today) THEN $2::date \
               ELSE $2::date - 1 \
             END \
         ) \
         SELECT $2::date AS activity_date, \
                (SELECT checked_in_at FROM today) AS checked_in_at, \
                COALESCE(( \
                  SELECT COUNT(*) FROM ordered \
                  WHERE activity_date = ( \
                    CASE WHEN EXISTS (SELECT 1 FROM today) THEN $2::date ELSE $2::date - 1 END \
                  ) - (position - 1) \
                ), 0)::bigint AS current_streak, \
                (SELECT COUNT(*) FROM activity.check_ins WHERE account_id = $1)::bigint \
                  AS total_days, \
                (($2::date + 1)::timestamp AT TIME ZONE 'Asia/Shanghai') AS next_reset_at",
    )
    .bind(account_id)
    .bind(activity_date)
    .fetch_one(connection)
    .await?;

    Ok(CheckInStatusDto {
        timezone: "Asia/Shanghai",
        date: row.activity_date.to_string(),
        checked_in: row.checked_in_at.is_some(),
        newly_checked_in,
        checked_in_at: row.checked_in_at.map(|timestamp| timestamp.timestamp()),
        current_streak: row.current_streak,
        total_days: row.total_days,
        next_reset_at: row.next_reset_at.timestamp(),
    })
}

use chrono::NaiveDate;
use shared::{AppError, AppResult, Page};
use sqlx::PgPool;

use crate::dto::{ActivityCalendarDto, ActivityDayDto, ActivityPolicyDto, ActivityWeightsDto};
use crate::models::{ActivityDayRow, ScorePolicyRow};

pub(crate) async fn current_activity_date(pool: &PgPool) -> AppResult<NaiveDate> {
    let date = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
        .fetch_one(pool)
        .await?;
    Ok(date)
}

pub(crate) async fn activity_calendar(
    pool: &PgPool,
    account_id: i64,
    from: NaiveDate,
    to: NaiveDate,
) -> AppResult<ActivityCalendarDto> {
    let policy = current_policy_row(pool).await?;
    let rows = sqlx::query_as::<_, ActivityDayRow>(
        "SELECT day::date AS activity_date, \
                COALESCE(counts.threads_created, 0)::int AS threads_created, \
                COALESCE(counts.comments_created, 0)::int AS comments_created, \
                COALESCE(counts.likes_given, 0)::int AS likes_given \
         FROM generate_series($2::date, $3::date, interval '1 day') day \
         LEFT JOIN activity.daily_counts counts \
           ON counts.account_id = $1 AND counts.activity_date = day::date \
         ORDER BY day",
    )
    .bind(account_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let weights = weights_from_row(&policy);
    let days = rows
        .into_iter()
        .map(|row| ActivityDayDto {
            date: row.activity_date.to_string(),
            threads: row.threads_created,
            comments: row.comments_created,
            likes: row.likes_given,
            score: i64::from(row.threads_created) * i64::from(weights.thread)
                + i64::from(row.comments_created) * i64::from(weights.comment)
                + i64::from(row.likes_given) * i64::from(weights.like),
        })
        .collect();

    Ok(ActivityCalendarDto {
        timezone: "Asia/Shanghai",
        from: from.to_string(),
        to: to.to_string(),
        policy_version: policy.version,
        weights,
        days,
    })
}

pub(crate) async fn current_policy(pool: &PgPool) -> AppResult<ActivityPolicyDto> {
    current_policy_row(pool).await.map(policy_to_dto)
}

pub(crate) async fn append_policy(
    pool: &PgPool,
    expected_version: i64,
    weights: &ActivityWeightsDto,
    reason: &str,
    changed_by: i64,
    changed_by_role: &str,
) -> AppResult<ActivityPolicyDto> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.score_policy'))")
        .execute(&mut *tx)
        .await?;
    let current_version: i64 = sqlx::query_scalar(
        "SELECT version FROM activity.score_policies ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await?;
    if current_version != expected_version {
        return Err(AppError::Conflict("activity policy version changed".into()));
    }

    let row = sqlx::query_as::<_, ScorePolicyRow>(
        "INSERT INTO activity.score_policies \
         (thread_weight, comment_weight, like_weight, reason, changed_by) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING version, thread_weight, comment_weight, like_weight, reason, \
                   changed_by, created_at",
    )
    .bind(weights.thread)
    .bind(weights.comment)
    .bind(weights.like)
    .bind(reason)
    .bind(changed_by)
    .fetch_one(&mut *tx)
    .await?;
    let metadata = serde_json::json!({
        "expectedVersion": expected_version,
        "weights": {
            "thread": weights.thread,
            "comment": weights.comment,
            "like": weights.like,
        },
    });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: changed_by, role: changed_by_role },
        "activity.policy.published",
        "activity_policy",
        &row.version.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(policy_to_dto(row))
}

pub(crate) async fn policy_history(
    pool: &PgPool,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<ActivityPolicyDto>> {
    let cursor_version = cursor.unwrap_or(i64::MAX);
    let fetch_limit = limit.clamp(1, 100) + 1;
    let rows = sqlx::query_as::<_, ScorePolicyRow>(
        "SELECT version, thread_weight, comment_weight, like_weight, reason, \
                changed_by, created_at \
         FROM activity.score_policies \
         WHERE version < $1 \
         ORDER BY version DESC \
         LIMIT $2",
    )
    .bind(cursor_version)
    .bind(fetch_limit)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() == fetch_limit as usize;
    let item_count = if has_more { rows.len() - 1 } else { rows.len() };
    let items: Vec<ActivityPolicyDto> =
        rows.into_iter().take(item_count).map(policy_to_dto).collect();
    let next_cursor =
        if has_more { items.last().map(|item| item.version.to_string()) } else { None };
    Ok(Page::new(items, next_cursor))
}

async fn current_policy_row(pool: &PgPool) -> AppResult<ScorePolicyRow> {
    let row = sqlx::query_as::<_, ScorePolicyRow>(
        "SELECT version, thread_weight, comment_weight, like_weight, reason, \
                changed_by, created_at \
         FROM activity.score_policies \
         ORDER BY version DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("activity policy is not seeded")))?;
    Ok(row)
}

fn weights_from_row(row: &ScorePolicyRow) -> ActivityWeightsDto {
    ActivityWeightsDto {
        thread: row.thread_weight,
        comment: row.comment_weight,
        like: row.like_weight,
    }
}

fn policy_to_dto(row: ScorePolicyRow) -> ActivityPolicyDto {
    ActivityPolicyDto {
        version: row.version,
        timezone: "Asia/Shanghai",
        weights: weights_from_row(&row),
        reason: row.reason,
        changed_by: row.changed_by.map_or_else(|| "system".into(), |id| id.to_string()),
        created_at: row.created_at.timestamp(),
    }
}

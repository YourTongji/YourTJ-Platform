//! Transaction-aware activity projection updates.
//!
//! A source relationship has at most one active positive event. Deactivation
//! appends a reversal event and decrements the positive event's original day,
//! so duplicate requests and like/vote toggles cannot inflate the heatmap.

use chrono::{DateTime, NaiveDate, Utc};
use shared::{AppError, AppResult};
use sqlx::PgConnection;

use crate::models::ActiveEventRow;

/// Public dimensions supported by the activity score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Thread,
    Comment,
    Like,
}

impl ActivityKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Thread => "thread",
            Self::Comment => "comment",
            Self::Like => "like",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionKind {
    Contribution(ActivityKind),
    CheckIn,
}

impl ProjectionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Contribution(kind) => kind.as_str(),
            Self::CheckIn => "check_in",
        }
    }
}

/// Activate a contribution inside the source mutation's transaction.
///
/// Returns `true` only when a new positive transition was projected. Repeating
/// the same activation while it is already active is an idempotent no-op.
pub async fn activate_contribution(
    connection: &mut PgConnection,
    account_id: i64,
    kind: ActivityKind,
    source_key: &str,
    occurred_at: DateTime<Utc>,
) -> AppResult<bool> {
    activate_projected_contribution(
        connection,
        account_id,
        ProjectionKind::Contribution(kind),
        source_key,
        occurred_at,
    )
    .await
}

pub(crate) async fn activate_check_in_contribution(
    connection: &mut PgConnection,
    account_id: i64,
    source_key: &str,
    occurred_at: DateTime<Utc>,
) -> AppResult<bool> {
    activate_projected_contribution(
        connection,
        account_id,
        ProjectionKind::CheckIn,
        source_key,
        occurred_at,
    )
    .await
}

async fn activate_projected_contribution(
    connection: &mut PgConnection,
    account_id: i64,
    kind: ProjectionKind,
    source_key: &str,
    occurred_at: DateTime<Utc>,
) -> AppResult<bool> {
    lock_contribution_source(connection, source_key).await?;

    if find_active_event(connection, source_key).await?.is_some() {
        return Ok(false);
    }
    crate::score_projection::lock_projection_shared(connection).await?;

    let generation: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(generation), 0)::int + 1 \
         FROM activity.events WHERE source_key = $1",
    )
    .bind(source_key)
    .fetch_one(&mut *connection)
    .await?;
    let activity_date = activity_date_at(connection, occurred_at).await?;
    let event_key = format!("{source_key}:{generation}:activate");

    sqlx::query(
        "INSERT INTO activity.events \
         (event_key, source_key, generation, account_id, kind, delta, activity_date, occurred_at) \
         VALUES ($1, $2, $3, $4, $5, 1, $6, $7)",
    )
    .bind(event_key)
    .bind(source_key)
    .bind(generation)
    .bind(account_id)
    .bind(kind.as_str())
    .bind(activity_date)
    .bind(occurred_at)
    .execute(&mut *connection)
    .await?;

    increment_daily_count(connection, account_id, activity_date, kind).await?;
    crate::score_projection::refresh_account(connection, account_id, activity_date).await?;
    Ok(true)
}

/// Deactivate a contribution inside the source mutation's transaction.
///
/// The reversal is assigned to the positive event's original activity date.
/// Returns `false` when the source relationship is already inactive.
pub async fn deactivate_contribution(
    connection: &mut PgConnection,
    source_key: &str,
    occurred_at: DateTime<Utc>,
) -> AppResult<bool> {
    lock_contribution_source(connection, source_key).await?;

    let Some(active_event) = find_active_event(connection, source_key).await? else {
        return Ok(false);
    };
    let kind = parse_kind(&active_event.kind)?;
    if kind == ProjectionKind::CheckIn {
        return Err(AppError::Internal(anyhow::anyhow!(
            "daily check-in activity facts are immutable"
        )));
    }
    crate::score_projection::lock_projection_shared(connection).await?;
    let event_key = format!("{source_key}:{}:deactivate", active_event.generation);

    sqlx::query(
        "INSERT INTO activity.events \
         (event_key, source_key, generation, account_id, kind, delta, activity_date, \
          occurred_at, reverses_event_id) \
         VALUES ($1, $2, $3, $4, $5, -1, $6, $7, $8)",
    )
    .bind(event_key)
    .bind(source_key)
    .bind(active_event.generation)
    .bind(active_event.account_id)
    .bind(kind.as_str())
    .bind(active_event.activity_date)
    .bind(occurred_at)
    .bind(active_event.id)
    .execute(&mut *connection)
    .await?;

    decrement_daily_count(connection, active_event.account_id, active_event.activity_date, kind)
        .await?;
    crate::score_projection::refresh_account(
        connection,
        active_event.account_id,
        active_event.activity_date,
    )
    .await?;
    Ok(true)
}

async fn activity_date_at(
    connection: &mut PgConnection,
    occurred_at: DateTime<Utc>,
) -> AppResult<NaiveDate> {
    let date = sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE 'Asia/Shanghai')::date")
        .bind(occurred_at)
        .fetch_one(connection)
        .await?;
    Ok(date)
}

/// Serialize state transitions for a source relationship in the active transaction.
pub async fn lock_contribution_source(
    connection: &mut PgConnection,
    source_key: &str,
) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(source_key)
        .execute(connection)
        .await?;
    Ok(())
}

async fn find_active_event(
    connection: &mut PgConnection,
    source_key: &str,
) -> AppResult<Option<ActiveEventRow>> {
    let row = sqlx::query_as::<_, ActiveEventRow>(
        "SELECT event.id, event.account_id, event.kind, event.generation, event.activity_date \
         FROM activity.events event \
         WHERE event.source_key = $1 AND event.delta = 1 \
           AND NOT EXISTS ( \
             SELECT 1 FROM activity.events reversal \
             WHERE reversal.reverses_event_id = event.id \
           ) \
         ORDER BY event.generation DESC \
         LIMIT 1",
    )
    .bind(source_key)
    .fetch_optional(connection)
    .await?;
    Ok(row)
}

async fn increment_daily_count(
    connection: &mut PgConnection,
    account_id: i64,
    activity_date: NaiveDate,
    kind: ProjectionKind,
) -> AppResult<()> {
    let statement = match kind {
        ProjectionKind::Contribution(ActivityKind::Thread) => {
            "INSERT INTO activity.daily_counts (account_id, activity_date, threads_created) \
             VALUES ($1, $2, 1) \
             ON CONFLICT (account_id, activity_date) DO UPDATE \
             SET threads_created = activity.daily_counts.threads_created + 1, updated_at = now()"
        }
        ProjectionKind::Contribution(ActivityKind::Comment) => {
            "INSERT INTO activity.daily_counts (account_id, activity_date, comments_created) \
             VALUES ($1, $2, 1) \
             ON CONFLICT (account_id, activity_date) DO UPDATE \
             SET comments_created = activity.daily_counts.comments_created + 1, updated_at = now()"
        }
        ProjectionKind::Contribution(ActivityKind::Like) => {
            "INSERT INTO activity.daily_counts (account_id, activity_date, likes_given) \
             VALUES ($1, $2, 1) \
             ON CONFLICT (account_id, activity_date) DO UPDATE \
             SET likes_given = activity.daily_counts.likes_given + 1, updated_at = now()"
        }
        ProjectionKind::CheckIn => {
            "INSERT INTO activity.daily_counts (account_id, activity_date, check_ins) \
             VALUES ($1, $2, 1) \
             ON CONFLICT (account_id, activity_date) DO UPDATE \
             SET check_ins = activity.daily_counts.check_ins + 1, updated_at = now()"
        }
    };

    sqlx::query(statement).bind(account_id).bind(activity_date).execute(connection).await?;
    Ok(())
}

async fn decrement_daily_count(
    connection: &mut PgConnection,
    account_id: i64,
    activity_date: NaiveDate,
    kind: ProjectionKind,
) -> AppResult<()> {
    let statement = match kind {
        ProjectionKind::Contribution(ActivityKind::Thread) => {
            "UPDATE activity.daily_counts \
             SET threads_created = threads_created - 1, updated_at = now() \
             WHERE account_id = $1 AND activity_date = $2 AND threads_created > 0"
        }
        ProjectionKind::Contribution(ActivityKind::Comment) => {
            "UPDATE activity.daily_counts \
             SET comments_created = comments_created - 1, updated_at = now() \
             WHERE account_id = $1 AND activity_date = $2 AND comments_created > 0"
        }
        ProjectionKind::Contribution(ActivityKind::Like) => {
            "UPDATE activity.daily_counts \
             SET likes_given = likes_given - 1, updated_at = now() \
             WHERE account_id = $1 AND activity_date = $2 AND likes_given > 0"
        }
        ProjectionKind::CheckIn => {
            "UPDATE activity.daily_counts \
             SET check_ins = check_ins - 1, updated_at = now() \
             WHERE account_id = $1 AND activity_date = $2 AND check_ins > 0"
        }
    };

    let affected = sqlx::query(statement)
        .bind(account_id)
        .bind(activity_date)
        .execute(connection)
        .await?
        .rows_affected();
    if affected != 1 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "activity projection is missing the count reversed by {source_kind}",
            source_kind = kind.as_str()
        )));
    }
    Ok(())
}

fn parse_kind(value: &str) -> AppResult<ProjectionKind> {
    match value {
        "thread" => Ok(ProjectionKind::Contribution(ActivityKind::Thread)),
        "comment" => Ok(ProjectionKind::Contribution(ActivityKind::Comment)),
        "like" => Ok(ProjectionKind::Contribution(ActivityKind::Like)),
        "check_in" => Ok(ProjectionKind::CheckIn),
        _ => Err(AppError::Internal(anyhow::anyhow!("unknown activity kind in database"))),
    }
}

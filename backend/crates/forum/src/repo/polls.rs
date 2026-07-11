//! Database access layer for polls.
//!
//! Polls are 1:1 with threads. Single-select votes replace the user's old vote;
//! multi-select votes are additive (UNIQUE handles duplicates).

use shared::AppResult;
use sqlx::{PgConnection, PgPool};

use crate::models::{PollOptionRow, PollRow};

/// A poll with its options, returned from `get_poll`.
#[derive(Debug, Clone)]
pub struct PollWithOptions {
    pub poll: PollRow,
    pub options: Vec<PollOptionRow>,
}

/// Insert a poll with its options in a single transaction.
///
/// Returns the new poll id.
pub async fn create_poll(
    pool: &PgPool,
    thread_id: i64,
    question: &str,
    multi_select: bool,
    closes_at: Option<chrono::DateTime<chrono::Utc>>,
    options: &[String],
) -> AppResult<i64> {
    let mut tx = pool.begin().await?;
    let poll_id =
        create_poll_tx(&mut tx, thread_id, question, multi_select, closes_at, options).await?;
    tx.commit().await?;
    Ok(poll_id)
}

/// Insert a poll and its options inside the caller's transaction.
pub(crate) async fn create_poll_tx(
    connection: &mut PgConnection,
    thread_id: i64,
    question: &str,
    multi_select: bool,
    closes_at: Option<chrono::DateTime<chrono::Utc>>,
    options: &[String],
) -> AppResult<i64> {
    let poll_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.polls (thread_id, question, multi_select, closes_at) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id",
    )
    .bind(thread_id)
    .bind(question)
    .bind(multi_select)
    .bind(closes_at)
    .fetch_one(&mut *connection)
    .await?;

    for (position, label) in options.iter().enumerate() {
        let position = i32::try_from(position).map_err(|error| {
            shared::AppError::Internal(anyhow::anyhow!("poll position overflow: {error}"))
        })?;
        sqlx::query(
            "INSERT INTO forum.poll_options (poll_id, position, label, vote_count) \
             VALUES ($1, $2, $3, 0)",
        )
        .bind(poll_id)
        .bind(position)
        .bind(label)
        .execute(&mut *connection)
        .await?;
    }

    Ok(poll_id)
}

/// Cast a vote for a poll option.
///
/// For single-select polls: any existing vote by this user for the same poll is
/// removed before inserting the new vote.
/// For multi-select polls: a simple UPSERT (UNIQUE constraint handles repeats).
///
/// The caller must provide the poll's `multi_select` flag and `poll_id` (resolved
/// from the option) for correctness.
pub async fn vote_option(
    pool: &PgPool,
    poll_id: i64,
    multi_select: bool,
    poll_option_id: i64,
    account_id: i64,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    if !multi_select {
        // Delete all existing votes by this user for any option of this poll.
        sqlx::query(
            "DELETE FROM forum.poll_votes pv \
             USING forum.poll_options po \
             WHERE pv.poll_option_id = po.id \
               AND po.poll_id = $1 \
               AND pv.account_id = $2",
        )
        .bind(poll_id)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    }

    // UPSERT — UNIQUE on (poll_option_id, account_id) handles repeat votes.
    sqlx::query(
        "INSERT INTO forum.poll_votes (poll_option_id, account_id) \
         VALUES ($1, $2) \
         ON CONFLICT (poll_option_id, account_id) DO NOTHING",
    )
    .bind(poll_option_id)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;

    // Recompute vote_count for the option.
    let new_count: i32 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.poll_votes WHERE poll_option_id = $1")
            .bind(poll_option_id)
            .fetch_one(&mut *tx)
            .await?;

    sqlx::query("UPDATE forum.poll_options SET vote_count = $1 WHERE id = $2")
        .bind(new_count)
        .bind(poll_option_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Get a poll by thread id, including all options.
pub async fn get_poll(pool: &PgPool, thread_id: i64) -> AppResult<Option<PollWithOptions>> {
    let poll: Option<PollRow> = sqlx::query_as::<_, PollRow>(
        "SELECT id, thread_id, question, multi_select, closes_at, created_at \
         FROM forum.polls WHERE thread_id = $1",
    )
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;

    match poll {
        Some(p) => {
            let options: Vec<PollOptionRow> = get_poll_results(pool, p.id).await?;
            Ok(Some(PollWithOptions { poll: p, options }))
        }
        None => Ok(None),
    }
}

/// Get a poll's id by thread_id.
pub async fn get_poll_id_by_thread(pool: &PgPool, thread_id: i64) -> AppResult<Option<i64>> {
    let id: Option<i64> = sqlx::query_scalar("SELECT id FROM forum.polls WHERE thread_id = $1")
        .bind(thread_id)
        .fetch_optional(pool)
        .await?;
    Ok(id)
}

/// Get poll options ordered by position.
pub async fn get_poll_results(pool: &PgPool, poll_id: i64) -> AppResult<Vec<PollOptionRow>> {
    let rows = sqlx::query_as::<_, PollOptionRow>(
        "SELECT id, poll_id, position, label, vote_count \
         FROM forum.poll_options \
         WHERE poll_id = $1 \
         ORDER BY position",
    )
    .bind(poll_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get the set of option ids that a user has voted for in a poll.
pub async fn get_voted_option_ids(
    pool: &PgPool,
    poll_id: i64,
    account_id: i64,
) -> AppResult<Vec<i64>> {
    let ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT pv.poll_option_id \
         FROM forum.poll_votes pv \
         JOIN forum.poll_options po ON po.id = pv.poll_option_id \
         WHERE po.poll_id = $1 AND pv.account_id = $2",
    )
    .bind(poll_id)
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(ids.into_iter().map(|r| r.0).collect())
}

/// Get a poll option row by id (for looking up poll_id from option_id).
pub async fn get_poll_option(pool: &PgPool, option_id: i64) -> AppResult<Option<PollOptionRow>> {
    let row = sqlx::query_as::<_, PollOptionRow>(
        "SELECT id, poll_id, position, label, vote_count \
         FROM forum.poll_options WHERE id = $1",
    )
    .bind(option_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Get a poll by id only while its parent thread remains publicly available.
pub async fn get_poll_by_id(pool: &PgPool, poll_id: i64) -> AppResult<Option<PollRow>> {
    let row = sqlx::query_as::<_, PollRow>(
        "SELECT poll.id, poll.thread_id, poll.question, poll.multi_select, \
                poll.closes_at, poll.created_at \
         FROM forum.polls poll \
         JOIN forum.threads thread ON thread.id = poll.thread_id \
         WHERE poll.id = $1 AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL",
    )
    .bind(poll_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

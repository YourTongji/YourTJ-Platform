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

/// Viewer selection after a poll vote mutation.
pub struct PollVoteOutcome {
    pub my_votes: Vec<i64>,
}

async fn lock_open_poll_option(
    connection: &mut PgConnection,
    poll_id: i64,
    poll_option_id: i64,
) -> AppResult<bool> {
    let poll: Option<(bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT poll.multi_select, poll.closes_at FROM forum.polls poll \
         JOIN forum.threads thread ON thread.id = poll.thread_id \
         WHERE poll.id = $1 AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL \
         FOR UPDATE OF poll, thread",
    )
    .bind(poll_id)
    .fetch_optional(&mut *connection)
    .await?;
    let (multi_select, closes_at) = poll.ok_or(shared::AppError::NotFound)?;
    if closes_at.is_some_and(|closes_at| closes_at <= chrono::Utc::now()) {
        return Err(shared::AppError::Conflict("poll is closed".into()));
    }
    sqlx::query_scalar::<_, i64>(
        "SELECT id FROM forum.poll_options WHERE id = $1 AND poll_id = $2 FOR UPDATE",
    )
    .bind(poll_option_id)
    .bind(poll_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| shared::AppError::BadRequest("option does not belong to this poll".into()))?;
    Ok(multi_select)
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

/// Cast a vote after locking and validating the open, visible parent poll.
pub async fn vote_option(
    pool: &PgPool,
    poll_id: i64,
    poll_option_id: i64,
    account_id: i64,
) -> AppResult<PollVoteOutcome> {
    let mut tx = pool.begin().await?;
    let multi_select = lock_open_poll_option(&mut tx, poll_id, poll_option_id).await?;
    let thread_author_id: Option<i64> = sqlx::query_scalar(
        "SELECT thread.author_id FROM forum.polls AS poll \
         JOIN forum.threads AS thread ON thread.id = poll.thread_id \
         WHERE poll.id = $1 FOR KEY SHARE OF poll, thread",
    )
    .bind(poll_id)
    .fetch_optional(&mut *tx)
    .await?;
    let thread_author_id = thread_author_id.ok_or(shared::AppError::NotFound)?;
    super::relationships::lock_pair_unblocked(&mut tx, account_id, thread_author_id).await?;

    if !multi_select {
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

    sqlx::query(
        "INSERT INTO forum.poll_votes (poll_option_id, account_id) \
         VALUES ($1, $2) \
         ON CONFLICT (poll_option_id, account_id) DO NOTHING",
    )
    .bind(poll_option_id)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE forum.poll_options option \
         SET vote_count = (SELECT COUNT(*)::int FROM forum.poll_votes vote \
                           WHERE vote.poll_option_id = option.id) \
         WHERE option.poll_id = $1",
    )
    .bind(poll_id)
    .execute(&mut *tx)
    .await?;

    let my_votes = sqlx::query_scalar(
        "SELECT vote.poll_option_id FROM forum.poll_votes vote \
         JOIN forum.poll_options option ON option.id = vote.poll_option_id \
         WHERE option.poll_id = $1 AND vote.account_id = $2 \
         ORDER BY option.position, option.id",
    )
    .bind(poll_id)
    .bind(account_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(PollVoteOutcome { my_votes })
}

/// Remove one poll selection after applying the same visibility and close checks as voting.
pub async fn remove_option_vote(
    pool: &PgPool,
    poll_id: i64,
    poll_option_id: i64,
    account_id: i64,
) -> AppResult<PollVoteOutcome> {
    let mut tx = pool.begin().await?;
    lock_open_poll_option(&mut tx, poll_id, poll_option_id).await?;

    sqlx::query(
        "DELETE FROM forum.poll_votes \
         WHERE poll_option_id = $1 AND account_id = $2",
    )
    .bind(poll_option_id)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE forum.poll_options option \
         SET vote_count = (SELECT COUNT(*)::int FROM forum.poll_votes vote \
                           WHERE vote.poll_option_id = option.id) \
         WHERE option.poll_id = $1",
    )
    .bind(poll_id)
    .execute(&mut *tx)
    .await?;
    let my_votes = sqlx::query_scalar(
        "SELECT vote.poll_option_id FROM forum.poll_votes vote \
         JOIN forum.poll_options option ON option.id = vote.poll_option_id \
         WHERE option.poll_id = $1 AND vote.account_id = $2 \
         ORDER BY option.position, option.id",
    )
    .bind(poll_id)
    .bind(account_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(PollVoteOutcome { my_votes })
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

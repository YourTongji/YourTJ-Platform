//! Transactional activity projection for forum content state changes.
//!
//! Every transition locks the parent thread first, then comments by id, then
//! contribution sources in a stable target/account order. This keeps parent
//! availability and descendant activity in one serialized state machine.
//! Vote mutations take the same content locks before their source lock, so the
//! ordered positive-vote set is stable without introducing an inverse vote-row lock.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection};

#[derive(Debug, FromRow)]
struct ThreadContributionRow {
    author_id: Option<i64>,
    created_at: DateTime<Utc>,
    is_visible: bool,
}

#[derive(Debug, FromRow)]
struct CommentContributionRow {
    id: i64,
    author_id: Option<i64>,
    created_at: DateTime<Utc>,
    is_visible: bool,
}

#[derive(Debug, FromRow)]
struct VoteContributionRow {
    post_type: String,
    post_id: i64,
    account_id: i64,
    updated_at: DateTime<Utc>,
    is_visible: bool,
}

/// Synchronize a thread and every descendant contribution after a state change.
pub async fn synchronize_thread_activity_subtree(
    connection: &mut PgConnection,
    thread_id: i64,
    transition_at: DateTime<Utc>,
) -> AppResult<()> {
    let thread = lock_thread_contribution(connection, thread_id).await?;
    let comments = lock_thread_comments(connection, thread_id, thread.is_visible).await?;

    synchronize_content_contribution(
        connection,
        thread.author_id,
        activity::contributions::ActivityKind::Thread,
        &format!("forum_thread:{thread_id}"),
        thread.created_at,
        thread.is_visible,
        transition_at,
    )
    .await?;

    for comment in comments {
        synchronize_content_contribution(
            connection,
            comment.author_id,
            activity::contributions::ActivityKind::Comment,
            &format!("forum_comment:{}", comment.id),
            comment.created_at,
            comment.is_visible,
            transition_at,
        )
        .await?;
    }

    let votes = positive_votes_for_thread(connection, thread_id, thread.is_visible).await?;
    synchronize_vote_contributions(connection, votes, transition_at).await
}

/// Synchronize one comment and its votes after a state change.
///
/// The parent is locked before the comment so a concurrent parent transition
/// cannot reactivate a child using a stale visibility snapshot.
pub async fn synchronize_comment_activity(
    connection: &mut PgConnection,
    comment_id: i64,
    transition_at: DateTime<Utc>,
) -> AppResult<()> {
    let thread_id: i64 = sqlx::query_scalar("SELECT thread_id FROM forum.comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(AppError::NotFound)?;
    let thread = lock_thread_contribution(connection, thread_id).await?;
    let comment =
        lock_comment_contribution(connection, thread_id, comment_id, thread.is_visible).await?;

    synchronize_content_contribution(
        connection,
        comment.author_id,
        activity::contributions::ActivityKind::Comment,
        &format!("forum_comment:{comment_id}"),
        comment.created_at,
        comment.is_visible,
        transition_at,
    )
    .await?;

    let votes = positive_votes_for_comment(connection, comment_id, comment.is_visible).await?;
    synchronize_vote_contributions(connection, votes, transition_at).await
}

async fn lock_thread_contribution(
    connection: &mut PgConnection,
    thread_id: i64,
) -> AppResult<ThreadContributionRow> {
    sqlx::query_as(
        "SELECT author_id, created_at, \
                status = 'visible' AND deleted_at IS NULL AND hidden_at IS NULL \
                  AND archived_at IS NULL AS is_visible \
         FROM forum.threads WHERE id = $1 FOR UPDATE",
    )
    .bind(thread_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn lock_thread_comments(
    connection: &mut PgConnection,
    thread_id: i64,
    parent_is_visible: bool,
) -> AppResult<Vec<CommentContributionRow>> {
    sqlx::query_as(
        "SELECT id, author_id, created_at, \
                $2 AND deleted_at IS NULL AND hidden_at IS NULL AS is_visible \
         FROM forum.comments WHERE thread_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(thread_id)
    .bind(parent_is_visible)
    .fetch_all(connection)
    .await
    .map_err(Into::into)
}

async fn lock_comment_contribution(
    connection: &mut PgConnection,
    thread_id: i64,
    comment_id: i64,
    parent_is_visible: bool,
) -> AppResult<CommentContributionRow> {
    sqlx::query_as(
        "SELECT id, author_id, created_at, \
                $3 AND deleted_at IS NULL AND hidden_at IS NULL AS is_visible \
         FROM forum.comments WHERE id = $1 AND thread_id = $2 FOR UPDATE",
    )
    .bind(comment_id)
    .bind(thread_id)
    .bind(parent_is_visible)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)
}

async fn positive_votes_for_thread(
    connection: &mut PgConnection,
    thread_id: i64,
    parent_is_visible: bool,
) -> AppResult<Vec<VoteContributionRow>> {
    sqlx::query_as(
        "SELECT vote.post_type, vote.post_id, vote.account_id, vote.updated_at, \
                CASE WHEN vote.post_type = 'thread' THEN $2 \
                     ELSE $2 AND comment.deleted_at IS NULL \
                       AND comment.hidden_at IS NULL END AS is_visible \
         FROM forum.votes vote \
         LEFT JOIN forum.comments comment \
           ON vote.post_type = 'comment' AND comment.id = vote.post_id \
         WHERE vote.value = 1 \
           AND ((vote.post_type = 'thread' AND vote.post_id = $1) \
             OR (vote.post_type = 'comment' AND comment.thread_id = $1)) \
         ORDER BY CASE vote.post_type WHEN 'thread' THEN 0 ELSE 1 END, \
                  vote.post_id, vote.account_id",
    )
    .bind(thread_id)
    .bind(parent_is_visible)
    .fetch_all(connection)
    .await
    .map_err(Into::into)
}

async fn positive_votes_for_comment(
    connection: &mut PgConnection,
    comment_id: i64,
    comment_is_visible: bool,
) -> AppResult<Vec<VoteContributionRow>> {
    sqlx::query_as(
        "SELECT post_type, post_id, account_id, updated_at, $2 AS is_visible \
         FROM forum.votes \
         WHERE value = 1 AND post_type = 'comment' AND post_id = $1 \
         ORDER BY post_id, account_id",
    )
    .bind(comment_id)
    .bind(comment_is_visible)
    .fetch_all(connection)
    .await
    .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)] // reason: a projection transition needs the complete immutable source fact
async fn synchronize_content_contribution(
    connection: &mut PgConnection,
    account_id: Option<i64>,
    kind: activity::contributions::ActivityKind,
    source_key: &str,
    source_created_at: DateTime<Utc>,
    is_visible: bool,
    transition_at: DateTime<Utc>,
) -> AppResult<()> {
    if is_visible {
        if let Some(account_id) = account_id {
            activity::contributions::activate_contribution(
                connection,
                account_id,
                kind,
                source_key,
                source_created_at,
            )
            .await?;
        }
    } else {
        activity::contributions::deactivate_contribution(connection, source_key, transition_at)
            .await?;
    }
    Ok(())
}

async fn synchronize_vote_contributions(
    connection: &mut PgConnection,
    votes: Vec<VoteContributionRow>,
    transition_at: DateTime<Utc>,
) -> AppResult<()> {
    for vote in votes {
        let source_key =
            format!("forum_vote:{}:{}:{}", vote.post_type, vote.post_id, vote.account_id);
        if vote.is_visible {
            activity::contributions::activate_contribution(
                connection,
                vote.account_id,
                activity::contributions::ActivityKind::Like,
                &source_key,
                vote.updated_at,
            )
            .await?;
        } else {
            activity::contributions::deactivate_contribution(
                connection,
                &source_key,
                transition_at,
            )
            .await?;
        }
    }
    Ok(())
}

use shared::AppResult;
use sqlx::{FromRow, PgConnection, PgPool};

#[derive(Debug, FromRow)]
struct VoteContributionRow {
    post_type: String,
    post_id: i64,
    account_id: i64,
    updated_at: chrono::DateTime<chrono::Utc>,
}

async fn positive_votes_for_target(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
    require_visible: bool,
) -> AppResult<Vec<VoteContributionRow>> {
    let query = match target_type {
        "thread" => {
            "SELECT vote.post_type, vote.post_id, vote.account_id, vote.updated_at \
             FROM forum.votes vote \
             LEFT JOIN forum.comments comment \
               ON vote.post_type = 'comment' AND comment.id = vote.post_id \
             WHERE vote.value = 1 \
               AND ((vote.post_type = 'thread' AND vote.post_id = $1) \
                    OR (vote.post_type = 'comment' AND comment.thread_id = $1)) \
               AND (NOT $2 OR ( \
                 (vote.post_type = 'thread' AND EXISTS ( \
                   SELECT 1 FROM forum.threads thread \
                   WHERE thread.id = vote.post_id AND thread.status = 'visible' \
                     AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
                     AND thread.archived_at IS NULL \
                 )) OR \
                 (vote.post_type = 'comment' AND EXISTS ( \
                   SELECT 1 FROM forum.comments visible_comment \
                   JOIN forum.threads thread ON thread.id = visible_comment.thread_id \
                   WHERE visible_comment.id = vote.post_id \
                     AND visible_comment.deleted_at IS NULL \
                     AND visible_comment.hidden_at IS NULL \
                     AND thread.status = 'visible' AND thread.deleted_at IS NULL \
                     AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
                 )) \
               ))"
        }
        "comment" => {
            "SELECT vote.post_type, vote.post_id, vote.account_id, vote.updated_at \
             FROM forum.votes vote \
             WHERE vote.value = 1 AND vote.post_type = 'comment' AND vote.post_id = $1 \
               AND (NOT $2 OR EXISTS ( \
                 SELECT 1 FROM forum.comments comment \
                 JOIN forum.threads thread ON thread.id = comment.thread_id \
                 WHERE comment.id = vote.post_id \
                   AND comment.deleted_at IS NULL AND comment.hidden_at IS NULL \
                   AND thread.status = 'visible' AND thread.deleted_at IS NULL \
                   AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
               ))"
        }
        _ => return Err(shared::AppError::BadRequest("target type must be thread/comment".into())),
    };
    sqlx::query_as(query)
        .bind(target_id)
        .bind(require_visible)
        .fetch_all(connection)
        .await
        .map_err(Into::into)
}

/// Remove heatmap credit for positive votes whose target became unavailable.
pub async fn deactivate_target_vote_contributions(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
    occurred_at: chrono::DateTime<chrono::Utc>,
) -> AppResult<()> {
    for vote in positive_votes_for_target(connection, target_type, target_id, false).await? {
        activity::contributions::deactivate_contribution(
            connection,
            &format!("forum_vote:{}:{}:{}", vote.post_type, vote.post_id, vote.account_id),
            occurred_at,
        )
        .await?;
    }
    Ok(())
}

/// Restore heatmap credit for current positive votes once their target is visible again.
pub async fn reactivate_target_vote_contributions(
    connection: &mut PgConnection,
    target_type: &str,
    target_id: i64,
) -> AppResult<()> {
    for vote in positive_votes_for_target(connection, target_type, target_id, true).await? {
        activity::contributions::activate_contribution(
            connection,
            vote.account_id,
            activity::contributions::ActivityKind::Like,
            &format!("forum_vote:{}:{}:{}", vote.post_type, vote.post_id, vote.account_id),
            vote.updated_at,
        )
        .await?;
    }
    Ok(())
}

/// Materialized vote result plus the transition needed for notifications.
pub struct VoteOutcome {
    pub vote_count: i32,
    pub post_author_id: Option<i64>,
    pub became_upvote: bool,
}

/// Vote on a thread or comment with one-vote-per-user.
///
/// Uses UPSERT on `forum.votes` so each account can only have one vote per post.
/// `post_type` must be "thread" or "comment". `value` is "up" (+1) or "down" (-1).
///
/// Returns the materialized count and transition metadata after this vote.
pub async fn vote_post(
    pool: &PgPool,
    post_type: &str,
    post_id: i64,
    account_id: i64,
    value: &str,
) -> AppResult<VoteOutcome> {
    let delta: i32 = match value {
        "up" => 1,
        "down" => -1,
        _ => return Err(shared::AppError::BadRequest("vote value must be 'up' or 'down'".into())),
    };

    if post_type != "thread" && post_type != "comment" {
        return Err(shared::AppError::BadRequest("post_type must be 'thread' or 'comment'".into()));
    }

    let mut tx = pool.begin().await?;
    let post_author_id: Option<i64> = if post_type == "thread" {
        sqlx::query_as::<_, (Option<i64>,)>(
            "SELECT author_id FROM forum.threads \
             WHERE id = $1 AND status = 'visible' \
               AND deleted_at IS NULL AND hidden_at IS NULL AND archived_at IS NULL \
             FOR UPDATE",
        )
        .bind(post_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(shared::AppError::NotFound)?
        .0
    } else {
        sqlx::query_as::<_, (Option<i64>,)>(
            "SELECT c.author_id FROM forum.comments c \
             JOIN forum.threads t ON t.id = c.thread_id \
             WHERE c.id = $1 \
               AND c.deleted_at IS NULL AND c.hidden_at IS NULL \
               AND t.status = 'visible' AND t.deleted_at IS NULL \
               AND t.hidden_at IS NULL AND t.archived_at IS NULL \
             FOR UPDATE OF c, t",
        )
        .bind(post_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(shared::AppError::NotFound)?
        .0
    };
    if post_author_id == Some(account_id) {
        return Err(shared::AppError::BadRequest("cannot vote on your own content".into()));
    }

    let source_key = format!("forum_vote:{post_type}:{post_id}:{account_id}");
    activity::contributions::lock_contribution_source(&mut tx, &source_key).await?;

    let previous_value: Option<i16> = sqlx::query_scalar(
        "SELECT value FROM forum.votes \
         WHERE post_type = $1 AND post_id = $2 AND account_id = $3 \
         FOR UPDATE",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;

    // UPSERT into forum.votes — same (post_type, post_id, account_id) → UPDATE value.
    sqlx::query(
        "INSERT INTO forum.votes (post_type, post_id, account_id, value) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (post_type, post_id, account_id) \
         DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(account_id)
    .bind(delta)
    .execute(&mut *tx)
    .await?;

    // Recompute the vote_count for the post by summing votes. `SUM(smallint)`
    // returns bigint in Postgres, so cast back to int to decode into i32.
    let new_vote_count: i32 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0)::int FROM forum.votes WHERE post_type = $1 AND post_id = $2",
    )
    .bind(post_type)
    .bind(post_id)
    .fetch_one(&mut *tx)
    .await?;

    // Update the post's materialised vote_count.
    if post_type == "thread" {
        sqlx::query("UPDATE forum.threads SET vote_count = $1 WHERE id = $2")
            .bind(new_vote_count)
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("UPDATE forum.comments SET vote_count = $1 WHERE id = $2")
            .bind(new_vote_count)
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
    }

    if delta == 1 && previous_value != Some(1) {
        activity::contributions::activate_contribution(
            &mut tx,
            account_id,
            activity::contributions::ActivityKind::Like,
            &source_key,
            chrono::Utc::now(),
        )
        .await?;
    } else if delta == -1 && previous_value == Some(1) {
        activity::contributions::deactivate_contribution(&mut tx, &source_key, chrono::Utc::now())
            .await?;
    }

    if previous_value.is_none() {
        sqlx::query(
            "INSERT INTO forum.user_stats (account_id, votes_cast) VALUES ($1, 1) \
             ON CONFLICT (account_id) DO UPDATE \
             SET votes_cast = forum.user_stats.votes_cast + 1, updated_at = now()",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    }
    let received_delta = match (previous_value, delta) {
        (Some(1), -1) => -1,
        (Some(-1), 1) | (None, 1) => 1,
        _ => 0,
    };
    if let Some(author_id) = post_author_id.filter(|_| received_delta != 0) {
        sqlx::query(
            "INSERT INTO forum.user_stats (account_id, votes_received) VALUES ($1, $2) \
             ON CONFLICT (account_id) DO UPDATE \
             SET votes_received = GREATEST(forum.user_stats.votes_received + $2, 0), \
                 updated_at = now()",
        )
        .bind(author_id)
        .bind(received_delta)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(VoteOutcome {
        vote_count: new_vote_count,
        post_author_id,
        became_upvote: delta == 1 && previous_value != Some(1),
    })
}

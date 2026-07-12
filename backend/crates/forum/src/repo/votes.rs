use shared::AppResult;
use sqlx::{PgConnection, PgPool};

struct LockedVoteTarget {
    post_author_id: Option<i64>,
    thread_id: i64,
    board_id: i64,
}

async fn lock_vote_target(
    connection: &mut PgConnection,
    post_type: &str,
    post_id: i64,
) -> AppResult<LockedVoteTarget> {
    if post_type == "thread" {
        let (post_author_id, thread_id, board_id) = sqlx::query_as(
            "SELECT author_id, id, board_id FROM forum.threads \
             WHERE id = $1 AND status = 'visible' \
               AND deleted_at IS NULL AND hidden_at IS NULL AND archived_at IS NULL \
             FOR UPDATE",
        )
        .bind(post_id)
        .fetch_optional(connection)
        .await?
        .ok_or(shared::AppError::NotFound)?;
        return Ok(LockedVoteTarget { post_author_id, thread_id, board_id });
    }

    let thread_id: i64 = sqlx::query_scalar("SELECT thread_id FROM forum.comments WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&mut *connection)
        .await?
        .ok_or(shared::AppError::NotFound)?;
    let board_id: i64 = sqlx::query_scalar(
        "SELECT board_id FROM forum.threads \
         WHERE id = $1 AND status = 'visible' \
           AND deleted_at IS NULL AND hidden_at IS NULL AND archived_at IS NULL \
         FOR UPDATE",
    )
    .bind(thread_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    let post_author_id: Option<i64> = sqlx::query_scalar(
        "SELECT author_id FROM forum.comments \
         WHERE id = $1 AND thread_id = $2 \
           AND deleted_at IS NULL AND hidden_at IS NULL \
         FOR UPDATE",
    )
    .bind(post_id)
    .bind(thread_id)
    .fetch_optional(connection)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    Ok(LockedVoteTarget { post_author_id, thread_id, board_id })
}

/// Materialized vote result plus the transition needed for notifications.
pub struct VoteOutcome {
    pub vote_count: i32,
    pub post_author_id: Option<i64>,
    pub became_upvote: bool,
    pub viewer_vote: Option<String>,
    pub thread_id: i64,
    pub board_id: i64,
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
    let target = lock_vote_target(&mut tx, post_type, post_id).await?;
    let LockedVoteTarget { post_author_id, thread_id, board_id } = target;
    if post_author_id == Some(account_id) {
        return Err(shared::AppError::BadRequest("cannot vote on your own content".into()));
    }
    if let Some(post_author_id) = post_author_id {
        super::relationships::lock_pair_unblocked(&mut tx, account_id, post_author_id).await?;
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
    let vote_updated_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO forum.votes (post_type, post_id, account_id, value) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (post_type, post_id, account_id) \
         DO UPDATE SET value = EXCLUDED.value, updated_at = now() \
         RETURNING updated_at",
    )
    .bind(post_type)
    .bind(post_id)
    .bind(account_id)
    .bind(delta)
    .fetch_one(&mut *tx)
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

    if delta == 1 && previous_value != Some(1) {
        if let Some(author_id) = post_author_id {
            platform::outbox::enqueue_notification_tx(
                &mut tx,
                &format!(
                    "forum-vote:{post_type}:{post_id}:{account_id}:{}",
                    vote_updated_at.timestamp_micros()
                ),
                author_id,
                Some(account_id),
                "vote",
                &serde_json::json!({
                    "postType": post_type,
                    "postId": post_id.to_string(),
                    "threadId": thread_id.to_string(),
                    "voterId": account_id.to_string(),
                    "voteUpdatedAtMicros": vote_updated_at.timestamp_micros().to_string(),
                    "title": "你的内容获得了赞同",
                }),
                Some(&format!("vote:{post_type}:{post_id}")),
                None,
            )
            .await?;
        }
    }

    tx.commit().await?;

    Ok(VoteOutcome {
        vote_count: new_vote_count,
        post_author_id,
        became_upvote: delta == 1 && previous_value != Some(1),
        viewer_vote: Some(value.to_owned()),
        thread_id,
        board_id,
    })
}

/// Remove the account's vote while keeping materialized counts and activity consistent.
pub async fn remove_vote(
    pool: &PgPool,
    post_type: &str,
    post_id: i64,
    account_id: i64,
) -> AppResult<VoteOutcome> {
    if !matches!(post_type, "thread" | "comment") {
        return Err(shared::AppError::BadRequest("postType must be thread/comment".into()));
    }

    let mut tx = pool.begin().await?;
    let target = lock_vote_target(&mut tx, post_type, post_id).await?;
    let LockedVoteTarget { post_author_id, thread_id, board_id } = target;

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

    if previous_value.is_some() {
        sqlx::query(
            "DELETE FROM forum.votes \
             WHERE post_type = $1 AND post_id = $2 AND account_id = $3",
        )
        .bind(post_type)
        .bind(post_id)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE forum.user_stats \
             SET votes_cast = GREATEST(votes_cast - 1, 0), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    }

    if previous_value == Some(1) {
        activity::contributions::deactivate_contribution(&mut tx, &source_key, chrono::Utc::now())
            .await?;
        if let Some(author_id) = post_author_id {
            sqlx::query(
                "UPDATE forum.user_stats \
                 SET votes_received = GREATEST(votes_received - 1, 0), updated_at = now() \
                 WHERE account_id = $1",
            )
            .bind(author_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    let vote_count: i32 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0)::int FROM forum.votes \
         WHERE post_type = $1 AND post_id = $2",
    )
    .bind(post_type)
    .bind(post_id)
    .fetch_one(&mut *tx)
    .await?;
    let update_query = if post_type == "thread" {
        "UPDATE forum.threads SET vote_count = $1 WHERE id = $2"
    } else {
        "UPDATE forum.comments SET vote_count = $1 WHERE id = $2"
    };
    sqlx::query(update_query).bind(vote_count).bind(post_id).execute(&mut *tx).await?;
    tx.commit().await?;

    Ok(VoteOutcome {
        vote_count,
        post_author_id,
        became_upvote: false,
        viewer_vote: None,
        thread_id,
        board_id,
    })
}

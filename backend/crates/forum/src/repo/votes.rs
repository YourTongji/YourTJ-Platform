use shared::AppResult;
use sqlx::PgPool;

/// Vote on a thread or comment with one-vote-per-user.
///
/// Uses UPSERT on `forum.votes` so each account can only have one vote per post.
/// `post_type` must be "thread" or "comment". `value` is "up" (+1) or "down" (-1).
///
/// Returns the new vote count for the post after this vote.
pub async fn vote_post(
    pool: &PgPool,
    post_type: &str,
    post_id: i64,
    account_id: i64,
    value: &str,
) -> AppResult<i32> {
    let delta: i32 = match value {
        "up" => 1,
        "down" => -1,
        _ => return Err(shared::AppError::BadRequest("vote value must be 'up' or 'down'".into())),
    };

    if post_type != "thread" && post_type != "comment" {
        return Err(shared::AppError::BadRequest("post_type must be 'thread' or 'comment'".into()));
    }

    // Validate that the post exists.
    let exists: bool = if post_type == "thread" {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.threads WHERE id = $1)")
            .bind(post_id)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.comments WHERE id = $1)")
            .bind(post_id)
            .fetch_one(pool)
            .await?
    };

    if !exists {
        return Err(shared::AppError::NotFound);
    }

    let mut tx = pool.begin().await?;

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

    // Recompute the vote_count for the post by summing votes.
    let new_vote_count: i32 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(value), 0) FROM forum.votes WHERE post_type = $1 AND post_id = $2",
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

    tx.commit().await?;

    Ok(new_vote_count)
}

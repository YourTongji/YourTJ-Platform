//! Follow, mute, and block persistence with pair-serialized safety invariants.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

/// Current directional relationship facts between a viewer and a target.
#[derive(Debug, Clone, Copy, Default, FromRow)]
pub struct RelationshipState {
    pub following: bool,
    pub followed_by: bool,
    pub muted: bool,
    pub blocked_by_me: bool,
    pub blocked_me: bool,
}

/// Materialized follow counts maintained by the database trigger.
#[derive(Debug, Clone, Copy, Default, FromRow)]
pub struct SocialCounts {
    pub follower_count: i32,
    pub following_count: i32,
}

/// One relationship-list row before identity visibility filtering.
#[derive(Debug, Clone, FromRow)]
pub struct FollowListRow {
    pub account_id: i64,
    pub followed_at: DateTime<Utc>,
}

async fn lock_pair(
    tx: &mut Transaction<'_, Postgres>,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<()> {
    let account_low_id = account_id_a.min(account_id_b);
    let account_high_id = account_id_a.max(account_id_b);
    let lock_key = format!("forum-social:{account_low_id}:{account_high_id}");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(lock_key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Serialize a direct interaction with block mutation and reject either block direction.
pub(crate) async fn lock_pair_unblocked(
    tx: &mut Transaction<'_, Postgres>,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<()> {
    if account_id_a == account_id_b {
        return Ok(());
    }
    lock_pair(tx, account_id_a, account_id_b).await?;
    let is_blocked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM forum.user_ignores \
         WHERE (account_id = $1 AND ignored_account_id = $2) \
            OR (account_id = $2 AND ignored_account_id = $1))",
    )
    .bind(account_id_a)
    .bind(account_id_b)
    .fetch_one(&mut **tx)
    .await?;
    if is_blocked {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Return every first-phase relationship flag in one query.
pub async fn get_relationship(
    pool: &PgPool,
    viewer_id: i64,
    target_id: i64,
) -> AppResult<RelationshipState> {
    let state = sqlx::query_as::<_, RelationshipState>(
        "SELECT \
           EXISTS(SELECT 1 FROM forum.user_follows \
                  WHERE follower_id = $1 AND followed_id = $2) AS following, \
           EXISTS(SELECT 1 FROM forum.user_follows \
                  WHERE follower_id = $2 AND followed_id = $1) AS followed_by, \
           EXISTS(SELECT 1 FROM forum.user_mutes \
                  WHERE account_id = $1 AND muted_account_id = $2) AS muted, \
           EXISTS(SELECT 1 FROM forum.user_ignores \
                  WHERE account_id = $1 AND ignored_account_id = $2) AS blocked_by_me, \
           EXISTS(SELECT 1 FROM forum.user_ignores \
                  WHERE account_id = $2 AND ignored_account_id = $1) AS blocked_me",
    )
    .bind(viewer_id)
    .bind(target_id)
    .fetch_one(pool)
    .await?;
    Ok(state)
}

/// Return whether either side has created a block boundary.
pub async fn pair_is_blocked(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<bool> {
    let blocked = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM forum.user_ignores \
           WHERE (account_id = $1 AND ignored_account_id = $2) \
              OR (account_id = $2 AND ignored_account_id = $1) \
         )",
    )
    .bind(account_id_a)
    .bind(account_id_b)
    .fetch_one(pool)
    .await?;
    Ok(blocked)
}

/// Return whether the first account follows the second.
pub async fn is_following(pool: &PgPool, follower_id: i64, followed_id: i64) -> AppResult<bool> {
    let follows = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.user_follows \
         WHERE follower_id = $1 AND followed_id = $2)",
    )
    .bind(follower_id)
    .bind(followed_id)
    .fetch_one(pool)
    .await?;
    Ok(follows)
}

/// Follow idempotently, serialized against block creation for the same pair.
pub async fn follow(pool: &PgPool, follower_id: i64, followed_id: i64) -> AppResult<bool> {
    if follower_id == followed_id {
        return Err(AppError::BadRequest("cannot follow yourself".into()));
    }
    let mut tx = pool.begin().await?;
    lock_pair_unblocked(&mut tx, follower_id, followed_id).await?;
    let inserted = sqlx::query(
        "INSERT INTO forum.user_follows (follower_id, followed_id) VALUES ($1, $2) \
         ON CONFLICT (follower_id, followed_id) DO NOTHING",
    )
    .bind(follower_id)
    .bind(followed_id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    tx.commit().await?;
    Ok(inserted)
}

/// Unfollow idempotently while preserving trigger-maintained counts.
pub async fn unfollow(pool: &PgPool, follower_id: i64, followed_id: i64) -> AppResult<()> {
    if follower_id == followed_id {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    lock_pair(&mut tx, follower_id, followed_id).await?;
    sqlx::query("DELETE FROM forum.user_follows WHERE follower_id = $1 AND followed_id = $2")
        .bind(follower_id)
        .bind(followed_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Remove one incoming follower idempotently while preserving trigger-maintained counts.
pub async fn remove_follower(pool: &PgPool, account_id: i64, follower_id: i64) -> AppResult<()> {
    unfollow(pool, follower_id, account_id).await
}

/// Mute one account without changing follow, access, or interaction permissions.
pub async fn mute(pool: &PgPool, account_id: i64, muted_account_id: i64) -> AppResult<()> {
    if account_id == muted_account_id {
        return Err(AppError::BadRequest("cannot mute yourself".into()));
    }
    sqlx::query(
        "INSERT INTO forum.user_mutes (account_id, muted_account_id) VALUES ($1, $2) \
         ON CONFLICT (account_id, muted_account_id) DO NOTHING",
    )
    .bind(account_id)
    .bind(muted_account_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Unmute one account idempotently.
pub async fn unmute(pool: &PgPool, account_id: i64, muted_account_id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM forum.user_mutes WHERE account_id = $1 AND muted_account_id = $2")
        .bind(account_id)
        .bind(muted_account_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Block one account and atomically remove follows in both directions.
pub async fn block(pool: &PgPool, account_id: i64, blocked_account_id: i64) -> AppResult<()> {
    if account_id == blocked_account_id {
        return Err(AppError::BadRequest("cannot block yourself".into()));
    }
    let mut tx = pool.begin().await?;
    lock_pair(&mut tx, account_id, blocked_account_id).await?;
    sqlx::query(
        "INSERT INTO forum.user_ignores (account_id, ignored_account_id) VALUES ($1, $2) \
         ON CONFLICT (account_id, ignored_account_id) DO NOTHING",
    )
    .bind(account_id)
    .bind(blocked_account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM forum.user_follows \
         WHERE (follower_id = $1 AND followed_id = $2) \
            OR (follower_id = $2 AND followed_id = $1)",
    )
    .bind(account_id)
    .bind(blocked_account_id)
    .execute(&mut *tx)
    .await?;
    let declined_requests: Vec<i64> = sqlx::query_scalar(
        "UPDATE forum.dm_conversations \
         SET request_status = 'declined', responded_at = now(), \
             request_cooldown_until = now() + interval '30 days' \
         WHERE account_low_id = LEAST($1, $2) AND account_high_id = GREATEST($1, $2) \
           AND request_status = 'pending' \
         RETURNING id",
    )
    .bind(account_id)
    .bind(blocked_account_id)
    .fetch_all(&mut *tx)
    .await?;
    if !declined_requests.is_empty() {
        sqlx::query(
            "DELETE FROM forum.dm_messages AS message \
             WHERE message.conversation_id = ANY($1) \
               AND NOT EXISTS (SELECT 1 FROM forum.dm_message_reports AS report \
                               WHERE report.message_id = message.id)",
        )
        .bind(&declined_requests)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Unblock one account without restoring prior follows.
pub async fn unblock(pool: &PgPool, account_id: i64, blocked_account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    lock_pair(&mut tx, account_id, blocked_account_id).await?;
    sqlx::query("DELETE FROM forum.user_ignores WHERE account_id = $1 AND ignored_account_id = $2")
        .bind(account_id)
        .bind(blocked_account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Return trigger-maintained social counts, defaulting missing projections to zero.
pub async fn get_social_counts(pool: &PgPool, account_id: i64) -> AppResult<SocialCounts> {
    let counts = sqlx::query_as::<_, SocialCounts>(
        "SELECT follower_count, following_count FROM forum.user_social_stats \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default();
    Ok(counts)
}

/// List raw follower ids using a stable descending account-id cursor.
pub async fn list_follower_ids(
    pool: &PgPool,
    account_id: i64,
    viewer_id: Option<i64>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<FollowListRow>, Option<i64>)> {
    list_ids(pool, account_id, viewer_id, cursor, limit, true).await
}

/// List raw followed-account ids using a stable descending account-id cursor.
pub async fn list_following_ids(
    pool: &PgPool,
    account_id: i64,
    viewer_id: Option<i64>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<FollowListRow>, Option<i64>)> {
    list_ids(pool, account_id, viewer_id, cursor, limit, false).await
}

async fn list_ids(
    pool: &PgPool,
    account_id: i64,
    viewer_id: Option<i64>,
    cursor: Option<i64>,
    limit: i64,
    lists_followers: bool,
) -> AppResult<(Vec<FollowListRow>, Option<i64>)> {
    let page_size = limit.clamp(1, 100);
    let query = if lists_followers {
        "SELECT relation.follower_id AS account_id, relation.created_at AS followed_at \
         FROM forum.user_follows AS relation \
         WHERE relation.followed_id = $1 \
           AND ($2::bigint IS NULL OR relation.follower_id < $2) \
           AND ($3::bigint IS NULL OR NOT EXISTS( \
             SELECT 1 FROM forum.user_ignores AS block \
             WHERE (block.account_id = $3 AND block.ignored_account_id = relation.follower_id) \
                OR (block.account_id = relation.follower_id AND block.ignored_account_id = $3) \
           )) \
         ORDER BY relation.follower_id DESC LIMIT $4"
    } else {
        "SELECT relation.followed_id AS account_id, relation.created_at AS followed_at \
         FROM forum.user_follows AS relation \
         WHERE relation.follower_id = $1 \
           AND ($2::bigint IS NULL OR relation.followed_id < $2) \
           AND ($3::bigint IS NULL OR NOT EXISTS( \
             SELECT 1 FROM forum.user_ignores AS block \
             WHERE (block.account_id = $3 AND block.ignored_account_id = relation.followed_id) \
                OR (block.account_id = relation.followed_id AND block.ignored_account_id = $3) \
           )) \
         ORDER BY relation.followed_id DESC LIMIT $4"
    };
    let mut rows = sqlx::query_as::<_, FollowListRow>(query)
        .bind(account_id)
        .bind(cursor)
        .bind(viewer_id)
        .bind(page_size + 1)
        .fetch_all(pool)
        .await?;
    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.account_id)).flatten();
    Ok((rows, next_cursor))
}

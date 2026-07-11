//! Public community-profile reads owned by the forum domain.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Default, FromRow)]
pub struct PublicProfileStats {
    pub thread_count: i32,
    pub comment_count: i32,
    pub votes_received: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct PublicUserThreadRow {
    pub id: i64,
    pub title: String,
    pub board_slug: String,
    pub reply_count: i32,
    pub vote_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PublicUserCommentRow {
    pub id: i64,
    pub thread_id: i64,
    pub thread_title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// Return the lifetime forum counters shown on a public community profile.
pub async fn get_public_profile_stats(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<PublicProfileStats> {
    let stats = sqlx::query_as::<_, PublicProfileStats>(
        "SELECT COALESCE(threads_created, 0) AS thread_count, \
                COALESCE(comments_created, 0) AS comment_count, \
                COALESCE(votes_received, 0) AS votes_received \
         FROM forum.user_stats WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default();
    Ok(stats)
}

/// List an author's publicly visible, non-archived threads with id cursor pagination.
pub async fn list_public_user_threads(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<PublicUserThreadRow>, Option<i64>)> {
    let page_size = limit.clamp(1, 100);
    let mut rows = sqlx::query_as::<_, PublicUserThreadRow>(
        "SELECT thread.id, thread.title, COALESCE(board.slug, '') AS board_slug, \
                thread.reply_count, thread.vote_count, thread.created_at \
         FROM forum.threads thread \
         JOIN forum.boards board ON board.id = thread.board_id \
         WHERE thread.author_id = $1 \
           AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL \
           AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL \
           AND ($2::bigint IS NULL OR thread.id < $2) \
         ORDER BY thread.id DESC LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.id)).flatten();
    Ok((rows, next_cursor))
}

/// List an author's comments only when both comment and parent thread remain public.
pub async fn list_public_user_comments(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<PublicUserCommentRow>, Option<i64>)> {
    let page_size = limit.clamp(1, 100);
    let mut rows = sqlx::query_as::<_, PublicUserCommentRow>(
        "SELECT comment.id, comment.thread_id, COALESCE(thread.title, '') AS thread_title, \
                COALESCE(LEFT(comment.body, 200), '') AS body, comment.created_at \
         FROM forum.comments comment \
         JOIN forum.threads thread ON thread.id = comment.thread_id \
         WHERE comment.author_id = $1 \
           AND comment.deleted_at IS NULL \
           AND comment.hidden_at IS NULL \
           AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL \
           AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL \
           AND ($2::bigint IS NULL OR comment.id < $2) \
         ORDER BY comment.id DESC LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.id)).flatten();
    Ok((rows, next_cursor))
}

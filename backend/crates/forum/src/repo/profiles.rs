//! Public community-profile reads owned by the forum domain.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
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
    pub body: Option<String>,
    pub content_format: String,
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
    pub content_format: String,
    pub reply_count: i32,
    pub vote_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProfileContentRow {
    pub target_type: String,
    pub id: i64,
    pub thread_id: i64,
    pub title: String,
    pub body: Option<String>,
    pub content_format: String,
    pub board_slug: String,
    pub author_id: i64,
    pub reply_count: i32,
    pub vote_count: i32,
    pub created_at: DateTime<Utc>,
    pub activity_at: DateTime<Utc>,
}

fn encode_profile_content_cursor(row: &ProfileContentRow) -> String {
    super::base64_encode_str(&format!(
        "{}|{}|{}",
        row.activity_at.timestamp_micros(),
        row.target_type,
        row.id
    ))
}

fn decode_profile_content_cursor(cursor: &str) -> AppResult<(DateTime<Utc>, String, i64)> {
    let decoded = super::base64_decode_str(cursor)
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let mut parts = decoded.split('|');
    let micros = parts
        .next()
        .and_then(|part| part.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    let target_type = parts
        .next()
        .filter(|target_type| matches!(*target_type, "thread" | "comment"))
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?
        .to_owned();
    let target_id = parts
        .next()
        .and_then(|part| part.parse::<i64>().ok())
        .filter(|target_id| *target_id > 0)
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    if parts.next().is_some() {
        return Err(AppError::BadRequest("invalid cursor".into()));
    }
    let timestamp = DateTime::from_timestamp_micros(micros)
        .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))?;
    Ok((timestamp, target_type, target_id))
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
        "SELECT thread.id, thread.title, thread.body, thread.content_format, \
                COALESCE(board.slug, '') AS board_slug, \
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
                comment.body, comment.content_format, comment.reply_count, comment.vote_count, \
                comment.created_at \
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

/// List an account's positive Forum votes over content still visible to the current viewer.
pub async fn list_public_user_likes(
    pool: &PgPool,
    account_id: i64,
    viewer_id: Option<i64>,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ProfileContentRow>, Option<String>)> {
    let (cursor_at, cursor_type, cursor_id) = match cursor {
        Some(cursor) => {
            let (timestamp, target_type, target_id) = decode_profile_content_cursor(cursor)?;
            (Some(timestamp), Some(target_type), Some(target_id))
        }
        None => (None, None, None),
    };
    let page_size = limit.clamp(1, 100);
    let mut rows = sqlx::query_as::<_, ProfileContentRow>(
        "SELECT content.* FROM ( \
           SELECT 'thread'::text AS target_type, thread.id, thread.id AS thread_id, \
                  thread.title, thread.body, thread.content_format, \
                  COALESCE(board.slug, '') AS board_slug, thread.author_id, \
                  thread.reply_count, thread.vote_count, thread.created_at, \
                  vote.updated_at AS activity_at \
           FROM forum.votes vote \
           JOIN forum.threads thread ON vote.post_type = 'thread' AND thread.id = vote.post_id \
           JOIN forum.boards board ON board.id = thread.board_id \
           WHERE vote.account_id = $1 AND vote.value = 1 \
             AND thread.status = 'visible' AND thread.deleted_at IS NULL \
             AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
             AND ($2::bigint IS NULL OR thread.author_id = $2 OR ( \
               NOT EXISTS (SELECT 1 FROM forum.user_ignores ignored \
                           WHERE (ignored.account_id = $2 AND ignored.ignored_account_id = thread.author_id) \
                              OR (ignored.account_id = thread.author_id AND ignored.ignored_account_id = $2)) \
               AND NOT EXISTS (SELECT 1 FROM forum.user_mutes muted \
                               WHERE muted.account_id = $2 AND muted.muted_account_id = thread.author_id) \
             )) \
           UNION ALL \
           SELECT 'comment'::text AS target_type, comment.id, comment.thread_id, \
                  thread.title, comment.body, comment.content_format, \
                  COALESCE(board.slug, '') AS board_slug, comment.author_id, \
                  comment.reply_count, comment.vote_count, comment.created_at, \
                  vote.updated_at AS activity_at \
           FROM forum.votes vote \
           JOIN forum.comments comment ON vote.post_type = 'comment' AND comment.id = vote.post_id \
           JOIN forum.threads thread ON thread.id = comment.thread_id \
           JOIN forum.boards board ON board.id = thread.board_id \
           WHERE vote.account_id = $1 AND vote.value = 1 \
             AND comment.deleted_at IS NULL AND comment.hidden_at IS NULL \
             AND thread.status = 'visible' AND thread.deleted_at IS NULL \
             AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
             AND ($2::bigint IS NULL OR comment.author_id = $2 OR ( \
               NOT EXISTS (SELECT 1 FROM forum.user_ignores ignored \
                           WHERE (ignored.account_id = $2 AND ignored.ignored_account_id = comment.author_id) \
                              OR (ignored.account_id = comment.author_id AND ignored.ignored_account_id = $2)) \
               AND NOT EXISTS (SELECT 1 FROM forum.user_mutes muted \
                               WHERE muted.account_id = $2 AND muted.muted_account_id = comment.author_id) \
             )) \
         ) content \
         WHERE ($3::timestamptz IS NULL OR \
                (content.activity_at, content.target_type, content.id) < ($3, $4, $5)) \
         ORDER BY content.activity_at DESC, content.target_type DESC, content.id DESC \
         LIMIT $6",
    )
    .bind(account_id)
    .bind(viewer_id)
    .bind(cursor_at)
    .bind(cursor_type)
    .bind(cursor_id)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(encode_profile_content_cursor)).flatten();
    Ok((rows, next_cursor))
}

/// List authored content that declares a platform image; Media still performs final disclosure.
pub async fn list_public_user_media_candidates(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<(Vec<ProfileContentRow>, Option<String>)> {
    let (cursor_at, cursor_type, cursor_id) = match cursor {
        Some(cursor) => {
            let (timestamp, target_type, target_id) = decode_profile_content_cursor(cursor)?;
            (Some(timestamp), Some(target_type), Some(target_id))
        }
        None => (None, None, None),
    };
    let page_size = limit.clamp(1, 100);
    let mut rows = sqlx::query_as::<_, ProfileContentRow>(
        "SELECT content.* FROM ( \
           SELECT 'thread'::text AS target_type, thread.id, thread.id AS thread_id, \
                  thread.title, thread.body, thread.content_format, \
                  COALESCE(board.slug, '') AS board_slug, thread.author_id, \
                  thread.reply_count, thread.vote_count, thread.created_at, \
                  thread.created_at AS activity_at \
           FROM forum.threads thread \
           JOIN forum.boards board ON board.id = thread.board_id \
           WHERE thread.author_id = $1 AND thread.content_format = 'markdown_v1' \
             AND position('yourtj-asset:' IN COALESCE(thread.body, '')) > 0 \
             AND thread.status = 'visible' AND thread.deleted_at IS NULL \
             AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
           UNION ALL \
           SELECT 'comment'::text AS target_type, comment.id, comment.thread_id, \
                  thread.title, comment.body, comment.content_format, \
                  COALESCE(board.slug, '') AS board_slug, comment.author_id, \
                  comment.reply_count, comment.vote_count, comment.created_at, \
                  comment.created_at AS activity_at \
           FROM forum.comments comment \
           JOIN forum.threads thread ON thread.id = comment.thread_id \
           JOIN forum.boards board ON board.id = thread.board_id \
           WHERE comment.author_id = $1 AND comment.content_format = 'markdown_v1' \
             AND position('yourtj-asset:' IN comment.body) > 0 \
             AND comment.deleted_at IS NULL AND comment.hidden_at IS NULL \
             AND thread.status = 'visible' AND thread.deleted_at IS NULL \
             AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
         ) content \
         WHERE ($2::timestamptz IS NULL OR \
                (content.activity_at, content.target_type, content.id) < ($2, $3, $4)) \
         ORDER BY content.activity_at DESC, content.target_type DESC, content.id DESC \
         LIMIT $5",
    )
    .bind(account_id)
    .bind(cursor_at)
    .bind(cursor_type)
    .bind(cursor_id)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > page_size as usize;
    if has_more {
        rows.truncate(page_size as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(encode_profile_content_cursor)).flatten();
    Ok((rows, next_cursor))
}

/// Batch-load currently visible content for owner bookmark projections.
pub async fn get_visible_profile_content(
    pool: &PgPool,
    thread_ids: &[i64],
    comment_ids: &[i64],
    viewer_id: i64,
) -> AppResult<Vec<ProfileContentRow>> {
    let rows = sqlx::query_as::<_, ProfileContentRow>(
        "SELECT 'thread'::text AS target_type, thread.id, thread.id AS thread_id, \
                thread.title, thread.body, thread.content_format, \
                COALESCE(board.slug, '') AS board_slug, thread.author_id, \
                thread.reply_count, thread.vote_count, thread.created_at, \
                thread.created_at AS activity_at \
         FROM forum.threads thread \
         JOIN forum.boards board ON board.id = thread.board_id \
         WHERE thread.id = ANY($1) AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL \
           AND (thread.author_id = $3 OR ( \
             NOT EXISTS (SELECT 1 FROM forum.user_ignores ignored \
                         WHERE (ignored.account_id = $3 AND ignored.ignored_account_id = thread.author_id) \
                            OR (ignored.account_id = thread.author_id AND ignored.ignored_account_id = $3)) \
             AND NOT EXISTS (SELECT 1 FROM forum.user_mutes muted \
                             WHERE muted.account_id = $3 AND muted.muted_account_id = thread.author_id) \
           )) \
         UNION ALL \
         SELECT 'comment'::text AS target_type, comment.id, comment.thread_id, \
                thread.title, comment.body, comment.content_format, \
                COALESCE(board.slug, '') AS board_slug, comment.author_id, \
                comment.reply_count, comment.vote_count, comment.created_at, \
                comment.created_at AS activity_at \
         FROM forum.comments comment \
         JOIN forum.threads thread ON thread.id = comment.thread_id \
         JOIN forum.boards board ON board.id = thread.board_id \
         WHERE comment.id = ANY($2) AND comment.deleted_at IS NULL \
           AND comment.hidden_at IS NULL AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL \
           AND (comment.author_id = $3 OR ( \
             NOT EXISTS (SELECT 1 FROM forum.user_ignores ignored \
                         WHERE (ignored.account_id = $3 AND ignored.ignored_account_id = comment.author_id) \
                            OR (ignored.account_id = comment.author_id AND ignored.ignored_account_id = $3)) \
             AND NOT EXISTS (SELECT 1 FROM forum.user_mutes muted \
                             WHERE muted.account_id = $3 AND muted.muted_account_id = comment.author_id) \
           ))",
    )
    .bind(thread_ids)
    .bind(comment_ids)
    .bind(viewer_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

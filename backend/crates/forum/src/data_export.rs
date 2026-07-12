//! Forum-owned projection for an account data export.
//!
//! Inbound DM bodies and moderation evidence are deliberately excluded because they are another
//! participant's private data or purpose-limited governance evidence.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumExport {
    threads: Vec<ExportThread>,
    comments: Vec<ExportComment>,
    drafts: Vec<ExportDraft>,
    relationships: ExportRelationships,
    notification_preferences: Option<serde_json::Value>,
    notifications: Vec<ExportNotification>,
    authored_direct_messages: Vec<ExportDirectMessage>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportThread {
    id: i64,
    board_id: Option<i64>,
    title: String,
    body: Option<String>,
    content_format: String,
    status: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    deleted_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    hidden_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportComment {
    id: i64,
    thread_id: Option<i64>,
    parent_id: Option<i64>,
    body: Option<String>,
    content_format: String,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    deleted_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    hidden_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDraft {
    draft_key: String,
    payload: serde_json::Value,
    #[serde(with = "chrono::serde::ts_seconds")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportRelationships {
    following_account_ids: Vec<String>,
    follower_account_ids: Vec<String>,
    muted_account_ids: Vec<String>,
    blocked_account_ids: Vec<String>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportNotification {
    id: i64,
    event_type: Option<String>,
    payload: Option<serde_json::Value>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    read_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDirectMessage {
    id: i64,
    conversation_id: i64,
    body: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<ForumExport> {
    let threads = sqlx::query_as::<_, ExportThread>(
        "SELECT id, board_id, title, body, content_format, status, deleted_at, hidden_at, \
                created_at, edited_at FROM forum.threads WHERE author_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let comments = sqlx::query_as::<_, ExportComment>(
        "SELECT id, thread_id, parent_id, body, content_format, deleted_at, hidden_at, \
                created_at, edited_at FROM forum.comments WHERE author_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let drafts = sqlx::query_as::<_, ExportDraft>(
        "SELECT draft_key, payload, updated_at FROM forum.drafts \
         WHERE account_id = $1 ORDER BY draft_key",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let following: Vec<i64> = sqlx::query_scalar(
        "SELECT followed_id FROM forum.user_follows WHERE follower_id = $1 ORDER BY followed_id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let followers: Vec<i64> = sqlx::query_scalar(
        "SELECT follower_id FROM forum.user_follows WHERE followed_id = $1 ORDER BY follower_id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let muted: Vec<i64> = sqlx::query_scalar(
        "SELECT muted_account_id FROM forum.user_mutes WHERE account_id = $1 ORDER BY muted_account_id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let blocked: Vec<i64> = sqlx::query_scalar(
        "SELECT ignored_account_id FROM forum.user_ignores \
         WHERE account_id = $1 ORDER BY ignored_account_id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let notification_preferences =
        sqlx::query_scalar("SELECT prefs FROM forum.notification_prefs WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(pool)
            .await?;
    let notifications = sqlx::query_as::<_, ExportNotification>(
        "SELECT id, type AS event_type, payload, read_at, created_at \
         FROM forum.notifications WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let authored_direct_messages = sqlx::query_as::<_, ExportDirectMessage>(
        "SELECT id, conversation_id, body, created_at FROM forum.dm_messages \
         WHERE sender_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(ForumExport {
        threads,
        comments,
        drafts,
        relationships: ExportRelationships {
            following_account_ids: following.into_iter().map(|id| id.to_string()).collect(),
            follower_account_ids: followers.into_iter().map(|id| id.to_string()).collect(),
            muted_account_ids: muted.into_iter().map(|id| id.to_string()).collect(),
            blocked_account_ids: blocked.into_iter().map(|id| id.to_string()).collect(),
        },
        notification_preferences,
        notifications,
        authored_direct_messages,
    })
}

/// Remove account-private forum projections while preserving public content and held evidence.
pub async fn purge_account_private_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE forum.threads thread SET vote_count = COALESCE(( \
             SELECT SUM(vote.value)::integer FROM forum.votes vote \
             WHERE vote.post_type = 'thread' AND vote.post_id = thread.id \
               AND vote.account_id <> $1), 0) \
         WHERE thread.id IN (SELECT post_id FROM forum.votes \
                             WHERE account_id = $1 AND post_type = 'thread')",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE forum.comments comment SET vote_count = COALESCE(( \
             SELECT SUM(vote.value)::integer FROM forum.votes vote \
             WHERE vote.post_type = 'comment' AND vote.post_id = comment.id \
               AND vote.account_id <> $1), 0) \
         WHERE comment.id IN (SELECT post_id FROM forum.votes \
                              WHERE account_id = $1 AND post_type = 'comment')",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "WITH received AS ( \
           SELECT author_id, COUNT(*)::integer AS upvote_count FROM ( \
             SELECT thread.author_id FROM forum.votes vote \
             JOIN forum.threads thread ON vote.post_type = 'thread' AND thread.id = vote.post_id \
             WHERE vote.account_id = $1 AND vote.value = 1 \
             UNION ALL \
             SELECT comment.author_id FROM forum.votes vote \
             JOIN forum.comments comment ON vote.post_type = 'comment' AND comment.id = vote.post_id \
             WHERE vote.account_id = $1 AND vote.value = 1 \
           ) authored WHERE author_id IS NOT NULL GROUP BY author_id \
         ) \
         UPDATE forum.user_stats stats \
         SET votes_received = GREATEST(stats.votes_received - received.upvote_count, 0), \
             updated_at = now() \
         FROM received WHERE stats.account_id = received.author_id",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE forum.poll_options option SET vote_count = GREATEST( \
             option.vote_count - (SELECT COUNT(*)::integer FROM forum.poll_votes vote \
                                  WHERE vote.poll_option_id = option.id \
                                    AND vote.account_id = $1), 0) \
         WHERE option.id IN (SELECT poll_option_id FROM forum.poll_votes WHERE account_id = $1)",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "WITH closed AS ( \
           UPDATE forum.dm_conversations SET request_status = 'declined', responded_at = now(), \
                  request_cooldown_until = now() + interval '30 days' \
           WHERE request_status = 'pending' \
             AND $1 IN (request_sender_id, request_recipient_id) RETURNING id \
         ) \
         DELETE FROM forum.dm_messages message USING closed \
         WHERE message.conversation_id = closed.id \
           AND NOT EXISTS (SELECT 1 FROM forum.dm_message_reports report \
                           WHERE report.message_id = message.id)",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    for statement in [
        "DELETE FROM forum.drafts WHERE account_id = $1",
        "DELETE FROM forum.bookmarks WHERE account_id = $1",
        "DELETE FROM forum.subscriptions WHERE account_id = $1",
        "DELETE FROM forum.thread_reads WHERE account_id = $1",
        "DELETE FROM forum.votes WHERE account_id = $1",
        "DELETE FROM forum.poll_votes WHERE account_id = $1",
        "DELETE FROM forum.notification_prefs WHERE account_id = $1",
        "DELETE FROM forum.notifications WHERE account_id = $1",
        "DELETE FROM forum.dm_request_attempts WHERE sender_id = $1 OR recipient_id = $1",
        "DELETE FROM forum.dm_request_idempotency WHERE sender_id = $1",
        "DELETE FROM forum.user_mutes WHERE account_id = $1 OR muted_account_id = $1",
        "DELETE FROM forum.user_ignores WHERE account_id = $1 OR ignored_account_id = $1",
        "DELETE FROM forum.user_follows WHERE follower_id = $1 OR followed_id = $1",
        "DELETE FROM forum.user_social_stats WHERE account_id = $1",
        "DELETE FROM forum.user_stats WHERE account_id = $1",
    ] {
        sqlx::query(statement).bind(account_id).execute(&mut *tx).await?;
    }
    sqlx::query(
        "UPDATE forum.dm_participants SET deleted_at = COALESCE(deleted_at, now()), \
             archived_at = COALESCE(archived_at, now()), muted_at = NULL \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

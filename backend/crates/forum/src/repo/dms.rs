//! Database access for 1:1 DMs.
//!
//! Every function takes `&PgPool` and returns `Result` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{FromRow, PgPool};

use crate::models::{DmConversationListRow, DmMessageRow};

/// Insert a new conversation and add both participants in a transaction.
pub async fn create_conversation(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<i64> {
    let mut tx = pool.begin().await?;

    let conv_id: i64 =
        sqlx::query_scalar("INSERT INTO forum.dm_conversations DEFAULT VALUES RETURNING id")
            .fetch_one(&mut *tx)
            .await?;

    sqlx::query("INSERT INTO forum.dm_participants (conversation_id, account_id) VALUES ($1, $2)")
        .bind(conv_id)
        .bind(account_id_a)
        .execute(&mut *tx)
        .await?;

    sqlx::query("INSERT INTO forum.dm_participants (conversation_id, account_id) VALUES ($1, $2)")
        .bind(conv_id)
        .bind(account_id_b)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(conv_id)
}

/// Find an existing conversation between two users (regardless of order).
pub async fn find_conversation(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT dp1.conversation_id \
         FROM forum.dm_participants dp1 \
         JOIN forum.dm_participants dp2 ON dp2.conversation_id = dp1.conversation_id \
         WHERE dp1.account_id = $1 AND dp2.account_id = $2 \
         LIMIT 1",
    )
    .bind(account_id_a)
    .bind(account_id_b)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0))
}

/// Find an existing conversation or create one.
pub async fn find_or_create_conversation(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<i64> {
    if let Some(conv_id) = find_conversation(pool, account_id_a, account_id_b).await? {
        return Ok(conv_id);
    }
    create_conversation(pool, account_id_a, account_id_b).await
}

/// A newly inserted message with its creation timestamp.
#[derive(Debug, Clone, FromRow)]
pub struct InsertedMessage {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

/// Insert a message into a conversation and return `(id, created_at)`.
/// Does NOT verify the sender is a participant — the caller is responsible
/// for that check.
pub async fn send_message(
    pool: &PgPool,
    conversation_id: i64,
    sender_id: i64,
    body: &str,
) -> AppResult<(i64, DateTime<Utc>)> {
    let row: InsertedMessage = sqlx::query_as(
        "INSERT INTO forum.dm_messages (conversation_id, sender_id, body) \
         VALUES ($1, $2, $3) RETURNING id, created_at",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(body)
    .fetch_one(pool)
    .await?;

    Ok((row.id, row.created_at))
}

/// List conversations for a user, with the other participant's handle and last
/// message time. Ordered by most recent message first.
pub async fn list_conversations(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<Vec<DmConversationListRow>> {
    let rows = sqlx::query_as::<_, DmConversationListRow>(
        "SELECT c.id, \
                a.id AS other_account_id, \
                a.handle AS other_handle, \
                COALESCE(lm.last_message_at, c.created_at) AS last_message_at \
         FROM forum.dm_conversations c \
         JOIN forum.dm_participants dp ON dp.conversation_id = c.id AND dp.account_id = $1 \
         JOIN forum.dm_participants dp2 ON dp2.conversation_id = c.id AND dp2.account_id != $1 \
         JOIN identity.accounts a ON a.id = dp2.account_id \
         LEFT JOIN LATERAL ( \
             SELECT created_at AS last_message_at \
             FROM forum.dm_messages \
             WHERE conversation_id = c.id \
             ORDER BY created_at DESC \
             LIMIT 1 \
         ) lm ON true \
         ORDER BY last_message_at DESC",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// List messages in a conversation with cursor pagination (by id, descending).
/// Verifies the account is a participant. Returns `(rows, next_cursor)`.
pub async fn list_messages(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<DmMessageRow>, Option<i64>)> {
    // Verify participant
    let is_participant = is_participant(pool, conversation_id, account_id).await?;
    if !is_participant {
        return Ok((vec![], None));
    }

    let rows = if let Some(cursor_id) = cursor {
        sqlx::query_as::<_, DmMessageRow>(
            "SELECT m.id, m.conversation_id, m.sender_id, \
                    a.handle AS sender_handle, m.body, m.created_at \
             FROM forum.dm_messages m \
             JOIN identity.accounts a ON a.id = m.sender_id \
             WHERE m.conversation_id = $1 AND m.id < $2 \
             ORDER BY m.id DESC \
             LIMIT $3",
        )
        .bind(conversation_id)
        .bind(cursor_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, DmMessageRow>(
            "SELECT m.id, m.conversation_id, m.sender_id, \
                    a.handle AS sender_handle, m.body, m.created_at \
             FROM forum.dm_messages m \
             JOIN identity.accounts a ON a.id = m.sender_id \
             WHERE m.conversation_id = $1 \
             ORDER BY m.id DESC \
             LIMIT $2",
        )
        .bind(conversation_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|r| r.id) } else { None };

    Ok((items, next_cursor))
}

/// Check whether an account is a participant in a conversation.
pub async fn is_participant(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM forum.dm_participants \
         WHERE conversation_id = $1 AND account_id = $2 \
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;

    Ok(exists.is_some())
}

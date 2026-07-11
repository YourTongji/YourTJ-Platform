//! Database access for private 1:1 conversations and their report queue.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgPool};

use crate::models::{DmConversationListRow, DmMessageReportRow, DmMessageRow};

/// A newly inserted message with its creation timestamp.
#[derive(Debug, Clone, FromRow)]
pub struct InsertedMessage {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

/// Look up an active, non-suspended recipient by public handle.
pub async fn find_available_recipient_by_handle(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<(i64, String, Option<String>)>> {
    let row = sqlx::query_as(
        "SELECT account.id, account.handle::text, account.avatar_url \
         FROM identity.accounts AS account \
         WHERE account.handle = $1 \
           AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id \
               AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL \
               AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           )",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Return whether either account has blocked the other.
pub async fn pair_is_blocked(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<bool> {
    let blocked = sqlx::query_scalar(
        "SELECT EXISTS ( \
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

/// Find or atomically create the canonical conversation for an unordered pair.
pub async fn find_or_create_conversation(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<i64> {
    let account_low_id = account_id_a.min(account_id_b);
    let account_high_id = account_id_a.max(account_id_b);
    if account_low_id == account_high_id {
        return Err(AppError::BadRequest("cannot start a conversation with yourself".into()));
    }

    let mut tx = pool.begin().await?;
    let conversation_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.dm_conversations (account_low_id, account_high_id) \
         VALUES ($1, $2) \
         ON CONFLICT (account_low_id, account_high_id) \
         DO UPDATE SET account_low_id = EXCLUDED.account_low_id \
         RETURNING id",
    )
    .bind(account_low_id)
    .bind(account_high_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO forum.dm_participants (conversation_id, account_id) \
         VALUES ($1, $2), ($1, $3) \
         ON CONFLICT (conversation_id, account_id) DO NOTHING",
    )
    .bind(conversation_id)
    .bind(account_low_id)
    .bind(account_high_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(conversation_id)
}

/// Return one conversation as visible to one participant.
pub async fn get_conversation(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<Option<DmConversationListRow>> {
    let row = sqlx::query_as::<_, DmConversationListRow>(
        "SELECT conversation.id, \
                other_account.id AS other_account_id, \
                other_account.handle::text AS other_handle, \
                other_account.avatar_url AS other_avatar_url, \
                LEFT(last_message.body, 160) AS last_message_excerpt, \
                COALESCE(last_message.created_at, conversation.created_at) AS last_message_at, \
                (SELECT COUNT(*) FROM forum.dm_messages AS unread \
                 WHERE unread.conversation_id = conversation.id \
                   AND unread.sender_id <> $2 \
                   AND (participant.last_read_message_id IS NULL \
                        OR unread.id > participant.last_read_message_id)) AS unread_count, \
                conversation.created_at \
         FROM forum.dm_conversations AS conversation \
         JOIN forum.dm_participants AS participant \
           ON participant.conversation_id = conversation.id \
          AND participant.account_id = $2 \
          AND participant.deleted_at IS NULL \
         JOIN forum.dm_participants AS other_participant \
           ON other_participant.conversation_id = conversation.id \
          AND other_participant.account_id <> $2 \
         JOIN identity.accounts AS other_account ON other_account.id = other_participant.account_id \
         LEFT JOIN LATERAL ( \
           SELECT message.body, message.created_at \
           FROM forum.dm_messages AS message \
           WHERE message.conversation_id = conversation.id \
           ORDER BY message.id DESC \
           LIMIT 1 \
         ) AS last_message ON true \
         WHERE conversation.id = $1",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Encode a stable last-activity/id cursor for the conversation inbox.
pub fn encode_conversation_cursor(last_message_at: DateTime<Utc>, id: i64) -> String {
    super::base64_encode_str(&format!("{}|{id}", last_message_at.to_rfc3339()))
}

/// Decode a conversation cursor without leaking parser details to clients.
pub fn decode_conversation_cursor(cursor: &str) -> AppResult<(DateTime<Utc>, i64)> {
    let decoded = super::base64_decode_str(cursor)
        .map_err(|_| AppError::BadRequest("invalid conversation cursor".into()))?;
    let (timestamp, id) = decoded
        .rsplit_once('|')
        .ok_or_else(|| AppError::BadRequest("invalid conversation cursor".into()))?;
    let last_message_at = DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| AppError::BadRequest("invalid conversation cursor".into()))?
        .with_timezone(&Utc);
    let conversation_id = id
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("invalid conversation cursor".into()))?;
    Ok((last_message_at, conversation_id))
}

/// List a participant's non-deleted conversations by latest activity.
pub async fn list_conversations(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<(DateTime<Utc>, i64)>,
    limit: i64,
) -> AppResult<(Vec<DmConversationListRow>, Option<String>)> {
    let bounded_limit = limit.clamp(1, 100);
    let rows = if let Some((cursor_time, cursor_id)) = cursor {
        sqlx::query_as::<_, DmConversationListRow>(
            "SELECT conversation.id, \
                    other_account.id AS other_account_id, \
                    other_account.handle::text AS other_handle, \
                    other_account.avatar_url AS other_avatar_url, \
                    LEFT(last_message.body, 160) AS last_message_excerpt, \
                    COALESCE(last_message.created_at, conversation.created_at) AS last_message_at, \
                    (SELECT COUNT(*) FROM forum.dm_messages AS unread \
                     WHERE unread.conversation_id = conversation.id \
                       AND unread.sender_id <> $1 \
                       AND (participant.last_read_message_id IS NULL \
                            OR unread.id > participant.last_read_message_id)) AS unread_count, \
                    conversation.created_at \
             FROM forum.dm_conversations AS conversation \
             JOIN forum.dm_participants AS participant \
               ON participant.conversation_id = conversation.id \
              AND participant.account_id = $1 \
              AND participant.deleted_at IS NULL \
             JOIN forum.dm_participants AS other_participant \
               ON other_participant.conversation_id = conversation.id \
              AND other_participant.account_id <> $1 \
             JOIN identity.accounts AS other_account \
               ON other_account.id = other_participant.account_id \
             LEFT JOIN LATERAL ( \
               SELECT message.body, message.created_at \
               FROM forum.dm_messages AS message \
               WHERE message.conversation_id = conversation.id \
               ORDER BY message.id DESC \
               LIMIT 1 \
             ) AS last_message ON true \
             WHERE COALESCE(last_message.created_at, conversation.created_at) < $2 \
                OR (COALESCE(last_message.created_at, conversation.created_at) = $2 \
                    AND conversation.id < $3) \
             ORDER BY last_message_at DESC, conversation.id DESC \
             LIMIT $4",
        )
        .bind(account_id)
        .bind(cursor_time)
        .bind(cursor_id)
        .bind(bounded_limit + 1)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, DmConversationListRow>(
            "SELECT conversation.id, \
                    other_account.id AS other_account_id, \
                    other_account.handle::text AS other_handle, \
                    other_account.avatar_url AS other_avatar_url, \
                    LEFT(last_message.body, 160) AS last_message_excerpt, \
                    COALESCE(last_message.created_at, conversation.created_at) AS last_message_at, \
                    (SELECT COUNT(*) FROM forum.dm_messages AS unread \
                     WHERE unread.conversation_id = conversation.id \
                       AND unread.sender_id <> $1 \
                       AND (participant.last_read_message_id IS NULL \
                            OR unread.id > participant.last_read_message_id)) AS unread_count, \
                    conversation.created_at \
             FROM forum.dm_conversations AS conversation \
             JOIN forum.dm_participants AS participant \
               ON participant.conversation_id = conversation.id \
              AND participant.account_id = $1 \
              AND participant.deleted_at IS NULL \
             JOIN forum.dm_participants AS other_participant \
               ON other_participant.conversation_id = conversation.id \
              AND other_participant.account_id <> $1 \
             JOIN identity.accounts AS other_account \
               ON other_account.id = other_participant.account_id \
             LEFT JOIN LATERAL ( \
               SELECT message.body, message.created_at \
               FROM forum.dm_messages AS message \
               WHERE message.conversation_id = conversation.id \
               ORDER BY message.id DESC \
               LIMIT 1 \
             ) AS last_message ON true \
             ORDER BY last_message_at DESC, conversation.id DESC \
             LIMIT $2",
        )
        .bind(account_id)
        .bind(bounded_limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > bounded_limit as usize;
    let items = if has_more { rows[..bounded_limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more {
        items.last().map(|row| encode_conversation_cursor(row.last_message_at, row.id))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// Return the other active participant for a sender in a conversation.
pub async fn find_available_other_participant(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<Option<i64>> {
    let other_account_id = sqlx::query_scalar(
        "SELECT account.id \
         FROM forum.dm_participants AS mine \
         JOIN forum.dm_participants AS other \
           ON other.conversation_id = mine.conversation_id \
          AND other.account_id <> mine.account_id \
         JOIN identity.accounts AS account ON account.id = other.account_id \
         WHERE mine.conversation_id = $1 \
           AND mine.account_id = $2 \
           AND mine.deleted_at IS NULL \
           AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id \
               AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL \
               AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           )",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;

    Ok(other_account_id)
}

/// Insert a message after the handler has checked membership and blocking.
pub async fn send_message(
    pool: &PgPool,
    conversation_id: i64,
    sender_id: i64,
    body: &str,
) -> AppResult<(i64, DateTime<Utc>)> {
    let row: InsertedMessage = sqlx::query_as(
        "INSERT INTO forum.dm_messages (conversation_id, sender_id, body) \
         SELECT $1, $2, $3 \
         WHERE EXISTS ( \
           SELECT 1 FROM forum.dm_participants \
           WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NULL \
         ) \
         RETURNING id, created_at",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(body)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Forbidden)?;

    Ok((row.id, row.created_at))
}

/// List messages in a conversation with an id cursor, newest first.
pub async fn list_messages(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<DmMessageRow>, Option<i64>)> {
    if !is_participant(pool, conversation_id, account_id).await? {
        return Err(AppError::Forbidden);
    }

    let bounded_limit = limit.clamp(1, 100);
    let rows = if let Some(cursor_id) = cursor {
        sqlx::query_as::<_, DmMessageRow>(
            "SELECT message.id, message.conversation_id, message.sender_id, \
                    account.handle::text AS sender_handle, message.body, message.created_at \
             FROM forum.dm_messages AS message \
             JOIN identity.accounts AS account ON account.id = message.sender_id \
             WHERE message.conversation_id = $1 AND message.id < $2 \
             ORDER BY message.id DESC \
             LIMIT $3",
        )
        .bind(conversation_id)
        .bind(cursor_id)
        .bind(bounded_limit + 1)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, DmMessageRow>(
            "SELECT message.id, message.conversation_id, message.sender_id, \
                    account.handle::text AS sender_handle, message.body, message.created_at \
             FROM forum.dm_messages AS message \
             JOIN identity.accounts AS account ON account.id = message.sender_id \
             WHERE message.conversation_id = $1 \
             ORDER BY message.id DESC \
             LIMIT $2",
        )
        .bind(conversation_id)
        .bind(bounded_limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > bounded_limit as usize;
    let items = if has_more { rows[..bounded_limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|row| row.id) } else { None };
    Ok((items, next_cursor))
}

/// Advance one participant's read pointer to a message in the conversation.
pub async fn advance_read_pointer(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
    message_id: Option<i64>,
) -> AppResult<()> {
    if !is_participant(pool, conversation_id, account_id).await? {
        return Err(AppError::Forbidden);
    }

    let message_id = if let Some(requested_message_id) = message_id {
        let message_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS ( \
               SELECT 1 FROM forum.dm_messages \
               WHERE id = $1 AND conversation_id = $2 \
             )",
        )
        .bind(requested_message_id)
        .bind(conversation_id)
        .fetch_one(pool)
        .await?;
        if !message_exists {
            return Err(AppError::NotFound);
        }
        requested_message_id
    } else if let Some(latest_message_id) =
        sqlx::query_scalar("SELECT MAX(id) FROM forum.dm_messages WHERE conversation_id = $1")
            .bind(conversation_id)
            .fetch_one(pool)
            .await?
    {
        latest_message_id
    } else {
        return Ok(());
    };

    let result = sqlx::query(
        "UPDATE forum.dm_participants \
         SET last_read_message_id = CASE \
               WHEN last_read_message_id IS NULL OR last_read_message_id < $3 THEN $3 \
               ELSE last_read_message_id \
             END \
         WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NULL",
    )
    .bind(conversation_id)
    .bind(account_id)
    .bind(message_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Check whether an active participant can access a conversation.
pub async fn is_participant(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let exists = sqlx::query_scalar(
        "SELECT EXISTS ( \
           SELECT 1 FROM forum.dm_participants \
           WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NULL \
         )",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Check whether an account participates in the reported message's conversation.
pub async fn can_access_message(
    pool: &PgPool,
    message_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let exists = sqlx::query_scalar(
        "SELECT EXISTS ( \
           SELECT 1 \
           FROM forum.dm_messages AS message \
           JOIN forum.dm_participants AS participant \
             ON participant.conversation_id = message.conversation_id \
           WHERE message.id = $1 \
             AND participant.account_id = $2 \
             AND participant.deleted_at IS NULL \
         )",
    )
    .bind(message_id)
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Insert one report per participant/message pair.
pub async fn report_message(
    pool: &PgPool,
    message_id: i64,
    reported_by: i64,
    reason: &str,
    note: Option<&str>,
) -> AppResult<i64> {
    let report_id = sqlx::query_scalar(
        "INSERT INTO forum.dm_message_reports \
           (message_id, conversation_id, reported_by, reason, note) \
         SELECT message.id, message.conversation_id, $2, $3, $4 \
         FROM forum.dm_messages AS message \
         JOIN forum.dm_participants AS participant \
           ON participant.conversation_id = message.conversation_id \
          AND participant.account_id = $2 \
          AND participant.deleted_at IS NULL \
         WHERE message.id = $1 \
         ON CONFLICT (message_id, reported_by) DO NOTHING \
         RETURNING id",
    )
    .bind(message_id)
    .bind(reported_by)
    .bind(reason)
    .bind(note)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Conflict("message already reported".into()))?;
    Ok(report_id)
}

/// List only reported messages; no general staff DM browsing query exists.
pub async fn list_message_reports(
    pool: &PgPool,
    status: &str,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<DmMessageReportRow>, Option<i64>)> {
    let bounded_limit = limit.clamp(1, 100);
    let before_id = cursor.unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, DmMessageReportRow>(
        "SELECT report.id, report.message_id, report.conversation_id, report.reported_by, \
                reporter.handle::text AS reporter_handle, \
                message.sender_id, sender.handle::text AS sender_handle, \
                LEFT(message.body, 1000) AS message_excerpt, \
                report.reason, report.note, report.status, \
                report.handled_by, report.handled_at, report.created_at \
         FROM forum.dm_message_reports AS report \
         JOIN forum.dm_messages AS message ON message.id = report.message_id \
         JOIN identity.accounts AS reporter ON reporter.id = report.reported_by \
         JOIN identity.accounts AS sender ON sender.id = message.sender_id \
         WHERE report.status = $1 AND report.id < $2 \
         ORDER BY report.id DESC \
         LIMIT $3",
    )
    .bind(status)
    .bind(before_id)
    .bind(bounded_limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > bounded_limit as usize;
    let items = if has_more { rows[..bounded_limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more { items.last().map(|row| row.id) } else { None };
    Ok((items, next_cursor))
}

/// Resolve one open DM report.
pub async fn resolve_message_report(
    pool: &PgPool,
    report_id: i64,
    action: &str,
    handled_by: i64,
    actor_role: &str,
    note: Option<&str>,
) -> AppResult<DmMessageReportRow> {
    let resolved_status = match action {
        "uphold" => "upheld",
        "reject" => "rejected",
        _ => return Err(AppError::BadRequest("invalid DM report decision".into())),
    };
    let mut tx = pool.begin().await?;
    let decision_reason = note.unwrap_or(match action {
        "uphold" => "DM report upheld",
        "reject" => "DM report rejected",
        _ => "DM report resolved",
    });
    let decision_metadata = serde_json::json!({ "decision": action });
    let result = sqlx::query(
        "UPDATE forum.dm_message_reports \
         SET status = $1, handled_by = $2, handled_at = now(), resolution_note = $3 \
         WHERE id = $4 AND status = 'open'",
    )
    .bind(resolved_status)
    .bind(handled_by)
    .bind(note)
    .bind(report_id)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM forum.dm_message_reports WHERE id = $1)",
        )
        .bind(report_id)
        .fetch_one(&mut *tx)
        .await?;
        return if exists {
            Err(AppError::Conflict("DM report is already resolved".into()))
        } else {
            Err(AppError::NotFound)
        };
    }

    sqlx::query(
        "INSERT INTO forum.mod_actions \
           (actor_id, action, target_type, target_id, reason, metadata) \
         VALUES ($1, $2, 'dm_report', $3, $4, $5)",
    )
    .bind(handled_by)
    .bind(format!("resolve_dm_report_{action}"))
    .bind(report_id)
    .bind(decision_reason)
    .bind(&decision_metadata)
    .execute(&mut *tx)
    .await?;

    let report_id_string = report_id.to_string();
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: handled_by, role: actor_role },
        "forum.dm_report.resolved",
        "dm_report",
        &report_id_string,
        decision_reason,
        Some(&decision_metadata),
    )
    .await?;

    let row = sqlx::query_as::<_, DmMessageReportRow>(
        "SELECT report.id, report.message_id, report.conversation_id, report.reported_by, \
                reporter.handle::text AS reporter_handle, \
                message.sender_id, sender.handle::text AS sender_handle, \
                LEFT(message.body, 1000) AS message_excerpt, \
                report.reason, report.note, report.status, \
                report.handled_by, report.handled_at, report.created_at \
         FROM forum.dm_message_reports AS report \
         JOIN forum.dm_messages AS message ON message.id = report.message_id \
         JOIN identity.accounts AS reporter ON reporter.id = report.reported_by \
         JOIN identity.accounts AS sender ON sender.id = message.sender_id \
         WHERE report.id = $1",
    )
    .bind(report_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

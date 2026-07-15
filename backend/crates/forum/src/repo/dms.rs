//! Database access for private 1:1 conversations and their report queue.

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::models::{DmConversationListRow, DmMessageReportRow, DmMessageRow};

/// A newly inserted message with its creation timestamp.
#[derive(Debug, Clone, FromRow)]
pub struct InsertedMessage {
    pub id: i64,
    pub created_at: DateTime<Utc>,
}

/// Return a prior send only when the client identity still describes the same message.
pub async fn find_send_replay(
    pool: &PgPool,
    sender_id: i64,
    client_message_id: Uuid,
    conversation_id: i64,
    body: &str,
) -> AppResult<Option<InsertedMessage>> {
    let existing: Option<(i64, i64, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, conversation_id, body, created_at FROM forum.dm_messages \
         WHERE sender_id = $1 AND client_message_id = $2",
    )
    .bind(sender_id)
    .bind(client_message_id)
    .fetch_optional(pool)
    .await?;
    let Some((id, stored_conversation_id, stored_body, created_at)) = existing else {
        return Ok(None);
    };
    if stored_conversation_id != conversation_id || stored_body != body {
        return Err(AppError::Conflict(
            "client message identity was already used for another message".into(),
        ));
    }
    Ok(Some(InsertedMessage { id, created_at }))
}

/// How a newly initiated conversation is authorized by the recipient's policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmStartMode {
    Direct,
    Request,
}

/// Durable result of starting or replaying a conversation.
#[derive(Debug, Clone)]
pub struct DmStartResult {
    pub conversation_id: i64,
    pub request_status: String,
    pub request_created: bool,
    pub message_created: bool,
}

/// Counts used by the global private-message badge.
#[derive(Debug, Clone, Copy, FromRow)]
pub struct DmCounts {
    pub unread_count: i64,
    pub request_count: i64,
}

#[derive(Debug, Clone, FromRow)]
struct ExistingConversation {
    id: i64,
    request_status: String,
    request_sender_id: Option<i64>,
    is_in_cooldown: bool,
}

async fn enforce_request_daily_limit(
    transaction: &mut Transaction<'_, Postgres>,
    sender_id: i64,
) -> AppResult<()> {
    let lock_key = format!("dm-request-budget:{sender_id}");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(lock_key)
        .execute(&mut **transaction)
        .await?;
    let recent_requests: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.dm_request_attempts \
         WHERE sender_id = $1 AND created_at > now() - interval '24 hours'",
    )
    .bind(sender_id)
    .fetch_one(&mut **transaction)
    .await?;
    if recent_requests >= 10 {
        return Err(AppError::RateLimited);
    }
    Ok(())
}

async fn insert_message_tx(
    transaction: &mut Transaction<'_, Postgres>,
    conversation_id: i64,
    sender_id: i64,
    body: &str,
) -> AppResult<InsertedMessage> {
    let message = sqlx::query_as(
        "INSERT INTO forum.dm_messages (conversation_id, sender_id, body) \
         VALUES ($1, $2, $3) RETURNING id, created_at",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(body)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(message)
}

/// Return a completed matching start request before consuming abuse-control budget.
pub async fn find_start_replay(
    pool: &PgPool,
    sender_id: i64,
    idempotency_key: &str,
    request_hash: &str,
) -> AppResult<Option<i64>> {
    let replay: Option<(String, i64)> = sqlx::query_as(
        "SELECT request_hash, conversation_id FROM forum.dm_request_idempotency \
         WHERE sender_id = $1 AND idempotency_key = $2",
    )
    .bind(sender_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await?;
    let Some((stored_hash, conversation_id)) = replay else {
        return Ok(None);
    };
    if stored_hash != request_hash {
        return Err(AppError::Conflict(
            "idempotency key was already used for another message request".into(),
        ));
    }
    Ok(Some(conversation_id))
}

/// Start a direct conversation or one-message request, with durable replay protection.
pub async fn start_conversation(
    pool: &PgPool,
    sender_id: i64,
    recipient_id: i64,
    initial_message: Option<&str>,
    idempotency: Option<(&str, &str)>,
) -> AppResult<DmStartResult> {
    if sender_id == recipient_id {
        return Err(AppError::BadRequest("cannot start a conversation with yourself".into()));
    }
    let account_low_id = sender_id.min(recipient_id);
    let account_high_id = sender_id.max(recipient_id);
    let mut transaction = pool.begin().await?;

    if !identity::public_accounts::lock_active_interaction_accounts(
        &mut transaction,
        &[account_low_id, account_high_id],
    )
    .await?
    {
        return Err(AppError::Forbidden);
    }
    super::relationships::lock_pair_unblocked(&mut transaction, sender_id, recipient_id).await?;

    if let Some((idempotency_key, request_hash)) = idempotency {
        let lock_key = format!("dm_request:{sender_id}:{idempotency_key}");
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock_key)
            .execute(&mut *transaction)
            .await?;
        let replay: Option<(String, i64, String)> = sqlx::query_as(
            "SELECT idempotency.request_hash, idempotency.conversation_id, \
                    conversation.request_status \
             FROM forum.dm_request_idempotency AS idempotency \
             JOIN forum.dm_conversations AS conversation \
               ON conversation.id = idempotency.conversation_id \
             WHERE idempotency.sender_id = $1 AND idempotency.idempotency_key = $2",
        )
        .bind(sender_id)
        .bind(idempotency_key)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some((stored_hash, conversation_id, request_status)) = replay {
            if stored_hash != request_hash {
                return Err(AppError::Conflict(
                    "idempotency key was already used for another message request".into(),
                ));
            }
            transaction.commit().await?;
            return Ok(DmStartResult {
                conversation_id,
                request_status,
                request_created: false,
                message_created: false,
            });
        }
    }

    let existing = sqlx::query_as::<_, ExistingConversation>(
        "SELECT id, request_status, request_sender_id, \
                COALESCE(request_cooldown_until > now(), FALSE) AS is_in_cooldown \
         FROM forum.dm_conversations \
         WHERE account_low_id = $1 AND account_high_id = $2 FOR UPDATE",
    )
    .bind(account_low_id)
    .bind(account_high_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let effective_mode = if existing
        .as_ref()
        .is_some_and(|conversation| conversation.request_status == "accepted")
    {
        DmStartMode::Direct
    } else {
        let recipient_policy =
            identity::public_accounts::lock_dm_policy(&mut transaction, recipient_id).await?;
        if recipient_policy == "nobody" {
            return Err(AppError::Forbidden);
        }
        let recipient_follows_sender: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM forum.user_follows \
             WHERE follower_id = $1 AND followed_id = $2)",
        )
        .bind(recipient_id)
        .bind(sender_id)
        .fetch_one(&mut *transaction)
        .await?;
        if recipient_follows_sender {
            DmStartMode::Direct
        } else if recipient_policy == "everyone" {
            DmStartMode::Request
        } else {
            return Err(AppError::Forbidden);
        }
    };
    if effective_mode == DmStartMode::Request && initial_message.is_none() {
        return Err(AppError::BadRequest(
            "requestMessage is required when the recipient has not accepted messages from you"
                .into(),
        ));
    }

    let mut request_created = false;
    let mut conversation_changed = false;
    let mut inserted_message = None;
    let (conversation_id, request_status) = match existing {
        None => match effective_mode {
            DmStartMode::Direct => {
                let conversation_id = sqlx::query_scalar(
                    "INSERT INTO forum.dm_conversations \
                       (account_low_id, account_high_id, request_status) \
                     VALUES ($1, $2, 'accepted') RETURNING id",
                )
                .bind(account_low_id)
                .bind(account_high_id)
                .fetch_one(&mut *transaction)
                .await?;
                conversation_changed = true;
                (conversation_id, "accepted".to_owned())
            }
            DmStartMode::Request => {
                enforce_request_daily_limit(&mut transaction, sender_id).await?;
                let conversation_id = sqlx::query_scalar(
                    "INSERT INTO forum.dm_conversations \
                       (account_low_id, account_high_id, request_status, request_sender_id, \
                        request_recipient_id, requested_at) \
                     VALUES ($1, $2, 'pending', $3, $4, now()) RETURNING id",
                )
                .bind(account_low_id)
                .bind(account_high_id)
                .bind(sender_id)
                .bind(recipient_id)
                .fetch_one(&mut *transaction)
                .await?;
                request_created = true;
                conversation_changed = true;
                (conversation_id, "pending".to_owned())
            }
        },
        Some(existing) if existing.request_status == "accepted" => {
            (existing.id, "accepted".to_owned())
        }
        Some(existing)
            if existing.request_status == "pending"
                && effective_mode == DmStartMode::Request
                && existing.request_sender_id == Some(sender_id) =>
        {
            (existing.id, "pending".to_owned())
        }
        Some(existing) if existing.request_status == "pending" => {
            sqlx::query(
                "UPDATE forum.dm_conversations \
                 SET request_status = 'accepted', responded_at = now(), \
                     request_cooldown_until = NULL \
                 WHERE id = $1",
            )
            .bind(existing.id)
            .execute(&mut *transaction)
            .await?;
            conversation_changed = true;
            (existing.id, "accepted".to_owned())
        }
        Some(existing) if effective_mode == DmStartMode::Direct => {
            sqlx::query(
                "UPDATE forum.dm_conversations \
                 SET request_status = 'accepted', responded_at = now(), \
                     request_cooldown_until = NULL \
                 WHERE id = $1",
            )
            .bind(existing.id)
            .execute(&mut *transaction)
            .await?;
            conversation_changed = true;
            (existing.id, "accepted".to_owned())
        }
        Some(existing) => {
            if existing.is_in_cooldown {
                return Err(AppError::Conflict(
                    "a declined message request cannot be retried yet".into(),
                ));
            }
            enforce_request_daily_limit(&mut transaction, sender_id).await?;
            sqlx::query(
                "UPDATE forum.dm_conversations \
                 SET request_status = 'pending', request_sender_id = $2, \
                     request_recipient_id = $3, requested_at = now(), responded_at = NULL, \
                     request_cooldown_until = NULL \
                 WHERE id = $1",
            )
            .bind(existing.id)
            .bind(sender_id)
            .bind(recipient_id)
            .execute(&mut *transaction)
            .await?;
            request_created = true;
            conversation_changed = true;
            (existing.id, "pending".to_owned())
        }
    };

    sqlx::query(
        "INSERT INTO forum.dm_participants (conversation_id, account_id) \
         VALUES ($1, $2), ($1, $3) \
         ON CONFLICT (conversation_id, account_id) DO UPDATE \
         SET deleted_at = NULL, archived_at = NULL",
    )
    .bind(conversation_id)
    .bind(account_low_id)
    .bind(account_high_id)
    .execute(&mut *transaction)
    .await?;

    if request_created {
        sqlx::query(
            "INSERT INTO forum.dm_request_attempts \
               (sender_id, recipient_id, conversation_id) VALUES ($1, $2, $3)",
        )
        .bind(sender_id)
        .bind(recipient_id)
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    }

    let should_insert_message =
        initial_message.is_some() && (request_status != "pending" || request_created);
    if let Some(message) = initial_message.filter(|_| should_insert_message) {
        inserted_message =
            Some(insert_message_tx(&mut transaction, conversation_id, sender_id, message).await?);
    }
    let message_created = inserted_message.is_some();

    if let Some((idempotency_key, request_hash)) =
        idempotency.filter(|_| conversation_changed || message_created)
    {
        sqlx::query(
            "INSERT INTO forum.dm_request_idempotency \
               (sender_id, idempotency_key, request_hash, conversation_id) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(sender_id)
        .bind(idempotency_key)
        .bind(request_hash)
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    }

    if request_created || message_created {
        let sender =
            identity::public_accounts::lock_notification_recipient(&mut transaction, sender_id)
                .await?
                .ok_or(AppError::Forbidden)?;
        if request_created {
            let requested_at: DateTime<Utc> =
                sqlx::query_scalar("SELECT requested_at FROM forum.dm_conversations WHERE id = $1")
                    .bind(conversation_id)
                    .fetch_one(&mut *transaction)
                    .await?;
            platform::outbox::enqueue_notification_tx(
                &mut transaction,
                &format!("dm-request:{conversation_id}:{}", requested_at.timestamp_micros()),
                recipient_id,
                Some(sender_id),
                "dm_request",
                &serde_json::json!({
                    "conversationId": conversation_id.to_string(),
                    "requestedAtMicros": requested_at.timestamp_micros().to_string(),
                    "senderHandle": &sender.handle,
                    "title": format!("{} 发来消息请求", sender.handle),
                }),
                Some(&conversation_id.to_string()),
                None,
            )
            .await?;
        } else if let (Some(message), Some(body)) = (inserted_message.as_ref(), initial_message) {
            platform::outbox::enqueue_notification_tx(
                &mut transaction,
                &format!("dm-message:{}", message.id),
                recipient_id,
                Some(sender_id),
                "dm",
                &serde_json::json!({
                    "conversationId": conversation_id.to_string(),
                    "messageId": message.id.to_string(),
                    "senderHandle": &sender.handle,
                    "title": format!("{} 发来私信", sender.handle),
                    "bodyExcerpt": body.chars().take(100).collect::<String>(),
                }),
                Some(&conversation_id.to_string()),
                None,
            )
            .await?;
        }
    }

    transaction.commit().await?;
    Ok(DmStartResult { conversation_id, request_status, request_created, message_created })
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

    sqlx::query(
        "UPDATE forum.dm_participants \
         SET deleted_at = NULL, archived_at = NULL \
         WHERE conversation_id = $1 AND account_id = $2",
    )
    .bind(conversation_id)
    .bind(account_id_a)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(conversation_id)
}

/// Return whether a pair already has an accepted conversation.
pub async fn pair_has_accepted_conversation(
    pool: &PgPool,
    account_id_a: i64,
    account_id_b: i64,
) -> AppResult<bool> {
    let accepted = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.dm_conversations \
         WHERE account_low_id = LEAST($1, $2) AND account_high_id = GREATEST($1, $2) \
           AND request_status = 'accepted')",
    )
    .bind(account_id_a)
    .bind(account_id_b)
    .fetch_one(pool)
    .await?;
    Ok(accepted)
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
                other_profile.display_name AS other_display_name, \
                LEFT(last_message.body, 160) AS last_message_excerpt, \
                COALESCE(last_message.created_at, conversation.created_at) AS last_message_at, \
                CASE WHEN conversation.request_status = 'accepted' THEN \
                  (SELECT COUNT(*) FROM forum.dm_messages AS unread \
                   WHERE unread.conversation_id = conversation.id \
                     AND unread.sender_id <> $2 \
                     AND (participant.last_read_message_id IS NULL \
                          OR unread.id > participant.last_read_message_id)) \
                ELSE 0::bigint END AS unread_count, \
                participant.archived_at IS NOT NULL AS is_archived, \
                participant.muted_at IS NOT NULL AS is_muted, \
                participant.deleted_at IS NOT NULL AS is_deleted, \
                conversation.request_status, \
                CASE WHEN conversation.request_status = 'pending' THEN \
                  CASE WHEN conversation.request_recipient_id = $2 THEN 'incoming' ELSE 'outgoing' END \
                ELSE NULL END AS request_direction, \
                conversation.request_status = 'accepted' AS can_send, \
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
         LEFT JOIN identity.profiles AS other_profile ON other_profile.account_id = other_account.id \
         LEFT JOIN LATERAL ( \
           SELECT message.body, message.created_at \
           FROM forum.dm_messages AS message \
           WHERE message.conversation_id = conversation.id \
           ORDER BY message.id DESC \
           LIMIT 1 \
         ) AS last_message ON true \
         WHERE conversation.id = $1 AND conversation.request_status <> 'declined'",
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

/// List a participant's conversations by participant-local lifecycle and latest activity.
pub async fn list_conversations(
    pool: &PgPool,
    account_id: i64,
    view: &str,
    search_query: Option<&str>,
    cursor: Option<(DateTime<Utc>, i64)>,
    limit: i64,
) -> AppResult<(Vec<DmConversationListRow>, Option<String>)> {
    let bounded_limit = limit.clamp(1, 100);
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT conversation.id, \
                other_account.id AS other_account_id, \
                other_account.handle::text AS other_handle, \
                other_profile.display_name AS other_display_name, \
                LEFT(last_message.body, 160) AS last_message_excerpt, \
                COALESCE(last_message.created_at, conversation.created_at) AS last_message_at, \
                CASE WHEN conversation.request_status = 'accepted' THEN \
                  (SELECT COUNT(*) FROM forum.dm_messages AS unread \
                   WHERE unread.conversation_id = conversation.id \
                     AND unread.sender_id <> ",
    );
    builder.push_bind(account_id).push(
        " AND (participant.last_read_message_id IS NULL \
                OR unread.id > participant.last_read_message_id)) \
         ELSE 0::bigint END AS unread_count, \
         participant.archived_at IS NOT NULL AS is_archived, \
         participant.muted_at IS NOT NULL AS is_muted, \
         participant.deleted_at IS NOT NULL AS is_deleted, \
         conversation.request_status, \
         CASE WHEN conversation.request_status = 'pending' THEN \
           CASE WHEN conversation.request_recipient_id = ",
    );
    builder.push_bind(account_id).push(
        " THEN 'incoming' ELSE 'outgoing' END \
         ELSE NULL END AS request_direction, \
         conversation.request_status = 'accepted' AS can_send, \
         conversation.created_at \
         FROM forum.dm_conversations AS conversation \
         JOIN forum.dm_participants AS participant \
           ON participant.conversation_id = conversation.id \
          AND participant.account_id = ",
    );
    builder.push_bind(account_id).push(
        " JOIN forum.dm_participants AS other_participant \
           ON other_participant.conversation_id = conversation.id \
          AND other_participant.account_id <> ",
    );
    builder.push_bind(account_id).push(
        " JOIN identity.accounts AS other_account \
           ON other_account.id = other_participant.account_id \
         LEFT JOIN identity.profiles AS other_profile \
           ON other_profile.account_id = other_account.id \
         LEFT JOIN LATERAL ( \
           SELECT message.body, message.created_at \
           FROM forum.dm_messages AS message \
           WHERE message.conversation_id = conversation.id \
           ORDER BY message.id DESC \
           LIMIT 1 \
         ) AS last_message ON true \
         WHERE ",
    );
    match view {
        "archived" => builder.push(
            "conversation.request_status = 'accepted' AND participant.deleted_at IS NULL \
             AND participant.archived_at IS NOT NULL",
        ),
        "deleted" => builder.push(
            "conversation.request_status = 'accepted' AND participant.deleted_at IS NOT NULL",
        ),
        "requests" => builder.push(
            "conversation.request_status = 'pending' \
             AND conversation.request_recipient_id = participant.account_id \
             AND participant.deleted_at IS NULL",
        ),
        "sent" => builder.push(
            "conversation.request_status = 'pending' \
             AND conversation.request_sender_id = participant.account_id \
             AND participant.deleted_at IS NULL",
        ),
        _ => builder.push(
            "conversation.request_status = 'accepted' AND participant.deleted_at IS NULL \
             AND participant.archived_at IS NULL",
        ),
    };
    if let Some(query) = search_query {
        let pattern = format!("%{query}%");
        builder.push(" AND (other_account.handle::text ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR COALESCE(last_message.body, '') ILIKE ");
        builder.push_bind(pattern);
        builder.push(")");
    }
    if let Some((cursor_time, cursor_id)) = cursor {
        builder.push(" AND (COALESCE(last_message.created_at, conversation.created_at) < ");
        builder.push_bind(cursor_time);
        builder.push(" OR (COALESCE(last_message.created_at, conversation.created_at) = ");
        builder.push_bind(cursor_time);
        builder.push(" AND conversation.id < ");
        builder.push_bind(cursor_id);
        builder.push("))");
    }
    builder.push(" ORDER BY last_message_at DESC, conversation.id DESC LIMIT ");
    builder.push_bind(bounded_limit + 1);

    let mut items = builder.build_query_as::<DmConversationListRow>().fetch_all(pool).await?;
    let has_more = items.len() > bounded_limit as usize;
    if has_more {
        items.truncate(bounded_limit as usize);
    }
    let next_cursor = if has_more {
        items.last().map(|row| encode_conversation_cursor(row.last_message_at, row.id))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// Count unread messages across the participant's active inbox.
pub async fn unread_count(pool: &PgPool, account_id: i64) -> AppResult<i64> {
    let count = sqlx::query_scalar(
        "SELECT COUNT(*) \
         FROM forum.dm_messages AS message \
         JOIN forum.dm_conversations AS conversation \
           ON conversation.id = message.conversation_id \
          AND conversation.request_status = 'accepted' \
         JOIN forum.dm_participants AS participant \
           ON participant.conversation_id = message.conversation_id \
          AND participant.account_id = $1 \
          AND participant.deleted_at IS NULL \
         WHERE message.sender_id <> $1 \
           AND (participant.last_read_message_id IS NULL \
                OR message.id > participant.last_read_message_id)",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Return accepted-message unread count and pending incoming request count separately.
pub async fn counts(pool: &PgPool, account_id: i64) -> AppResult<DmCounts> {
    let counts = sqlx::query_as::<_, DmCounts>(
        "SELECT \
           (SELECT COUNT(*) \
            FROM forum.dm_messages AS message \
            JOIN forum.dm_conversations AS conversation \
              ON conversation.id = message.conversation_id \
             AND conversation.request_status = 'accepted' \
            JOIN forum.dm_participants AS participant \
              ON participant.conversation_id = conversation.id \
             AND participant.account_id = $1 \
             AND participant.deleted_at IS NULL \
            WHERE message.sender_id <> $1 \
              AND (participant.last_read_message_id IS NULL \
                   OR message.id > participant.last_read_message_id)) AS unread_count, \
           (SELECT COUNT(*) FROM forum.dm_conversations AS request \
            JOIN forum.dm_participants AS participant \
              ON participant.conversation_id = request.id \
             AND participant.account_id = $1 \
             AND participant.deleted_at IS NULL \
            WHERE request.request_status = 'pending' \
              AND request.request_recipient_id = $1) AS request_count",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(counts)
}

/// Return whether the participant muted notifications for one conversation.
pub async fn participant_is_muted(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let is_muted = sqlx::query_scalar(
        "SELECT COALESCE(( \
           SELECT muted_at IS NOT NULL FROM forum.dm_participants \
           WHERE conversation_id = $1 AND account_id = $2 \
         ), false)",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(is_muted)
}

/// Set or clear participant-local archive state for an active conversation.
pub async fn set_archived(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
    is_archived: bool,
) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE forum.dm_participants \
         SET archived_at = CASE WHEN $3 THEN COALESCE(archived_at, now()) ELSE NULL END \
         WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NULL \
           AND EXISTS (SELECT 1 FROM forum.dm_conversations \
                       WHERE id = $1 AND request_status = 'accepted')",
    )
    .bind(conversation_id)
    .bind(account_id)
    .bind(is_archived)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Set or clear participant-local notification mute state.
pub async fn set_muted(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
    is_muted: bool,
) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE forum.dm_participants \
         SET muted_at = CASE WHEN $3 THEN COALESCE(muted_at, now()) ELSE NULL END \
         WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NULL \
           AND EXISTS (SELECT 1 FROM forum.dm_conversations \
                       WHERE id = $1 AND request_status = 'accepted')",
    )
    .bind(conversation_id)
    .bind(account_id)
    .bind(is_muted)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Hide a conversation only from one participant's inbox.
pub async fn delete_for_participant(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE forum.dm_participants \
         SET deleted_at = now(), archived_at = NULL \
         WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NULL \
           AND EXISTS (SELECT 1 FROM forum.dm_conversations \
                       WHERE id = $1 AND request_status = 'accepted')",
    )
    .bind(conversation_id)
    .bind(account_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Recover a participant-hidden conversation without changing the other participant's state.
pub async fn recover_for_participant(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE forum.dm_participants \
         SET deleted_at = NULL, archived_at = NULL \
         WHERE conversation_id = $1 AND account_id = $2 AND deleted_at IS NOT NULL \
           AND EXISTS (SELECT 1 FROM forum.dm_conversations \
                       WHERE id = $1 AND request_status = 'accepted')",
    )
    .bind(conversation_id)
    .bind(account_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
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
         JOIN forum.dm_conversations AS conversation \
           ON conversation.id = mine.conversation_id \
          AND conversation.request_status = 'accepted' \
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

/// Insert a message after transactionally rechecking the canonical participant pair.
pub async fn send_message(
    pool: &PgPool,
    conversation_id: i64,
    sender_id: i64,
    recipient_id: i64,
    body: &str,
    client_message_id: Option<Uuid>,
) -> AppResult<(i64, DateTime<Utc>)> {
    let mut transaction = pool.begin().await?;
    if !identity::public_accounts::lock_active_interaction_accounts(
        &mut transaction,
        &[sender_id.min(recipient_id), sender_id.max(recipient_id)],
    )
    .await?
    {
        return Err(AppError::Forbidden);
    }
    super::relationships::lock_pair_unblocked(&mut transaction, sender_id, recipient_id).await?;
    let is_canonical_pair: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
           SELECT 1 FROM forum.dm_conversations AS conversation \
           JOIN forum.dm_participants AS sender \
             ON sender.conversation_id = conversation.id AND sender.account_id = $2 \
           JOIN forum.dm_participants AS recipient \
             ON recipient.conversation_id = conversation.id AND recipient.account_id = $3 \
           WHERE conversation.id = $1 AND conversation.request_status = 'accepted' \
             AND sender.deleted_at IS NULL \
         )",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(recipient_id)
    .fetch_one(&mut *transaction)
    .await?;
    if !is_canonical_pair {
        return Err(AppError::Forbidden);
    }

    if let Some(client_message_id) = client_message_id {
        let lock_key = format!("dm-message:{sender_id}:{client_message_id}");
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock_key)
            .execute(&mut *transaction)
            .await?;
        let existing: Option<(i64, i64, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, conversation_id, body, created_at FROM forum.dm_messages \
             WHERE sender_id = $1 AND client_message_id = $2",
        )
        .bind(sender_id)
        .bind(client_message_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some((id, stored_conversation_id, stored_body, created_at)) = existing {
            if stored_conversation_id != conversation_id || stored_body != body {
                return Err(AppError::Conflict(
                    "client message identity was already used for another message".into(),
                ));
            }
            transaction.commit().await?;
            return Ok((id, created_at));
        }
    }

    let row: InsertedMessage = sqlx::query_as(
        "INSERT INTO forum.dm_messages (conversation_id, sender_id, body, client_message_id) \
         SELECT $1, $2, $3, $4 \
         WHERE EXISTS ( \
           SELECT 1 FROM forum.dm_participants AS participant \
           JOIN forum.dm_conversations AS conversation \
             ON conversation.id = participant.conversation_id \
            AND conversation.request_status = 'accepted' \
           WHERE participant.conversation_id = $1 AND participant.account_id = $2 \
             AND participant.deleted_at IS NULL \
         ) \
         RETURNING id, created_at",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(body)
    .bind(client_message_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::Forbidden)?;

    sqlx::query(
        "UPDATE forum.dm_participants \
         SET deleted_at = NULL, archived_at = NULL \
         WHERE conversation_id = $1",
    )
    .bind(conversation_id)
    .execute(&mut *transaction)
    .await?;

    let sender =
        identity::public_accounts::lock_notification_recipient(&mut transaction, sender_id)
            .await?
            .ok_or(AppError::Forbidden)?;
    platform::outbox::enqueue_notification_tx(
        &mut transaction,
        &format!("dm-message:{}", row.id),
        recipient_id,
        Some(sender_id),
        "dm",
        &serde_json::json!({
            "conversationId": conversation_id.to_string(),
            "messageId": row.id.to_string(),
            "senderHandle": &sender.handle,
            "title": format!("{} 发来私信", sender.handle),
            "bodyExcerpt": body.chars().take(100).collect::<String>(),
        }),
        Some(&conversation_id.to_string()),
        None,
    )
    .await?;

    transaction.commit().await?;

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
                    account.handle::text AS sender_handle, \
                    sp.display_name AS sender_display_name, \
                    message.body, message.created_at \
             FROM forum.dm_messages AS message \
             JOIN identity.accounts AS account ON account.id = message.sender_id \
             LEFT JOIN identity.profiles AS sp ON sp.account_id = account.id \
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
                    account.handle::text AS sender_handle, \
                    sp.display_name AS sender_display_name, \
                    message.body, message.created_at \
             FROM forum.dm_messages AS message \
             JOIN identity.accounts AS account ON account.id = message.sender_id \
             LEFT JOIN identity.profiles AS sp ON sp.account_id = account.id \
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
           SELECT 1 FROM forum.dm_participants AS participant \
           JOIN forum.dm_conversations AS conversation \
             ON conversation.id = participant.conversation_id \
            AND conversation.request_status IN ('accepted', 'pending') \
           WHERE participant.conversation_id = $1 AND participant.account_id = $2 \
             AND participant.deleted_at IS NULL \
         )",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Directional accounts for one pending request.
#[derive(Debug, Clone, Copy, FromRow)]
pub struct DmRequestParties {
    pub sender_id: i64,
    pub recipient_id: i64,
}

/// Current request state retained after acceptance for idempotent response replay.
#[derive(Debug, Clone, FromRow)]
pub struct DmRequestState {
    pub sender_id: i64,
    pub recipient_id: i64,
    pub request_status: String,
}

/// Result of accepting a request, including whether this call performed the transition.
#[derive(Debug, Clone, Copy)]
pub struct DmAcceptResult {
    pub sender_id: i64,
    pub changed: bool,
}

/// Return a request-originated conversation state when the viewer is one of its parties.
pub async fn get_request_state(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<Option<DmRequestState>> {
    let state = sqlx::query_as::<_, DmRequestState>(
        "SELECT request_sender_id AS sender_id, request_recipient_id AS recipient_id, \
                request_status \
         FROM forum.dm_conversations \
         WHERE id = $1 AND request_sender_id IS NOT NULL AND request_recipient_id IS NOT NULL \
           AND $2 IN (request_sender_id, request_recipient_id)",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    Ok(state)
}

/// Accept a pending request as its recipient and unlock normal bidirectional delivery.
pub async fn accept_request(
    pool: &PgPool,
    conversation_id: i64,
    recipient_id: i64,
) -> AppResult<DmAcceptResult> {
    let state =
        get_request_state(pool, conversation_id, recipient_id).await?.ok_or(AppError::NotFound)?;
    if state.recipient_id != recipient_id {
        return Err(AppError::Forbidden);
    }
    if state.request_status == "accepted" {
        return Ok(DmAcceptResult { sender_id: state.sender_id, changed: false });
    }
    if state.request_status != "pending" {
        return Err(AppError::Conflict("message request is no longer pending".into()));
    }

    let account_low_id = state.sender_id.min(state.recipient_id);
    let account_high_id = state.sender_id.max(state.recipient_id);
    let mut transaction = pool.begin().await?;
    if !identity::public_accounts::lock_active_interaction_accounts(
        &mut transaction,
        &[account_low_id, account_high_id],
    )
    .await?
    {
        return Err(AppError::Forbidden);
    }
    super::relationships::lock_pair_unblocked(
        &mut transaction,
        state.sender_id,
        state.recipient_id,
    )
    .await?;
    if identity::public_accounts::lock_dm_policy(&mut transaction, recipient_id).await? == "nobody"
    {
        return Err(AppError::Forbidden);
    }
    let requested_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "UPDATE forum.dm_conversations \
         SET request_status = 'accepted', responded_at = now(), request_cooldown_until = NULL \
         WHERE id = $1 AND request_status = 'pending' AND request_recipient_id = $2 \
           AND request_sender_id = $3 \
         RETURNING requested_at",
    )
    .bind(conversation_id)
    .bind(recipient_id)
    .bind(state.sender_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let requested_at = requested_at
        .ok_or_else(|| AppError::Conflict("message request is no longer pending".into()))?;
    sqlx::query(
        "UPDATE forum.dm_participants \
         SET deleted_at = NULL, archived_at = NULL, \
             last_read_message_id = CASE WHEN account_id = $2 THEN \
               (SELECT MAX(id) FROM forum.dm_messages WHERE conversation_id = $1) \
               ELSE last_read_message_id END \
         WHERE conversation_id = $1",
    )
    .bind(conversation_id)
    .bind(recipient_id)
    .execute(&mut *transaction)
    .await?;
    platform::outbox::enqueue_notification_tx(
        &mut transaction,
        &format!("dm-request-accepted:{conversation_id}:{}", requested_at.timestamp_micros()),
        state.sender_id,
        Some(recipient_id),
        "dm_request_accepted",
        &serde_json::json!({
            "conversationId": conversation_id.to_string(),
            "requestedAtMicros": requested_at.timestamp_micros().to_string(),
            "title": "对方已接受你的消息请求",
        }),
        Some(&conversation_id.to_string()),
        None,
    )
    .await?;
    transaction.commit().await?;
    Ok(DmAcceptResult { sender_id: state.sender_id, changed: true })
}

/// Decline an incoming request or withdraw an outgoing request without creating a block.
pub async fn decline_request(
    pool: &PgPool,
    conversation_id: i64,
    account_id: i64,
) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    let _parties = sqlx::query_as::<_, DmRequestParties>(
        "SELECT request_sender_id AS sender_id, request_recipient_id AS recipient_id \
         FROM forum.dm_conversations \
         WHERE id = $1 AND request_status = 'pending' \
           AND $2 IN (request_sender_id, request_recipient_id) \
         FOR UPDATE",
    )
    .bind(conversation_id)
    .bind(account_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    sqlx::query(
        "UPDATE forum.dm_conversations \
         SET request_status = 'declined', responded_at = now(), \
             request_cooldown_until = now() + CASE \
               WHEN request_recipient_id = $2 THEN interval '30 days' \
               ELSE interval '5 minutes' END \
         WHERE id = $1",
    )
    .bind(conversation_id)
    .bind(account_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "DELETE FROM forum.dm_messages AS message \
         WHERE message.conversation_id = $1 \
           AND NOT EXISTS (SELECT 1 FROM forum.dm_message_reports AS report \
                           WHERE report.message_id = message.id)",
    )
    .bind(conversation_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// Report the single request message and atomically remove the request from the inbox.
pub async fn report_request(
    pool: &PgPool,
    conversation_id: i64,
    recipient_id: i64,
    reason: &str,
    note: Option<&str>,
) -> AppResult<i64> {
    let mut transaction = pool.begin().await?;
    let parties = sqlx::query_as::<_, DmRequestParties>(
        "SELECT request_sender_id AS sender_id, request_recipient_id AS recipient_id \
         FROM forum.dm_conversations \
         WHERE id = $1 AND request_status = 'pending' AND request_recipient_id = $2 \
         FOR UPDATE",
    )
    .bind(conversation_id)
    .bind(recipient_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    let message_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.dm_messages \
         WHERE conversation_id = $1 AND sender_id = $2 ORDER BY id ASC LIMIT 1",
    )
    .bind(conversation_id)
    .bind(parties.sender_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    let report_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.dm_message_reports \
           (message_id, conversation_id, reported_by, reason, note) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (message_id, reported_by) DO UPDATE SET message_id = EXCLUDED.message_id \
         RETURNING id",
    )
    .bind(message_id)
    .bind(conversation_id)
    .bind(recipient_id)
    .bind(reason)
    .bind(note)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE forum.dm_conversations \
         SET request_status = 'declined', responded_at = now(), \
             request_cooldown_until = now() + interval '30 days' \
         WHERE id = $1",
    )
    .bind(conversation_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(report_id)
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
           JOIN forum.dm_conversations AS conversation \
             ON conversation.id = message.conversation_id \
            AND conversation.request_status IN ('accepted', 'pending') \
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
                rp.display_name AS reporter_display_name, \
                message.sender_id, sender.handle::text AS sender_handle, \
                sp.display_name AS sender_display_name, \
                LEFT(message.body, 1000) AS message_excerpt, \
                report.reason, report.note, report.status, \
                report.handled_by, report.handled_at, report.created_at \
         FROM forum.dm_message_reports AS report \
         JOIN forum.dm_messages AS message ON message.id = report.message_id \
         JOIN identity.accounts AS reporter ON reporter.id = report.reported_by \
         LEFT JOIN identity.profiles AS rp ON rp.account_id = reporter.id \
         JOIN identity.accounts AS sender ON sender.id = message.sender_id \
         LEFT JOIN identity.profiles AS sp ON sp.account_id = sender.id \
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
                rp.display_name AS reporter_display_name, \
                message.sender_id, sender.handle::text AS sender_handle, \
                sp.display_name AS sender_display_name, \
                LEFT(message.body, 1000) AS message_excerpt, \
                report.reason, report.note, report.status, \
                report.handled_by, report.handled_at, report.created_at \
         FROM forum.dm_message_reports AS report \
         JOIN forum.dm_messages AS message ON message.id = report.message_id \
         JOIN identity.accounts AS reporter ON reporter.id = report.reported_by \
         LEFT JOIN identity.profiles AS rp ON rp.account_id = reporter.id \
         JOIN identity.accounts AS sender ON sender.id = message.sender_id \
         LEFT JOIN identity.profiles AS sp ON sp.account_id = sender.id \
         WHERE report.id = $1",
    )
    .bind(report_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

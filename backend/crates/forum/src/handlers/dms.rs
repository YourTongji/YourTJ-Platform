//! DM (1:1 private message) handlers.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{
    DmConversationCreatedDto, DmConversationDto, DmConversationInput, DmMessageDto, DmMessageInput,
};
use crate::repo;

use super::default_limit;

// ---------------------------------------------------------------------------
// query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageListQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// POST /api/v2/forum/dm/conversations
///
/// Find or create a 1:1 conversation with another user.
/// Requires trust level >= 1 (TL0 cannot initiate DMs).
pub async fn create_or_get_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DmConversationInput>,
) -> AppResult<Json<DmConversationCreatedDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    // TL0 gate
    let tl = crate::trust_levels::get_trust_level(state.redis.as_ref(), &state.db, auth.id).await?;
    if tl == 0 {
        return Err(AppError::Forbidden);
    }

    let recipient_id: i64 = body
        .recipient_id
        .parse()
        .map_err(|_| AppError::BadRequest("invalid recipientId".into()))?;

    // Cannot DM self
    if recipient_id == auth.id {
        return Err(AppError::BadRequest("cannot start a conversation with yourself".into()));
    }

    // Recipient must exist
    let recipient_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE id = $1)")
            .bind(recipient_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(false);

    if !recipient_exists {
        return Err(AppError::BadRequest("recipient not found".into()));
    }

    // Check if recipient has blocked sender
    let blocked = crate::repo::is_ignored(&state.db, recipient_id, auth.id).await?;
    if blocked {
        return Err(AppError::Forbidden);
    }

    let conversation_id =
        repo::dms::find_or_create_conversation(&state.db, auth.id, recipient_id).await?;

    Ok(Json(DmConversationCreatedDto { id: conversation_id.to_string() }))
}

/// POST /api/v2/forum/dm/conversations/{id}/messages
///
/// Send a message in a DM conversation.
/// Rate limited: 30 messages per 60 seconds per sender.
pub async fn send_message_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(body): Json<DmMessageInput>,
) -> AppResult<Json<DmMessageDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    if body.body.trim().is_empty() {
        return Err(AppError::BadRequest("message body must not be empty".into()));
    }

    let conversation_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Must be a participant
    let participant = repo::dms::is_participant(&state.db, conversation_id, auth.id).await?;
    if !participant {
        return Err(AppError::Forbidden);
    }

    // Rate limit: 30 messages per 60 seconds per sender
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "dm_send",
        &auth.id.to_string(),
        30,
        60,
    )
    .await?;

    let (message_id, created_at) =
        repo::dms::send_message(&state.db, conversation_id, auth.id, &body.body).await?;

    // Find the other participant and create notification (fire-and-forget).
    let pool = state.db.clone();
    let conv_id_str = conversation_id.to_string();
    let body_excerpt = body.body.chars().take(100).collect::<String>();
    let sender_id = auth.id;
    tokio::spawn(async move {
        let other: Option<(i64,)> = sqlx::query_as(
            "SELECT dp.account_id \
             FROM forum.dm_participants dp \
             WHERE dp.conversation_id = $1 AND dp.account_id != $2 \
             LIMIT 1",
        )
        .bind(&conv_id_str)
        .bind(sender_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if let Some((recipient_id,)) = other {
            let sender_handle: String =
                sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
                    .bind(sender_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap_or_default();

            crate::notification_hooks::create_notification(
                &pool,
                recipient_id,
                "dm",
                serde_json::json!({
                    "conversationId": conv_id_str,
                    "senderHandle": sender_handle,
                    "bodyExcerpt": body_excerpt,
                }),
                Some(&conv_id_str),
                Some(sender_id),
            )
            .await;
        }
    });

    // Build the DTO
    let sender_handle: String =
        sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
            .bind(auth.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or_default();

    let dto = DmMessageDto {
        id: message_id.to_string(),
        conversation_id: conversation_id.to_string(),
        sender_id: auth.id.to_string(),
        sender_handle,
        body: body.body,
        created_at: created_at.timestamp(),
    };

    Ok(Json(dto))
}

/// GET /api/v2/forum/dm/conversations
///
/// List DM conversations for the authenticated user.
pub async fn list_conversations_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<DmConversationDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let rows = repo::dms::list_conversations(&state.db, auth.id).await?;

    let items: Vec<DmConversationDto> = rows
        .into_iter()
        .map(|r| DmConversationDto {
            id: r.id.to_string(),
            participant_handle: r.other_handle,
            participant_id: r.other_account_id.to_string(),
            last_message_at: r.last_message_at.timestamp(),
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/v2/forum/dm/conversations/{id}/messages
///
/// List messages in a DM conversation (paginated, newest first).
pub async fn list_messages_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Query(q): Query<DmMessageListQuery>,
) -> AppResult<Json<Page<DmMessageDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let conversation_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Must be a participant
    let participant = repo::dms::is_participant(&state.db, conversation_id, auth.id).await?;
    if !participant {
        return Err(AppError::Forbidden);
    }

    let cursor_id: Option<i64> = q
        .cursor
        .as_deref()
        .map(|c| c.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into())))
        .transpose()?;

    let (rows, next_cursor) =
        repo::dms::list_messages(&state.db, conversation_id, auth.id, cursor_id, q.limit).await?;

    let items: Vec<DmMessageDto> = rows
        .into_iter()
        .map(|r| DmMessageDto {
            id: r.id.to_string(),
            conversation_id: r.conversation_id.to_string(),
            sender_id: r.sender_id.to_string(),
            sender_handle: r.sender_handle,
            body: r.body,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    let next_str = next_cursor.map(|c| c.to_string());
    Ok(Json(Page::new(items, next_str)))
}

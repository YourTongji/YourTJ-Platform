//! Private 1:1 conversation handlers and the scoped DM report queue.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use shared::auth::Capability;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{
    DmConversationDto, DmConversationInput, DmMessageDto, DmMessageInput, DmMessageReportDto,
    DmMessageReportInput, DmReadInput, DmReportResolveInput,
};
use crate::models::{DmConversationListRow, DmMessageReportRow};
use crate::repo;

use super::default_limit;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmConversationListQuery {
    pub view: Option<String>,
    pub q: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageListQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmReportListQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn validate_limit(limit: i64) -> AppResult<i64> {
    if !(1..=100).contains(&limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    Ok(limit)
}

fn conversation_to_dto(row: DmConversationListRow) -> DmConversationDto {
    DmConversationDto {
        id: row.id.to_string(),
        participant_id: row.other_account_id.to_string(),
        participant_handle: row.other_handle,
        participant_avatar_url: row.other_avatar_url,
        last_message_excerpt: row.last_message_excerpt,
        last_message_at: row.last_message_at.timestamp(),
        unread_count: row.unread_count,
        is_archived: row.is_archived,
        is_muted: row.is_muted,
        is_deleted: row.is_deleted,
        created_at: row.created_at.timestamp(),
    }
}

fn report_to_dto(row: DmMessageReportRow) -> DmMessageReportDto {
    DmMessageReportDto {
        id: row.id.to_string(),
        message_id: row.message_id.to_string(),
        conversation_id: row.conversation_id.to_string(),
        reporter_id: row.reported_by.to_string(),
        reporter_handle: row.reporter_handle,
        sender_id: row.sender_id.to_string(),
        sender_handle: row.sender_handle,
        message_excerpt: row.message_excerpt,
        reason: row.reason,
        note: row.note,
        status: row.status,
        handled_by: row.handled_by.map(|id| id.to_string()),
        handled_at: row.handled_at.map(|timestamp| timestamp.timestamp()),
        created_at: row.created_at.timestamp(),
    }
}

/// Find or create the canonical conversation with a recipient handle.
pub async fn create_or_get_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DmConversationInput>,
) -> AppResult<Json<DmConversationDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    if identity::sanctions::is_silenced(state.redis.as_ref(), &state.db, auth.id).await? {
        return Err(AppError::Forbidden);
    }

    let trust_level = crate::trust_levels::get_trust_level(&state.db, auth.id).await?;
    if trust_level == 0 {
        return Err(AppError::Forbidden);
    }

    let recipient_handle = body.recipient_handle.trim();
    if !(3..=30).contains(&recipient_handle.chars().count()) {
        return Err(AppError::BadRequest("recipientHandle must contain 3 to 30 characters".into()));
    }

    let recipient =
        identity::public_accounts::find_public_account_by_handle(&state.db, recipient_handle)
            .await?
            .ok_or(AppError::NotFound)?;
    let recipient_id = recipient.id;
    if recipient_id == auth.id {
        return Err(AppError::BadRequest("cannot start a conversation with yourself".into()));
    }

    if repo::relationships::pair_is_blocked(&state.db, auth.id, recipient_id).await? {
        return Err(AppError::Forbidden);
    }
    let can_start_conversation = match recipient.dm_policy.as_str() {
        "everyone" => true,
        "following" => {
            crate::repo::relationships::is_following(&state.db, recipient_id, auth.id).await?
        }
        "nobody" => false,
        _ => false,
    };
    if !can_start_conversation {
        return Err(AppError::Forbidden);
    }

    let conversation_id =
        repo::dms::find_or_create_conversation(&state.db, auth.id, recipient_id).await?;
    let conversation = repo::dms::get_conversation(&state.db, conversation_id, auth.id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(conversation_to_dto(conversation)))
}

/// Return the authenticated participant's paginated inbox.
pub async fn list_conversations_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DmConversationListQuery>,
) -> AppResult<Json<Page<DmConversationDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let limit = validate_limit(query.limit)?;
    let view = query.view.as_deref().unwrap_or("inbox");
    if !matches!(view, "inbox" | "archived" | "deleted") {
        return Err(AppError::BadRequest("view must be inbox, archived, or deleted".into()));
    }
    let search_query = query.q.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if search_query.is_some_and(|value| !(2..=100).contains(&value.chars().count())) {
        return Err(AppError::BadRequest("q must contain 2 to 100 characters".into()));
    }
    let cursor = query.cursor.as_deref().map(repo::dms::decode_conversation_cursor).transpose()?;
    let (rows, next_cursor) =
        repo::dms::list_conversations(&state.db, auth.id, view, search_query, cursor, limit)
            .await?;
    let items = rows.into_iter().map(conversation_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// Return an account-scoped unread DM count for the global navigation badge.
pub async fn unread_dm_count_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let count = repo::dms::unread_count(&state.db, auth.id).await?;
    Ok(Json(serde_json::json!({ "count": count })))
}

async fn participant_action(
    state: &AppState,
    headers: &HeaderMap,
    id_str: &str,
    action: &str,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let conversation_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let changed = match action {
        "archive" => repo::dms::set_archived(&state.db, conversation_id, auth.id, true).await?,
        "unarchive" => repo::dms::set_archived(&state.db, conversation_id, auth.id, false).await?,
        "mute" => repo::dms::set_muted(&state.db, conversation_id, auth.id, true).await?,
        "unmute" => repo::dms::set_muted(&state.db, conversation_id, auth.id, false).await?,
        "delete" => repo::dms::delete_for_participant(&state.db, conversation_id, auth.id).await?,
        "recover" => {
            repo::dms::recover_for_participant(&state.db, conversation_id, auth.id).await?
        }
        _ => false,
    };
    if !changed {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Archive a conversation only for the current participant.
pub async fn archive_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    participant_action(&state, &headers, &id, "archive").await
}

/// Return a participant's archived conversation to the inbox.
pub async fn unarchive_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    participant_action(&state, &headers, &id, "unarchive").await
}

/// Mute notifications without changing unread message facts.
pub async fn mute_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    participant_action(&state, &headers, &id, "mute").await
}

/// Restore notifications for a conversation.
pub async fn unmute_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    participant_action(&state, &headers, &id, "unmute").await
}

/// Hide a conversation only for the current participant.
pub async fn delete_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    participant_action(&state, &headers, &id, "delete").await
}

/// Recover a participant-hidden conversation.
pub async fn recover_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    participant_action(&state, &headers, &id, "recover").await
}

/// Send a message after rechecking sanctions, lifecycle, membership, and blocks.
pub async fn send_message_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(body): Json<DmMessageInput>,
) -> AppResult<(StatusCode, Json<DmMessageDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let character_count = body.body.chars().count();
    if body.body.trim().is_empty() || !(1..=16000).contains(&character_count) {
        return Err(AppError::BadRequest("message body must contain 1 to 16000 characters".into()));
    }
    if identity::sanctions::is_silenced(state.redis.as_ref(), &state.db, auth.id).await? {
        return Err(AppError::Forbidden);
    }

    let conversation_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let recipient_id =
        repo::dms::find_available_other_participant(&state.db, conversation_id, auth.id)
            .await?
            .ok_or(AppError::Forbidden)?;
    if repo::relationships::pair_is_blocked(&state.db, auth.id, recipient_id).await? {
        return Err(AppError::Forbidden);
    }
    let recipient_is_muted =
        repo::dms::participant_is_muted(&state.db, conversation_id, recipient_id).await?;

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
    let sender_handle: String =
        sqlx::query_scalar("SELECT handle::text FROM identity.accounts WHERE id = $1")
            .bind(auth.id)
            .fetch_one(&state.db)
            .await?;

    if !recipient_is_muted {
        let pool = state.db.clone();
        let conversation_id_string = conversation_id.to_string();
        let body_excerpt = body.body.chars().take(100).collect::<String>();
        let sender_id = auth.id;
        let notification_sender_handle = sender_handle.clone();
        tokio::spawn(async move {
            crate::notification_hooks::create_notification(
                &pool,
                recipient_id,
                "dm",
                serde_json::json!({
                    "conversationId": conversation_id_string,
                    "senderHandle": notification_sender_handle,
                    "bodyExcerpt": body_excerpt,
                }),
                Some(&conversation_id.to_string()),
                Some(sender_id),
            )
            .await;
        });
    }

    Ok((
        StatusCode::CREATED,
        Json(DmMessageDto {
            id: message_id.to_string(),
            conversation_id: conversation_id.to_string(),
            sender_id: auth.id.to_string(),
            sender_handle,
            body: body.body,
            created_at: created_at.timestamp(),
        }),
    ))
}

/// List messages in a conversation for a participant only.
pub async fn list_messages_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Query(query): Query<DmMessageListQuery>,
) -> AppResult<Json<Page<DmMessageDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let conversation_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let limit = validate_limit(query.limit)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| {
            value.parse::<i64>().map_err(|_| AppError::BadRequest("invalid message cursor".into()))
        })
        .transpose()?;

    let (rows, next_cursor) =
        repo::dms::list_messages(&state.db, conversation_id, auth.id, cursor, limit).await?;
    let items = rows
        .into_iter()
        .map(|row| DmMessageDto {
            id: row.id.to_string(),
            conversation_id: row.conversation_id.to_string(),
            sender_id: row.sender_id.to_string(),
            sender_handle: row.sender_handle,
            body: row.body,
            created_at: row.created_at.timestamp(),
        })
        .collect();
    Ok(Json(Page::new(items, next_cursor.map(|id| id.to_string()))))
}

/// Advance the current participant's read pointer.
pub async fn read_conversation_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    body: Option<Json<DmReadInput>>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let conversation_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let message_id = body
        .and_then(|Json(input)| input.last_read_message_id)
        .as_deref()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("invalid lastReadMessageId".into()))
        })
        .transpose()?;
    repo::dms::advance_read_pointer(&state.db, conversation_id, auth.id, message_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Report one message from a conversation the reporter participates in.
pub async fn report_message_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(body): Json<DmMessageReportInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let message_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let reason = body.reason.trim();
    if !matches!(reason, "spam" | "abuse" | "harassment" | "fraud" | "illegal" | "other") {
        return Err(AppError::BadRequest("invalid DM report reason".into()));
    }
    if body.note.as_ref().is_some_and(|note| note.chars().count() > 1000) {
        return Err(AppError::BadRequest("report note must not exceed 1000 characters".into()));
    }
    if !repo::dms::can_access_message(&state.db, message_id, auth.id).await? {
        return Err(AppError::Forbidden);
    }

    let report_id =
        repo::dms::report_message(&state.db, message_id, auth.id, reason, body.note.as_deref())
            .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "id": report_id.to_string(), "status": "open" })),
    ))
}

/// List reported messages only; this is not a general DM browsing endpoint.
pub async fn list_dm_reports_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DmReportListQuery>,
) -> AppResult<Json<Page<DmMessageReportDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(Capability::ModerateContent).map_err(|_| AppError::Forbidden)?;

    let status = query.status.as_deref().unwrap_or("open");
    if !matches!(status, "open" | "upheld" | "rejected") {
        return Err(AppError::BadRequest("invalid DM report status".into()));
    }
    let limit = validate_limit(query.limit)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("invalid DM report cursor".into()))
        })
        .transpose()?;
    let (rows, next_cursor) =
        repo::dms::list_message_reports(&state.db, status, cursor, limit).await?;
    let evidence_count = rows.len();
    let audit_metadata = serde_json::json!({
        "count": evidence_count,
        "status": status,
    });
    governance::record_account_event(
        &state.db,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "forum.dm_report.evidence_listed",
        "dm_report_queue",
        status,
        "DM report evidence listed",
        Some(&audit_metadata),
    )
    .await?;
    let items = rows.into_iter().map(report_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor.map(|id| id.to_string()))))
}

/// Resolve one open DM report and record the staff action atomically.
pub async fn resolve_dm_report_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(body): Json<DmReportResolveInput>,
) -> AppResult<Json<DmMessageReportDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(Capability::ModerateContent).map_err(|_| AppError::Forbidden)?;
    if !matches!(body.action.as_str(), "uphold" | "reject") {
        return Err(AppError::BadRequest("action must be uphold or reject".into()));
    }
    let note = body.note.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if note.is_some_and(|value| value.chars().count() > 1000) {
        return Err(AppError::BadRequest("resolution note must not exceed 1000 characters".into()));
    }
    let report_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let report = repo::dms::resolve_message_report(
        &state.db,
        report_id,
        &body.action,
        auth.id,
        &auth.role,
        note,
    )
    .await?;
    Ok(Json(report_to_dto(report)))
}

//! Reasoned forum report review with atomic target and audit transitions.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::models::{FlagQueueRow, FlagRow};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagsQueueQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFlagInput {
    pub action: String,
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFlagDto {
    id: String,
    target_type: String,
    target_id: String,
    reporter_id: String,
    reason: String,
    note: Option<String>,
    weight: f32,
    status: String,
    handled_by: Option<String>,
    handled_at: Option<i64>,
    resolution_note: Option<String>,
    created_at: i64,
    author_handle: Option<String>,
    target_title: Option<String>,
    content_excerpt: Option<String>,
}

fn flag_dto(row: FlagRow) -> AdminFlagDto {
    AdminFlagDto {
        id: row.id.to_string(),
        target_type: row.target_type,
        target_id: row.target_id.to_string(),
        reporter_id: row.reporter_id.to_string(),
        reason: row.reason,
        note: row.note,
        weight: row.weight,
        status: row.status,
        handled_by: row.handled_by.map(|id| id.to_string()),
        handled_at: row.handled_at.map(|timestamp| timestamp.timestamp()),
        resolution_note: row.resolution_note,
        created_at: row.created_at.timestamp(),
        author_handle: None,
        target_title: None,
        content_excerpt: None,
    }
}

fn queue_flag_dto(row: FlagQueueRow) -> AdminFlagDto {
    AdminFlagDto {
        id: row.id.to_string(),
        target_type: row.target_type,
        target_id: row.target_id.to_string(),
        reporter_id: row.reporter_id.to_string(),
        reason: row.reason,
        note: row.note,
        weight: row.weight,
        status: row.status,
        handled_by: row.handled_by.map(|id| id.to_string()),
        handled_at: row.handled_at.map(|timestamp| timestamp.timestamp()),
        resolution_note: row.resolution_note,
        created_at: row.created_at.timestamp(),
        author_handle: row.author_handle,
        target_title: row.target_title,
        content_excerpt: row.content_excerpt,
    }
}

/// GET /api/v2/admin/forum/flags — list the report queue.
pub async fn list_flags_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<FlagsQueueQuery>,
) -> AppResult<Json<Page<AdminFlagDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let status = query.status.as_deref().unwrap_or("open");
    if !matches!(status, "open" | "upheld" | "rejected" | "ignored" | "all") {
        return Err(AppError::BadRequest("invalid flag status".into()));
    }
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let (rows, next_cursor) =
        crate::repo::list_flag_queue(&state.db, Some(status), cursor, query.limit).await?;
    Ok(Json(Page::new(
        rows.into_iter().map(queue_flag_dto).collect(),
        next_cursor.map(|cursor| cursor.to_string()),
    )))
}

/// POST /api/v2/admin/forum/flags/{id}/resolve — resolve all open reports for the target.
pub async fn resolve_flag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(flag_id): Path<String>,
    Json(body): Json<ResolveFlagInput>,
) -> AppResult<Json<AdminFlagDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ModerateContent)
        .map_err(|_| AppError::Forbidden)?;

    let flag_id: i64 = flag_id.parse().map_err(|_| AppError::NotFound)?;
    let note = body.note.trim();
    if !(3..=500).contains(&note.chars().count()) {
        return Err(AppError::BadRequest("note must be 3–500 characters".into()));
    }
    if !matches!(body.action.as_str(), "uphold" | "reject" | "ignore") {
        return Err(AppError::BadRequest("action must be uphold/reject/ignore".into()));
    }

    let mut tx = state.db.begin().await?;
    let author_id = crate::repo::flags::lock_flag_target_author(&mut tx, flag_id).await?;
    super::require_lower_author_role(&mut tx, &auth, author_id).await?;
    let outcome = crate::repo::resolve_flag(&mut tx, flag_id, &body.action, auth.id, note).await?;
    crate::repo::insert_mod_action(
        &mut *tx,
        auth.id,
        &format!("resolve_flag_{}", body.action),
        "flag",
        flag_id,
        Some(note),
        None,
    )
    .await?;
    let metadata = serde_json::json!({ "contentChanged": outcome.content_changed });
    let governance_event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        &format!("forum.flag.{}", body.action),
        "forum_content",
        &format!("{}:{}", outcome.flag.target_type, outcome.flag.target_id),
        note,
        Some(&metadata),
    )
    .await?;
    if body.action == "uphold" && outcome.content_changed {
        if let Some(author_id) = author_id {
            governance::notices::create_notice_tx(
                &mut tx,
                author_id,
                "content_restricted",
                &format!("audit:{governance_event_id}:forum-flag"),
                Some(governance_event_id),
                None,
                if outcome.flag.target_type == "thread" { "forum_thread" } else { "forum_comment" },
                &outcome.flag.target_id.to_string(),
                "你的社区内容在举报复核后被软移除，可在申诉中心查看并申请复核。",
            )
            .await?;
            activity::trust::apply_governance_demotion_tx(
                &mut tx,
                author_id,
                governance_event_id,
                "forum flag upheld",
            )
            .await?;
        }
    }
    tx.commit().await?;
    crate::cache::invalidate_thread_surfaces(
        state.redis.as_ref(),
        outcome.thread_id,
        outcome.board_id,
    )
    .await;
    crate::meili::reconcile_thread_in_background(&state, outcome.thread_id);

    Ok(Json(flag_dto(outcome.flag)))
}

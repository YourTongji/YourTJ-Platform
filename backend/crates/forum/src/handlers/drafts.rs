//! Cross-device draft handlers with bounded typed payloads and optimistic concurrency.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{DraftDto, DraftInput, DraftPayload};
use crate::repo::drafts::DraftRow;

const MAX_DRAFTS: i64 = 50;
const MAX_DRAFT_KEY_CHARS: usize = 96;
const MAX_THREAD_BODY_CHARS: usize = 50_000;
const MAX_COMMENT_BODY_CHARS: usize = 16_000;

fn positive_id(value: &str) -> bool {
    value.parse::<i64>().is_ok_and(|id| id > 0)
}

fn validate_draft(draft_key: &str, payload: &DraftPayload) -> AppResult<()> {
    if draft_key.is_empty()
        || draft_key.chars().count() > MAX_DRAFT_KEY_CHARS
        || draft_key.chars().any(char::is_control)
    {
        return Err(AppError::BadRequest("invalid draftKey".into()));
    }

    match payload {
        DraftPayload::Thread {
            board_id, title, body, tags, poll_question, poll_options, ..
        } => {
            let target = draft_key.strip_prefix("thread:").ok_or_else(|| {
                AppError::BadRequest("thread draftKey must start with thread:".into())
            })?;
            if target != "new" && !positive_id(target) {
                return Err(AppError::BadRequest("invalid thread draftKey target".into()));
            }
            if let Some(board_id) = board_id {
                if !positive_id(board_id) {
                    return Err(AppError::BadRequest("invalid draft boardId".into()));
                }
                if target != "new" && target != board_id {
                    return Err(AppError::BadRequest("draftKey does not match boardId".into()));
                }
            }
            if title.chars().count() > 120 || body.chars().count() > MAX_THREAD_BODY_CHARS {
                return Err(AppError::BadRequest("thread draft exceeds content limits".into()));
            }
            if tags.len() > 3
                || tags.iter().any(|tag| tag.is_empty() || tag.chars().count() > 32)
                || poll_question.chars().count() > 300
                || poll_options.len() > 20
                || poll_options.iter().any(|option| option.chars().count() > 200)
            {
                return Err(AppError::BadRequest("thread draft metadata exceeds limits".into()));
            }
        }
        DraftPayload::Comment { thread_id, body, parent_id, .. } => {
            let target = draft_key.strip_prefix("comment:").ok_or_else(|| {
                AppError::BadRequest("comment draftKey must start with comment:".into())
            })?;
            if !positive_id(thread_id) || target != thread_id {
                return Err(AppError::BadRequest("draftKey does not match threadId".into()));
            }
            if parent_id.as_deref().is_some_and(|id| !positive_id(id)) {
                return Err(AppError::BadRequest("invalid draft parentId".into()));
            }
            if body.chars().count() > MAX_COMMENT_BODY_CHARS {
                return Err(AppError::BadRequest("comment draft exceeds content limits".into()));
            }
        }
    }
    Ok(())
}

fn draft_dto(row: DraftRow) -> DraftDto {
    DraftDto {
        draft_key: row.draft_key,
        payload: row.payload.0,
        version: row.version,
        updated_at: row.updated_at.timestamp(),
    }
}

/// Save a new draft at version zero or update the exact version last read by the client.
pub async fn save_draft_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DraftInput>,
) -> AppResult<Json<DraftDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    if body.expected_version < 0 {
        return Err(AppError::BadRequest("expectedVersion cannot be negative".into()));
    }
    validate_draft(&body.draft_key, &body.payload)?;

    let mut transaction = state.db.begin().await?;
    crate::repo::lock_draft_owner(&mut transaction, auth.id).await?;
    if body.expected_version == 0 {
        if crate::repo::draft_exists(&mut transaction, auth.id, &body.draft_key).await? {
            return Err(AppError::Conflict("draft already exists".into()));
        }
        if crate::repo::count_drafts(&mut transaction, auth.id).await? >= MAX_DRAFTS {
            return Err(AppError::BadRequest(format!("draft limit of {MAX_DRAFTS} reached")));
        }
    }
    let row = crate::repo::save_draft(
        &mut transaction,
        auth.id,
        &body.draft_key,
        &body.payload,
        body.expected_version,
    )
    .await?;
    transaction.commit().await?;

    Ok(Json(draft_dto(row)))
}

/// List all account-owned drafts in a bounded terminal page.
pub async fn list_drafts_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Page<DraftDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let items =
        crate::repo::list_drafts(&state.db, auth.id).await?.into_iter().map(draft_dto).collect();
    Ok(Json(Page::last(items)))
}

/// Return one full account-owned draft, including its compare-and-swap version.
pub async fn get_draft_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(draft_key): Path<String>,
) -> AppResult<Json<DraftDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    let row =
        crate::repo::get_draft(&state.db, auth.id, &draft_key).await?.ok_or(AppError::NotFound)?;
    Ok(Json(draft_dto(row)))
}

/// Idempotently delete one account-owned draft.
pub async fn delete_draft_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(draft_key): Path<String>,
) -> AppResult<impl IntoResponse> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;

    crate::repo::delete_draft(&state.db, auth.id, &draft_key).await?;
    Ok((StatusCode::NO_CONTENT, ()))
}

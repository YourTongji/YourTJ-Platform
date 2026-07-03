use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

use crate::dto::{CommentDto, CommentInput, CommentUpdateInput, RevisionDto};
use crate::repo;

use super::{comment_to_dto, default_limit};

// ---------------------------------------------------------------------------
// query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentListQuery {
    cursor: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/forum/threads/{thread_id}/comments — public
pub async fn list_comments(
    State(state): State<AppState>,
    Path(thread_id_str): Path<String>,
    Query(q): Query<CommentListQuery>,
) -> AppResult<Json<Page<CommentDto>>> {
    let thread_id: i64 = thread_id_str.parse().map_err(|_| AppError::NotFound)?;
    let (rows, next_cursor) =
        repo::list_comments(&state.db, thread_id, q.cursor.as_deref(), q.limit).await?;
    let items: Vec<CommentDto> = rows.iter().map(comment_to_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/forum/threads/{thread_id}/comments — auth required
pub async fn create_comment(
    State(state): State<AppState>,
    Path(thread_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CommentInput>,
) -> AppResult<Json<CommentDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    // Check that the account is not silenced
    crate::sanctions::require_can_post(state.redis.as_ref(), &state.db, auth.id).await?;

    let tl = crate::trust_levels::get_trust_level(state.redis.as_ref(), &state.db, auth.id).await?;
    if tl == 0 {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "comment_create_tl0",
            &auth.id.to_string(),
            5,
            86400,
        )
        .await?;
    } else {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "comment_create",
            &auth.id.to_string(),
            20,
            60,
        )
        .await?;
    }

    // Check watched words on the body text.
    let watched_word_action: Option<String> =
        crate::watched_words::check_watched_words(&body.body).map(|(_matched, action)| action);

    // Block action: reject before insert.
    if watched_word_action.as_deref() == Some("block") {
        return Err(AppError::BadRequest("content contains blocked words".into()));
    }

    let thread_id: i64 = thread_id_str.parse().map_err(|_| AppError::NotFound)?;

    let parent_id: Option<i64> = body
        .parent_id
        .as_deref()
        .map(|s| s.parse::<i64>().map_err(|_| AppError::BadRequest("invalid parentId".into())))
        .transpose()?;

    let row = repo::create_comment(&state.db, thread_id, auth.id, &body.body, parent_id).await?;

    // Update user_stats: comments_created +1 (best-effort).
    let _ = sqlx::query(
        "INSERT INTO forum.user_stats (account_id, comments_created, last_posted_at) \
         VALUES ($1, 1, now()) \
         ON CONFLICT (account_id) \
         DO UPDATE SET comments_created = forum.user_stats.comments_created + 1, \
                       last_posted_at = now()",
    )
    .bind(auth.id)
    .execute(&state.db)
    .await;

    // Auto-track: commenter automatically follows the thread.
    let _ =
        crate::repo::set_subscription(&state.db, auth.id, "thread", thread_id, "tracking").await;

    // Look up thread author for notification.
    let thread_author_id: i64 =
        sqlx::query_scalar("SELECT author_id FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&state.db)
            .await?;

    // Notify thread author of reply (fire-and-forget).
    if row.author_id != thread_author_id {
        let pool = state.db.clone();
        let payload = serde_json::json!({
            "threadId": thread_id.to_string(),
            "commentId": row.id.to_string(),
            "authorHandle": row.author_handle,
            "bodyExcerpt": row.body.chars().take(100).collect::<String>(),
        });
        tokio::spawn(async move {
            crate::notification_hooks::create_notification(
                &pool,
                thread_author_id,
                "reply",
                payload,
                None,
            )
            .await;
        });
    }

    // Look up own handle for self-mention filtering.
    let my_handle: String =
        sqlx::query_scalar("SELECT handle FROM identity.accounts WHERE id = $1")
            .bind(auth.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or_default();

    // Parse @mentions from body and notify mentioned users (fire-and-forget).
    let mention_re = regex::Regex::new(r"@([\p{L}\p{N}_-]+)").unwrap();
    let handles: Vec<String> = mention_re
        .captures_iter(&body.body)
        .map(|c| c[1].to_string())
        .filter(|h| h != &my_handle) // skip self-mentions
        .take(10)
        .collect();

    if !handles.is_empty() {
        let pool = state.db.clone();
        let comment_author = row.author_handle.clone();
        let comment_body = row.body.chars().take(100).collect::<String>();
        let thread_id_val = thread_id;
        let comment_id_val = row.id;
        tokio::spawn(async move {
            for handle in handles {
                let account_id: Option<i64> =
                    sqlx::query_scalar("SELECT id FROM identity.accounts WHERE handle = $1")
                        .bind(&handle)
                        .fetch_optional(&pool)
                        .await
                        .unwrap_or(None);
                if let Some(aid) = account_id {
                    crate::notification_hooks::create_notification(
                        &pool,
                        aid,
                        "mention",
                        serde_json::json!({
                            "threadId": thread_id_val.to_string(),
                            "commentId": comment_id_val.to_string(),
                            "authorHandle": comment_author,
                            "handle": handle,
                            "bodyExcerpt": comment_body,
                        }),
                        None,
                    )
                    .await;
                }
            }
        });
    }

    // Handle queue action: auto-hide the comment.
    if watched_word_action.as_deref() == Some("queue") {
        let _ = sqlx::query("UPDATE forum.comments SET hidden_at = now() WHERE id = $1")
            .bind(row.id)
            .execute(&state.db)
            .await;
    }

    // Build response DTO, applying censorship for censor action.
    let mut dto = comment_to_dto(&row);
    if watched_word_action.as_deref() == Some("censor") {
        dto.body = crate::watched_words::censor_text(&dto.body);
    }

    // Bump thread cache version.
    shared::cache::bump_version_opt(state.redis.as_ref(), "thread", &thread_id.to_string())
        .await
        .ok();

    Ok(Json(dto))
}

/// PATCH /api/v2/forum/comments/{id} — auth required (author or mod)
pub async fn update_comment(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CommentUpdateInput>,
) -> AppResult<Json<CommentDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Check ownership or mod
    let comment = repo::find_comment(&state.db, id).await?.ok_or(AppError::NotFound)?;
    if comment.author_id != auth.id && auth.role != "mod" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    // Check grace period: edits within 5 min don't create a revision
    let now = chrono::Utc::now();
    let within_grace = comment.created_at > now - chrono::Duration::minutes(5);

    // Save revision if outside grace period
    if !within_grace {
        let old_body = comment.body.clone();
        repo::create_revision(&state.db, "comment", id, auth.id, None, &old_body).await?;
    }

    // Update the comment
    let row = repo::update_comment(&state.db, id, &body.body).await?;

    // Bump thread cache
    shared::cache::bump_version_silent(state.redis.as_ref(), "thread", &row.thread_id.to_string())
        .await;

    Ok(Json(comment_to_dto(&row)))
}

/// DELETE /api/v2/forum/comments/{id} — auth required (author only)
pub async fn delete_comment(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let comment = repo::find_comment(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if comment.author_id != auth.id {
        return Err(AppError::Forbidden);
    }

    sqlx::query(
        "UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 WHERE id = $2 AND deleted_at IS NULL",
    )
    .bind(auth.id)
    .bind(id)
    .execute(&state.db)
    .await?;

    shared::cache::bump_version_silent(
        state.redis.as_ref(),
        "thread",
        &comment.thread_id.to_string(),
    )
    .await;

    Ok(Json(json!({"ok": true})))
}

/// GET /api/v2/forum/comments/{id}/revisions — auth required (author + mod)
pub async fn list_comment_revisions(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<RevisionDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let comment = repo::find_comment(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if comment.author_id != auth.id && auth.role != "mod" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let revs = repo::list_revisions(&state.db, "comment", id).await?;
    let dtos: Vec<RevisionDto> = revs
        .into_iter()
        .map(|r| RevisionDto {
            id: r.id.to_string(),
            seq: r.seq,
            editor_id: r.editor_id.to_string(),
            old_title: r.old_title,
            old_body: r.old_body,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(dtos))
}

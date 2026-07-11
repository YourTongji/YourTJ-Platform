//! Public community-profile handlers.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

use crate::dto::{UserBadgeDto, UserCommentDto, UserProfileDto, UserThreadDto};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPostPageQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

fn parse_cursor(cursor: Option<&str>) -> AppResult<Option<i64>> {
    cursor
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))
}

async fn account_by_handle(
    state: &AppState,
    handle: &str,
) -> AppResult<identity::public_accounts::PublicAccount> {
    identity::public_accounts::find_public_account_by_handle(&state.db, handle)
        .await?
        .ok_or(AppError::NotFound)
}

/// GET /api/v2/users/{handle} — public community profile.
#[tracing::instrument(skip(state))]
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> AppResult<Json<UserProfileDto>> {
    let account = account_by_handle(&state, &handle).await?;
    let stats = crate::repo::profiles::get_public_profile_stats(&state.db, account.id).await?;
    let badges = crate::badges::list_account_badges(&state.db, account.id)
        .await?
        .into_iter()
        .map(|badge| UserBadgeDto { slug: badge.slug, name: badge.name })
        .collect();

    Ok(Json(UserProfileDto {
        id: account.id.to_string(),
        handle: account.handle,
        avatar_url: account.avatar_url,
        role: account.role,
        trust_level: account.trust_level,
        badges,
        thread_count: stats.thread_count,
        comment_count: stats.comment_count,
        votes_received: stats.votes_received,
        created_at: account.created_at.timestamp(),
    }))
}

/// GET /api/v2/users/{handle}/threads — public user thread list.
#[tracing::instrument(skip(state))]
pub async fn list_user_threads(
    State(state): State<AppState>,
    Path(handle): Path<String>,
    Query(query): Query<PublicPostPageQuery>,
) -> AppResult<Json<shared::Page<UserThreadDto>>> {
    let account = account_by_handle(&state, &handle).await?;
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_threads(
        &state.db,
        account.id,
        parse_cursor(query.cursor.as_deref())?,
        query.limit.unwrap_or(20),
    )
    .await?;
    let items = rows
        .into_iter()
        .map(|row| UserThreadDto {
            id: row.id.to_string(),
            title: row.title,
            board_slug: row.board_slug,
            reply_count: row.reply_count,
            vote_count: row.vote_count,
            created_at: row.created_at.timestamp(),
        })
        .collect();
    Ok(Json(shared::Page::new(items, next_cursor.map(|cursor| cursor.to_string()))))
}

/// GET /api/v2/users/{handle}/comments — public user comment list.
#[tracing::instrument(skip(state))]
pub async fn list_user_comments(
    State(state): State<AppState>,
    Path(handle): Path<String>,
    Query(query): Query<PublicPostPageQuery>,
) -> AppResult<Json<shared::Page<UserCommentDto>>> {
    let account = account_by_handle(&state, &handle).await?;
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_comments(
        &state.db,
        account.id,
        parse_cursor(query.cursor.as_deref())?,
        query.limit.unwrap_or(20),
    )
    .await?;
    let items = rows
        .into_iter()
        .map(|row| UserCommentDto {
            id: row.id.to_string(),
            thread_id: row.thread_id.to_string(),
            thread_title: row.thread_title,
            body: row.body,
            created_at: row.created_at.timestamp(),
        })
        .collect();
    Ok(Json(shared::Page::new(items, next_cursor.map(|cursor| cursor.to_string()))))
}

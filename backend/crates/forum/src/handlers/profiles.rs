//! Public community-profile handlers.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
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

async fn visible_account_by_handle(
    state: &AppState,
    headers: &HeaderMap,
    handle: &str,
    includes_profile_content: bool,
) -> AppResult<identity::public_accounts::PublicAccount> {
    let viewer = super::relationships::optional_viewer(state, headers).await?;
    let viewer_id = viewer.as_ref().map(|account| account.id);
    let account = account_by_handle(state, handle).await?;
    let relationship = super::relationships::relationship_for(state, viewer_id, account.id).await?;
    let is_visible = if includes_profile_content {
        super::relationships::profile_content_is_visible(&account, viewer_id, relationship)
    } else {
        super::relationships::profile_is_visible(&account, viewer_id, relationship)
    };
    if !is_visible {
        return Err(AppError::NotFound);
    }
    Ok(account)
}

/// GET /api/v2/users/{handle} — public community profile.
#[tracing::instrument(skip(state))]
pub async fn get_user_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<Json<UserProfileDto>> {
    let account = visible_account_by_handle(&state, &headers, &handle, false).await?;
    let asset_ids: Vec<i64> =
        [account.avatar_asset_id, account.banner_asset_id].into_iter().flatten().collect();
    let (stats, social_counts, badge_rows, asset_urls, verifications) = tokio::try_join!(
        crate::repo::profiles::get_public_profile_stats(&state.db, account.id),
        crate::repo::relationships::get_social_counts(&state.db, account.id),
        crate::badges::list_account_badges(&state.db, account.id),
        media::resolve_clean_profile_images(&state.db, &asset_ids),
        platform::verifications::list_public_account_verifications(&state.db, account.id),
    )?;
    let badges = badge_rows
        .into_iter()
        .map(|badge| UserBadgeDto { slug: badge.slug, name: badge.name })
        .collect();
    Ok(Json(UserProfileDto {
        id: account.id.to_string(),
        handle: account.handle,
        display_name: account.display_name,
        bio: account.bio,
        website: account.website,
        avatar_url: account.avatar_asset_id.and_then(|id| asset_urls.get(&id).cloned()),
        banner_url: account.banner_asset_id.and_then(|id| asset_urls.get(&id).cloned()),
        role: account.role,
        trust_level: account.trust_level,
        badges,
        verifications,
        thread_count: stats.thread_count,
        comment_count: stats.comment_count,
        votes_received: stats.votes_received,
        follower_count: social_counts.follower_count,
        following_count: social_counts.following_count,
        created_at: account.created_at.timestamp(),
    }))
}

/// GET /api/v2/users/{handle}/threads — public user thread list.
#[tracing::instrument(skip(state))]
pub async fn list_user_threads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Query(query): Query<PublicPostPageQuery>,
) -> AppResult<Json<shared::Page<UserThreadDto>>> {
    let account = visible_account_by_handle(&state, &headers, &handle, true).await?;
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
    headers: HeaderMap,
    Path(handle): Path<String>,
    Query(query): Query<PublicPostPageQuery>,
) -> AppResult<Json<shared::Page<UserCommentDto>>> {
    let account = visible_account_by_handle(&state, &headers, &handle, true).await?;
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_comments(
        &state.db,
        account.id,
        parse_cursor(query.cursor.as_deref())?,
        query.limit.unwrap_or(20),
    )
    .await?;
    let items = rows
        .into_iter()
        .map(|row| {
            let source_format = crate::dto::ContentFormat::from_db(&row.content_format);
            UserCommentDto {
                id: row.id.to_string(),
                thread_id: row.thread_id.to_string(),
                thread_title: row.thread_title,
                body: crate::content_policy::plain_text_projection(&row.body, source_format, 200),
                content_format: crate::dto::ContentFormat::PlainV1,
                created_at: row.created_at.timestamp(),
            }
        })
        .collect();
    Ok(Json(shared::Page::new(items, next_cursor.map(|cursor| cursor.to_string()))))
}

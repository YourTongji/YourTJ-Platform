//! Public follow graph and private mute/block relationship handlers.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState, Page};

use crate::dto::{UserRelationshipDto, UserSummaryDto};
use crate::repo::relationships::{FollowListRow, RelationshipState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
enum RelationshipListKind {
    Followers,
    Following,
}

pub(super) async fn optional_viewer(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<Option<shared::AuthAccount>> {
    identity::auth_middleware::authenticate_optional(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

pub(super) async fn relationship_for(
    state: &AppState,
    viewer_id: Option<i64>,
    target_id: i64,
) -> AppResult<Option<RelationshipState>> {
    let Some(viewer_id) = viewer_id else {
        return Ok(None);
    };
    if viewer_id == target_id {
        return Ok(Some(RelationshipState::default()));
    }
    crate::repo::relationships::get_relationship(&state.db, viewer_id, target_id).await.map(Some)
}

pub(super) fn profile_is_visible(
    account: &identity::public_accounts::PublicAccount,
    viewer_id: Option<i64>,
    relationship: Option<RelationshipState>,
) -> bool {
    if viewer_id == Some(account.id) {
        return true;
    }
    if relationship.is_some_and(|state| state.blocked_me) {
        return false;
    }
    match account.profile_visibility.as_str() {
        "public" => true,
        "campus" => viewer_id.is_some(),
        "only_me" => false,
        _ => false,
    }
}

pub(super) fn profile_content_is_visible(
    account: &identity::public_accounts::PublicAccount,
    viewer_id: Option<i64>,
    relationship: Option<RelationshipState>,
) -> bool {
    profile_is_visible(account, viewer_id, relationship)
        && !relationship.is_some_and(|state| state.blocked_by_me || state.blocked_me)
}

pub(super) fn activity_is_visible(
    account: &identity::public_accounts::PublicAccount,
    viewer_id: Option<i64>,
    relationship: Option<RelationshipState>,
) -> bool {
    if !profile_content_is_visible(account, viewer_id, relationship) {
        return false;
    }
    if viewer_id == Some(account.id) {
        return true;
    }
    match account.activity_visibility.as_str() {
        "public" => true,
        "campus" => viewer_id.is_some(),
        "only_me" => false,
        _ => false,
    }
}

fn list_is_visible(
    policy: &str,
    account_id: i64,
    viewer_id: Option<i64>,
    relationship: Option<RelationshipState>,
) -> bool {
    if viewer_id == Some(account_id) {
        return true;
    }
    if relationship.is_some_and(|state| state.blocked_by_me || state.blocked_me) {
        return false;
    }
    match policy {
        "public" => true,
        "campus" => viewer_id.is_some(),
        "followers" => relationship.is_some_and(|state| state.following),
        "only_me" => false,
        _ => false,
    }
}

async fn authenticate_and_find_target(
    state: &AppState,
    headers: &HeaderMap,
    handle: &str,
) -> AppResult<(shared::AuthAccount, identity::public_accounts::PublicAccount)> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let target = identity::public_accounts::find_public_account_by_handle(&state.db, handle)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok((auth, target))
}

async fn authenticate_and_find_cleanup_target(
    state: &AppState,
    headers: &HeaderMap,
    handle: &str,
) -> AppResult<(shared::AuthAccount, i64)> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let target_id = identity::public_accounts::find_account_id_by_handle_for_relationship_cleanup(
        &state.db, handle,
    )
    .await?
    .ok_or(AppError::NotFound)?;
    Ok((auth, target_id))
}

async fn authenticate_and_find_safety_target(
    state: &AppState,
    headers: &HeaderMap,
    handle: &str,
) -> AppResult<(shared::AuthAccount, i64)> {
    let auth = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    let target_id =
        identity::public_accounts::find_account_id_by_handle_for_safety_action(&state.db, handle)
            .await?
            .ok_or(AppError::NotFound)?;
    Ok((auth, target_id))
}

/// GET /api/v2/users/{handle}/relationship
pub async fn get_relationship_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<Json<UserRelationshipDto>> {
    let (auth, target) = authenticate_and_find_target(&state, &headers, &handle).await?;
    let is_self = auth.id == target.id;
    let relationship = if is_self {
        RelationshipState::default()
    } else {
        crate::repo::relationships::get_relationship(&state.db, auth.id, target.id).await?
    };
    if !is_self && target.profile_visibility == "only_me" {
        return Err(AppError::NotFound);
    }
    let is_blocked = relationship.blocked_by_me || relationship.blocked_me;
    let can_start_conversation = !is_self
        && !is_blocked
        && match target.dm_policy.as_str() {
            "everyone" => true,
            "following" => relationship.followed_by,
            "nobody" => false,
            _ => false,
        };
    let can_mention = !is_self
        && !is_blocked
        && match target.mention_policy.as_str() {
            "everyone" => true,
            "following" => relationship.followed_by,
            "nobody" => false,
            _ => false,
        };
    Ok(Json(UserRelationshipDto {
        is_self,
        following: relationship.following,
        followed_by: relationship.followed_by,
        muted: relationship.muted,
        blocked_by_me: relationship.blocked_by_me,
        blocked_me: relationship.blocked_me,
        can_follow: !is_self && !is_blocked,
        can_start_conversation,
        can_mention,
    }))
}

/// PUT /api/v2/users/{handle}/follow
pub async fn follow_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, target) = authenticate_and_find_target(&state, &headers, &handle).await?;
    let relationship =
        crate::repo::relationships::get_relationship(&state.db, auth.id, target.id).await?;
    if !profile_is_visible(&target, Some(auth.id), Some(relationship)) {
        return Err(AppError::NotFound);
    }
    crate::repo::relationships::follow(&state.db, auth.id, target.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/users/{handle}/follow
pub async fn unfollow_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, target_id) = authenticate_and_find_cleanup_target(&state, &headers, &handle).await?;
    crate::repo::relationships::unfollow(&state.db, auth.id, target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/me/followers/{handle}
pub async fn remove_follower_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, follower_id) =
        authenticate_and_find_cleanup_target(&state, &headers, &handle).await?;
    crate::repo::relationships::remove_follower(&state.db, auth.id, follower_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/v2/users/{handle}/mute
pub async fn mute_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, target_id) = authenticate_and_find_safety_target(&state, &headers, &handle).await?;
    crate::repo::relationships::mute(&state.db, auth.id, target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/users/{handle}/mute
pub async fn unmute_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, target_id) = authenticate_and_find_cleanup_target(&state, &headers, &handle).await?;
    crate::repo::relationships::unmute(&state.db, auth.id, target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/v2/users/{handle}/block
pub async fn block_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, target_id) = authenticate_and_find_safety_target(&state, &headers, &handle).await?;
    crate::repo::relationships::block(&state.db, auth.id, target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v2/users/{handle}/block
pub async fn unblock_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<StatusCode> {
    let (auth, target_id) = authenticate_and_find_cleanup_target(&state, &headers, &handle).await?;
    crate::repo::relationships::unblock(&state.db, auth.id, target_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_relationships(
    state: &AppState,
    headers: &HeaderMap,
    handle: &str,
    query: RelationshipListQuery,
    kind: RelationshipListKind,
) -> AppResult<Json<Page<UserSummaryDto>>> {
    let viewer = optional_viewer(state, headers).await?;
    let viewer_id = viewer.as_ref().map(|account| account.id);
    let account = identity::public_accounts::find_public_account_by_handle(&state.db, handle)
        .await?
        .ok_or(AppError::NotFound)?;
    let relationship = relationship_for(state, viewer_id, account.id).await?;
    if !profile_is_visible(&account, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    let list_policy = match kind {
        RelationshipListKind::Followers => &account.followers_visibility,
        RelationshipListKind::Following => &account.following_visibility,
    };
    if !list_is_visible(list_policy, account.id, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| AppError::BadRequest("invalid cursor".into()))?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let (rows, next_cursor) = match kind {
        RelationshipListKind::Followers => {
            crate::repo::relationships::list_follower_ids(
                &state.db, account.id, viewer_id, cursor, limit,
            )
            .await?
        }
        RelationshipListKind::Following => {
            crate::repo::relationships::list_following_ids(
                &state.db, account.id, viewer_id, cursor, limit,
            )
            .await?
        }
    };
    let items = relationship_rows_to_dtos(state, rows, account.id, viewer_id).await?;
    Ok(Json(Page::new(items, next_cursor.map(|value| value.to_string()))))
}

async fn relationship_rows_to_dtos(
    state: &AppState,
    rows: Vec<FollowListRow>,
    list_owner_id: i64,
    viewer_id: Option<i64>,
) -> AppResult<Vec<UserSummaryDto>> {
    let account_ids: Vec<i64> = rows.iter().map(|row| row.account_id).collect();
    let accounts =
        identity::public_accounts::find_public_accounts_by_ids(&state.db, &account_ids).await?;
    let accounts: HashMap<i64, _> = accounts
        .into_iter()
        .filter(|account| {
            viewer_id == Some(list_owner_id)
                || viewer_id == Some(account.id)
                || account.discoverable
        })
        .map(|account| (account.id, account))
        .collect();
    let asset_ids: Vec<i64> =
        accounts.values().filter_map(|account| account.avatar_asset_id).collect();
    let urls = media::resolve_clean_profile_images(&state.db, &asset_ids).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(account) = accounts.get(&row.account_id) else {
            continue;
        };
        items.push(UserSummaryDto {
            id: account.id.to_string(),
            handle: account.handle.clone(),
            display_name: account.display_name.clone(),
            avatar_url: account.avatar_asset_id.and_then(|id| urls.get(&id).cloned()),
            role: account.role.clone(),
            followed_at: row.followed_at.timestamp(),
        });
    }
    Ok(items)
}

/// GET /api/v2/users/{handle}/followers
pub async fn list_followers_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Query(query): Query<RelationshipListQuery>,
) -> AppResult<Json<Page<UserSummaryDto>>> {
    list_relationships(&state, &headers, &handle, query, RelationshipListKind::Followers).await
}

/// GET /api/v2/users/{handle}/following
pub async fn list_following_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Query(query): Query<RelationshipListQuery>,
) -> AppResult<Json<Page<UserSummaryDto>>> {
    list_relationships(&state, &headers, &handle, query, RelationshipListKind::Following).await
}

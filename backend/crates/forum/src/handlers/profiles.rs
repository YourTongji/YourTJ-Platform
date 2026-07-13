//! Public community-profile handlers.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::{AppError, AppResult, AppState};

use crate::dto::{ProfileContentDto, UserBadgeDto, UserCommentDto, UserProfileDto, UserThreadDto};

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
) -> AppResult<(
    identity::public_accounts::PublicAccount,
    Option<i64>,
    Option<crate::repo::relationships::RelationshipState>,
)> {
    let viewer = super::relationships::optional_viewer(state, headers).await?;
    let viewer_id = viewer.as_ref().map(|account| account.id);
    let account = account_by_handle(state, handle).await?;
    let relationship = super::relationships::relationship_for(state, viewer_id, account.id).await?;
    if !super::relationships::profile_is_visible(&account, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    Ok((account, viewer_id, relationship))
}

pub(crate) async fn hydrate_profile_content(
    state: &AppState,
    viewer_id: Option<i64>,
    rows: Vec<crate::repo::profiles::ProfileContentRow>,
) -> AppResult<Vec<ProfileContentDto>> {
    let author_ids = rows.iter().map(|row| row.author_id).collect::<Vec<_>>();
    let thread_ids = rows
        .iter()
        .filter_map(|row| (row.target_type == "thread").then_some(row.id))
        .collect::<Vec<_>>();
    let comment_ids = rows
        .iter()
        .filter_map(|row| (row.target_type == "comment").then_some(row.id))
        .collect::<Vec<_>>();
    let (accounts, thread_states, comment_states, mut thread_attachments, mut comment_attachments) =
        tokio::try_join!(
            identity::public_accounts::find_public_accounts_by_ids(&state.db, &author_ids),
            async {
                match viewer_id {
                    Some(account_id) => {
                        crate::repo::get_post_viewer_states(
                            &state.db,
                            account_id,
                            "thread",
                            &thread_ids,
                        )
                        .await
                    }
                    None => Ok(HashMap::new()),
                }
            },
            async {
                match viewer_id {
                    Some(account_id) => {
                        crate::repo::get_post_viewer_states(
                            &state.db,
                            account_id,
                            "comment",
                            &comment_ids,
                        )
                        .await
                    }
                    None => Ok(HashMap::new()),
                }
            },
            media::attachments::resolve_forum_attachments_batch(
                &state.db,
                media::attachments::ForumTargetType::Thread,
                &thread_ids,
            ),
            media::attachments::resolve_forum_attachments_batch(
                &state.db,
                media::attachments::ForumTargetType::Comment,
                &comment_ids,
            ),
        )?;
    let accounts =
        accounts.into_iter().map(|account| (account.id, account)).collect::<HashMap<_, _>>();

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(author) = accounts.get(&row.author_id) else {
            continue;
        };
        let source_format = crate::dto::ContentFormat::from_db(&row.content_format);
        let target_type = if row.target_type == "thread" {
            media::attachments::ForumTargetType::Thread
        } else {
            media::attachments::ForumTargetType::Comment
        };
        let projected = if row.target_type == "thread" {
            thread_attachments.remove(&row.id).unwrap_or_default()
        } else {
            comment_attachments.remove(&row.id).unwrap_or_default()
        };
        let references = crate::content_policy::image_references_for_stored_content(
            row.body.as_deref(),
            source_format,
            target_type,
        )
        .unwrap_or_else(|error| {
            tracing::warn!(
                ?error,
                target_type = row.target_type,
                target_id = row.id,
                "stored profile content image references are invalid"
            );
            Vec::new()
        });
        let attachments =
            if crate::content_policy::attachment_projection_matches(&references, &projected) {
                projected
            } else {
                tracing::warn!(
                    target_type = row.target_type,
                    target_id = row.id,
                    "profile content attachment projection mismatch"
                );
                Vec::new()
            };
        let viewer_state = if row.target_type == "thread" {
            thread_states.get(&row.id)
        } else {
            comment_states.get(&row.id)
        };
        let body = row
            .body
            .as_deref()
            .map(|body| crate::content_policy::plain_text_projection(body, source_format, 280))
            .filter(|body| !body.is_empty());
        items.push(ProfileContentDto {
            target_type: row.target_type,
            id: row.id.to_string(),
            thread_id: row.thread_id.to_string(),
            title: row.title,
            body,
            content_format: crate::dto::ContentFormat::PlainV1,
            board_slug: row.board_slug,
            author_handle: author.handle.clone(),
            author_display_name: author.display_name.clone(),
            reply_count: row.reply_count,
            vote_count: row.vote_count,
            viewer_vote: viewer_state.and_then(|state| state.vote.clone()),
            is_bookmarked: viewer_state.is_some_and(|state| state.is_bookmarked),
            attachments,
            created_at: row.created_at.timestamp(),
            activity_at: row.activity_at.timestamp(),
        });
    }
    Ok(items)
}

/// GET /api/v2/users/{handle} — public community profile.
#[tracing::instrument(skip(state))]
pub async fn get_user_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> AppResult<Json<UserProfileDto>> {
    let (account, viewer_id, relationship) =
        visible_account_by_handle(&state, &headers, &handle).await?;
    let can_view_activity =
        super::relationships::activity_is_visible(&account, viewer_id, relationship);
    let asset_ids: Vec<i64> =
        [account.avatar_asset_id, account.banner_asset_id].into_iter().flatten().collect();
    let (stats, social_counts, badge_rows, asset_urls, verifications) = tokio::try_join!(
        crate::repo::profiles::get_public_profile_stats(&state.db, account.id),
        crate::repo::relationships::get_social_counts(&state.db, account.id),
        platform::achievements::list_public_account_achievements(&state.db, account.id),
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
        school: account.school,
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
        can_view_activity,
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
    let (account, viewer_id, relationship) =
        visible_account_by_handle(&state, &headers, &handle).await?;
    if !super::relationships::activity_is_visible(&account, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_threads(
        &state.db,
        account.id,
        parse_cursor(query.cursor.as_deref())?,
        query.limit.unwrap_or(20),
    )
    .await?;
    let thread_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let (viewer_states, mut attachments) = tokio::try_join!(
        async {
            match viewer_id {
                Some(account_id) => {
                    crate::repo::get_post_viewer_states(
                        &state.db,
                        account_id,
                        "thread",
                        &thread_ids,
                    )
                    .await
                }
                None => Ok(HashMap::new()),
            }
        },
        media::attachments::resolve_forum_attachments_batch(
            &state.db,
            media::attachments::ForumTargetType::Thread,
            &thread_ids,
        ),
    )?;
    let items = rows
        .into_iter()
        .map(|row| {
            let source_format = crate::dto::ContentFormat::from_db(&row.content_format);
            let projected = attachments.remove(&row.id).unwrap_or_default();
            let references = crate::content_policy::image_references_for_stored_content(
                row.body.as_deref(),
                source_format,
                media::attachments::ForumTargetType::Thread,
            )
            .unwrap_or_default();
            let attachment =
                if crate::content_policy::attachment_projection_matches(&references, &projected) {
                    projected.into_iter().take(1).collect()
                } else {
                    Vec::new()
                };
            let viewer_state = viewer_states.get(&row.id);
            let body_excerpt = row
                .body
                .as_deref()
                .map(|body| crate::content_policy::plain_text_projection(body, source_format, 280))
                .filter(|body| !body.is_empty());
            UserThreadDto {
                id: row.id.to_string(),
                title: row.title,
                body_excerpt,
                content_format: crate::dto::ContentFormat::PlainV1,
                board_slug: row.board_slug,
                reply_count: row.reply_count,
                vote_count: row.vote_count,
                viewer_vote: viewer_state.and_then(|state| state.vote.clone()),
                is_bookmarked: viewer_state.is_some_and(|state| state.is_bookmarked),
                attachments: attachment,
                created_at: row.created_at.timestamp(),
            }
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
    let (account, viewer_id, relationship) =
        visible_account_by_handle(&state, &headers, &handle).await?;
    if !super::relationships::activity_is_visible(&account, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_comments(
        &state.db,
        account.id,
        parse_cursor(query.cursor.as_deref())?,
        query.limit.unwrap_or(20),
    )
    .await?;
    let comment_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let (viewer_states, mut attachments) = tokio::try_join!(
        async {
            match viewer_id {
                Some(account_id) => {
                    crate::repo::get_post_viewer_states(
                        &state.db,
                        account_id,
                        "comment",
                        &comment_ids,
                    )
                    .await
                }
                None => Ok(HashMap::new()),
            }
        },
        media::attachments::resolve_forum_attachments_batch(
            &state.db,
            media::attachments::ForumTargetType::Comment,
            &comment_ids,
        ),
    )?;
    let items = rows
        .into_iter()
        .map(|row| {
            let source_format = crate::dto::ContentFormat::from_db(&row.content_format);
            let projected = attachments.remove(&row.id).unwrap_or_default();
            let references = crate::content_policy::image_references_for_stored_content(
                Some(&row.body),
                source_format,
                media::attachments::ForumTargetType::Comment,
            )
            .unwrap_or_default();
            let attachment =
                if crate::content_policy::attachment_projection_matches(&references, &projected) {
                    projected
                } else {
                    Vec::new()
                };
            let viewer_state = viewer_states.get(&row.id);
            UserCommentDto {
                id: row.id.to_string(),
                thread_id: row.thread_id.to_string(),
                thread_title: row.thread_title,
                body: crate::content_policy::plain_text_projection(&row.body, source_format, 200),
                content_format: crate::dto::ContentFormat::PlainV1,
                reply_count: row.reply_count,
                vote_count: row.vote_count,
                viewer_vote: viewer_state.and_then(|state| state.vote.clone()),
                is_bookmarked: viewer_state.is_some_and(|state| state.is_bookmarked),
                attachments: attachment,
                created_at: row.created_at.timestamp(),
            }
        })
        .collect();
    Ok(Json(shared::Page::new(items, next_cursor.map(|cursor| cursor.to_string()))))
}

/// GET /api/v2/users/{handle}/media — visible authored content with clean images.
#[tracing::instrument(skip(state))]
pub async fn list_user_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Query(query): Query<PublicPostPageQuery>,
) -> AppResult<Json<shared::Page<ProfileContentDto>>> {
    let (account, viewer_id, relationship) =
        visible_account_by_handle(&state, &headers, &handle).await?;
    if !super::relationships::activity_is_visible(&account, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_media_candidates(
        &state.db,
        account.id,
        query.cursor.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await?;
    let items = hydrate_profile_content(&state, viewer_id, rows)
        .await?
        .into_iter()
        .filter(|item| !item.attachments.is_empty())
        .collect();
    Ok(Json(shared::Page::new(items, next_cursor)))
}

/// GET /api/v2/users/{handle}/likes — positive votes over currently visible Forum content.
#[tracing::instrument(skip(state))]
pub async fn list_user_likes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Query(query): Query<PublicPostPageQuery>,
) -> AppResult<Json<shared::Page<ProfileContentDto>>> {
    let (account, viewer_id, relationship) =
        visible_account_by_handle(&state, &headers, &handle).await?;
    if !super::relationships::activity_is_visible(&account, viewer_id, relationship) {
        return Err(AppError::NotFound);
    }
    let (rows, next_cursor) = crate::repo::profiles::list_public_user_likes(
        &state.db,
        account.id,
        viewer_id,
        query.cursor.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await?;
    let items = hydrate_profile_content(&state, viewer_id, rows).await?;
    Ok(Json(shared::Page::new(items, next_cursor)))
}

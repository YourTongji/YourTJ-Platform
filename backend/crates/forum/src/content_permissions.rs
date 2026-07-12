//! Server-authoritative viewer actions for canonical forum content.

use std::collections::HashMap;

use shared::auth::{AuthAccount, Capability};
use shared::AppResult;
use sqlx::PgPool;

use crate::dto::{CommentDto, ThreadDetailDto, ThreadDto};
use crate::models::{CommentRowJoined, ThreadRowJoined};

fn role_rank(role: &str) -> Option<u8> {
    match role {
        "user" => Some(0),
        "mod" => Some(1),
        "admin" => Some(2),
        _ => None,
    }
}

async fn moderation_roles(
    pool: &PgPool,
    actor: Option<&AuthAccount>,
    author_ids: &[i64],
) -> AppResult<HashMap<i64, String>> {
    if actor.is_some_and(|account| account.has_capability(Capability::ModerateContent)) {
        identity::public_accounts::find_account_roles_by_ids(pool, author_ids).await
    } else {
        Ok(HashMap::new())
    }
}

fn can_moderate(actor: &AuthAccount, author_id: i64, author_role: Option<&str>) -> bool {
    if actor.id == author_id || !actor.has_capability(Capability::ModerateContent) {
        return false;
    }
    role_rank(&actor.role)
        .zip(author_role.and_then(role_rank))
        .is_some_and(|(actor_rank, author_rank)| actor_rank > author_rank)
}

/// Authorize revision disclosure without treating a staff role as a global history reader.
pub(crate) async fn can_read_revisions(
    pool: &PgPool,
    actor: &AuthAccount,
    author_id: i64,
) -> AppResult<bool> {
    if actor.id == author_id {
        return Ok(true);
    }
    if !actor.has_capability(Capability::ModerateContent) {
        return Ok(false);
    }
    let roles = moderation_roles(pool, Some(actor), &[author_id]).await?;
    Ok(can_moderate(actor, author_id, roles.get(&author_id).map(String::as_str)))
}

pub(crate) async fn hydrate_thread_summaries(
    pool: &PgPool,
    actor: Option<&AuthAccount>,
    rows: &[ThreadRowJoined],
    dtos: &mut [ThreadDto],
) -> AppResult<()> {
    let author_ids = rows.iter().map(|row| row.author_id).collect::<Vec<_>>();
    let roles = moderation_roles(pool, actor, &author_ids).await?;
    for (row, dto) in rows.iter().zip(dtos) {
        let is_author = actor.is_some_and(|account| account.id == row.author_id);
        dto.can_edit = is_author;
        dto.can_delete = is_author;
        dto.can_moderate = actor.is_some_and(|account| {
            can_moderate(account, row.author_id, roles.get(&row.author_id).map(String::as_str))
        });
    }
    Ok(())
}

pub(crate) async fn hydrate_thread_detail(
    pool: &PgPool,
    actor: Option<&AuthAccount>,
    dto: &mut ThreadDetailDto,
) -> AppResult<()> {
    let author_id = dto.author_id.parse::<i64>().ok();
    let author_ids = author_id.into_iter().collect::<Vec<_>>();
    let roles = moderation_roles(pool, actor, &author_ids).await?;
    let is_author =
        actor.zip(author_id).is_some_and(|(account, author_id)| account.id == author_id);
    dto.can_edit = is_author
        && dto.deleted_at.is_none()
        && dto.hidden_at.is_none()
        && dto.archived_at.is_none();
    dto.can_delete = is_author && dto.deleted_at.is_none() && dto.hidden_at.is_none();
    dto.can_moderate = actor.zip(author_id).is_some_and(|(account, author_id)| {
        can_moderate(account, author_id, roles.get(&author_id).map(String::as_str))
    });
    Ok(())
}

pub(crate) async fn hydrate_comments(
    pool: &PgPool,
    actor: Option<&AuthAccount>,
    rows: &[CommentRowJoined],
    parent_allows_edit: bool,
    dtos: &mut [CommentDto],
) -> AppResult<()> {
    let author_ids = rows.iter().map(|row| row.author_id).collect::<Vec<_>>();
    let roles = moderation_roles(pool, actor, &author_ids).await?;
    for (row, dto) in rows.iter().zip(dtos) {
        let is_author = actor.is_some_and(|account| account.id == row.author_id);
        let is_available = row.deleted_at.is_none() && row.hidden_at.is_none();
        dto.can_edit = is_author && is_available && parent_allows_edit;
        dto.can_delete = is_author && is_available;
        dto.can_moderate = actor.is_some_and(|account| {
            can_moderate(account, row.author_id, roles.get(&row.author_id).map(String::as_str))
        });
    }
    Ok(())
}

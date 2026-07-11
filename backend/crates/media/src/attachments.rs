//! Media-owned Forum attachment binding, disclosure, and draft validation.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

/// A Forum content target that can bind clean image assets.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ForumTargetType {
    Thread,
    Comment,
}

impl ForumTargetType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Thread => "forum_thread",
            Self::Comment => "forum_comment",
        }
    }

    pub const fn max_images(self) -> usize {
        match self {
            Self::Thread => 8,
            Self::Comment => 4,
        }
    }
}

/// One ordered image reference parsed from canonical Markdown.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ForumAssetReference {
    pub asset_id: i64,
    pub position: i16,
    pub alt_text: String,
}

impl ForumAssetReference {
    pub fn canonical_reference(&self) -> String {
        format!("yourtj-asset:{}", self.asset_id)
    }
}

/// Minimal clean attachment projection disclosed with authorized Forum content.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumAttachment {
    pub asset_id: String,
    pub reference: String,
    pub position: i16,
    pub alt: String,
    pub url: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, FromRow)]
struct BindableUploadRow {
    account_id: i64,
    kind: String,
    status: String,
    usage: Option<String>,
}

#[derive(Debug, FromRow, Eq, PartialEq)]
struct ActiveUsageRow {
    asset_id: i64,
    position: i16,
    alt_text: String,
}

#[derive(Debug, FromRow)]
struct AttachmentProjectionRow {
    target_id: i64,
    asset_id: i64,
    position: i16,
    alt_text: String,
    url: String,
    image_width: Option<i32>,
    image_height: Option<i32>,
}

fn validate_reference_shape(
    target_type: ForumTargetType,
    references: &[ForumAssetReference],
) -> AppResult<()> {
    if references.len() > target_type.max_images() {
        return Err(AppError::BadRequest(format!(
            "{} content supports at most {} images",
            target_type.as_str(),
            target_type.max_images()
        )));
    }
    let mut asset_ids = HashSet::with_capacity(references.len());
    for (index, reference) in references.iter().enumerate() {
        if reference.asset_id <= 0
            || reference.position != index as i16
            || !(1..=300).contains(&reference.alt_text.chars().count())
            || reference.alt_text.trim() != reference.alt_text
            || !asset_ids.insert(reference.asset_id)
        {
            return Err(AppError::BadRequest("invalid or duplicate attachment reference".into()));
        }
    }
    Ok(())
}

async fn lock_bindable_uploads(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    target_type: ForumTargetType,
    asset_ids: &[i64],
    clean_only: bool,
) -> AppResult<()> {
    if asset_ids.is_empty() {
        return Ok(());
    }
    let rows = sqlx::query_as::<_, BindableUploadRow>(
        "SELECT account_id, kind, status, usage \
         FROM media.uploads WHERE id = ANY($1) ORDER BY id FOR SHARE",
    )
    .bind(asset_ids)
    .fetch_all(&mut **transaction)
    .await?;
    if rows.len() != asset_ids.len() {
        return Err(AppError::NotFound);
    }
    if rows.iter().any(|row| {
        row.account_id != account_id
            || row.kind != "image"
            || row.usage.as_deref() != Some(target_type.as_str())
            || (clean_only && row.status != "clean")
    }) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Validate that draft asset ids are image uploads owned by the draft owner and intended for the
/// exact Forum surface. Pending and blocked rows may remain in private drafts, but never authorize
/// a public binding.
pub async fn validate_owned_forum_draft_assets(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    target_type: ForumTargetType,
    asset_ids: &[i64],
) -> AppResult<()> {
    if asset_ids.len() > target_type.max_images() {
        return Err(AppError::BadRequest("draft contains too many image uploads".into()));
    }
    let unique_ids = asset_ids.iter().copied().collect::<HashSet<_>>();
    if unique_ids.len() != asset_ids.len() || asset_ids.iter().any(|asset_id| *asset_id <= 0) {
        return Err(AppError::BadRequest("draft attachment ids must be unique".into()));
    }
    lock_bindable_uploads(transaction, account_id, target_type, asset_ids, false).await
}

/// Replace the active binding set after the owning Forum row is locked and mutated. Validation,
/// detachment, and insertion run in the caller's transaction, so stale edits leave no media state.
pub async fn sync_forum_asset_bindings(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    target_type: ForumTargetType,
    target_id: i64,
    content_version: i64,
    references: &[ForumAssetReference],
) -> AppResult<()> {
    validate_reference_shape(target_type, references)?;
    let asset_ids = references.iter().map(|reference| reference.asset_id).collect::<Vec<_>>();
    lock_bindable_uploads(transaction, account_id, target_type, &asset_ids, true).await?;

    let active = sqlx::query_as::<_, ActiveUsageRow>(
        "SELECT asset_id, position, alt_text FROM media.asset_usages \
         WHERE target_type = $1 AND target_id = $2 AND detached_at IS NULL \
         ORDER BY position FOR UPDATE",
    )
    .bind(target_type.as_str())
    .bind(target_id)
    .fetch_all(&mut **transaction)
    .await?;
    let desired = references
        .iter()
        .map(|reference| ActiveUsageRow {
            asset_id: reference.asset_id,
            position: reference.position,
            alt_text: reference.alt_text.clone(),
        })
        .collect::<Vec<_>>();
    if active == desired {
        return Ok(());
    }

    sqlx::query(
        "UPDATE media.asset_usages \
         SET detached_at = now(), detached_reason = 'content_edit', \
             detached_content_version = $3, gc_eligible_at = now() + interval '30 days' \
         WHERE target_type = $1 AND target_id = $2 AND detached_at IS NULL",
    )
    .bind(target_type.as_str())
    .bind(target_id)
    .bind(content_version)
    .execute(&mut **transaction)
    .await?;

    for reference in references {
        sqlx::query(
            "INSERT INTO media.asset_usages \
             (asset_id, owner_account_id, target_type, target_id, position, alt_text, \
              bound_content_version) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(reference.asset_id)
        .bind(account_id)
        .bind(target_type.as_str())
        .bind(target_id)
        .bind(reference.position)
        .bind(&reference.alt_text)
        .bind(content_version)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

/// Detach every active usage for a soft-deleted Forum target. Objects are only GC candidates after
/// a grace period; this operation never deletes provider objects.
pub async fn detach_forum_asset_bindings(
    transaction: &mut Transaction<'_, Postgres>,
    target_type: ForumTargetType,
    target_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE media.asset_usages \
         SET detached_at = now(), detached_reason = 'target_deleted', \
             detached_content_version = NULL, gc_eligible_at = now() + interval '30 days' \
         WHERE target_type = $1 AND target_id = $2 AND detached_at IS NULL",
    )
    .bind(target_type.as_str())
    .bind(target_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn attachment_from_row(row: AttachmentProjectionRow) -> ForumAttachment {
    ForumAttachment {
        asset_id: row.asset_id.to_string(),
        reference: format!("yourtj-asset:{}", row.asset_id),
        position: row.position,
        alt: row.alt_text,
        url: row.url,
        width: row.image_width,
        height: row.image_height,
    }
}

/// Resolve active clean bindings for a bounded set of authorized Forum targets. Storage keys,
/// hashes, owner ids, and pending/blocked rows never cross this boundary.
pub async fn resolve_forum_attachments_batch(
    pool: &PgPool,
    target_type: ForumTargetType,
    target_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<ForumAttachment>>> {
    if target_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, AttachmentProjectionRow>(
        "SELECT usage.target_id, usage.asset_id, usage.position, usage.alt_text, upload.url, \
                upload.image_width, upload.image_height \
         FROM media.asset_usages usage \
         JOIN media.uploads upload ON upload.id = usage.asset_id \
         WHERE usage.target_type = $1 AND usage.target_id = ANY($2) \
           AND usage.detached_at IS NULL AND upload.kind = 'image' AND upload.status = 'clean' \
         ORDER BY usage.target_id, usage.position",
    )
    .bind(target_type.as_str())
    .bind(target_ids)
    .fetch_all(pool)
    .await?;
    let mut attachments = HashMap::<i64, Vec<ForumAttachment>>::new();
    for row in rows {
        attachments.entry(row.target_id).or_default().push(attachment_from_row(row));
    }
    Ok(attachments)
}

/// Resolve the clean binding snapshot applicable to one historical content version.
pub async fn resolve_forum_attachments_at_version(
    pool: &PgPool,
    target_type: ForumTargetType,
    target_id: i64,
    content_version: i64,
) -> AppResult<Vec<ForumAttachment>> {
    let rows = sqlx::query_as::<_, AttachmentProjectionRow>(
        "SELECT usage.target_id, usage.asset_id, usage.position, usage.alt_text, upload.url, \
                upload.image_width, upload.image_height \
         FROM media.asset_usages usage \
         JOIN media.uploads upload ON upload.id = usage.asset_id \
         WHERE usage.target_type = $1 AND usage.target_id = $2 \
           AND usage.bound_content_version <= $3 \
           AND (usage.detached_at IS NULL OR (usage.detached_reason = 'content_edit' \
                AND usage.detached_content_version > $3)) \
           AND upload.kind = 'image' AND upload.status = 'clean' \
         ORDER BY usage.position",
    )
    .bind(target_type.as_str())
    .bind(target_id)
    .bind(content_version)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(attachment_from_row).collect())
}

#[cfg(test)]
mod tests {
    use super::{validate_reference_shape, ForumAssetReference, ForumTargetType};

    #[test]
    fn attachment_shape_rejects_duplicates_and_noncanonical_positions() {
        let duplicate = vec![
            ForumAssetReference { asset_id: 1, position: 0, alt_text: "一".into() },
            ForumAssetReference { asset_id: 1, position: 1, alt_text: "二".into() },
        ];
        assert!(validate_reference_shape(ForumTargetType::Thread, &duplicate).is_err());

        let wrong_position =
            vec![ForumAssetReference { asset_id: 1, position: 1, alt_text: "一".into() }];
        assert!(validate_reference_shape(ForumTargetType::Comment, &wrong_position).is_err());
    }
}

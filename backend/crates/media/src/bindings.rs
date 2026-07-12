//! Media-owned live bindings for profile and platform image references.

use shared::{AppError, AppResult};
use sqlx::PgConnection;

/// A non-versioned business target that can hold one live image binding.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AssetBindingType {
    ProfileAvatar,
    ProfileBanner,
    PlatformPromotion,
}

impl AssetBindingType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProfileAvatar => "profile_avatar",
            Self::ProfileBanner => "profile_banner",
            Self::PlatformPromotion => "platform_promotion",
        }
    }

    const fn required_upload_usage(self) -> Option<&'static str> {
        match self {
            Self::ProfileAvatar => Some("profile_avatar"),
            Self::ProfileBanner => Some("profile_banner"),
            Self::PlatformPromotion => None,
        }
    }
}

async fn lock_target(
    connection: &mut PgConnection,
    target_type: AssetBindingType,
    target_id: i64,
) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("media-binding:{}:{target_id}", target_type.as_str()))
        .execute(connection)
        .await?;
    Ok(())
}

async fn validate_asset(
    connection: &mut PgConnection,
    owner_account_id: i64,
    target_type: AssetBindingType,
    asset_id: i64,
) -> AppResult<()> {
    let valid: Option<bool> = sqlx::query_scalar(
        "SELECT account_id = $2 AND kind = 'image' AND status = 'clean' \
                AND ($3::text IS NULL OR usage = $3) \
         FROM media.uploads WHERE id = $1 FOR SHARE",
    )
    .bind(asset_id)
    .bind(owner_account_id)
    .bind(target_type.required_upload_usage())
    .fetch_optional(connection)
    .await?;
    if valid != Some(true) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Replace or clear one live binding in the caller's business transaction.
pub async fn sync_asset_binding(
    connection: &mut PgConnection,
    owner_account_id: i64,
    target_type: AssetBindingType,
    target_id: i64,
    asset_id: Option<i64>,
    detached_reason: &str,
) -> AppResult<()> {
    if target_id <= 0 || !matches!(detached_reason, "replaced" | "cleared" | "archived") {
        return Err(AppError::BadRequest("invalid media binding target".into()));
    }
    lock_target(connection, target_type, target_id).await?;
    if let Some(asset_id) = asset_id {
        validate_asset(connection, owner_account_id, target_type, asset_id).await?;
    }
    let current: Option<i64> = sqlx::query_scalar(
        "SELECT asset_id FROM media.asset_bindings \
         WHERE target_type = $1 AND target_id = $2 AND detached_at IS NULL FOR UPDATE",
    )
    .bind(target_type.as_str())
    .bind(target_id)
    .fetch_optional(&mut *connection)
    .await?;
    if current == asset_id {
        return Ok(());
    }
    sqlx::query(
        "UPDATE media.asset_bindings \
         SET detached_at = now(), detached_reason = $3, \
             gc_eligible_at = now() + interval '30 days' \
         WHERE target_type = $1 AND target_id = $2 AND detached_at IS NULL",
    )
    .bind(target_type.as_str())
    .bind(target_id)
    .bind(detached_reason)
    .execute(&mut *connection)
    .await?;
    if let Some(asset_id) = asset_id {
        sqlx::query(
            "INSERT INTO media.asset_bindings \
             (asset_id, owner_account_id, target_type, target_id) VALUES ($1, $2, $3, $4)",
        )
        .bind(asset_id)
        .bind(owner_account_id)
        .bind(target_type.as_str())
        .bind(target_id)
        .execute(connection)
        .await?;
    }
    Ok(())
}

/// Detach profile bindings after an account purge becomes irreversible.
pub async fn detach_account_profile_bindings(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<()> {
    for target_type in [AssetBindingType::ProfileAvatar, AssetBindingType::ProfileBanner] {
        lock_target(connection, target_type, account_id).await?;
        sqlx::query(
            "UPDATE media.asset_bindings \
             SET detached_at = now(), detached_reason = 'account_purge', gc_eligible_at = now() \
             WHERE target_type = $1 AND target_id = $2 AND detached_at IS NULL",
        )
        .bind(target_type.as_str())
        .bind(account_id)
        .execute(&mut *connection)
        .await?;
        sqlx::query(
            "UPDATE media.asset_bindings SET gc_eligible_at = now() \
             WHERE target_type = $1 AND target_id = $2 AND detached_at IS NOT NULL \
               AND gc_eligible_at > now()",
        )
        .bind(target_type.as_str())
        .bind(account_id)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

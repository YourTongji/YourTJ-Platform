//! Privacy-safe account directory queries for other domains.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{FromRow, PgConnection, PgPool};

/// Public account fields that may be shared with another domain or anonymous client.
#[derive(Debug, Clone, FromRow)]
pub struct PublicAccount {
    pub id: i64,
    pub handle: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_asset_id: Option<i64>,
    pub banner_asset_id: Option<i64>,
    pub role: String,
    pub trust_level: i16,
    pub profile_visibility: String,
    pub followers_visibility: String,
    pub following_visibility: String,
    pub discoverable: bool,
    pub dm_policy: String,
    pub created_at: DateTime<Utc>,
}

/// Minimal account state used by other domains for privileged target checks.
#[derive(Debug, Clone, FromRow)]
pub struct AccountAuthorizationState {
    pub role: String,
    pub status: String,
}

/// Find an active, non-suspended account by public handle without selecting PII.
pub async fn find_public_account_by_handle(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<PublicAccount>> {
    let account = sqlx::query_as::<_, PublicAccount>(
        "SELECT account.id, account.handle::text, profile.display_name, profile.bio, \
                profile.website, profile.avatar_asset_id, profile.banner_asset_id, \
                account.role::text, account.trust_level, \
                COALESCE(privacy.profile_visibility, 'campus') AS profile_visibility, \
                COALESCE(privacy.followers_visibility, 'followers') AS followers_visibility, \
                COALESCE(privacy.following_visibility, 'followers') AS following_visibility, \
                COALESCE(privacy.discoverable, TRUE) AS discoverable, \
                COALESCE(privacy.dm_policy, 'following') AS dm_policy, account.created_at \
         FROM identity.accounts AS account \
         LEFT JOIN identity.profiles AS profile ON profile.account_id = account.id \
         LEFT JOIN identity.profile_privacy AS privacy ON privacy.account_id = account.id \
         WHERE account.handle = $1::citext \
           AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           )",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await?;
    Ok(account)
}

/// Find one active, non-suspended account by id without selecting PII.
pub async fn find_public_account_by_id(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<Option<PublicAccount>> {
    let accounts = find_public_accounts_by_ids(pool, &[account_id]).await?;
    Ok(accounts.into_iter().next())
}

/// Batch-load active, non-suspended public account projections for relationship lists.
pub async fn find_public_accounts_by_ids(
    pool: &PgPool,
    account_ids: &[i64],
) -> AppResult<Vec<PublicAccount>> {
    if account_ids.is_empty() {
        return Ok(Vec::new());
    }
    let accounts = sqlx::query_as::<_, PublicAccount>(
        "SELECT account.id, account.handle::text, profile.display_name, profile.bio, \
                profile.website, profile.avatar_asset_id, profile.banner_asset_id, \
                account.role::text, account.trust_level, \
                COALESCE(privacy.profile_visibility, 'campus') AS profile_visibility, \
                COALESCE(privacy.followers_visibility, 'followers') AS followers_visibility, \
                COALESCE(privacy.following_visibility, 'followers') AS following_visibility, \
                COALESCE(privacy.discoverable, TRUE) AS discoverable, \
                COALESCE(privacy.dm_policy, 'following') AS dm_policy, account.created_at \
         FROM identity.accounts AS account \
         LEFT JOIN identity.profiles AS profile ON profile.account_id = account.id \
         LEFT JOIN identity.profile_privacy AS privacy ON privacy.account_id = account.id \
         WHERE account.id = ANY($1) AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           )",
    )
    .bind(account_ids)
    .fetch_all(pool)
    .await?;
    Ok(accounts)
}

/// Resolve an exact handle only so another domain can remove an owner-held relationship.
///
/// This intentionally includes suspended and lifecycle-closed accounts: an account owner
/// must still be able to unfollow, unmute, or unblock them. No status or PII is returned.
pub async fn find_account_id_by_handle_for_relationship_cleanup(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<i64>> {
    let account_id =
        sqlx::query_scalar("SELECT id FROM identity.accounts WHERE handle = $1::citext")
            .bind(handle)
            .fetch_optional(pool)
            .await?;
    Ok(account_id)
}

/// Resolve an exact non-deleted handle for owner-initiated mute or block safety actions.
///
/// Suspended accounts remain valid targets so a temporary sanction cannot prevent an owner
/// from establishing a durable personal safety boundary.
pub async fn find_account_id_by_handle_for_safety_action(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<i64>> {
    let account_id = sqlx::query_scalar(
        "SELECT id FROM identity.accounts \
         WHERE handle = $1::citext AND status <> 'deleted'::identity.account_status",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await?;
    Ok(account_id)
}

/// Return an account role for cross-domain authorization without selecting PII.
///
/// Deleted accounts remain role-bearing for moderation hierarchy so deleting an
/// account cannot weaken protection on content it authored.
pub async fn find_account_role_by_id(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<Option<String>> {
    let role =
        sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1 FOR SHARE")
            .bind(account_id)
            .fetch_optional(connection)
            .await?;
    Ok(role)
}

/// Lock and return the role/status needed for a cross-domain privileged mutation.
pub async fn find_account_authorization_state_by_id(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<Option<AccountAuthorizationState>> {
    let account = sqlx::query_as::<_, AccountAuthorizationState>(
        "SELECT role::text, status::text FROM identity.accounts WHERE id = $1 FOR SHARE",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?;
    Ok(account)
}

/// Return whether an account may receive a controlled credit reward.
///
/// Eligibility requires an active account and no unrevoked, unexpired suspend
/// sanction. The query selects no email or other private account fields.
pub async fn is_credit_recipient_eligible(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<bool> {
    let eligible = sqlx::query_scalar::<_, bool>(
        "SELECT true FROM identity.accounts account \
         WHERE account.id = $1 AND account.status = 'active' \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           ) \
         FOR SHARE OF account",
    )
    .bind(account_id)
    .fetch_optional(&mut *connection)
    .await?
    .unwrap_or(false);
    Ok(eligible)
}

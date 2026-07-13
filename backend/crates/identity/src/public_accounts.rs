//! Privacy-safe account directory queries for other domains.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgConnection, PgPool};

/// Public account fields that may be shared with another domain or anonymous client.
#[derive(Debug, Clone, FromRow)]
pub struct PublicAccount {
    pub id: i64,
    pub handle: String,
    pub display_name: Option<String>,
    pub school: String,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_asset_id: Option<i64>,
    pub banner_asset_id: Option<i64>,
    pub role: String,
    pub trust_level: i16,
    pub profile_visibility: String,
    pub activity_visibility: String,
    pub followers_visibility: String,
    pub following_visibility: String,
    pub discoverable: bool,
    pub dm_policy: String,
    pub mention_policy: String,
    pub is_campus_verified: bool,
    pub created_at: DateTime<Utc>,
}

/// Minimal active-account projection used to resolve semantic mention recipients.
#[derive(Debug, Clone, FromRow)]
pub struct MentionTarget {
    pub id: i64,
    pub handle: String,
    pub mention_policy: String,
}

/// Minimal active-account projection used by the notification delivery boundary.
#[derive(Debug, Clone, FromRow)]
pub struct NotificationRecipient {
    pub handle: String,
    pub mention_policy: String,
}

/// Minimal account state used by other domains for privileged target checks.
#[derive(Debug, Clone, FromRow)]
pub struct AccountAuthorizationState {
    pub role: String,
    pub status: String,
}

/// Purpose-limited account projection for bounded cross-domain staff target checks.
#[derive(Debug, Clone, FromRow)]
pub struct StaffTargetAuthorizationState {
    pub account_id: i64,
    pub role: String,
}

const MAX_STAFF_TARGET_BATCH: usize = 200;

/// Batch-load only role for staff authorization targets.
///
/// Callers must supply IDs already selected by their owning domain. This API intentionally returns
/// no email, handle, profile, sanctions, or relationship data and rejects unbounded batches.
pub async fn find_staff_target_authorization_states_by_ids(
    pool: &PgPool,
    account_ids: &[i64],
) -> AppResult<HashMap<i64, StaffTargetAuthorizationState>> {
    let unique_ids = account_ids.iter().copied().collect::<HashSet<_>>();
    if unique_ids.len() > MAX_STAFF_TARGET_BATCH {
        return Err(AppError::BadRequest("staff target authorization batch is too large".into()));
    }
    if unique_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let unique_ids = unique_ids.into_iter().collect::<Vec<_>>();
    let rows = sqlx::query_as::<_, StaffTargetAuthorizationState>(
        "SELECT id AS account_id, role::text FROM identity.accounts WHERE id = ANY($1)",
    )
    .bind(&unique_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|row| (row.account_id, row)).collect())
}

/// Find an active, non-suspended account by public handle without selecting PII.
pub async fn find_public_account_by_handle(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<PublicAccount>> {
    let account = sqlx::query_as::<_, PublicAccount>(
        "SELECT account.id, account.handle::text, profile.display_name, \
                COALESCE(profile.school, '同济大学') AS school, profile.bio, \
                profile.website, profile.avatar_asset_id, profile.banner_asset_id, \
                account.role::text, account.trust_level, \
                COALESCE(privacy.profile_visibility, 'campus') AS profile_visibility, \
                COALESCE(privacy.activity_visibility, 'only_me') AS activity_visibility, \
                COALESCE(privacy.followers_visibility, 'followers') AS followers_visibility, \
                COALESCE(privacy.following_visibility, 'followers') AS following_visibility, \
                COALESCE(privacy.discoverable, TRUE) AS discoverable, \
                COALESCE(privacy.dm_policy, 'following') AS dm_policy, \
                COALESCE(privacy.mention_policy, 'everyone') AS mention_policy, \
                account.email_verified_at IS NOT NULL AS is_campus_verified, account.created_at \
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
        "SELECT account.id, account.handle::text, profile.display_name, \
                COALESCE(profile.school, '同济大学') AS school, profile.bio, \
                profile.website, profile.avatar_asset_id, profile.banner_asset_id, \
                account.role::text, account.trust_level, \
                COALESCE(privacy.profile_visibility, 'campus') AS profile_visibility, \
                COALESCE(privacy.activity_visibility, 'only_me') AS activity_visibility, \
                COALESCE(privacy.followers_visibility, 'followers') AS followers_visibility, \
                COALESCE(privacy.following_visibility, 'followers') AS following_visibility, \
                COALESCE(privacy.discoverable, TRUE) AS discoverable, \
                COALESCE(privacy.dm_policy, 'following') AS dm_policy, \
                COALESCE(privacy.mention_policy, 'everyone') AS mention_policy, \
                account.email_verified_at IS NOT NULL AS is_campus_verified, account.created_at \
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

/// Batch-resolve exact handles to active, non-suspended mention targets without selecting PII.
pub async fn find_mention_targets_by_handles(
    pool: &PgPool,
    handles: &[String],
) -> AppResult<Vec<MentionTarget>> {
    let mut seen = HashSet::new();
    let normalized_handles: Vec<String> = handles
        .iter()
        .map(|handle| handle.to_ascii_lowercase())
        .filter(|handle| seen.insert(handle.clone()))
        .collect();
    if normalized_handles.is_empty() {
        return Ok(Vec::new());
    }
    let targets = sqlx::query_as::<_, MentionTarget>(
        "SELECT account.id, account.handle::text, \
                COALESCE(privacy.mention_policy, 'everyone') AS mention_policy \
         FROM identity.accounts AS account \
         LEFT JOIN identity.profile_privacy AS privacy ON privacy.account_id = account.id \
         WHERE lower(account.handle::text) = ANY($1) \
           AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           ) \
         ORDER BY account.id",
    )
    .bind(&normalized_handles)
    .fetch_all(pool)
    .await?;
    Ok(targets)
}

/// Resolve active mention candidates in the caller's content transaction.
pub async fn find_mention_targets_by_handles_tx(
    connection: &mut PgConnection,
    handles: &[String],
) -> AppResult<Vec<MentionTarget>> {
    let mut seen = HashSet::new();
    let normalized_handles: Vec<String> = handles
        .iter()
        .map(|handle| handle.to_ascii_lowercase())
        .filter(|handle| seen.insert(handle.clone()))
        .collect();
    if normalized_handles.is_empty() {
        return Ok(Vec::new());
    }
    let targets = sqlx::query_as::<_, MentionTarget>(
        "SELECT account.id, account.handle::text, \
                COALESCE(privacy.mention_policy, 'everyone') AS mention_policy \
         FROM identity.accounts AS account \
         LEFT JOIN identity.profile_privacy AS privacy ON privacy.account_id = account.id \
         WHERE lower(account.handle::text) = ANY($1) \
           AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           ) \
         ORDER BY account.id \
         FOR SHARE OF account",
    )
    .bind(&normalized_handles)
    .fetch_all(connection)
    .await?;
    Ok(targets)
}

/// Lock and return the minimum recipient policy needed to deliver an interaction notification.
///
/// Lifecycle-closed and currently suspended accounts return `None`; no email or profile body is
/// selected. Forum applies its own block, mute, follow, conversation, and channel policy after
/// this owner-domain check.
pub async fn lock_notification_recipient(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<Option<NotificationRecipient>> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("identity-profile-privacy:{account_id}"))
        .execute(&mut *connection)
        .await?;
    let handle: Option<String> = sqlx::query_scalar(
        "SELECT account.handle::text \
         FROM identity.accounts AS account \
         WHERE account.id = $1 AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           ) \
         FOR SHARE OF account",
    )
    .bind(account_id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(handle) = handle else {
        return Ok(None);
    };
    let mention_policy = sqlx::query_scalar(
        "SELECT mention_policy FROM identity.profile_privacy \
         WHERE account_id = $1 FOR SHARE",
    )
    .bind(account_id)
    .fetch_optional(&mut *connection)
    .await?
    .unwrap_or_else(|| "everyone".to_owned());
    Ok(Some(NotificationRecipient { handle, mention_policy }))
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

/// Resolve an exact active or suspended handle for owner-initiated mute or block safety actions.
///
/// Suspended accounts remain valid targets so a temporary sanction cannot prevent an owner
/// from establishing a durable personal safety boundary.
pub async fn find_account_id_by_handle_for_safety_action(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<i64>> {
    let account_id = sqlx::query_scalar(
        "SELECT id FROM identity.accounts \
         WHERE handle = $1::citext AND status IN ('active', 'suspended')",
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
            .fetch_optional(&mut *connection)
            .await?;
    Ok(role)
}

/// Batch-load role-only projections for cross-domain authorization displays.
///
/// This intentionally includes lifecycle-closed accounts: content authored before an
/// account closes still participates in the same moderation hierarchy. No PII is selected.
pub async fn find_account_roles_by_ids(
    pool: &PgPool,
    account_ids: &[i64],
) -> AppResult<HashMap<i64, String>> {
    if account_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, role::text FROM identity.accounts WHERE id = ANY($1)")
            .bind(account_ids)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
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

/// Exclusively lock the minimal account authorization state for a cross-domain mutation.
pub async fn lock_account_authorization_state_by_id(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<Option<AccountAuthorizationState>> {
    let account = sqlx::query_as::<_, AccountAuthorizationState>(
        "SELECT role::text, status::text FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?;
    Ok(account)
}

/// Serialize an account-owned mutation with lifecycle transitions and recheck writability.
pub async fn lock_active_account_for_owned_mutation(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<()> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *connection)
            .await?;
    match status.as_deref() {
        Some("active") => {}
        Some(_) => return Err(AppError::Forbidden),
        None => return Err(AppError::NotFound),
    }
    let is_suspended: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM identity.sanctions sanction \
           WHERE sanction.account_id = $1 AND sanction.kind = 'suspend' \
             AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
             AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
         )",
    )
    .bind(account_id)
    .fetch_one(&mut *connection)
    .await?;
    if is_suspended {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Lock active, non-suspended accounts before a cross-domain direct interaction.
///
/// The account locks serialize the eligibility check with staff suspension, while
/// returning only an authorization fact and never selecting profile or contact data.
pub async fn lock_active_interaction_accounts(
    connection: &mut PgConnection,
    account_ids: &[i64],
) -> AppResult<bool> {
    if account_ids.is_empty() {
        return Ok(false);
    }
    let mut expected_ids = account_ids.to_vec();
    expected_ids.sort_unstable();
    expected_ids.dedup();
    let eligible_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT account.id FROM identity.accounts AS account \
         WHERE account.id = ANY($1) \
           AND account.status = 'active'::identity.account_status \
           AND NOT EXISTS ( \
             SELECT 1 FROM identity.sanctions AS sanction \
             WHERE sanction.account_id = account.id AND sanction.kind = 'suspend' \
               AND sanction.revoked_at IS NULL AND sanction.starts_at <= now() \
               AND (sanction.ends_at IS NULL OR sanction.ends_at > now()) \
           ) \
         ORDER BY account.id FOR SHARE OF account",
    )
    .bind(&expected_ids)
    .fetch_all(connection)
    .await?;
    Ok(eligible_ids == expected_ids)
}

/// Lock and return the recipient's new-conversation policy for an interaction transaction.
pub async fn lock_dm_policy(connection: &mut PgConnection, account_id: i64) -> AppResult<String> {
    let policy = sqlx::query_scalar(
        "SELECT dm_policy FROM identity.profile_privacy WHERE account_id = $1 FOR SHARE",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?
    .unwrap_or_else(|| "following".to_owned());
    Ok(policy)
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

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::find_staff_target_authorization_states_by_ids;

    #[tokio::test]
    async fn staff_target_projection_rejects_unbounded_unique_ids_before_database_access() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
            .expect("syntactically valid lazy test pool");
        let account_ids = (1_i64..=201).collect::<Vec<_>>();
        let error = find_staff_target_authorization_states_by_ids(&pool, &account_ids)
            .await
            .expect_err("more than the bounded authorization batch must fail");
        assert!(error.to_string().contains("batch is too large"));
    }
}

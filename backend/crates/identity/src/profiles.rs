//! Owner-editable profile fields and privacy policy persistence.

use shared::AppResult;
use sqlx::{FromRow, PgConnection, PgPool};

/// Controlled profile image slot updated only by the media domain.
#[derive(Debug, Clone, Copy)]
pub enum ProfileAssetKind {
    Avatar,
    Banner,
}

/// Stored profile fields without email or other account PII.
#[derive(Debug, Clone, FromRow)]
pub struct ProfileRecord {
    pub account_id: i64,
    pub display_name: Option<String>,
    pub school: String,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_asset_id: Option<i64>,
    pub banner_asset_id: Option<i64>,
}

/// Stored visibility and new-conversation policy.
#[derive(Debug, Clone, FromRow)]
pub struct ProfilePrivacyRecord {
    pub profile_visibility: String,
    pub activity_visibility: String,
    pub followers_visibility: String,
    pub following_visibility: String,
    pub discoverable: bool,
    pub dm_policy: String,
    pub mention_policy: String,
}

/// Return profile fields, lazily creating the one-to-one row when needed.
pub async fn get_or_create_profile(pool: &PgPool, account_id: i64) -> AppResult<ProfileRecord> {
    sqlx::query(
        "INSERT INTO identity.profiles (account_id) VALUES ($1) \
         ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .execute(pool)
    .await?;
    let profile = sqlx::query_as::<_, ProfileRecord>(
        "SELECT account_id, display_name, school, bio, website, avatar_asset_id, banner_asset_id \
         FROM identity.profiles WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(profile)
}

/// Replace every owner-editable text field while preserving media references.
pub async fn replace_profile_text(
    pool: &PgPool,
    account_id: i64,
    display_name: Option<&str>,
    school: Option<&str>,
    bio: Option<&str>,
    website: Option<&str>,
) -> AppResult<ProfileRecord> {
    let profile = sqlx::query_as::<_, ProfileRecord>(
        "INSERT INTO identity.profiles (account_id, display_name, school, bio, website) \
         VALUES ($1, $2, COALESCE($3, '同济大学'), $4, $5) \
         ON CONFLICT (account_id) DO UPDATE \
         SET display_name = EXCLUDED.display_name, \
             school = COALESCE($3, identity.profiles.school), bio = EXCLUDED.bio, \
             website = EXCLUDED.website, updated_at = now() \
         RETURNING account_id, display_name, school, bio, website, avatar_asset_id, banner_asset_id",
    )
    .bind(account_id)
    .bind(display_name)
    .bind(school)
    .bind(bio)
    .bind(website)
    .fetch_one(pool)
    .await?;
    Ok(profile)
}

/// Return privacy defaults, lazily creating the one-to-one row when needed.
pub async fn get_or_create_privacy(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<ProfilePrivacyRecord> {
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id) VALUES ($1) \
         ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .execute(pool)
    .await?;
    let privacy = sqlx::query_as::<_, ProfilePrivacyRecord>(
        "SELECT profile_visibility, activity_visibility, followers_visibility, \
                following_visibility, discoverable, dm_policy, mention_policy \
         FROM identity.profile_privacy WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    Ok(privacy)
}

/// Replace all privacy policy fields atomically.
#[allow(clippy::too_many_arguments)] // reason: the call mirrors one complete privacy-policy document.
pub async fn replace_privacy(
    pool: &PgPool,
    account_id: i64,
    profile_visibility: &str,
    activity_visibility: Option<&str>,
    followers_visibility: &str,
    following_visibility: &str,
    discoverable: bool,
    dm_policy: &str,
    mention_policy: Option<&str>,
) -> AppResult<ProfilePrivacyRecord> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("identity-profile-privacy:{account_id}"))
        .execute(&mut *transaction)
        .await?;
    let privacy = sqlx::query_as::<_, ProfilePrivacyRecord>(
        "INSERT INTO identity.profile_privacy \
         (account_id, profile_visibility, activity_visibility, followers_visibility, \
          following_visibility, discoverable, dm_policy, mention_policy) \
         VALUES ($1, $2, COALESCE($3, 'only_me'), $4, $5, $6, $7, \
                 COALESCE($8, 'everyone')) \
         ON CONFLICT (account_id) DO UPDATE \
         SET profile_visibility = EXCLUDED.profile_visibility, \
             activity_visibility = COALESCE($3, identity.profile_privacy.activity_visibility), \
             followers_visibility = EXCLUDED.followers_visibility, \
             following_visibility = EXCLUDED.following_visibility, \
             discoverable = EXCLUDED.discoverable, dm_policy = EXCLUDED.dm_policy, \
             mention_policy = COALESCE($8, identity.profile_privacy.mention_policy), \
             updated_at = now() \
         RETURNING profile_visibility, activity_visibility, followers_visibility, \
                   following_visibility, discoverable, dm_policy, mention_policy",
    )
    .bind(account_id)
    .bind(profile_visibility)
    .bind(activity_visibility)
    .bind(followers_visibility)
    .bind(following_visibility)
    .bind(discoverable)
    .bind(dm_policy)
    .bind(mention_policy)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(privacy)
}

/// Bind or clear one controlled media reference after the media domain validates it.
pub async fn set_profile_asset(
    connection: &mut PgConnection,
    account_id: i64,
    kind: ProfileAssetKind,
    asset_id: Option<i64>,
) -> AppResult<()> {
    let column = match kind {
        ProfileAssetKind::Avatar => "avatar_asset_id",
        ProfileAssetKind::Banner => "banner_asset_id",
    };
    let statement = format!(
        "INSERT INTO identity.profiles (account_id, {column}) VALUES ($1, $2) \
         ON CONFLICT (account_id) DO UPDATE SET {column} = EXCLUDED.{column}, updated_at = now()"
    );
    sqlx::query(&statement).bind(account_id).bind(asset_id).execute(connection).await?;
    Ok(())
}

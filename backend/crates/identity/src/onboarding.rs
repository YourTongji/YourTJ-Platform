//! Resumable first-run onboarding with versioned terms acceptance.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::{AppError, AppResult};
use sqlx::{FromRow, PgPool};

pub const CURRENT_TERMS_VERSION: &str = "2026-07-12";

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    pub required: bool,
    pub current_terms_version: String,
    pub accepted_terms_version: Option<String>,
    pub handle: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub profile_visibility: String,
    pub activity_visibility: String,
    pub discoverable: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct OnboardingChoices<'a> {
    pub handle: &'a str,
    pub display_name: Option<&'a str>,
    pub bio: Option<&'a str>,
    pub profile_visibility: &'a str,
    pub activity_visibility: &'a str,
    pub discoverable: bool,
    pub accepted_terms_version: &'a str,
}

pub async fn is_required(pool: &PgPool, account_id: i64) -> AppResult<bool> {
    let required = sqlx::query_scalar(
        "SELECT completed_at IS NULL FROM identity.account_onboarding WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(true);
    Ok(required)
}

pub async fn get(pool: &PgPool, account_id: i64) -> AppResult<OnboardingState> {
    sqlx::query_as::<_, OnboardingState>(
        "SELECT onboarding.completed_at IS NULL AS required, \
                onboarding.required_terms_version AS current_terms_version, \
                onboarding.accepted_terms_version, account.handle::text AS handle, \
                profile.display_name, profile.bio, \
                COALESCE(privacy.profile_visibility, 'campus') AS profile_visibility, \
                COALESCE(privacy.activity_visibility, 'only_me') AS activity_visibility, \
                COALESCE(privacy.discoverable, TRUE) AS discoverable, onboarding.completed_at \
         FROM identity.accounts account \
         JOIN identity.account_onboarding onboarding ON onboarding.account_id = account.id \
         LEFT JOIN identity.profiles profile ON profile.account_id = account.id \
         LEFT JOIN identity.profile_privacy privacy ON privacy.account_id = account.id \
         WHERE account.id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn complete(
    pool: &PgPool,
    account_id: i64,
    choices: OnboardingChoices<'_>,
) -> AppResult<OnboardingState> {
    if choices.accepted_terms_version != CURRENT_TERMS_VERSION {
        return Err(AppError::Conflict(
            "terms version changed; reload onboarding before accepting".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    let account_status: String =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    if account_status != "active" {
        return Err(AppError::Conflict("account is not active".into()));
    }
    let required_version: String = sqlx::query_scalar(
        "SELECT required_terms_version FROM identity.account_onboarding \
         WHERE account_id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if required_version != CURRENT_TERMS_VERSION {
        return Err(AppError::Conflict(
            "terms version changed; reload onboarding before accepting".into(),
        ));
    }
    let handle_taken: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE handle = $1::citext AND id <> $2)",
    )
    .bind(choices.handle)
    .bind(account_id)
    .fetch_one(&mut *tx)
    .await?;
    if handle_taken {
        return Err(AppError::Conflict("handle is already taken".into()));
    }
    let handle_update = sqlx::query(
        "UPDATE identity.accounts SET handle = $2, updated_at = now() \
         WHERE id = $1 AND status = 'active'",
    )
    .bind(account_id)
    .bind(choices.handle)
    .execute(&mut *tx)
    .await;
    if let Err(error) = handle_update {
        if error.as_database_error().is_some_and(|database| database.is_unique_violation()) {
            return Err(AppError::Conflict("handle is already taken".into()));
        }
        return Err(error.into());
    }
    sqlx::query(
        "INSERT INTO identity.profiles (account_id, display_name, bio) VALUES ($1, $2, $3) \
         ON CONFLICT (account_id) DO UPDATE SET display_name = EXCLUDED.display_name, \
             bio = EXCLUDED.bio, updated_at = now()",
    )
    .bind(account_id)
    .bind(choices.display_name)
    .bind(choices.bio)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO identity.profile_privacy \
         (account_id, profile_visibility, activity_visibility, discoverable) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (account_id) DO UPDATE SET profile_visibility = EXCLUDED.profile_visibility, \
             activity_visibility = EXCLUDED.activity_visibility, discoverable = EXCLUDED.discoverable, \
             updated_at = now()",
    )
    .bind(account_id)
    .bind(choices.profile_visibility)
    .bind(choices.activity_visibility)
    .bind(choices.discoverable)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.account_onboarding SET \
             accepted_at = CASE WHEN accepted_terms_version IS DISTINCT FROM $2 \
                                THEN now() ELSE accepted_at END, \
             accepted_terms_version = $2, completed_at = now(), \
             updated_at = now() WHERE account_id = $1",
    )
    .bind(account_id)
    .bind(CURRENT_TERMS_VERSION)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    get(pool, account_id).await
}

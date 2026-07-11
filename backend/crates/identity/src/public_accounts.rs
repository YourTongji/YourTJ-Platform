//! Privacy-safe account directory queries for other domains.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{FromRow, PgConnection, PgPool};

/// Public account fields that may be shared with another domain or anonymous client.
#[derive(Debug, Clone, FromRow)]
pub struct PublicAccount {
    pub id: i64,
    pub handle: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub trust_level: i16,
    pub created_at: DateTime<Utc>,
}

/// Find a non-deleted account by public handle without selecting or decrypting its email.
pub async fn find_public_account_by_handle(
    pool: &PgPool,
    handle: &str,
) -> AppResult<Option<PublicAccount>> {
    let account = sqlx::query_as::<_, PublicAccount>(
        "SELECT id, handle::text, avatar_url, role::text, trust_level, created_at \
         FROM identity.accounts \
         WHERE handle = $1::citext AND status <> 'deleted'::identity.account_status",
    )
    .bind(handle)
    .fetch_optional(pool)
    .await?;
    Ok(account)
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

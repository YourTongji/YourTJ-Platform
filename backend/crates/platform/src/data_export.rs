//! Platform-owned account-visible receipts, achievements, and verification projection.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformExport {
    announcement_receipts: Vec<ExportAnnouncementReceipt>,
    achievements: Vec<ExportAchievement>,
    verifications: Vec<ExportVerification>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportAnnouncementReceipt {
    announcement_id: i64,
    revision: i64,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    first_seen_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    dismissed_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    acknowledged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportAchievement {
    slug: String,
    name: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    awarded_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportVerification {
    category: String,
    label: String,
    display_on_profile: bool,
    #[serde(with = "chrono::serde::ts_seconds")]
    issued_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    expires_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    revoked_at: Option<DateTime<Utc>>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<PlatformExport> {
    let announcement_receipts = sqlx::query_as::<_, ExportAnnouncementReceipt>(
        "SELECT announcement_id, revision, first_seen_at, dismissed_at, acknowledged_at \
         FROM platform.announcement_receipts WHERE account_id = $1 \
         ORDER BY announcement_id, revision",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let achievements = sqlx::query_as::<_, ExportAchievement>(
        "SELECT badge.slug, badge.name, badge_grant.awarded_at, badge_grant.revoked_at \
         FROM platform.account_badges badge_grant \
         JOIN platform.badges badge ON badge.id = badge_grant.badge_id \
         WHERE badge_grant.account_id = $1 ORDER BY badge_grant.awarded_at, badge.id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let verifications = sqlx::query_as::<_, ExportVerification>(
        "SELECT definition.category, definition.label, verification_grant.display_on_profile, \
                verification_grant.issued_at, verification_grant.expires_at, \
                verification_grant.revoked_at \
         FROM platform.verification_grants verification_grant \
         JOIN platform.verification_types definition \
           ON definition.id = verification_grant.verification_type_id \
         WHERE verification_grant.account_id = $1 \
         ORDER BY verification_grant.issued_at, verification_grant.id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(PlatformExport { announcement_receipts, achievements, verifications })
}

pub async fn purge_account_private_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM platform.announcement_receipts WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE platform.verification_grants \
         SET display_on_profile = FALSE, evidence_reference = NULL, \
             issue_reason = 'account identity purged', \
             revoked_at = COALESCE(revoked_at, now()), \
             revoke_reason = COALESCE(revoke_reason, 'account identity purged') \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

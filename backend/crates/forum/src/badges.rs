//! Forum-owned eligibility rules for platform achievement awards.
//!
//! The platform crate owns definitions, grants, revocations, history, and contribution-mint
//! idempotency. This module only decides when a forum contribution satisfies a rule.

use shared::AppResult;
use sqlx::PgPool;

async fn award(
    pool: &PgPool,
    account_id: i64,
    slug: &str,
    awarded_by: i64,
    award_reason: &str,
) -> AppResult<bool> {
    let Some(result) = platform::achievements::award_achievement_by_slug(
        pool,
        account_id,
        slug,
        awarded_by,
        award_reason,
    )
    .await?
    else {
        tracing::warn!(slug, "active achievement definition is missing");
        return Ok(false);
    };

    if result.newly_awarded {
        crate::notification_hooks::create_notification(
            pool,
            account_id,
            "badge",
            serde_json::json!({ "badgeSlug": slug, "badgeName": result.name }),
            None,
            None,
        )
        .await;
    }
    Ok(result.newly_awarded)
}

/// Award the first-thread achievement after the account has published a visible thread.
pub async fn award_first_thread_badge(pool: &PgPool, account_id: i64) -> AppResult<bool> {
    let has_thread: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.threads \
         WHERE author_id = $1 AND deleted_at IS NULL)",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    if !has_thread {
        return Ok(false);
    }
    award(pool, account_id, "first-thread", account_id, "published a first forum thread").await
}

/// Award the quality-author achievement when staff features an account's thread.
pub async fn award_quality_author_badge(
    pool: &PgPool,
    account_id: i64,
    awarded_by: i64,
) -> AppResult<bool> {
    award(pool, account_id, "quality-author", awarded_by, "staff featured a forum thread").await
}

/// Award the first-comment achievement after the account has published a visible comment.
pub async fn award_first_comment_badge(pool: &PgPool, account_id: i64) -> AppResult<bool> {
    let has_comment: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.comments \
         WHERE author_id = $1 AND deleted_at IS NULL)",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;
    if !has_comment {
        return Ok(false);
    }
    award(pool, account_id, "first-comment", account_id, "published a first forum comment").await
}

//! Badge auto-award logic for the forum domain.
//!
//! Badge records live in the `platform` schema (`platform.badges` and
//! `platform.account_badges`) but the award rules are forum-specific, so they
//! belong in this crate. Credit minting is logged but not wired (the forum
//! crate does not depend on credit) — the `api` crate or a future refactor can
//! bridge that gap.

use sqlx::PgPool;

/// Standard badges seeded at startup.
pub const BADGES: &[(&str, &str, &str, &str, i64)] = &[
    ("first-thread", "首次发帖", "发表你的第一个主题", "", 5),
    ("quality-author", "优质作者", "你的主题被标记为精选", "", 10),
    ("first-comment", "首次评论", "发表你的第一条评论", "", 2),
];

/// Seed the standard badges on startup if they don't already exist.
///
/// This is idempotent — `slug` has a UNIQUE constraint so conflicts are
/// silently ignored.
pub async fn seed_badges(pool: &PgPool) {
    for (slug, name, description, icon_url, mint_amount) in BADGES {
        let result = sqlx::query(
            "INSERT INTO platform.badges (slug, name, description, icon_url, mint_amount) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (slug) DO NOTHING",
        )
        .bind(slug)
        .bind(name)
        .bind(description)
        .bind(icon_url)
        .bind(mint_amount)
        .execute(pool)
        .await;

        match result {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = %e, slug, "failed to seed badge");
            }
        }
    }
    tracing::info!("badges seeded");
}

/// Look up a badge id by slug. Returns `None` if the badge does not exist.
async fn find_badge_id(pool: &PgPool, slug: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT id FROM platform.badges WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// Insert an `account_badges` row. Returns `true` if a new row was inserted,
/// `false` if the badge was already held (unique constraint violation).
async fn try_award_badge(
    pool: &PgPool,
    account_id: i64,
    badge_id: i64,
    awarded_by: i64,
) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "INSERT INTO platform.account_badges (account_id, badge_id, awarded_by) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (account_id, badge_id) DO NOTHING",
    )
    .bind(account_id)
    .bind(badge_id)
    .bind(awarded_by)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Read `mint_amount` for a badge.
async fn badge_mint_amount(pool: &PgPool, badge_id: i64) -> i64 {
    sqlx::query_scalar("SELECT mint_amount FROM platform.badges WHERE id = $1")
        .bind(badge_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// Auto-award "first-thread" badge after creating a first thread.
///
/// Returns `true` if the badge was newly awarded, `false` if already held.
pub async fn award_first_thread_badge(pool: &PgPool, account_id: i64) -> anyhow::Result<bool> {
    let badge_id = match find_badge_id(pool, "first-thread").await {
        Some(id) => id,
        None => {
            // Seed on demand as a safety net.
            seed_badges(pool).await;
            match find_badge_id(pool, "first-thread").await {
                Some(id) => id,
                None => {
                    tracing::warn!("first-thread badge not found after seeding");
                    return Ok(false);
                }
            }
        }
    };

    // Check that this is actually the user's first thread.
    let thread_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.threads WHERE author_id = $1 AND deleted_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    if thread_count > 1 {
        return Ok(false);
    }

    // System account (id=0 or similar) can be the awarder for auto-awards.
    // We use the account's own id as awarded_by for automated badges.
    let newly_awarded = try_award_badge(pool, account_id, badge_id, account_id).await?;

    if newly_awarded {
        let mint_amount = badge_mint_amount(pool, badge_id).await;
        if mint_amount > 0 {
            let idempotency_key = format!("badge:first-thread:{account_id}");
            let _ = sqlx::query(
                "INSERT INTO platform.pending_mints (account_id, amount, idempotency_key, badge_slug) \
                 VALUES ($1, $2, $3, 'first-thread') \
                 ON CONFLICT (idempotency_key) DO NOTHING",
            )
            .bind(account_id)
            .bind(mint_amount)
            .bind(&idempotency_key)
            .execute(pool)
            .await;
        }

        // Spawn notification creation (fire-and-forget).
        let pool = pool.clone();
        tokio::spawn(async move {
            crate::notification_hooks::create_notification(
                &pool,
                account_id,
                "badge",
                serde_json::json!({ "badgeSlug": "first-thread", "badgeName": "首次发帖" }),
                None,
                None,
            )
            .await;
        });
    }

    Ok(newly_awarded)
}

/// Auto-award "quality-author" badge when a mod marks a thread as featured.
///
/// `awarded_by` is the mod/admin who performed the feature action.
pub async fn award_quality_author_badge(
    pool: &PgPool,
    account_id: i64,
    awarded_by: i64,
) -> anyhow::Result<bool> {
    let badge_id = match find_badge_id(pool, "quality-author").await {
        Some(id) => id,
        None => {
            seed_badges(pool).await;
            match find_badge_id(pool, "quality-author").await {
                Some(id) => id,
                None => {
                    tracing::warn!("quality-author badge not found after seeding");
                    return Ok(false);
                }
            }
        }
    };

    let newly_awarded = try_award_badge(pool, account_id, badge_id, awarded_by).await?;

    if newly_awarded {
        let mint_amount = badge_mint_amount(pool, badge_id).await;
        if mint_amount > 0 {
            let idempotency_key = format!("badge:quality-author:{account_id}");
            let _ = sqlx::query(
                "INSERT INTO platform.pending_mints (account_id, amount, idempotency_key, badge_slug) \
                 VALUES ($1, $2, $3, 'quality-author') \
                 ON CONFLICT (idempotency_key) DO NOTHING",
            )
            .bind(account_id)
            .bind(mint_amount)
            .bind(&idempotency_key)
            .execute(pool)
            .await;
        }

        let pool = pool.clone();
        tokio::spawn(async move {
            crate::notification_hooks::create_notification(
                &pool,
                account_id,
                "badge",
                serde_json::json!({ "badgeSlug": "quality-author", "badgeName": "优质作者" }),
                None,
                None,
            )
            .await;
        });
    }

    Ok(newly_awarded)
}

/// Auto-award "first-comment" badge after creating a first comment.
///
/// Returns `true` if the badge was newly awarded, `false` if already held.
pub async fn award_first_comment_badge(pool: &PgPool, account_id: i64) -> anyhow::Result<bool> {
    let badge_id = match find_badge_id(pool, "first-comment").await {
        Some(id) => id,
        None => {
            seed_badges(pool).await;
            match find_badge_id(pool, "first-comment").await {
                Some(id) => id,
                None => {
                    tracing::warn!("first-comment badge not found after seeding");
                    return Ok(false);
                }
            }
        }
    };

    // Check that this is actually the user's first comment.
    let comment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.comments WHERE author_id = $1 AND deleted_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    if comment_count > 1 {
        return Ok(false);
    }

    let newly_awarded = try_award_badge(pool, account_id, badge_id, account_id).await?;

    if newly_awarded {
        let mint_amount = badge_mint_amount(pool, badge_id).await;
        if mint_amount > 0 {
            let idempotency_key = format!("badge:first-comment:{account_id}");
            let _ = sqlx::query(
                "INSERT INTO platform.pending_mints (account_id, amount, idempotency_key, badge_slug) \
                 VALUES ($1, $2, $3, 'first-comment') \
                 ON CONFLICT (idempotency_key) DO NOTHING",
            )
            .bind(account_id)
            .bind(mint_amount)
            .bind(&idempotency_key)
            .execute(pool)
            .await;
        }

        let pool = pool.clone();
        tokio::spawn(async move {
            crate::notification_hooks::create_notification(
                &pool,
                account_id,
                "badge",
                serde_json::json!({ "badgeSlug": "first-comment", "badgeName": "首次评论" }),
                None,
                None,
            )
            .await;
        });
    }

    Ok(newly_awarded)
}

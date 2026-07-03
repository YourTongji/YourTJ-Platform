//! Forum-side sanction check wrappers.
//!
//! These call into the identity crate's public API, maintaining domain boundaries.
//! Note: The identity crate will expose its own `is_silenced` / `is_suspended` functions.
//! These forum-side wrappers will call them.

use sqlx::PgPool;

use shared::AppResult;

/// Check if an account is currently silenced (can't write).
/// Delegates to identity crate's check.
pub async fn require_can_post(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    account_id: i64,
) -> AppResult<()> {
    if is_silenced(redis, pool, account_id).await? {
        return Err(shared::AppError::Forbidden);
    }
    Ok(())
}

/// Check if account is silenced (cached).
async fn is_silenced(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    account_id: i64,
) -> AppResult<bool> {
    // Try cache first
    if let Some(r) = redis {
        let key = format!("identity:sanction:{account_id}");
        let mut conn = r.get().await.map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
        if let Ok(Some(val)) =
            redis::cmd("GET").arg(&key).query_async::<Option<String>>(&mut conn).await
        {
            if val == "silence" {
                return Ok(true);
            }
            if val == "none" {
                return Ok(false);
            }
        }
    }

    // Check DB: active silence (no end or ends_at > now, not revoked)
    let silenced: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
         SELECT 1 FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'silence' \
         AND revoked_at IS NULL \
         AND (ends_at IS NULL OR ends_at > now()) \
        )",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    // Cache result
    if let Some(r) = redis {
        let key = format!("identity:sanction:{account_id}");
        let mut conn = r.get().await.map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
        let val = if silenced { "silence" } else { "none" };
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(60)
            .arg(val)
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    Ok(silenced)
}

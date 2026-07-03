//! Sanction enforcement: silence and suspend checks with Redis caching.

use shared::AppResult;
use sqlx::PgPool;

/// Check if an account is currently silenced (can't write).
/// Results cached in Redis for 60s.
pub async fn is_silenced(
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

/// Check if an account is currently suspended (can't login).
/// Results cached in Redis for 60s.
pub async fn is_suspended(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    account_id: i64,
) -> AppResult<bool> {
    if let Some(r) = redis {
        let key = format!("identity:sanction:{account_id}");
        let mut conn = r.get().await.map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
        if let Ok(Some(val)) =
            redis::cmd("GET").arg(&key).query_async::<Option<String>>(&mut conn).await
        {
            if val == "suspend" {
                return Ok(true);
            }
            if val == "none" {
                return Ok(false);
            }
        }
    }

    let suspended: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
         SELECT 1 FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'suspend' \
         AND revoked_at IS NULL \
         AND (ends_at IS NULL OR ends_at > now()) \
        )",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await?;

    if let Some(r) = redis {
        let key = format!("identity:sanction:{account_id}");
        let mut conn = r.get().await.map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
        let val = if suspended { "suspend" } else { "none" };
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(60)
            .arg(val)
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    Ok(suspended)
}

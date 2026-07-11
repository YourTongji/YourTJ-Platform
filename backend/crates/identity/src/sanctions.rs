//! Sanction issuance and enforcement with Redis caching.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{PgConnection, PgPool};

/// Issue an identity-owned system silence inside the caller's transaction.
///
/// Staff accounts are protected and an already-active silence is left unchanged.
/// Returns `true` only when a new sanction and matching audit event are written.
pub async fn issue_system_silence_tx(
    connection: &mut PgConnection,
    account_id: i64,
    reason: &str,
    ends_at: DateTime<Utc>,
    audit_metadata: Option<&serde_json::Value>,
) -> AppResult<bool> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(shared::AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    if ends_at <= Utc::now() {
        return Err(shared::AppError::BadRequest("endsAt must be in the future".into()));
    }

    let role: String = sqlx::query_scalar(
        "SELECT role::text FROM identity.accounts \
         WHERE id = $1 AND status <> 'deleted'::identity.account_status FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    if matches!(role.as_str(), "mod" | "admin") {
        return Ok(false);
    }

    let has_active_silence: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM identity.sanctions \
           WHERE account_id = $1 AND kind = 'silence' AND revoked_at IS NULL \
             AND starts_at <= now() \
             AND (ends_at IS NULL OR ends_at > now()) \
         )",
    )
    .bind(account_id)
    .fetch_one(&mut *connection)
    .await?;
    if has_active_silence {
        return Ok(false);
    }

    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, ends_at) \
         VALUES ($1, 'silence', $2, $3)",
    )
    .bind(account_id)
    .bind(reason)
    .bind(ends_at)
    .execute(&mut *connection)
    .await?;
    governance::record_system_event_tx(
        connection,
        "identity.sanction.auto_silence",
        "account",
        &account_id.to_string(),
        reason,
        audit_metadata,
    )
    .await?;
    Ok(true)
}

/// Invalidate the cached silence state after a committed sanction mutation.
pub async fn invalidate_silence_cache(redis: Option<&deadpool_redis::Pool>, account_id: i64) {
    let Some(redis) = redis else {
        return;
    };
    let Ok(mut connection) = redis.get().await else {
        tracing::warn!(account_id, "failed to acquire Redis connection for sanction invalidation");
        return;
    };
    let result: redis::RedisResult<()> = redis::cmd("DEL")
        .arg(format!("identity:silence:{account_id}"))
        .query_async(&mut connection)
        .await;
    if let Err(error) = result {
        tracing::warn!(account_id, ?error, "failed to invalidate silence cache");
    }
}

/// Check if an account is currently silenced (can't write).
/// Results cached in Redis for 60s.
pub async fn is_silenced(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    account_id: i64,
) -> AppResult<bool> {
    // Try cache first
    if let Some(r) = redis {
        let key = format!("identity:silence:{account_id}");
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
        let key = format!("identity:silence:{account_id}");
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
        let key = format!("identity:suspend:{account_id}");
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
        let key = format!("identity:suspend:{account_id}");
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

//! Sanction issuance and enforcement with Redis caching.

use chrono::{DateTime, Utc};
use shared::AppResult;
use sqlx::{PgConnection, PgPool};

#[derive(Debug, Clone)]
pub struct AppealableSanctionTarget {
    pub account_id: i64,
    pub account_role: String,
    pub kind: String,
}

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

    let sanction_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sanctions (account_id, kind, reason, ends_at) \
         VALUES ($1, 'silence', $2, $3) RETURNING id",
    )
    .bind(account_id)
    .bind(reason)
    .bind(ends_at)
    .fetch_one(&mut *connection)
    .await?;
    let mut metadata = audit_metadata.cloned().unwrap_or_else(|| serde_json::json!({}));
    metadata
        .as_object_mut()
        .ok_or_else(|| shared::AppError::BadRequest("audit metadata must be an object".into()))?
        .insert("sanctionId".into(), serde_json::json!(sanction_id.to_string()));
    let event_id = governance::record_system_event_with_id_tx(
        connection,
        "identity.sanction.auto_silence",
        "account",
        &account_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    governance::notices::create_notice_tx(
        connection,
        account_id,
        "sanction_applied",
        &format!("audit:{event_id}:sanction"),
        Some(event_id),
        None,
        "sanction",
        &sanction_id.to_string(),
        "账号已被自动禁言，可在申诉中心查看并申请复核。",
    )
    .await?;
    Ok(true)
}

/// Lock and validate an identity sanction as an appeal target owned by the appellant.
pub async fn inspect_appealable_sanction_tx(
    connection: &mut PgConnection,
    sanction_id: i64,
    appellant_account_id: i64,
) -> AppResult<AppealableSanctionTarget> {
    let target = sqlx::query_as::<_, (i64, String, String, Option<chrono::DateTime<Utc>>)>(
        "SELECT sanction.account_id, account.role::text, sanction.kind, sanction.revoked_at \
         FROM identity.sanctions sanction \
         JOIN identity.accounts account ON account.id = sanction.account_id \
         WHERE sanction.id = $1 FOR SHARE OF sanction, account",
    )
    .bind(sanction_id)
    .fetch_optional(connection)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    if target.0 != appellant_account_id {
        return Err(shared::AppError::NotFound);
    }
    if target.3.is_some() {
        return Err(shared::AppError::Conflict("the sanction is no longer active".into()));
    }
    Ok(AppealableSanctionTarget { account_id: target.0, account_role: target.1, kind: target.2 })
}

/// Revoke the exact sanction under appeal without changing its immutable issuance record.
pub async fn overturn_sanction_for_appeal_tx(
    connection: &mut PgConnection,
    sanction_id: i64,
    appellant_account_id: i64,
    reviewer_account_id: i64,
) -> AppResult<String> {
    let changed = sqlx::query(
        "UPDATE identity.sanctions SET revoked_at = now(), revoked_by = $1 \
         WHERE id = $2 AND account_id = $3 AND revoked_at IS NULL",
    )
    .bind(reviewer_account_id)
    .bind(sanction_id)
    .bind(appellant_account_id)
    .execute(&mut *connection)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(shared::AppError::Conflict(
            "the sanction changed before the appeal decision".into(),
        ));
    }
    let kind = sqlx::query_scalar("SELECT kind FROM identity.sanctions WHERE id = $1")
        .bind(sanction_id)
        .fetch_one(&mut *connection)
        .await?;
    Ok(kind)
}

/// Shorten an active sanction. Appeal amendments can never extend a restriction.
pub async fn amend_sanction_for_appeal_tx(
    connection: &mut PgConnection,
    sanction_id: i64,
    appellant_account_id: i64,
    amended_ends_at: DateTime<Utc>,
) -> AppResult<String> {
    if amended_ends_at <= Utc::now() {
        return Err(shared::AppError::BadRequest(
            "amendedEndsAt must be in the future; use overturned to end immediately".into(),
        ));
    }
    let changed = sqlx::query(
        "UPDATE identity.sanctions SET ends_at = $1 \
         WHERE id = $2 AND account_id = $3 AND revoked_at IS NULL \
           AND (ends_at IS NULL OR ends_at > $1)",
    )
    .bind(amended_ends_at)
    .bind(sanction_id)
    .bind(appellant_account_id)
    .execute(&mut *connection)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(shared::AppError::Conflict(
            "amendment must shorten the active sanction".into(),
        ));
    }
    let kind = sqlx::query_scalar("SELECT kind FROM identity.sanctions WHERE id = $1")
        .bind(sanction_id)
        .fetch_one(&mut *connection)
        .await?;
    Ok(kind)
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

/// Invalidate both sanction-enforcement cache keys after an appeal commits.
pub async fn invalidate_sanction_caches(redis: Option<&deadpool_redis::Pool>, account_id: i64) {
    let Some(redis) = redis else {
        return;
    };
    let Ok(mut connection) = redis.get().await else {
        tracing::warn!(account_id, "failed to acquire Redis connection for sanction invalidation");
        return;
    };
    let result: redis::RedisResult<()> = redis::cmd("DEL")
        .arg(format!("identity:silence:{account_id}"))
        .arg(format!("identity:suspend:{account_id}"))
        .query_async(&mut connection)
        .await;
    if let Err(error) = result {
        tracing::warn!(account_id, ?error, "failed to invalidate sanction caches");
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

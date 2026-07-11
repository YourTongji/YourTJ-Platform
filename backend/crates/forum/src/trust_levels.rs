//! Trust level computation and daily promotion/demotion task.
//!
//! The current thresholds and known promotion/demotion policy mismatches are documented in
//! `docs/product/trust-safety-and-administration.md`. Keep that product inventory synchronized
//! when this scheduler changes.

use sqlx::PgPool;

use shared::AppResult;

/// Run the daily trust level promotion scan.
/// Returns (promoted_count, demoted_count) for logging.
#[allow(dead_code)]
pub async fn run_daily_tl_promotion(pool: &PgPool) -> (i64, i64) {
    let promoted_tl1 = promote_tl(
        pool,
        0,
        1,
        &[
            "EXTRACT(EPOCH FROM (now() - created_at)) / 86400 >= 2",
            "(SELECT COALESCE(threads_created + comments_created, 0) \
             FROM forum.user_stats WHERE account_id = a.id) >= 3",
            "(SELECT COALESCE(COUNT(*), 0) FROM forum.thread_reads tr \
             WHERE tr.account_id = a.id) >= 10",
        ],
    )
    .await;

    let promoted_tl2 = promote_tl(
        pool,
        1,
        2,
        &[
            "EXTRACT(EPOCH FROM (now() - created_at)) / 86400 >= 15",
            "(SELECT COALESCE(votes_received, 0) \
         FROM forum.user_stats WHERE account_id = a.id) >= 10",
            "(SELECT COALESCE(flagged_upheld, 0) \
         FROM forum.user_stats WHERE account_id = a.id) = 0",
        ],
    )
    .await;

    let promoted_tl3 = promote_tl(
        pool,
        2,
        3,
        &[
            "EXTRACT(EPOCH FROM (now() - created_at)) / 86400 >= 60",
            "(SELECT COALESCE(votes_received, 0) \
         FROM forum.user_stats WHERE account_id = a.id) >= 50",
            "(SELECT COALESCE(flags_upheld, 0) \
         FROM forum.user_stats WHERE account_id = a.id) >= 3",
        ],
    )
    .await;

    let demoted = demote_tl(pool).await;

    (promoted_tl1 + promoted_tl2 + promoted_tl3, demoted)
}

#[allow(dead_code)]
async fn promote_tl(pool: &PgPool, from: i16, to: i16, conditions: &[&str]) -> i64 {
    let where_clause = conditions.join(" AND ");
    let sql = format!(
        "UPDATE identity.accounts a SET trust_level = $1 \
         WHERE a.trust_level = $2 AND {}",
        where_clause
    );
    match sqlx::query(&sql).bind(to).bind(from).execute(pool).await {
        Ok(r) => r.rows_affected() as i64,
        Err(e) => {
            tracing::warn!(error = %e, from, to, "TL promotion query failed");
            0
        }
    }
}

#[allow(dead_code)]
async fn demote_tl(pool: &PgPool) -> i64 {
    let tl3_count = match sqlx::query(
        "UPDATE identity.accounts a SET trust_level = 2 \
         WHERE a.trust_level = 3 \
         AND (SELECT COALESCE(flagged_upheld, 0) \
              FROM forum.user_stats WHERE account_id = a.id) > 0",
    )
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected() as i64,
        Err(e) => {
            tracing::warn!(error = %e, "TL3 demotion query failed");
            0
        }
    };

    let tl2_count = match sqlx::query(
        "UPDATE identity.accounts a SET trust_level = 1 \
         WHERE a.trust_level = 2 \
         AND (SELECT COALESCE(flagged_upheld, 0) \
              FROM forum.user_stats WHERE account_id = a.id) > 0",
    )
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected() as i64,
        Err(e) => {
            tracing::warn!(error = %e, "TL2 demotion query failed");
            0
        }
    };

    tl3_count + tl2_count
}

/// Look up account trust level with Redis cache (60s).
#[allow(dead_code)]
pub async fn get_trust_level(
    redis: Option<&deadpool_redis::Pool>,
    pool: &PgPool,
    account_id: i64,
) -> AppResult<i16> {
    // Try cache first
    if let Some(r) = redis {
        let key = format!("identity:tl:{account_id}");
        let mut conn = r.get().await.map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
        if let Ok(Some(val)) =
            redis::cmd("GET").arg(&key).query_async::<Option<String>>(&mut conn).await
        {
            if let Ok(tl) = val.parse::<i16>() {
                return Ok(tl);
            }
        }
    }

    // Fallback to DB. `trust_level` is smallint, but `COALESCE(trust_level, 0)`
    // unifies with the int4 literal `0` and yields int4, which does not decode
    // into i16 — cast the result back to smallint.
    let tl: i16 = sqlx::query_scalar(
        "SELECT COALESCE(trust_level, 0)::smallint FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    // Cache result
    if let Some(r) = redis {
        let key = format!("identity:tl:{account_id}");
        let mut conn = r.get().await.map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(60)
            .arg(tl.to_string())
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    Ok(tl)
}

//! Trust-level reads and the daily one-step promotion/demotion policy.

use sqlx::PgPool;

use shared::AppResult;

/// Apply at most one trust-level transition per active account.
///
/// Active days are distinct UTC days with a forum thread, comment, vote, or
/// tracked read. Demotion takes priority over promotion for the same scan.
pub async fn run_daily_tl_promotion(pool: &PgPool) -> (i64, i64) {
    let result = sqlx::query_as::<_, (i64, i64)>(
        "WITH account_metrics AS ( \
           SELECT account.id, account.trust_level, account.created_at, \
                  COALESCE(stats.threads_created, 0) AS threads_created, \
                  COALESCE(stats.comments_created, 0) AS comments_created, \
                  COALESCE(stats.votes_received, 0) AS votes_received, \
                  COALESCE(stats.flags_upheld, 0) AS flags_upheld, \
                  COALESCE(stats.flagged_upheld, 0) AS flagged_upheld, \
                  (SELECT COUNT(*)::int FROM forum.thread_reads thread_read \
                   WHERE thread_read.account_id = account.id) AS topics_read, \
                  COALESCE(activity.active_days, 0) AS active_days, \
                  COALESCE(activity.active_days_60, 0) AS active_days_60 \
           FROM identity.accounts account \
           LEFT JOIN forum.user_stats stats ON stats.account_id = account.id \
           LEFT JOIN LATERAL ( \
             SELECT COUNT(DISTINCT (event.occurred_at AT TIME ZONE 'UTC')::date)::int \
                      AS active_days, \
                    COUNT(DISTINCT (event.occurred_at AT TIME ZONE 'UTC')::date) \
                      FILTER (WHERE event.occurred_at >= now() - interval '60 days')::int \
                      AS active_days_60 \
             FROM ( \
               SELECT thread.created_at AS occurred_at FROM forum.threads thread \
               WHERE thread.author_id = account.id \
               UNION ALL \
               SELECT comment.created_at FROM forum.comments comment \
               WHERE comment.author_id = account.id \
               UNION ALL \
               SELECT vote.updated_at FROM forum.votes vote \
               WHERE vote.account_id = account.id \
               UNION ALL \
               SELECT thread_read.updated_at FROM forum.thread_reads thread_read \
               WHERE thread_read.account_id = account.id \
             ) event \
           ) activity ON TRUE \
           WHERE account.status = 'active' \
         ), decisions AS ( \
           SELECT id, \
                  CASE \
                    WHEN trust_level IN (2, 3) AND flagged_upheld > 0 \
                      THEN trust_level - 1 \
                    WHEN trust_level = 0 \
                      AND now() - created_at >= interval '2 days' \
                      AND threads_created + comments_created >= 3 \
                      AND topics_read >= 10 THEN 1 \
                    WHEN trust_level = 1 \
                      AND now() - created_at >= interval '15 days' \
                      AND active_days >= 8 \
                      AND votes_received >= 10 \
                      AND flagged_upheld = 0 THEN 2 \
                    WHEN trust_level = 2 \
                      AND now() - created_at >= interval '60 days' \
                      AND active_days_60 >= 20 \
                      AND votes_received >= 50 \
                      AND flags_upheld >= 3 THEN 3 \
                    ELSE trust_level \
                  END AS next_level, \
                  CASE \
                    WHEN trust_level IN (2, 3) AND flagged_upheld > 0 THEN 'demoted' \
                    ELSE 'promoted' \
                  END AS transition \
           FROM account_metrics \
         ), updated AS ( \
           UPDATE identity.accounts account SET trust_level = decision.next_level \
           FROM decisions decision \
           WHERE account.id = decision.id AND account.trust_level <> decision.next_level \
           RETURNING decision.transition \
         ) \
         SELECT COUNT(*) FILTER (WHERE transition = 'promoted')::bigint, \
                COUNT(*) FILTER (WHERE transition = 'demoted')::bigint \
         FROM updated",
    )
    .fetch_one(pool)
    .await;

    match result {
        Ok(counts) => counts,
        Err(error) => {
            tracing::warn!(%error, "trust-level policy scan failed");
            (0, 0)
        }
    }
}

/// Read the authoritative trust level used for posting and interaction policy.
pub async fn get_trust_level(pool: &PgPool, account_id: i64) -> AppResult<i16> {
    let trust_level: i16 = sqlx::query_scalar(
        "SELECT COALESCE(trust_level, 0)::smallint FROM identity.accounts \
         WHERE id = $1 AND status = 'active'",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    Ok(trust_level)
}

//! Write-maintained qualifying-score projection for trust evaluation.

use shared::AppResult;
use sqlx::PgConnection;

#[derive(Debug, sqlx::FromRow)]
struct ScorePolicy {
    score_policy_version: i64,
    trust_policy_version: i64,
    thread_weight: i32,
    comment_weight: i32,
    like_weight: i32,
    check_in_weight: i32,
    like_daily_cap: i32,
}

pub(crate) async fn lock_projection_shared(connection: &mut PgConnection) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtext('activity.score_projection'))")
        .execute(connection)
        .await?;
    Ok(())
}

pub(crate) async fn lock_projection_exclusive(connection: &mut PgConnection) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.score_projection'))")
        .execute(connection)
        .await?;
    Ok(())
}

pub(crate) async fn refresh_account(
    connection: &mut PgConnection,
    account_id: i64,
    activity_date: chrono::NaiveDate,
) -> AppResult<()> {
    let policy = load_current_policy(connection).await?;
    let counts: (i32, i32, i32, i32, i64) = sqlx::query_as(
        "SELECT threads_created, comments_created, likes_given, check_ins, score \
         FROM activity.daily_counts \
         WHERE account_id = $1 AND activity_date = $2 FOR UPDATE",
    )
    .bind(account_id)
    .bind(activity_date)
    .fetch_one(&mut *connection)
    .await?;
    let daily_score = score((counts.0, counts.1, counts.2, counts.3), &policy);
    let score_delta = daily_score - counts.4;
    sqlx::query(
        "UPDATE activity.daily_counts SET score = $3 \
         WHERE account_id = $1 AND activity_date = $2",
    )
    .bind(account_id)
    .bind(activity_date)
    .bind(daily_score)
    .execute(&mut *connection)
    .await?;
    let projected = sqlx::query_scalar::<_, bool>(
        "INSERT INTO activity.account_scores \
         (account_id, qualifying_score, score_policy_version, trust_policy_version) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (account_id) DO UPDATE \
         SET qualifying_score = activity.account_scores.qualifying_score + $5, \
             score_policy_version = EXCLUDED.score_policy_version, \
             trust_policy_version = EXCLUDED.trust_policy_version, updated_at = now() \
         WHERE activity.account_scores.score_policy_version = EXCLUDED.score_policy_version \
           AND activity.account_scores.trust_policy_version = EXCLUDED.trust_policy_version \
         RETURNING true",
    )
    .bind(account_id)
    .bind(daily_score)
    .bind(policy.score_policy_version)
    .bind(policy.trust_policy_version)
    .bind(score_delta)
    .fetch_optional(connection)
    .await?
    .unwrap_or(false);
    if !projected {
        return Err(shared::AppError::Internal(anyhow::anyhow!(
            "activity score projection policy is stale"
        )));
    }
    Ok(())
}

pub(crate) async fn reproject_all(
    connection: &mut PgConnection,
    trust_policy_version: i64,
) -> AppResult<()> {
    let policy = load_policy(connection, trust_policy_version).await?;
    sqlx::query(
        "UPDATE activity.daily_counts \
         SET score = ( \
           threads_created::bigint * $1 \
           + comments_created::bigint * $2 \
           + LEAST(likes_given::bigint * $3, $4::bigint) \
           + check_ins::bigint * $5 \
         )",
    )
    .bind(policy.thread_weight)
    .bind(policy.comment_weight)
    .bind(policy.like_weight)
    .bind(policy.like_daily_cap)
    .bind(policy.check_in_weight)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE activity.account_scores \
         SET qualifying_score = 0, score_policy_version = $1, trust_policy_version = $2, \
             updated_at = now()",
    )
    .bind(policy.score_policy_version)
    .bind(policy.trust_policy_version)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "INSERT INTO activity.account_scores \
         (account_id, qualifying_score, score_policy_version, trust_policy_version) \
         SELECT account_id, SUM(score)::bigint, $1, $2 \
         FROM activity.daily_counts GROUP BY account_id \
         ON CONFLICT (account_id) DO UPDATE \
         SET qualifying_score = EXCLUDED.qualifying_score, \
             score_policy_version = EXCLUDED.score_policy_version, \
             trust_policy_version = EXCLUDED.trust_policy_version, updated_at = now()",
    )
    .bind(policy.score_policy_version)
    .bind(policy.trust_policy_version)
    .execute(connection)
    .await?;
    Ok(())
}

async fn load_current_policy(connection: &mut PgConnection) -> AppResult<ScorePolicy> {
    let trust_policy_version: i64 = sqlx::query_scalar(
        "SELECT version FROM activity.trust_level_policies ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(&mut *connection)
    .await?;
    load_policy(connection, trust_policy_version).await
}

async fn load_policy(
    connection: &mut PgConnection,
    trust_policy_version: i64,
) -> AppResult<ScorePolicy> {
    Ok(sqlx::query_as(
        "SELECT score.version AS score_policy_version, \
                trust.version AS trust_policy_version, \
                score.thread_weight, score.comment_weight, \
                score.like_weight, score.check_in_weight, trust.like_daily_cap \
         FROM activity.trust_level_policies trust \
         INNER JOIN activity.score_policies score ON score.version = trust.score_policy_version \
         WHERE trust.version = $1",
    )
    .bind(trust_policy_version)
    .fetch_one(connection)
    .await?)
}

fn score(counts: (i32, i32, i32, i32), policy: &ScorePolicy) -> i64 {
    i64::from(counts.0) * i64::from(policy.thread_weight)
        + i64::from(counts.1) * i64::from(policy.comment_weight)
        + (i64::from(counts.2) * i64::from(policy.like_weight))
            .min(i64::from(policy.like_daily_cap))
        + i64::from(counts.3) * i64::from(policy.check_in_weight)
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn qualifying_score_applies_daily_like_cap_and_check_in_weight() {
        let policy = super::ScorePolicy {
            score_policy_version: 2,
            trust_policy_version: 3,
            thread_weight: 10,
            comment_weight: 3,
            like_weight: 2,
            check_in_weight: 4,
            like_daily_cap: 20,
        };
        assert_eq!(score((1, 2, 50, 1), &policy), 40);
    }
}

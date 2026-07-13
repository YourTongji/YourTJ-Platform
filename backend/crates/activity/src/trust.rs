//! Unified trust-level evaluation owned by the activity domain.
//!
//! Registered accounts persist levels 1–6. Level 0 is only a visitor UI state.
//! Automatic upgrades advance at most one level per evaluation and never
//! override a staff pin. Automatic demotions consume one unique governance
//! event id and lower the level by at most one.

use chrono::NaiveDate;
use shared::{AppError, AppResult, Page};
use sqlx::{PgConnection, PgPool};

use crate::dto::{
    TrustLevelAdjustInput, TrustLevelEventDto, TrustLevelPolicyDto, TrustLevelPolicyUpdateInput,
    TrustProgressDto,
};
use crate::models::{TrustLevelEventRow, TrustLevelPolicyRow, TrustProgressRow};

const TEA_NAMES: [&str; 7] = ["茶苗", "绿茶", "白茶", "黄茶", "青茶", "红茶", "黑茶"];
const TRUST_EVALUATION_BATCH_SIZE: i64 = 50;
const TRUST_EVALUATION_MAX_ATTEMPTS: i32 = 8;

/// Public tea display name for a trust level in the 0–6 range.
pub fn tea_name(level: i16) -> &'static str {
    TEA_NAMES[usize::try_from(level.clamp(0, 6)).unwrap_or(0)]
}

/// Ensure a registered account has progress and a projected identity trust level.
pub async fn ensure_registered_progress(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<i16> {
    if let Some(progress) = load_progress(connection, account_id).await? {
        project_identity_level(connection, account_id, effective_level(&progress)).await?;
        return Ok(effective_level(&progress));
    }

    let policy = current_policy_row_tx(connection).await?;
    let score = compute_qualifying_score_tx(connection, account_id, &policy).await?;
    // New progress always starts at the registered-account baseline. Existing
    // accounts are initialized from history only by the explicit migration;
    // lazy creation must not bypass the one-level-per-evaluation invariant.
    let level = 1_i16;
    sqlx::query(
        "INSERT INTO activity.account_trust_progress \
         (account_id, trust_level, qualifying_score, policy_version) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .bind(level)
    .bind(score)
    .bind(policy.version)
    .execute(&mut *connection)
    .await?;
    let _ = insert_event_tx(
        connection,
        account_id,
        "registration",
        0,
        level,
        score,
        policy.version,
        "system",
        None,
        Some("registered campus account starts at Lv.1"),
        None,
        &format!("trust:registration:{account_id}"),
    )
    .await?;
    project_identity_level(connection, account_id, level).await?;
    Ok(level)
}

/// Read the effective trust level used by permission checks.
pub async fn get_trust_level(pool: &PgPool, account_id: i64) -> AppResult<i16> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("activity.trust:{account_id}"))
        .execute(&mut *tx)
        .await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?;
    if status.as_deref() != Some("active") {
        return Ok(0);
    }
    let level = if let Some(progress) = load_progress(&mut tx, account_id).await? {
        effective_level(&progress)
    } else {
        ensure_registered_progress(&mut tx, account_id).await?
    };
    tx.commit().await?;
    Ok(level)
}

/// Current authenticated trust progress for the home growth card.
pub async fn trust_progress(pool: &PgPool, account_id: i64) -> AppResult<TrustProgressDto> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("activity.trust:{account_id}"))
        .execute(&mut *tx)
        .await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?;
    if status.as_deref() != Some("active") {
        return Err(AppError::NotFound);
    }
    let _ = ensure_registered_progress(&mut tx, account_id).await?;
    let policy = current_policy_row_tx(&mut tx).await?;
    let score = compute_qualifying_score_tx(&mut tx, account_id, &policy).await?;
    let progress = load_progress(&mut tx, account_id).await?.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("trust progress missing after ensure"))
    })?;
    let effective = if progress.override_level.is_some() {
        effective_level(&progress)
    } else {
        // Keep the stored automatic level unless a scan advances it later; progress
        // still shows the live qualifying score under the current policy.
        progress.trust_level
    };
    let result = progress_dto(effective, score, &policy, &progress);
    tx.commit().await?;
    Ok(result)
}

/// Apply automatic one-step upgrades for active non-overridden accounts.
pub async fn run_trust_evaluation(pool: &PgPool) -> (i64, i64) {
    let result = evaluate_all_with_policy_guard(pool, None).await;
    match result {
        Ok(counts) => counts,
        Err(error) => {
            tracing::warn!(?error, "trust evaluation failed");
            (0, 0)
        }
    }
}

/// Run at most one resumable trust evaluation for the current Shanghai day.
pub async fn run_scheduled_trust_evaluation(pool: &PgPool) -> (i64, i64) {
    match run_scheduled(pool).await {
        Ok(counts) => counts,
        Err(error) => {
            tracing::warn!(?error, "scheduled trust evaluation failed");
            (0, 0)
        }
    }
}

async fn run_scheduled(pool: &PgPool) -> AppResult<(i64, i64)> {
    let activity_date: NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Shanghai')::date")
            .fetch_one(pool)
            .await?;
    sqlx::query(
        "INSERT INTO activity.trust_evaluation_runs (activity_date, status) \
         VALUES ($1, 'queued') ON CONFLICT (activity_date) DO NOTHING",
    )
    .bind(activity_date)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE activity.trust_evaluation_runs \
         SET status = 'dead', lease_token = NULL, lease_expires_at = NULL, \
             error_code = 'attempts_exhausted', updated_at = now() \
         WHERE activity_date = $1 AND attempts >= $2 \
           AND (status IN ('queued', 'failed') \
                OR (status = 'running' AND lease_expires_at < now()))",
    )
    .bind(activity_date)
    .bind(TRUST_EVALUATION_MAX_ATTEMPTS)
    .execute(pool)
    .await?;
    let lease_token = uuid::Uuid::new_v4();
    let attempt = sqlx::query_scalar::<_, i32>(
        "UPDATE activity.trust_evaluation_runs \
         SET status = 'running', lease_token = $2, lease_expires_at = now() + interval '10 minutes', \
             attempts = attempts + 1, started_at = COALESCE(started_at, now()), \
             error_code = NULL, updated_at = now() \
         WHERE activity_date = $1 \
           AND attempts < $3 AND next_attempt_at <= now() \
           AND (status IN ('queued', 'failed') \
                OR (status = 'running' AND lease_expires_at < now())) \
         RETURNING attempts",
    )
    .bind(activity_date)
    .bind(lease_token)
    .bind(TRUST_EVALUATION_MAX_ATTEMPTS)
    .fetch_optional(pool)
    .await?;
    let Some(attempt) = attempt else {
        return Ok((0, 0));
    };

    match evaluate_scheduled_with_policy_guard(pool, activity_date, lease_token).await {
        Ok(upgraded) => {
            finalize_scheduled_attempt(pool, activity_date, lease_token, attempt, upgraded)
                .await
                .map(|()| (upgraded, 0))
        }
        Err(error) => {
            fail_scheduled_attempt(pool, activity_date, lease_token, attempt, "evaluation_failed")
                .await?;
            Err(error)
        }
    }
}

async fn evaluate_scheduled_with_policy_guard(
    pool: &PgPool,
    activity_date: NaiveDate,
    lease_token: uuid::Uuid,
) -> AppResult<i64> {
    let mut policy_guard = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.score_policy'))")
        .execute(&mut *policy_guard)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.trust_policy'))")
        .execute(&mut *policy_guard)
        .await?;
    let result = evaluate_scheduled_batches(pool, activity_date, lease_token).await;
    policy_guard.commit().await?;
    result
}

async fn evaluate_scheduled_batches(
    pool: &PgPool,
    activity_date: NaiveDate,
    lease_token: uuid::Uuid,
) -> AppResult<i64> {
    let mut upgraded_total = 0_i64;
    loop {
        let cursor: i64 = sqlx::query_scalar(
            "SELECT cursor_account_id FROM activity.trust_evaluation_runs \
             WHERE activity_date = $1 AND status = 'running' AND lease_token = $2 \
               AND lease_expires_at > now()",
        )
        .bind(activity_date)
        .bind(lease_token)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Conflict("trust evaluation lease was lost".into()))?;
        let account_ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM identity.accounts \
             WHERE status = 'active' AND id > $1 ORDER BY id LIMIT $2",
        )
        .bind(cursor)
        .bind(TRUST_EVALUATION_BATCH_SIZE)
        .fetch_all(pool)
        .await?;
        if account_ids.is_empty() {
            break;
        }

        let mut batch_upgraded = 0_i64;
        let mut last_account_id = cursor;
        for account_id in account_ids {
            last_account_id = account_id;
            let mut tx = pool.begin().await?;
            let active_lease: Option<uuid::Uuid> = sqlx::query_scalar(
                "SELECT lease_token FROM activity.trust_evaluation_runs \
                 WHERE activity_date = $1 AND status = 'running' AND lease_token = $2 \
                   AND lease_expires_at > now() FOR SHARE",
            )
            .bind(activity_date)
            .bind(lease_token)
            .fetch_optional(&mut *tx)
            .await?;
            if active_lease != Some(lease_token) {
                return Err(AppError::Conflict("trust evaluation lease was lost".into()));
            }
            match evaluate_account_tx(&mut tx, account_id, Some(activity_date)).await {
                Ok(was_upgraded) => {
                    sqlx::query(
                        "DELETE FROM activity.trust_evaluation_failures \
                         WHERE activity_date = $1 AND account_id = $2",
                    )
                    .bind(activity_date)
                    .bind(account_id)
                    .execute(&mut *tx)
                    .await?;
                    tx.commit().await?;
                    if was_upgraded {
                        batch_upgraded += 1;
                    }
                }
                Err(error) => {
                    tx.rollback().await?;
                    tracing::warn!(
                        account_id,
                        ?error,
                        error_code = "account_evaluation_failed",
                        "trust evaluation isolated an account failure"
                    );
                    sqlx::query(
                        "INSERT INTO activity.trust_evaluation_failures \
                         (activity_date, account_id, attempts, error_code) \
                         VALUES ($1, $2, 1, 'account_evaluation_failed') \
                         ON CONFLICT (activity_date, account_id) DO UPDATE \
                         SET attempts = LEAST(activity.trust_evaluation_failures.attempts + 1, 8), \
                             error_code = EXCLUDED.error_code, last_failed_at = now()",
                    )
                    .bind(activity_date)
                    .bind(account_id)
                    .execute(pool)
                    .await?;
                }
            }
        }
        let renewed = sqlx::query(
            "UPDATE activity.trust_evaluation_runs \
             SET cursor_account_id = $3, lease_expires_at = now() + interval '10 minutes', \
                 upgraded_count = upgraded_count + $4, updated_at = now() \
             WHERE activity_date = $1 AND status = 'running' AND lease_token = $2",
        )
        .bind(activity_date)
        .bind(lease_token)
        .bind(last_account_id)
        .bind(i32::try_from(batch_upgraded).unwrap_or(i32::MAX))
        .execute(pool)
        .await?
        .rows_affected();
        if renewed != 1 {
            return Err(AppError::Conflict("trust evaluation lease was lost".into()));
        }
        upgraded_total += batch_upgraded;
    }
    Ok(upgraded_total)
}

async fn finalize_scheduled_attempt(
    pool: &PgPool,
    activity_date: NaiveDate,
    lease_token: uuid::Uuid,
    attempt: i32,
    upgraded: i64,
) -> AppResult<()> {
    sqlx::query(
        "DELETE FROM activity.trust_evaluation_failures failure \
         WHERE failure.activity_date = $1 AND NOT EXISTS ( \
           SELECT 1 FROM identity.accounts account \
           WHERE account.id = failure.account_id AND account.status = 'active' \
         )",
    )
    .bind(activity_date)
    .execute(pool)
    .await?;
    let failed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity.trust_evaluation_failures WHERE activity_date = $1",
    )
    .bind(activity_date)
    .fetch_one(pool)
    .await?;
    if failed_count > 0 {
        tracing::warn!(
            activity_date = %activity_date,
            failed_count,
            attempt,
            "trust evaluation completed its scan with isolated account failures"
        );
        return fail_scheduled_attempt(
            pool,
            activity_date,
            lease_token,
            attempt,
            "account_evaluation_failed",
        )
        .await;
    }
    let completed = sqlx::query(
        "UPDATE activity.trust_evaluation_runs \
         SET status = 'completed', lease_token = NULL, lease_expires_at = NULL, \
             failed_count = 0, completed_at = now(), updated_at = now() \
         WHERE activity_date = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(activity_date)
    .bind(lease_token)
    .execute(pool)
    .await?
    .rows_affected();
    if completed != 1 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "trust evaluation lease was lost before completion"
        )));
    }
    tracing::info!(activity_date = %activity_date, upgraded, "trust evaluation completed");
    Ok(())
}

async fn fail_scheduled_attempt(
    pool: &PgPool,
    activity_date: NaiveDate,
    lease_token: uuid::Uuid,
    attempt: i32,
    error_code: &'static str,
) -> AppResult<()> {
    let failed_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity.trust_evaluation_failures WHERE activity_date = $1",
    )
    .bind(activity_date)
    .fetch_one(pool)
    .await?;
    let status = if attempt >= TRUST_EVALUATION_MAX_ATTEMPTS { "dead" } else { "failed" };
    let exponent = u32::try_from((attempt - 1).clamp(0, 7)).unwrap_or(0);
    let backoff_seconds = (30_i64 * 2_i64.pow(exponent)).min(3_600);
    let affected = sqlx::query(
        "UPDATE activity.trust_evaluation_runs \
         SET status = $3, lease_token = NULL, lease_expires_at = NULL, \
             cursor_account_id = CASE WHEN $3 = 'failed' THEN 0 ELSE cursor_account_id END, \
             failed_count = $4, error_code = $5, \
             next_attempt_at = now() + ($6::bigint * interval '1 second'), updated_at = now() \
         WHERE activity_date = $1 AND status = 'running' AND lease_token = $2",
    )
    .bind(activity_date)
    .bind(lease_token)
    .bind(status)
    .bind(i32::try_from(failed_count).unwrap_or(i32::MAX))
    .bind(error_code)
    .bind(backoff_seconds)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("trust evaluation lease was lost".into()));
    }
    Ok(())
}

async fn evaluate_all_with_policy_guard(
    pool: &PgPool,
    scheduled_evaluation_date: Option<NaiveDate>,
) -> AppResult<(i64, i64)> {
    let mut policy_guard = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.score_policy'))")
        .execute(&mut *policy_guard)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.trust_policy'))")
        .execute(&mut *policy_guard)
        .await?;
    let result = evaluate_all(pool, scheduled_evaluation_date).await;
    policy_guard.commit().await?;
    result
}

async fn evaluate_all(
    pool: &PgPool,
    scheduled_evaluation_date: Option<NaiveDate>,
) -> AppResult<(i64, i64)> {
    let account_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT account.id \
         FROM identity.accounts account \
         LEFT JOIN activity.account_trust_progress progress \
           ON progress.account_id = account.id \
         WHERE account.status = 'active' \
           AND (progress.override_level IS NULL OR progress.account_id IS NULL) \
         ORDER BY account.id",
    )
    .fetch_all(pool)
    .await?;

    let mut upgraded = 0_i64;
    for account_id in account_ids {
        let mut tx = pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("activity.trust:{account_id}"))
            .execute(&mut *tx)
            .await?;
        if evaluate_account_tx(&mut tx, account_id, scheduled_evaluation_date).await? {
            upgraded += 1;
        }
        tx.commit().await?;
    }
    Ok((upgraded, 0))
}

async fn evaluate_account_tx(
    connection: &mut PgConnection,
    account_id: i64,
    scheduled_evaluation_date: Option<NaiveDate>,
) -> AppResult<bool> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *connection)
            .await?;
    if status.as_deref() != Some("active") {
        return Ok(false);
    }

    let policy = current_policy_row_tx(connection).await?;
    let score = compute_qualifying_score_tx(connection, account_id, &policy).await?;
    let progress = match load_progress(connection, account_id).await? {
        Some(progress) => progress,
        None => {
            ensure_registered_progress(connection, account_id).await?;
            load_progress(connection, account_id)
                .await?
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("trust progress missing")))?
        }
    };
    if scheduled_evaluation_date.is_some()
        && progress.last_scheduled_evaluation_date == scheduled_evaluation_date
    {
        return Ok(false);
    }
    if progress.override_level.is_some() {
        sqlx::query(
            "UPDATE activity.account_trust_progress \
             SET qualifying_score = $2, policy_version = $3, \
                 last_scheduled_evaluation_date = COALESCE($4, last_scheduled_evaluation_date), \
                 last_evaluated_at = now(), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .bind(score)
        .bind(policy.version)
        .bind(scheduled_evaluation_date)
        .execute(&mut *connection)
        .await?;
        project_identity_level(connection, account_id, effective_level(&progress)).await?;
        return Ok(false);
    }

    let target = level_for_score(score, &policy).max(1);
    let cooldown_elapsed = match progress.promotion_blocked_until {
        Some(blocked_until) => blocked_until <= chrono::Utc::now(),
        None => true,
    };
    let has_new_activity = match progress.promotion_score_floor {
        Some(floor) => score > floor,
        None => true,
    };
    let should_upgrade = target > progress.trust_level && cooldown_elapsed && has_new_activity;
    let next = if should_upgrade { progress.trust_level + 1 } else { progress.trust_level };
    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET trust_level = $2, qualifying_score = $3, policy_version = $4, \
             last_scheduled_evaluation_date = COALESCE($5, last_scheduled_evaluation_date), \
             promotion_blocked_until = CASE WHEN $6 THEN NULL ELSE promotion_blocked_until END, \
             promotion_score_floor = CASE WHEN $6 THEN NULL ELSE promotion_score_floor END, \
             last_evaluated_at = now(), updated_at = now() \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .bind(next)
    .bind(score)
    .bind(policy.version)
    .bind(scheduled_evaluation_date)
    .bind(should_upgrade)
    .execute(&mut *connection)
    .await?;
    project_identity_level(connection, account_id, next).await?;
    if next > progress.trust_level {
        let evaluation_key = scheduled_evaluation_date
            .map_or_else(|| "manual".to_owned(), |activity_date| activity_date.to_string());
        let event_key = format!(
            "trust:upgrade:{account_id}:{}:{}:{evaluation_key}:{}",
            progress.trust_level,
            next,
            uuid::Uuid::new_v4()
        );
        let inserted = insert_event_tx(
            connection,
            account_id,
            "upgrade",
            progress.trust_level,
            next,
            score,
            policy.version,
            "system",
            None,
            Some("automatic activity trust upgrade"),
            None,
            &event_key,
        )
        .await?;
        if inserted {
            platform::outbox::enqueue_notification_tx(
                connection,
                &format!("notification:{event_key}"),
                account_id,
                None,
                "trust_level_upgraded",
                &serde_json::json!({
                    "fromLevel": progress.trust_level,
                    "toLevel": next,
                    "teaName": tea_name(next),
                    "targetUrl": "/",
                }),
                None,
                None,
            )
            .await?;
        }
        return Ok(inserted);
    }
    Ok(false)
}

/// Consume one governance event into at most one demotion step.
pub async fn apply_governance_demotion_tx(
    connection: &mut PgConnection,
    account_id: i64,
    governance_event_id: i64,
    reason: &str,
) -> AppResult<bool> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("activity.trust:{account_id}"))
        .execute(&mut *connection)
        .await?;
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM activity.trust_level_events \
           WHERE event_kind = 'demotion' AND governance_event_id = $1 \
         )",
    )
    .bind(governance_event_id)
    .fetch_one(&mut *connection)
    .await?;
    if already {
        return Ok(false);
    }

    let _ = ensure_registered_progress(connection, account_id).await?;
    let policy = current_policy_row_tx(connection).await?;
    let score = compute_qualifying_score_tx(connection, account_id, &policy).await?;
    let progress = load_progress(connection, account_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("trust progress missing")))?;
    let current = effective_level(&progress);
    sqlx::query(
        "UPDATE activity.account_trust_progress \
         SET qualifying_score = $2, policy_version = $3, \
             promotion_blocked_until = GREATEST( \
               COALESCE(promotion_blocked_until, now()), \
               now() + ($4::int * interval '1 day') \
             ), \
             promotion_score_floor = GREATEST(COALESCE(promotion_score_floor, 0), $2), \
             last_evaluated_at = now(), updated_at = now() \
         WHERE account_id = $1",
    )
    .bind(account_id)
    .bind(score)
    .bind(policy.version)
    .bind(policy.demotion_cooldown_days)
    .execute(&mut *connection)
    .await?;
    if current <= 1 {
        let _ = insert_event_tx(
            connection,
            account_id,
            "demotion",
            current,
            current,
            score,
            policy.version,
            "system",
            None,
            Some(reason),
            Some(governance_event_id),
            &format!("trust:demotion:gov:{governance_event_id}"),
        )
        .await?;
        return Ok(false);
    }
    let next = current - 1;
    if progress.override_level.is_some() {
        sqlx::query(
            "UPDATE activity.account_trust_progress \
             SET override_level = $2, trust_level = LEAST(trust_level, $2), \
                 qualifying_score = $3, policy_version = $4, \
                 last_evaluated_at = now(), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .bind(next)
        .bind(score)
        .bind(policy.version)
        .execute(&mut *connection)
        .await?;
    } else {
        sqlx::query(
            "UPDATE activity.account_trust_progress \
             SET trust_level = $2, qualifying_score = $3, policy_version = $4, \
                 last_evaluated_at = now(), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .bind(next)
        .bind(score)
        .bind(policy.version)
        .execute(&mut *connection)
        .await?;
    }
    project_identity_level(connection, account_id, next).await?;
    insert_event_tx(
        connection,
        account_id,
        "demotion",
        current,
        next,
        score,
        policy.version,
        "system",
        None,
        Some(reason),
        Some(governance_event_id),
        &format!("trust:demotion:gov:{governance_event_id}"),
    )
    .await?;
    Ok(true)
}

/// Publish a new versioned trust threshold policy.
pub async fn current_policy(pool: &PgPool) -> AppResult<TrustLevelPolicyDto> {
    let mut connection = pool.acquire().await?;
    Ok(policy_to_dto(current_policy_row_tx(&mut connection).await?))
}

/// Append a trust threshold policy revision.
pub async fn append_policy(
    pool: &PgPool,
    input: &TrustLevelPolicyUpdateInput,
    changed_by: i64,
    changed_by_role: &str,
) -> AppResult<TrustLevelPolicyDto> {
    validate_policy_thresholds(input)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.score_policy'))")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.trust_policy'))")
        .execute(&mut *tx)
        .await?;
    crate::score_projection::lock_projection_exclusive(&mut tx).await?;
    let current_version: i64 = sqlx::query_scalar(
        "SELECT version FROM activity.trust_level_policies ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await?;
    if current_version != input.expected_version {
        return Err(AppError::Conflict("trust policy version changed".into()));
    }
    let score_policy_version: i64 = sqlx::query_scalar(
        "SELECT version FROM activity.score_policies ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await?;
    let row = sqlx::query_as::<_, TrustLevelPolicyRow>(
        "INSERT INTO activity.trust_level_policies \
         (score_policy_version, threshold_level_2, threshold_level_3, threshold_level_4, \
          threshold_level_5, threshold_level_6, like_daily_cap, demotion_cooldown_days, \
          reason, changed_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING version, score_policy_version, threshold_level_2, threshold_level_3, \
                   threshold_level_4, threshold_level_5, threshold_level_6, like_daily_cap, \
                   demotion_cooldown_days, reason, changed_by, created_at",
    )
    .bind(score_policy_version)
    .bind(input.threshold_level_2)
    .bind(input.threshold_level_3)
    .bind(input.threshold_level_4)
    .bind(input.threshold_level_5)
    .bind(input.threshold_level_6)
    .bind(input.like_daily_cap)
    .bind(input.demotion_cooldown_days)
    .bind(input.reason.trim())
    .bind(changed_by)
    .fetch_one(&mut *tx)
    .await?;
    crate::score_projection::reproject_all(&mut tx, row.version).await?;
    let metadata = serde_json::json!({
        "expectedVersion": input.expected_version,
        "thresholds": {
            "level2": input.threshold_level_2,
            "level3": input.threshold_level_3,
            "level4": input.threshold_level_4,
            "level5": input.threshold_level_5,
            "level6": input.threshold_level_6,
        },
        "likeDailyCap": input.like_daily_cap,
        "demotionCooldownDays": input.demotion_cooldown_days,
        "scorePolicyVersion": score_policy_version,
    });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: changed_by, role: changed_by_role },
        "activity.trust_policy.published",
        "trust_policy",
        &row.version.to_string(),
        input.reason.trim(),
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(policy_to_dto(row))
}

/// Cursor history of trust threshold policies.
pub async fn policy_history(
    pool: &PgPool,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<TrustLevelPolicyDto>> {
    let cursor_version = cursor.unwrap_or(i64::MAX);
    let fetch_limit = limit.clamp(1, 100) + 1;
    let rows = sqlx::query_as::<_, TrustLevelPolicyRow>(
        "SELECT version, score_policy_version, threshold_level_2, threshold_level_3, \
                threshold_level_4, threshold_level_5, threshold_level_6, like_daily_cap, \
                demotion_cooldown_days, reason, changed_by, created_at \
         FROM activity.trust_level_policies \
         WHERE version < $1 \
         ORDER BY version DESC \
         LIMIT $2",
    )
    .bind(cursor_version)
    .bind(fetch_limit)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() == fetch_limit as usize;
    let item_count = if has_more { rows.len() - 1 } else { rows.len() };
    let items: Vec<TrustLevelPolicyDto> =
        rows.into_iter().take(item_count).map(policy_to_dto).collect();
    let next_cursor =
        if has_more { items.last().map(|item| item.version.to_string()) } else { None };
    Ok(Page::new(items, next_cursor))
}

/// Staff pin or clear a registered account's trust level.
pub async fn adjust_trust_level(
    pool: &PgPool,
    account_id: i64,
    input: &TrustLevelAdjustInput,
    actor_id: i64,
    actor_role: &str,
) -> AppResult<TrustProgressDto> {
    let reason = input.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must contain 3 to 500 characters".into()));
    }
    if input.clear_override == input.trust_level.is_some() {
        return Err(AppError::BadRequest(
            "provide exactly one of trustLevel or clearOverride".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("activity.trust:{account_id}"))
        .execute(&mut *tx)
        .await?;
    let target =
        identity::public_accounts::find_account_authorization_state_by_id(&mut tx, account_id)
            .await?;
    let Some(target) = target else {
        return Err(AppError::NotFound);
    };
    if target.status != "active" {
        return Err(AppError::NotFound);
    }
    if actor_id == account_id || role_rank(actor_role) <= role_rank(&target.role) {
        return Err(AppError::Forbidden);
    }
    let _ = ensure_registered_progress(&mut tx, account_id).await?;
    let policy = current_policy_row_tx(&mut tx).await?;
    let score = compute_qualifying_score_tx(&mut tx, account_id, &policy).await?;
    let progress = load_progress(&mut tx, account_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("trust progress missing")))?;
    let from = effective_level(&progress);

    let (event_kind, to_level) = if input.clear_override {
        let automatic = level_for_score(score, &policy).max(1);
        sqlx::query(
            "UPDATE activity.account_trust_progress \
             SET trust_level = $2, qualifying_score = $3, policy_version = $4, \
                 override_level = NULL, override_reason = NULL, override_by = NULL, \
                 override_at = NULL, last_evaluated_at = now(), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .bind(automatic)
        .bind(score)
        .bind(policy.version)
        .execute(&mut *tx)
        .await?;
        ("override_clear", automatic)
    } else {
        let level = input.trust_level.ok_or_else(|| {
            AppError::BadRequest("trustLevel is required unless clearOverride is true".into())
        })?;
        if !(1..=6).contains(&level) {
            return Err(AppError::BadRequest("trustLevel must be between 1 and 6".into()));
        }
        sqlx::query(
            "UPDATE activity.account_trust_progress \
             SET trust_level = $2, qualifying_score = $3, policy_version = $4, \
                 override_level = $2, override_reason = $5, override_by = $6, \
                 override_at = now(), last_evaluated_at = now(), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .bind(level)
        .bind(score)
        .bind(policy.version)
        .bind(reason)
        .bind(actor_id)
        .execute(&mut *tx)
        .await?;
        ("manual_set", level)
    };
    project_identity_level(&mut tx, account_id, to_level).await?;
    insert_event_tx(
        &mut tx,
        account_id,
        event_kind,
        from,
        to_level,
        score,
        policy.version,
        "account",
        Some(actor_id),
        Some(reason),
        None,
        &format!(
            "trust:{event_kind}:{account_id}:{}:{}:{}",
            to_level,
            policy.version,
            chrono::Utc::now().timestamp_micros()
        ),
    )
    .await?;
    let metadata = serde_json::json!({
        "fromLevel": from,
        "toLevel": to_level,
        "clearOverride": input.clear_override,
        "qualifyingScore": score,
        "policyVersion": policy.version,
    });
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: actor_id, role: actor_role },
        if input.clear_override {
            "activity.trust.override_cleared"
        } else {
            "activity.trust.manual_set"
        },
        "account",
        &account_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    trust_progress(pool, account_id).await
}

fn role_rank(role: &str) -> u8 {
    match role {
        "admin" => 2,
        "mod" => 1,
        _ => 0,
    }
}

/// Append-only trust history for one account.
pub async fn event_history(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<TrustLevelEventDto>> {
    let cursor_id = cursor.unwrap_or(i64::MAX);
    let fetch_limit = limit.clamp(1, 100) + 1;
    let rows = sqlx::query_as::<_, TrustLevelEventRow>(
        "SELECT id, account_id, event_kind, from_level, to_level, qualifying_score, \
                policy_version, actor_kind, actor_account_id, reason, governance_event_id, \
                created_at \
         FROM activity.trust_level_events \
         WHERE account_id = $1 AND id < $2 \
         ORDER BY id DESC \
         LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor_id)
    .bind(fetch_limit)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() == fetch_limit as usize;
    let item_count = if has_more { rows.len() - 1 } else { rows.len() };
    let items: Vec<TrustLevelEventDto> =
        rows.into_iter().take(item_count).map(event_to_dto).collect();
    let next_cursor = if has_more { items.last().map(|item| item.id.clone()) } else { None };
    Ok(Page::new(items, next_cursor))
}

fn effective_level(progress: &TrustProgressRow) -> i16 {
    progress.override_level.unwrap_or(progress.trust_level)
}

fn level_for_score(score: i64, policy: &TrustLevelPolicyRow) -> i16 {
    if score >= i64::from(policy.threshold_level_6) {
        6
    } else if score >= i64::from(policy.threshold_level_5) {
        5
    } else if score >= i64::from(policy.threshold_level_4) {
        4
    } else if score >= i64::from(policy.threshold_level_3) {
        3
    } else if score >= i64::from(policy.threshold_level_2) {
        2
    } else {
        1
    }
}

fn progress_dto(
    level: i16,
    score: i64,
    policy: &TrustLevelPolicyRow,
    progress: &TrustProgressRow,
) -> TrustProgressDto {
    let next_level = if level >= 6 { None } else { Some(level + 1) };
    let next_threshold = next_level.map(|target| threshold_for(target, policy));
    let remaining = next_threshold.map(|threshold| (i64::from(threshold) - score).max(0));
    let percent = match (next_level, next_threshold) {
        (Some(target), Some(threshold)) => {
            let previous = if target <= 2 { 0 } else { threshold_for(target - 1, policy) };
            let span = (threshold - previous).max(1);
            let gained = (score - i64::from(previous)).clamp(0, i64::from(span));
            ((gained * 100) / i64::from(span)) as i32
        }
        _ => 100,
    };
    TrustProgressDto {
        trust_level: level,
        tea_name: tea_name(level).into(),
        qualifying_score: score,
        next_level,
        next_threshold,
        remaining_score: remaining,
        progress_percent: percent,
        policy_version: policy.version,
        is_max_level: level >= 6,
        override_active: progress.override_level.is_some(),
        promotion_blocked_until: progress.promotion_blocked_until.map(|value| value.timestamp()),
        promotion_requires_new_activity: progress
            .promotion_score_floor
            .is_some_and(|floor| score <= floor),
    }
}

fn threshold_for(level: i16, policy: &TrustLevelPolicyRow) -> i32 {
    match level {
        2 => policy.threshold_level_2,
        3 => policy.threshold_level_3,
        4 => policy.threshold_level_4,
        5 => policy.threshold_level_5,
        6 => policy.threshold_level_6,
        _ => 0,
    }
}

fn policy_to_dto(row: TrustLevelPolicyRow) -> TrustLevelPolicyDto {
    TrustLevelPolicyDto {
        version: row.version,
        score_policy_version: row.score_policy_version,
        threshold_level_2: row.threshold_level_2,
        threshold_level_3: row.threshold_level_3,
        threshold_level_4: row.threshold_level_4,
        threshold_level_5: row.threshold_level_5,
        threshold_level_6: row.threshold_level_6,
        like_daily_cap: row.like_daily_cap,
        demotion_cooldown_days: row.demotion_cooldown_days,
        reason: row.reason,
        changed_by: row.changed_by.map_or_else(|| "system".into(), |id| id.to_string()),
        created_at: row.created_at.timestamp(),
    }
}

fn event_to_dto(row: TrustLevelEventRow) -> TrustLevelEventDto {
    TrustLevelEventDto {
        id: row.id.to_string(),
        account_id: row.account_id.to_string(),
        event_kind: row.event_kind,
        from_level: row.from_level,
        to_level: row.to_level,
        qualifying_score: row.qualifying_score,
        policy_version: row.policy_version,
        actor_kind: row.actor_kind,
        actor_account_id: row.actor_account_id.map(|id| id.to_string()),
        reason: row.reason,
        governance_event_id: row.governance_event_id.map(|id| id.to_string()),
        created_at: row.created_at.timestamp(),
    }
}

fn validate_policy_thresholds(input: &TrustLevelPolicyUpdateInput) -> AppResult<()> {
    if input.threshold_level_2 <= 0
        || input.threshold_level_3 <= input.threshold_level_2
        || input.threshold_level_4 <= input.threshold_level_3
        || input.threshold_level_5 <= input.threshold_level_4
        || input.threshold_level_6 <= input.threshold_level_5
    {
        return Err(AppError::BadRequest(
            "trust thresholds must be strictly increasing positive integers".into(),
        ));
    }
    if !(0..=100_000).contains(&input.like_daily_cap) {
        return Err(AppError::BadRequest("likeDailyCap must be between 0 and 100000".into()));
    }
    if !(0..=365).contains(&input.demotion_cooldown_days) {
        return Err(AppError::BadRequest("demotionCooldownDays must be between 0 and 365".into()));
    }
    let reason = input.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must contain 3 to 500 characters".into()));
    }
    Ok(())
}

async fn load_progress(
    connection: &mut PgConnection,
    account_id: i64,
) -> AppResult<Option<TrustProgressRow>> {
    let row = sqlx::query_as::<_, TrustProgressRow>(
        "SELECT account_id, trust_level, qualifying_score, policy_version, override_level, \
                override_reason, override_by, override_at, promotion_blocked_until, \
                promotion_score_floor, last_evaluated_at, last_scheduled_evaluation_date, \
                updated_at \
         FROM activity.account_trust_progress WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?;
    Ok(row)
}

async fn current_policy_row_tx(connection: &mut PgConnection) -> AppResult<TrustLevelPolicyRow> {
    sqlx::query_as::<_, TrustLevelPolicyRow>(
        "SELECT version, score_policy_version, threshold_level_2, threshold_level_3, \
                threshold_level_4, threshold_level_5, threshold_level_6, like_daily_cap, \
                demotion_cooldown_days, reason, changed_by, created_at \
         FROM activity.trust_level_policies \
         ORDER BY version DESC LIMIT 1",
    )
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("trust policy is not seeded")))
}

async fn compute_qualifying_score_tx(
    connection: &mut PgConnection,
    account_id: i64,
    policy: &TrustLevelPolicyRow,
) -> AppResult<i64> {
    let projection: Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT qualifying_score, score_policy_version, trust_policy_version \
         FROM activity.account_scores WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(connection)
    .await?;
    match projection {
        None => Ok(0),
        Some((score, score_policy_version, trust_policy_version))
            if score_policy_version == policy.score_policy_version
                && trust_policy_version == policy.version =>
        {
            Ok(score)
        }
        Some(_) => {
            Err(AppError::Internal(anyhow::anyhow!("activity score projection policy is stale")))
        }
    }
}

async fn project_identity_level(
    connection: &mut PgConnection,
    account_id: i64,
    level: i16,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE identity.accounts SET trust_level = $2, updated_at = now() \
         WHERE id = $1 AND status <> 'purged' AND trust_level IS DISTINCT FROM $2",
    )
    .bind(account_id)
    .bind(level)
    .execute(connection)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)] // reason: explicit event fields avoid silent defaults for audit history
async fn insert_event_tx(
    connection: &mut PgConnection,
    account_id: i64,
    event_kind: &str,
    from_level: i16,
    to_level: i16,
    qualifying_score: i64,
    policy_version: i64,
    actor_kind: &str,
    actor_account_id: Option<i64>,
    reason: Option<&str>,
    governance_event_id: Option<i64>,
    event_key: &str,
) -> AppResult<bool> {
    let result = sqlx::query(
        "INSERT INTO activity.trust_level_events \
         (account_id, event_kind, from_level, to_level, qualifying_score, policy_version, \
          actor_kind, actor_account_id, reason, governance_event_id, event_key) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         ON CONFLICT (event_key) DO NOTHING",
    )
    .bind(account_id)
    .bind(event_kind)
    .bind(from_level)
    .bind(to_level)
    .bind(qualifying_score)
    .bind(policy_version)
    .bind(actor_kind)
    .bind(actor_account_id)
    .bind(reason)
    .bind(governance_event_id)
    .bind(event_key)
    .execute(connection)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::{level_for_score, tea_name};
    use crate::models::TrustLevelPolicyRow;
    use chrono::Utc;

    fn sample_policy() -> TrustLevelPolicyRow {
        TrustLevelPolicyRow {
            version: 1,
            score_policy_version: 1,
            threshold_level_2: 30,
            threshold_level_3: 120,
            threshold_level_4: 400,
            threshold_level_5: 1200,
            threshold_level_6: 3000,
            like_daily_cap: 20,
            demotion_cooldown_days: 7,
            reason: "test".into(),
            changed_by: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn maps_thresholds_without_skipping() {
        let policy = sample_policy();
        assert_eq!(level_for_score(0, &policy), 1);
        assert_eq!(level_for_score(29, &policy), 1);
        assert_eq!(level_for_score(30, &policy), 2);
        assert_eq!(level_for_score(3000, &policy), 6);
    }

    #[test]
    fn tea_names_cover_full_range() {
        assert_eq!(tea_name(0), "茶苗");
        assert_eq!(tea_name(1), "绿茶");
        assert_eq!(tea_name(6), "黑茶");
    }
}

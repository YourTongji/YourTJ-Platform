//! Unified trust-level evaluation owned by the activity domain.
//!
//! Registered accounts persist levels 1–6. Level 0 is only a visitor UI state.
//! Automatic upgrades advance at most one level per evaluation and never
//! override a staff pin. Automatic demotions consume one unique governance
//! event id and lower the level by at most one.

use shared::{AppError, AppResult, Page};
use sqlx::{PgConnection, PgPool};

use crate::dto::{
    TrustLevelAdjustInput, TrustLevelEventDto, TrustLevelPolicyDto, TrustLevelPolicyUpdateInput,
    TrustProgressDto,
};
use crate::models::{TrustLevelEventRow, TrustLevelPolicyRow, TrustProgressRow};

const TEA_NAMES: [&str; 7] = ["茶苗", "绿茶", "白茶", "黄茶", "青茶", "红茶", "黑茶"];

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
    let level = level_for_score(score, &policy).max(1);
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
    let mut connection = pool.acquire().await?;
    if let Some(progress) = load_progress(&mut connection, account_id).await? {
        return Ok(effective_level(&progress));
    }
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status::text FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(&mut *connection)
    .await?;
    match status.as_deref() {
        Some("active") => ensure_registered_progress(&mut connection, account_id).await,
        _ => Ok(0),
    }
}

/// Current authenticated trust progress for the home growth card.
pub async fn trust_progress(pool: &PgPool, account_id: i64) -> AppResult<TrustProgressDto> {
    let mut connection = pool.acquire().await?;
    let _ = ensure_registered_progress(&mut connection, account_id).await?;
    let policy = current_policy_row_tx(&mut connection).await?;
    let score = compute_qualifying_score_tx(&mut connection, account_id, &policy).await?;
    let progress = load_progress(&mut connection, account_id)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("trust progress missing after ensure")))?;
    let effective = if progress.override_level.is_some() {
        effective_level(&progress)
    } else {
        // Keep the stored automatic level unless a scan advances it later; progress
        // still shows the live qualifying score under the current policy.
        progress.trust_level
    };
    Ok(progress_dto(effective, score, &policy, &progress))
}

/// Apply automatic one-step upgrades for active non-overridden accounts.
pub async fn run_trust_evaluation(pool: &PgPool) -> (i64, i64) {
    let result = evaluate_all(pool).await;
    match result {
        Ok(counts) => counts,
        Err(error) => {
            tracing::warn!(?error, "trust evaluation failed");
            (0, 0)
        }
    }
}

async fn evaluate_all(pool: &PgPool) -> AppResult<(i64, i64)> {
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
        if evaluate_account_tx(&mut tx, account_id).await? {
            upgraded += 1;
        }
        tx.commit().await?;
    }
    Ok((upgraded, 0))
}

async fn evaluate_account_tx(connection: &mut PgConnection, account_id: i64) -> AppResult<bool> {
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
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
    if progress.override_level.is_some() {
        sqlx::query(
            "UPDATE activity.account_trust_progress \
             SET qualifying_score = $2, policy_version = $3, \
                 last_evaluated_at = now(), updated_at = now() \
             WHERE account_id = $1",
        )
        .bind(account_id)
        .bind(score)
        .bind(policy.version)
        .execute(&mut *connection)
        .await?;
        project_identity_level(connection, account_id, effective_level(&progress)).await?;
        return Ok(false);
    }

    let target = level_for_score(score, &policy).max(1);
    let next = if target > progress.trust_level {
        progress.trust_level + 1
    } else {
        progress.trust_level
    };
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
    project_identity_level(connection, account_id, next).await?;
    if next > progress.trust_level {
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
            &format!(
                "trust:upgrade:{account_id}:{}:{}:{}",
                next, policy.version, score
            ),
        )
        .await?;
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
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('activity.trust_policy'))")
        .execute(&mut *tx)
        .await?;
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
          threshold_level_5, threshold_level_6, like_daily_cap, reason, changed_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING version, score_policy_version, threshold_level_2, threshold_level_3, \
                   threshold_level_4, threshold_level_5, threshold_level_6, like_daily_cap, \
                   reason, changed_by, created_at",
    )
    .bind(score_policy_version)
    .bind(input.threshold_level_2)
    .bind(input.threshold_level_3)
    .bind(input.threshold_level_4)
    .bind(input.threshold_level_5)
    .bind(input.threshold_level_6)
    .bind(input.like_daily_cap)
    .bind(input.reason.trim())
    .bind(changed_by)
    .fetch_one(&mut *tx)
    .await?;
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
                reason, changed_by, created_at \
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
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("activity.trust:{account_id}"))
        .execute(&mut *tx)
        .await?;
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status::text FROM identity.accounts WHERE id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    if status.as_deref() == Some("purged") || status.is_none() {
        return Err(AppError::NotFound);
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

/// Increment lifetime totals when a contribution activates.
pub async fn increment_totals_tx(
    connection: &mut PgConnection,
    account_id: i64,
    kind: crate::contributions::ActivityKind,
) -> AppResult<()> {
    let statement = match kind {
        crate::contributions::ActivityKind::Thread => {
            "INSERT INTO activity.account_totals (account_id, threads_created) \
             VALUES ($1, 1) \
             ON CONFLICT (account_id) DO UPDATE \
             SET threads_created = activity.account_totals.threads_created + 1, \
                 updated_at = now()"
        }
        crate::contributions::ActivityKind::Comment => {
            "INSERT INTO activity.account_totals (account_id, comments_created) \
             VALUES ($1, 1) \
             ON CONFLICT (account_id) DO UPDATE \
             SET comments_created = activity.account_totals.comments_created + 1, \
                 updated_at = now()"
        }
        crate::contributions::ActivityKind::Like => {
            "INSERT INTO activity.account_totals (account_id, likes_given) \
             VALUES ($1, 1) \
             ON CONFLICT (account_id) DO UPDATE \
             SET likes_given = activity.account_totals.likes_given + 1, \
                 updated_at = now()"
        }
    };
    sqlx::query(statement).bind(account_id).execute(connection).await?;
    Ok(())
}

/// Decrement lifetime totals when a contribution reverses.
pub async fn decrement_totals_tx(
    connection: &mut PgConnection,
    account_id: i64,
    kind: crate::contributions::ActivityKind,
) -> AppResult<()> {
    let statement = match kind {
        crate::contributions::ActivityKind::Thread => {
            "UPDATE activity.account_totals \
             SET threads_created = GREATEST(threads_created - 1, 0), updated_at = now() \
             WHERE account_id = $1"
        }
        crate::contributions::ActivityKind::Comment => {
            "UPDATE activity.account_totals \
             SET comments_created = GREATEST(comments_created - 1, 0), updated_at = now() \
             WHERE account_id = $1"
        }
        crate::contributions::ActivityKind::Like => {
            "UPDATE activity.account_totals \
             SET likes_given = GREATEST(likes_given - 1, 0), updated_at = now() \
             WHERE account_id = $1"
        }
    };
    sqlx::query(statement).bind(account_id).execute(connection).await?;
    Ok(())
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
        override_reason: progress.override_reason.clone(),
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
                override_reason, override_by, override_at, last_evaluated_at, updated_at \
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
                reason, changed_by, created_at \
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
    let (thread_weight, comment_weight, like_weight): (i32, i32, i32) = sqlx::query_as(
        "SELECT thread_weight, comment_weight, like_weight \
         FROM activity.score_policies WHERE version = $1",
    )
    .bind(policy.score_policy_version)
    .fetch_one(&mut *connection)
    .await?;
    let score: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM( \
            counts.threads_created * $2 \
            + counts.comments_created * $3 \
            + LEAST(counts.likes_given * $4, $5) \
         ), 0)::bigint \
         FROM activity.daily_counts counts \
         WHERE counts.account_id = $1",
    )
    .bind(account_id)
    .bind(thread_weight)
    .bind(comment_weight)
    .bind(like_weight)
    .bind(policy.like_daily_cap)
    .fetch_one(connection)
    .await?;
    Ok(score.unwrap_or(0))
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

//! Durable, read-only reconciliation of the append-only ledger and wallet projection.

use chrono::{DateTime, Utc};
use governance::{record_account_event_tx, record_system_event_tx, AccountActor};
use serde_json::json;
use sha2::{Digest, Sha256};
use shared::{AppError, AppResult, Page};
use sqlx::{Acquire, FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::dto::{ReconciliationRunDto, ReconciliationStatsDto, ReconciliationWalletDto};

const MAX_PAGE_LIMIT: i64 = 100;
/// Stable PostgreSQL advisory-lock namespace used by reconciliation workers.
#[doc(hidden)]
pub const RECONCILIATION_ADVISORY_LOCK_ID: i64 = 2_026_071_203_038;
const DATABASE_ERROR_CODE: &str = "RECONCILIATION_DATABASE_ERROR";
const EXECUTION_ERROR_CODE: &str = "RECONCILIATION_EXECUTION_FAILED";

#[derive(Debug, Clone, FromRow)]
struct ReconciliationRunRow {
    id: i64,
    public_id: Uuid,
    requested_by: i64,
    reason: String,
    request_fingerprint: String,
    status: String,
    ledger_ok: Option<bool>,
    ledger_latest_seq: Option<i64>,
    ledger_latest_hash: Option<String>,
    ledger_failure_seq: Option<i64>,
    wallets_checked: i64,
    drifted_wallets: i64,
    missing_wallets: i64,
    balance_drifted_wallets: i64,
    sequence_drifted_wallets: i64,
    total_absolute_drift: String,
    error_code: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ReconciliationWalletRow {
    account_id: i64,
    expected_balance: String,
    actual_balance: Option<String>,
    delta: String,
    expected_last_seq: i64,
    actual_last_seq: Option<i64>,
    wallet_exists: bool,
    has_balance_drift: bool,
    has_sequence_drift: bool,
}

struct ScheduledRun {
    row: ReconciliationRunRow,
    was_created: bool,
}

struct RunMetrics {
    wallets_checked: i64,
    drifted_wallets: i64,
    missing_wallets: i64,
    balance_drifted_wallets: i64,
    sequence_drifted_wallets: i64,
    total_absolute_drift: String,
}

const RUN_COLUMNS: &str =
    "id, public_id, requested_by, reason, request_fingerprint, status, ledger_ok, \
     ledger_latest_seq, ledger_latest_hash, ledger_failure_seq, wallets_checked, \
     drifted_wallets, missing_wallets, balance_drifted_wallets, \
     sequence_drifted_wallets, total_absolute_drift::text AS total_absolute_drift, \
     error_code, created_at, started_at, completed_at";

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be between 3 and 500 characters".into()));
    }
    Ok(reason)
}

fn validate_idempotency_key(idempotency_key: &str) -> AppResult<()> {
    if !(8..=128).contains(&idempotency_key.len())
        || !idempotency_key.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
    {
        return Err(AppError::BadRequest(
            "Idempotency-Key must contain 8 to 128 visible ASCII characters".into(),
        ));
    }
    Ok(())
}

fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn execution_error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Internal(inner) if inner.downcast_ref::<sqlx::Error>().is_some() => {
            DATABASE_ERROR_CODE
        }
        _ => EXECUTION_ERROR_CODE,
    }
}

fn run_dto(row: ReconciliationRunRow) -> ReconciliationRunDto {
    ReconciliationRunDto {
        id: row.public_id.to_string(),
        status: row.status,
        requested_by: row.requested_by.to_string(),
        reason: row.reason,
        ledger_ok: row.ledger_ok,
        ledger_latest_seq: row.ledger_latest_seq,
        ledger_latest_hash: row.ledger_latest_hash,
        ledger_failure_seq: row.ledger_failure_seq,
        wallets_checked: row.wallets_checked,
        drifted_wallets: row.drifted_wallets,
        missing_wallets: row.missing_wallets,
        balance_drifted_wallets: row.balance_drifted_wallets,
        sequence_drifted_wallets: row.sequence_drifted_wallets,
        total_absolute_drift: row.total_absolute_drift,
        error_code: row.error_code,
        created_at: row.created_at.timestamp(),
        started_at: row.started_at.map(|timestamp| timestamp.timestamp()),
        completed_at: row.completed_at.map(|timestamp| timestamp.timestamp()),
    }
}

fn wallet_dto(row: ReconciliationWalletRow) -> ReconciliationWalletDto {
    ReconciliationWalletDto {
        account_id: row.account_id.to_string(),
        expected_balance: row.expected_balance,
        actual_balance: row.actual_balance,
        delta: row.delta,
        expected_last_seq: row.expected_last_seq,
        actual_last_seq: row.actual_last_seq,
        wallet_exists: row.wallet_exists,
        has_balance_drift: row.has_balance_drift,
        has_sequence_drift: row.has_sequence_drift,
    }
}

fn page_limit(limit: i64) -> AppResult<i64> {
    if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    Ok(limit)
}

fn numeric_cursor(cursor: Option<&str>) -> AppResult<Option<i64>> {
    cursor
        .map(|value| {
            value
                .parse::<i64>()
                .ok()
                .filter(|parsed| *parsed > 0)
                .ok_or_else(|| AppError::BadRequest("invalid cursor".into()))
        })
        .transpose()
}

async fn find_run_by_public_id_on(
    conn: &mut PgConnection,
    public_id: Uuid,
) -> AppResult<Option<ReconciliationRunRow>> {
    let query =
        format!("SELECT {RUN_COLUMNS} FROM credit.reconciliation_runs WHERE public_id = $1");
    Ok(sqlx::query_as(&query).bind(public_id).fetch_optional(conn).await?)
}

async fn find_run_by_public_id(
    pool: &PgPool,
    public_id: Uuid,
) -> AppResult<Option<ReconciliationRunRow>> {
    let mut conn = pool.acquire().await?;
    find_run_by_public_id_on(&mut conn, public_id).await
}

async fn schedule_run(
    pool: &PgPool,
    actor: AccountActor<'_>,
    reason: &str,
    idempotency_key: &str,
) -> AppResult<ScheduledRun> {
    validate_idempotency_key(idempotency_key)?;
    let reason = validate_reason(reason)?;
    let key_hash = sha256_hex(idempotency_key);
    let request_fingerprint = sha256_hex(&json!({ "reason": reason }).to_string());
    let mut tx = pool.begin().await?;

    let query = format!(
        "SELECT {RUN_COLUMNS} FROM credit.reconciliation_runs \
         WHERE requested_by = $1 AND idempotency_key_hash = $2"
    );
    if let Some(existing) = sqlx::query_as::<_, ReconciliationRunRow>(&query)
        .bind(actor.account_id)
        .bind(&key_hash)
        .fetch_optional(&mut *tx)
        .await?
    {
        if existing.request_fingerprint != request_fingerprint {
            return Err(AppError::Conflict(
                "Idempotency-Key was already used with a different reason".into(),
            ));
        }
        tx.commit().await?;
        return Ok(ScheduledRun { row: existing, was_created: false });
    }

    let public_id = Uuid::new_v4();
    let insert_query = format!(
        "INSERT INTO credit.reconciliation_runs \
         (public_id, requested_by, reason, idempotency_key_hash, request_fingerprint) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (requested_by, idempotency_key_hash) DO NOTHING \
         RETURNING {RUN_COLUMNS}"
    );
    let inserted = match sqlx::query_as::<_, ReconciliationRunRow>(&insert_query)
        .bind(public_id)
        .bind(actor.account_id)
        .bind(reason)
        .bind(&key_hash)
        .bind(&request_fingerprint)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(row) => row,
        Err(sqlx::Error::Database(database_error))
            if database_error.constraint() == Some("credit_reconciliation_one_active_idx") =>
        {
            tx.rollback().await?;
            return Err(AppError::Conflict("a credit reconciliation run is already active".into()));
        }
        Err(error) => return Err(error.into()),
    };

    let (row, was_created) = if let Some(row) = inserted {
        let metadata = json!({ "capability": "credit.integrity" });
        record_account_event_tx(
            &mut tx,
            actor,
            "credit.reconciliation.requested",
            "credit_reconciliation",
            &row.public_id.to_string(),
            reason,
            Some(&metadata),
        )
        .await?;
        (row, true)
    } else {
        let existing = sqlx::query_as::<_, ReconciliationRunRow>(&query)
            .bind(actor.account_id)
            .bind(&key_hash)
            .fetch_one(&mut *tx)
            .await?;
        if existing.request_fingerprint != request_fingerprint {
            return Err(AppError::Conflict(
                "Idempotency-Key was already used with a different reason".into(),
            ));
        }
        (existing, false)
    };
    tx.commit().await?;
    Ok(ScheduledRun { row, was_created })
}

async fn mark_running(conn: &mut PgConnection, run: &ReconciliationRunRow) -> AppResult<bool> {
    let mut tx = conn.begin().await?;
    let changed = sqlx::query(
        "UPDATE credit.reconciliation_runs \
         SET status = 'running', started_at = COALESCE(started_at, now()), error_code = NULL \
         WHERE id = $1 AND status = 'queued'",
    )
    .bind(run.id)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if changed {
        let metadata = json!({ "correlationId": run.public_id.to_string() });
        record_system_event_tx(
            &mut tx,
            "credit.reconciliation.started",
            "credit_reconciliation",
            &run.public_id.to_string(),
            &run.reason,
            Some(&metadata),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(changed)
}

async fn record_resume_request(
    conn: &mut PgConnection,
    actor: AccountActor<'_>,
    run: &ReconciliationRunRow,
    reason: &str,
) -> AppResult<()> {
    let mut tx = conn.begin().await?;
    let metadata = json!({ "correlationId": run.public_id.to_string() });
    record_account_event_tx(
        &mut tx,
        actor,
        "credit.reconciliation.resume_requested",
        "credit_reconciliation",
        &run.public_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn persist_wallet_results(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: i64,
    latest_seq: Option<i64>,
) -> AppResult<RunMetrics> {
    sqlx::query("DELETE FROM credit.reconciliation_wallet_results WHERE run_id = $1")
        .bind(run_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "WITH ledger_effects AS ( \
           SELECT to_account AS account_id, amount::numeric AS balance_delta, seq \
           FROM credit.ledger WHERE to_account IS NOT NULL AND ($2::bigint IS NULL OR seq <= $2) \
           UNION ALL \
           SELECT from_account AS account_id, -amount::numeric AS balance_delta, seq \
           FROM credit.ledger WHERE from_account IS NOT NULL AND ($2::bigint IS NULL OR seq <= $2) \
         ), derived AS ( \
           SELECT account_id, SUM(balance_delta) AS expected_balance, MAX(seq) AS expected_last_seq \
           FROM ledger_effects GROUP BY account_id \
         ), compared AS ( \
           SELECT COALESCE(derived.account_id, wallets.account_id) AS account_id, \
                  COALESCE(derived.expected_balance, 0::numeric) AS expected_balance, \
                  wallets.balance AS actual_balance, \
                  COALESCE(wallets.balance, 0)::numeric \
                    - COALESCE(derived.expected_balance, 0::numeric) AS delta, \
                  COALESCE(derived.expected_last_seq, 0) AS expected_last_seq, \
                  wallets.last_seq AS actual_last_seq, \
                  wallets.account_id IS NOT NULL AS wallet_exists \
           FROM derived FULL OUTER JOIN credit.wallets wallets \
             ON wallets.account_id = derived.account_id \
         ) \
         INSERT INTO credit.reconciliation_wallet_results \
           (run_id, account_id, expected_balance, actual_balance, delta, expected_last_seq, \
            actual_last_seq, wallet_exists, has_balance_drift, has_sequence_drift) \
         SELECT $1, account_id, expected_balance, actual_balance, delta, expected_last_seq, \
                actual_last_seq, wallet_exists, \
                COALESCE(actual_balance, 0)::numeric <> expected_balance, \
                COALESCE(actual_last_seq, 0) <> expected_last_seq \
         FROM compared",
    )
    .bind(run_id)
    .bind(latest_seq)
    .execute(&mut **tx)
    .await?;

    let row: (i64, i64, i64, i64, i64, String) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, \
                COUNT(*) FILTER (WHERE NOT wallet_exists OR has_balance_drift OR has_sequence_drift)::bigint, \
                COUNT(*) FILTER (WHERE NOT wallet_exists)::bigint, \
                COUNT(*) FILTER (WHERE has_balance_drift)::bigint, \
                COUNT(*) FILTER (WHERE has_sequence_drift)::bigint, \
                COALESCE(SUM(ABS(delta)), 0)::text \
         FROM credit.reconciliation_wallet_results WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(RunMetrics {
        wallets_checked: row.0,
        drifted_wallets: row.1,
        missing_wallets: row.2,
        balance_drifted_wallets: row.3,
        sequence_drifted_wallets: row.4,
        total_absolute_drift: row.5,
    })
}

async fn reconcile_in_transaction(
    conn: &mut PgConnection,
    run: &ReconciliationRunRow,
    system_public_key_b64: &str,
    wallet_key_resolver: &dyn crate::wallet_keys::WalletKeyResolver,
) -> AppResult<()> {
    let mut tx = conn.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ").execute(&mut *tx).await?;
    let ledger_tip: Option<(i64, String)> =
        sqlx::query_as("SELECT seq, hash FROM credit.ledger ORDER BY seq DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let verification =
        crate::repo::verify_full_ledger_conn(&mut tx, system_public_key_b64, wallet_key_resolver)
            .await?;
    let (latest_seq, latest_hash) = ledger_tip.unzip();
    let metrics = if verification.ok {
        persist_wallet_results(&mut tx, run.id, latest_seq).await?
    } else {
        sqlx::query("DELETE FROM credit.reconciliation_wallet_results WHERE run_id = $1")
            .bind(run.id)
            .execute(&mut *tx)
            .await?;
        RunMetrics {
            wallets_checked: 0,
            drifted_wallets: 0,
            missing_wallets: 0,
            balance_drifted_wallets: 0,
            sequence_drifted_wallets: 0,
            total_absolute_drift: "0".into(),
        }
    };

    let finalized = sqlx::query(
        "UPDATE credit.reconciliation_runs \
         SET status = 'succeeded', ledger_ok = $2, ledger_latest_seq = $3, \
             ledger_latest_hash = $4, ledger_failure_seq = $5, wallets_checked = $6, \
             drifted_wallets = $7, missing_wallets = $8, balance_drifted_wallets = $9, \
             sequence_drifted_wallets = $10, total_absolute_drift = $11::numeric, \
             error_code = NULL, completed_at = now() \
         WHERE id = $1 AND status = 'running'",
    )
    .bind(run.id)
    .bind(verification.ok)
    .bind(latest_seq)
    .bind(&latest_hash)
    .bind((!verification.ok).then_some(verification.latest_seq).flatten())
    .bind(metrics.wallets_checked)
    .bind(metrics.drifted_wallets)
    .bind(metrics.missing_wallets)
    .bind(metrics.balance_drifted_wallets)
    .bind(metrics.sequence_drifted_wallets)
    .bind(&metrics.total_absolute_drift)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if finalized != 1 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "credit reconciliation final state transition was not applied"
        )));
    }

    let audit_metadata = json!({
        "correlationId": run.public_id.to_string(),
        "ledgerOk": verification.ok,
        "walletsChecked": metrics.wallets_checked,
        "driftedWallets": metrics.drifted_wallets,
        "missingWallets": metrics.missing_wallets,
        "totalAbsoluteDrift": metrics.total_absolute_drift,
    });
    record_system_event_tx(
        &mut tx,
        "credit.reconciliation.succeeded",
        "credit_reconciliation",
        &run.public_id.to_string(),
        &run.reason,
        Some(&audit_metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn mark_failed(
    conn: &mut PgConnection,
    run: &ReconciliationRunRow,
    error_code: &str,
) -> AppResult<()> {
    let mut tx = conn.begin().await?;
    let failed = sqlx::query(
        "UPDATE credit.reconciliation_runs \
         SET status = 'failed', started_at = COALESCE(started_at, now()), completed_at = now(), \
             error_code = $2, ledger_ok = NULL \
         WHERE id = $1 AND status IN ('queued', 'running')",
    )
    .bind(run.id)
    .bind(error_code)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if failed != 1 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "credit reconciliation failure state transition was not applied"
        )));
    }
    let metadata = json!({
        "correlationId": run.public_id.to_string(),
        "errorCode": error_code,
    });
    record_system_event_tx(
        &mut tx,
        "credit.reconciliation.failed",
        "credit_reconciliation",
        &run.public_id.to_string(),
        &run.reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn execute_if_available(
    pool: &PgPool,
    run: &ReconciliationRunRow,
    system_public_key_b64: &str,
    wallet_key_resolver: &dyn crate::wallet_keys::WalletKeyResolver,
    resume: Option<(AccountActor<'_>, &str)>,
) -> AppResult<ReconciliationRunRow> {
    if matches!(run.status.as_str(), "succeeded" | "failed") {
        return Ok(run.clone());
    }
    let mut conn = pool.acquire().await?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(RECONCILIATION_ADVISORY_LOCK_ID)
        .fetch_one(&mut *conn)
        .await?;
    if !acquired {
        return find_run_by_public_id_on(&mut conn, run.public_id).await?.ok_or(AppError::NotFound);
    }

    let outcome = async {
        let current =
            find_run_by_public_id_on(&mut conn, run.public_id).await?.ok_or(AppError::NotFound)?;
        if matches!(current.status.as_str(), "succeeded" | "failed") {
            return Ok(current);
        }
        if let Some((actor, reason)) = resume {
            record_resume_request(&mut conn, actor, &current, reason).await?;
        }
        mark_running(&mut conn, &current).await?;
        if let Err(error) = reconcile_in_transaction(
            &mut conn,
            &current,
            system_public_key_b64,
            wallet_key_resolver,
        )
        .await
        {
            let error_code = execution_error_code(&error);
            tracing::warn!(
                run_id = %current.public_id,
                error_code,
                "credit reconciliation failed"
            );
            mark_failed(&mut conn, &current, error_code).await?;
        }
        find_run_by_public_id_on(&mut conn, current.public_id).await?.ok_or(AppError::NotFound)
    }
    .await;

    if let Err(unlock_error) = sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock($1)")
        .bind(RECONCILIATION_ADVISORY_LOCK_ID)
        .fetch_one(&mut *conn)
        .await
    {
        tracing::warn!(
            run_id = %run.public_id,
            error_code = "RECONCILIATION_UNLOCK_FAILED",
            error = ?unlock_error,
            "credit reconciliation advisory lock release failed"
        );
    }
    outcome
}

/// Create or replay an idempotent read-only reconciliation request and execute it when available.
pub async fn request_run(
    pool: &PgPool,
    actor: AccountActor<'_>,
    reason: &str,
    idempotency_key: &str,
    system_public_key_b64: &str,
    wallet_key_resolver: &dyn crate::wallet_keys::WalletKeyResolver,
) -> AppResult<(ReconciliationRunDto, bool)> {
    let scheduled = schedule_run(pool, actor, reason, idempotency_key).await?;
    let row = execute_if_available(
        pool,
        &scheduled.row,
        system_public_key_b64,
        wallet_key_resolver,
        None,
    )
    .await?;
    Ok((run_dto(row), scheduled.was_created))
}

/// Resume an interrupted active run without creating a second reconciliation snapshot.
pub async fn resume_run(
    pool: &PgPool,
    actor: AccountActor<'_>,
    public_id: Uuid,
    reason: &str,
    system_public_key_b64: &str,
    wallet_key_resolver: &dyn crate::wallet_keys::WalletKeyResolver,
) -> AppResult<ReconciliationRunDto> {
    let reason = validate_reason(reason)?;
    let run = find_run_by_public_id(pool, public_id).await?.ok_or(AppError::NotFound)?;
    let row = execute_if_available(
        pool,
        &run,
        system_public_key_b64,
        wallet_key_resolver,
        Some((actor, reason)),
    )
    .await?;
    Ok(run_dto(row))
}

/// Return one reconciliation run by its public identifier.
pub async fn get_run(pool: &PgPool, public_id: Uuid) -> AppResult<ReconciliationRunDto> {
    find_run_by_public_id(pool, public_id).await?.map(run_dto).ok_or(AppError::NotFound)
}

/// Return newest reconciliation runs with strict cursor pagination.
pub async fn list_runs(
    pool: &PgPool,
    cursor: Option<&str>,
    limit: i64,
) -> AppResult<Page<ReconciliationRunDto>> {
    let cursor = numeric_cursor(cursor)?;
    let limit = page_limit(limit)?;
    let query = format!(
        "SELECT {RUN_COLUMNS} FROM credit.reconciliation_runs \
         WHERE ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2"
    );
    let mut rows: Vec<ReconciliationRunRow> =
        sqlx::query_as(&query).bind(cursor).bind(limit + 1).fetch_all(pool).await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.id.to_string())).flatten();
    Ok(Page::new(rows.into_iter().map(run_dto).collect(), next_cursor))
}

/// Return per-wallet findings for one run without modifying the wallet projection.
pub async fn list_wallet_results(
    pool: &PgPool,
    public_id: Uuid,
    cursor: Option<&str>,
    limit: i64,
    drift_only: bool,
) -> AppResult<Page<ReconciliationWalletDto>> {
    let run = find_run_by_public_id(pool, public_id).await?.ok_or(AppError::NotFound)?;
    let cursor = numeric_cursor(cursor)?.unwrap_or(0);
    let limit = page_limit(limit)?;
    let mut rows: Vec<ReconciliationWalletRow> = sqlx::query_as(
        "SELECT account_id, expected_balance::text AS expected_balance, \
                actual_balance::text AS actual_balance, delta::text AS delta, \
                expected_last_seq, actual_last_seq, wallet_exists, \
                has_balance_drift, has_sequence_drift \
         FROM credit.reconciliation_wallet_results \
         WHERE run_id = $1 AND account_id > $2 \
           AND (NOT $3 OR NOT wallet_exists OR has_balance_drift OR has_sequence_drift) \
         ORDER BY account_id ASC LIMIT $4",
    )
    .bind(run.id)
    .bind(cursor)
    .bind(drift_only)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }
    let next_cursor = has_more.then(|| rows.last().map(|row| row.account_id.to_string())).flatten();
    Ok(Page::new(rows.into_iter().map(wallet_dto).collect(), next_cursor))
}

/// Return all-time reconciliation outcome counters and the newest run.
pub async fn stats(pool: &PgPool) -> AppResult<ReconciliationStatsDto> {
    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*)::bigint, \
                COUNT(*) FILTER (WHERE status = 'failed')::bigint, \
                COUNT(*) FILTER (WHERE status = 'succeeded' AND ledger_ok = false)::bigint, \
                COUNT(*) FILTER (WHERE status = 'succeeded' AND drifted_wallets > 0)::bigint \
         FROM credit.reconciliation_runs",
    )
    .fetch_one(pool)
    .await?;
    let query =
        format!("SELECT {RUN_COLUMNS} FROM credit.reconciliation_runs ORDER BY id DESC LIMIT 1");
    let latest =
        sqlx::query_as::<_, ReconciliationRunRow>(&query).fetch_optional(pool).await?.map(run_dto);
    Ok(ReconciliationStatsDto {
        total_runs: counts.0,
        failed_runs: counts.1,
        ledger_failure_runs: counts.2,
        runs_with_drift: counts.3,
        latest_run: latest,
    })
}

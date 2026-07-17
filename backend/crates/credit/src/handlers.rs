//! Axum request handlers for the credit domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use shared::{AppResult, AppState, Page};

use crate::dto::{
    LedgerEntryDto, LedgerVerify, ProductDto, ProductInput, PurchaseAction, PurchaseDto,
    ReconciliationRunDto, ReconciliationRunInput, ReconciliationStatsDto, ReconciliationWalletDto,
    SigningIntentInput, SigningIntentOutcomeDto, SigningIntentOutcomeInput, SigningIntentOutput,
    TaskAction, TaskDto, TaskInput, TipInput, WalletDto,
};
use crate::error::CreditError;
use crate::repo;
use crate::CreditState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_limit() -> i64 {
    20
}

fn parse_pagination(cursor: Option<&str>, limit: i64) -> AppResult<(Option<i64>, i64)> {
    if !(1..=100).contains(&limit) {
        return Err(shared::AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    let cursor = cursor
        .map(|value| {
            value
                .parse::<i64>()
                .ok()
                .filter(|parsed| *parsed > 0)
                .ok_or_else(|| shared::AppError::BadRequest("invalid cursor".into()))
        })
        .transpose()?;
    Ok((cursor, limit))
}

/// Helper: convert the `Response` error from `authenticate` into `AppError`.
fn map_auth_err(response: axum::response::Response) -> shared::AppError {
    match response.status() {
        axum::http::StatusCode::UNAUTHORIZED => shared::AppError::Unauthorized,
        axum::http::StatusCode::FORBIDDEN => shared::AppError::Forbidden,
        status => shared::AppError::Internal(anyhow::anyhow!(
            "credit authentication failed with status {status}"
        )),
    }
}

async fn lock_actor_for_write(
    state: &CreditState,
    conn: &mut sqlx::PgConnection,
    account_id: i64,
) -> AppResult<()> {
    if !state.account_eligibility_resolver.are_eligible_on(conn, &[account_id]).await? {
        return Err(shared::AppError::Forbidden);
    }
    Ok(())
}

async fn lock_actor_and_counterparty(
    state: &CreditState,
    conn: &mut sqlx::PgConnection,
    actor_id: i64,
    counterparty_id: i64,
) -> AppResult<bool> {
    if state
        .account_eligibility_resolver
        .are_eligible_on(conn, &[actor_id, counterparty_id])
        .await?
    {
        return Ok(true);
    }
    if !state.account_eligibility_resolver.is_eligible_on(conn, actor_id).await? {
        return Err(shared::AppError::Forbidden);
    }
    Ok(false)
}

async fn append_consumed_user_ledger(
    conn: &mut sqlx::PgConnection,
    account_id: i64,
    expected_type: &str,
    consumed: crate::signing::ConsumedIntent,
) -> AppResult<()> {
    let prepared = consumed.ledger_entry.ok_or(CreditError::InvalidSignature)?;
    if prepared.type_ != expected_type
        || prepared.from_account != Some(account_id)
        || prepared.signer != account_id.to_string()
    {
        return Err(CreditError::InvalidSignature.into());
    }
    repo::append_ledger_entry_tx(
        conn,
        &prepared.tx_id,
        &prepared.type_,
        prepared.from_account,
        prepared.to_account,
        prepared.amount,
        &prepared.nonce,
        prepared.metadata.as_ref(),
        &prepared.signer,
        &consumed.signature,
        prepared.created_at,
    )
    .await?;
    Ok(())
}

async fn append_system_release(
    conn: &mut sqlx::PgConnection,
    state: &AppState,
    to_account: i64,
    amount: i64,
    metadata: &serde_json::Value,
) -> AppResult<()> {
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().timestamp();
    let canonical = crate::ledger::build_ledger_canonical(
        &tx_id,
        "escrow_release",
        None,
        Some(to_account),
        amount,
        &nonce,
        Some(metadata),
        "system",
        created_at,
    );
    let signature = crate::ledger::sign_with_seed(&canonical, &state.system_private_key);
    repo::append_ledger_entry_tx(
        conn,
        &tx_id,
        "escrow_release",
        None,
        Some(to_account),
        amount,
        &nonce,
        Some(metadata),
        "system",
        &signature,
        created_at,
    )
    .await?;
    Ok(())
}

/// POST /api/v2/credit/signing-intents — return exact bytes for wallet signing.
pub(crate) async fn create_signing_intent(
    State(state): State<CreditState>,
    headers: HeaderMap,
    Json(body): Json<SigningIntentInput>,
) -> AppResult<Json<SigningIntentOutput>> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|header| header.to_str().ok())
        .filter(|header| !header.is_empty())
        .ok_or(CreditError::IntentUnavailable)?;
    Ok(Json(
        crate::signing::create_intent(
            &state.app.db,
            state.account_eligibility_resolver.as_ref(),
            state.wallet_key_resolver.as_ref(),
            auth.id,
            &body,
            idempotency_key,
        )
        .await?,
    ))
}

/// POST /api/v2/credit/signing-intent-outcome — owner-only lock-aware intent outcome.
pub async fn get_signing_intent_outcome(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SigningIntentOutcomeInput>,
) -> AppResult<Json<SigningIntentOutcomeDto>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let intent_id = body.intent_id.parse::<uuid::Uuid>().map_err(|_| shared::AppError::NotFound)?;
    Ok(Json(crate::signing::intent_outcome(&state.db, auth.id, intent_id).await?))
}

// ---------------------------------------------------------------------------
// Wallet
// ---------------------------------------------------------------------------

/// GET /api/v2/wallet — authenticated wallet balance.
pub(crate) async fn get_wallet(
    State(state): State<CreditState>,
    headers: HeaderMap,
) -> AppResult<Json<WalletDto>> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let active_public_key =
        state.wallet_key_resolver.active_public_key(&state.app.db, auth.id).await?;
    let wallet = repo::get_wallet(&state.app.db, auth.id, active_public_key).await?;
    Ok(Json(wallet))
}

// ---------------------------------------------------------------------------
// Ledger
// ---------------------------------------------------------------------------

/// Query parameters for GET /api/v2/wallet/ledger.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// GET /api/v2/wallet/ledger — authenticated ledger entries.
pub async fn get_ledger(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<LedgerQuery>,
) -> AppResult<Json<Page<LedgerEntryDto>>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let (cursor, limit) = parse_pagination(params.cursor.as_deref(), params.limit)?;
    let page = repo::list_ledger(&state.db, auth.id, cursor, limit).await?;
    Ok(Json(page))
}

/// GET /api/v2/wallet/ledger/verify — public verification result.
pub(crate) async fn verify_ledger(
    State(state): State<CreditState>,
) -> AppResult<Json<LedgerVerify>> {
    let result = repo::verify_full_ledger(
        &state.app.db,
        &state.app.system_public_key_b64,
        state.wallet_key_resolver.as_ref(),
    )
    .await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationRunsQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationWalletsQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_true")]
    pub drift_only: bool,
}

fn default_true() -> bool {
    true
}

fn parse_reconciliation_id(id: &str) -> AppResult<uuid::Uuid> {
    id.parse().map_err(|_| shared::AppError::BadRequest("invalid reconciliation id".into()))
}

async fn authenticate_credit_integrity(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<shared::AuthAccount> {
    let auth = crate::auth::authenticate(headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    auth.require_capability(shared::auth::Capability::ManageCreditIntegrity)
        .map_err(|_| shared::AppError::Forbidden)?;
    Ok(auth)
}

/// POST /api/v2/admin/credit/reconciliations — run a read-only integrity comparison.
pub(crate) async fn request_reconciliation_run(
    State(state): State<CreditState>,
    headers: HeaderMap,
    Json(body): Json<ReconciliationRunInput>,
) -> AppResult<(StatusCode, Json<ReconciliationRunDto>)> {
    let auth = authenticate_credit_integrity(&state.app, &headers).await?;
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| shared::AppError::BadRequest("Idempotency-Key is required".into()))?;
    let actor = governance::AccountActor { account_id: auth.id, role: &auth.role };
    let (run, was_created) = crate::reconciliation::request_run(
        &state.app.db,
        actor,
        &body.reason,
        idempotency_key,
        &state.app.system_public_key_b64,
        state.wallet_key_resolver.as_ref(),
    )
    .await?;
    let status = if was_created { StatusCode::CREATED } else { StatusCode::OK };
    Ok((status, Json(run)))
}

/// GET /api/v2/admin/credit/reconciliations — list durable integrity runs.
pub async fn list_reconciliation_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ReconciliationRunsQuery>,
) -> AppResult<Json<Page<ReconciliationRunDto>>> {
    authenticate_credit_integrity(&state, &headers).await?;
    Ok(Json(
        crate::reconciliation::list_runs(&state.db, query.cursor.as_deref(), query.limit).await?,
    ))
}

/// GET /api/v2/admin/credit/reconciliations/{id} — inspect one run summary.
pub async fn get_reconciliation_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<ReconciliationRunDto>> {
    authenticate_credit_integrity(&state, &headers).await?;
    Ok(Json(crate::reconciliation::get_run(&state.db, parse_reconciliation_id(&id)?).await?))
}

/// POST /api/v2/admin/credit/reconciliations/{id}/resume — resume an interrupted run.
pub(crate) async fn resume_reconciliation_run(
    State(state): State<CreditState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ReconciliationRunInput>,
) -> AppResult<Json<ReconciliationRunDto>> {
    let auth = authenticate_credit_integrity(&state.app, &headers).await?;
    let actor = governance::AccountActor { account_id: auth.id, role: &auth.role };
    Ok(Json(
        crate::reconciliation::resume_run(
            &state.app.db,
            actor,
            parse_reconciliation_id(&id)?,
            &body.reason,
            &state.app.system_public_key_b64,
            state.wallet_key_resolver.as_ref(),
        )
        .await?,
    ))
}

/// GET /api/v2/admin/credit/reconciliations/{id}/wallets — inspect wallet drift.
pub async fn list_reconciliation_wallets(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<ReconciliationWalletsQuery>,
) -> AppResult<Json<Page<ReconciliationWalletDto>>> {
    authenticate_credit_integrity(&state, &headers).await?;
    Ok(Json(
        crate::reconciliation::list_wallet_results(
            &state.db,
            parse_reconciliation_id(&id)?,
            query.cursor.as_deref(),
            query.limit,
            query.drift_only,
        )
        .await?,
    ))
}

/// GET /api/v2/admin/credit/reconciliations/stats — integrity outcome counters.
pub async fn reconciliation_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ReconciliationStatsDto>> {
    authenticate_credit_integrity(&state, &headers).await?;
    Ok(Json(crate::reconciliation::stats(&state.db).await?))
}

// ---------------------------------------------------------------------------
// Tip
// ---------------------------------------------------------------------------

/// POST /api/v2/wallet/tip
///
/// Wallet-signed value transfer. Requires:
/// 1. JWT auth (bearer token)
/// 2. `X-Wallet-Sig` header with a base64 Ed25519 signature over the canonical payload
/// 3. The signer must have a bound Ed25519 public key
/// 4. The signer must have sufficient balance
#[tracing::instrument(skip(state, headers, body))]
pub(crate) async fn tip(
    State(state): State<CreditState>,
    headers: HeaderMap,
    Json(body): Json<TipInput>,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    shared::ratelimit::check_token_bucket(
        state.app.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;
    let to_account_id: i64 = body
        .to_account_id
        .parse()
        .map_err(|_| shared::AppError::BadRequest("invalid to_account_id".into()))?;

    if body.amount <= 0 {
        return Err(shared::AppError::BadRequest("amount must be positive".into()));
    }
    if to_account_id == auth.id {
        return Err(CreditError::InvalidAction("cannot tip your own content".into()).into());
    }
    if !matches!(body.target_type.as_str(), "review" | "thread" | "comment") {
        return Err(shared::AppError::BadRequest("unsupported targetType".into()));
    }
    let target_id = body
        .target_id
        .parse::<i64>()
        .ok()
        .filter(|target_id| *target_id > 0)
        .ok_or_else(|| shared::AppError::BadRequest("invalid targetId".into()))?;

    let request = serde_json::to_value(&body)
        .map_err(|error| shared::AppError::Internal(anyhow::Error::new(error)))?;
    let preview_target = {
        let mut conn = state.app.db.acquire().await?;
        state.tip_target_resolver.resolve(&mut conn, &body.target_type, target_id).await?
    };
    let mut tx = state.app.db.begin().await?;
    let is_counterparty_eligible = if let Some(target) = preview_target.as_ref() {
        lock_actor_and_counterparty(&state, &mut tx, auth.id, target.author_id).await?
    } else {
        lock_actor_for_write(&state, &mut tx, auth.id).await?;
        false
    };
    let consumed = crate::signing::consume_intent(
        &mut tx,
        state.wallet_key_resolver.as_ref(),
        &headers,
        auth.id,
        "credit.tip",
        &request,
    )
    .await?;
    let preview_target = preview_target.ok_or(shared::AppError::NotFound)?;
    if !is_counterparty_eligible {
        return Err(shared::AppError::NotFound);
    }
    if preview_target.canonical_type != body.target_type
        || preview_target.canonical_id != target_id
        || preview_target.author_id != to_account_id
    {
        return Err(
            CreditError::InvalidAction("tip recipient must be the target author".into()).into()
        );
    }
    let target = state
        .tip_target_resolver
        .resolve(&mut tx, &body.target_type, target_id)
        .await?
        .ok_or(shared::AppError::NotFound)?;
    if target.canonical_type != body.target_type
        || target.canonical_id != target_id
        || target.author_id != to_account_id
    {
        return Err(
            CreditError::InvalidAction("tip recipient must be the target author".into()).into()
        );
    }
    let wallet_balance = repo::lock_wallet_for_debit_tx(&mut tx, auth.id).await?;
    if wallet_balance < body.amount {
        return Err(CreditError::InsufficientBalance.into());
    }
    append_consumed_user_ledger(&mut tx, auth.id, "tip", consumed).await?;
    tx.commit().await?;

    tracing::info!(from = auth.id, to = to_account_id, amount = body.amount, "tip processed");

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Tasks (escrow market — bounty-style)
// ---------------------------------------------------------------------------

/// Query parameters for GET /api/v2/credit/tasks.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasksQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn task_status_filter(status: Option<&str>) -> AppResult<Option<&str>> {
    match status {
        None | Some("all") => Ok(None),
        Some(status @ ("open" | "in_progress" | "submitted" | "completed" | "cancelled")) => {
            Ok(Some(status))
        }
        Some(_) => Err(shared::AppError::BadRequest("invalid task status".into())),
    }
}

/// GET /api/v2/credit/tasks
pub async fn list_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TasksQuery>,
) -> AppResult<Json<Page<TaskDto>>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let (cursor, limit) = parse_pagination(params.cursor.as_deref(), params.limit)?;
    let status = task_status_filter(params.status.as_deref())?;
    let page = repo::list_tasks(&state.db, status, cursor, limit).await?;

    let items: Vec<TaskDto> = page
        .items
        .into_iter()
        .map(|r| TaskDto {
            id: r.id.to_string(),
            creator_id: r.creator_id.to_string(),
            acceptor_id: r.acceptor_id.map(|v| v.to_string()),
            title: r.title,
            description: r.description,
            reward_amount: r.reward_amount,
            // Only expose contact_info to the creator or the accepted worker.
            contact_info: if auth.id == r.creator_id || r.acceptor_id == Some(auth.id) {
                r.contact_info
            } else {
                None
            },
            status: r.status,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/credit/tasks — create a new task with escrow_hold.
///
/// Atomic: wraps escrow_hold + insert_task in a single transaction.
/// Requires `X-Wallet-Sig` header.
#[tracing::instrument(skip(state, headers, body))]
pub(crate) async fn create_task(
    State(state): State<CreditState>,
    headers: HeaderMap,
    Json(body): Json<TaskInput>,
) -> AppResult<(StatusCode, Json<TaskDto>)> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    shared::ratelimit::check_token_bucket(
        state.app.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;

    if body.reward_amount <= 0 {
        return Err(shared::AppError::BadRequest("reward_amount must be positive".into()));
    }

    let request = serde_json::to_value(&body)
        .map_err(|error| shared::AppError::Internal(anyhow::Error::new(error)))?;
    let mut tx = state.app.db.begin().await?;
    lock_actor_for_write(&state, &mut tx, auth.id).await?;
    let consumed = crate::signing::consume_intent(
        &mut tx,
        state.wallet_key_resolver.as_ref(),
        &headers,
        auth.id,
        "credit.task.create",
        &request,
    )
    .await?;
    let wallet_balance = repo::lock_wallet_for_debit_tx(&mut tx, auth.id).await?;
    if wallet_balance < body.reward_amount {
        return Err(CreditError::InsufficientBalance.into());
    }
    let hold_tx_id =
        consumed.ledger_entry.as_ref().ok_or(CreditError::InvalidSignature)?.tx_id.clone();
    append_consumed_user_ledger(&mut tx, auth.id, "escrow_hold", consumed).await?;

    let task = repo::insert_task_tx(
        &mut tx,
        auth.id,
        &body.title,
        body.description.as_deref(),
        body.reward_amount,
        body.contact_info.as_deref(),
        &hold_tx_id,
    )
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(TaskDto {
            id: task.id.to_string(),
            creator_id: task.creator_id.to_string(),
            acceptor_id: task.acceptor_id.map(|v| v.to_string()),
            title: task.title,
            description: task.description,
            reward_amount: task.reward_amount,
            contact_info: task.contact_info,
            status: task.status,
            created_at: task.created_at.timestamp(),
        }),
    ))
}

/// POST /api/v2/credit/tasks/{id}/accept — acceptor claims a task.
pub(crate) async fn accept_task(
    State(state): State<CreditState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let preview = repo::find_task(&state.app.db, id).await?.ok_or(CreditError::TaskNotFound)?;
    let mut tx = state.app.db.begin().await?;
    if !lock_actor_and_counterparty(&state, &mut tx, auth.id, preview.creator_id).await? {
        return Err(CreditError::TaskNotFound.into());
    }
    let task =
        repo::find_task_for_update_tx(&mut tx, id).await?.ok_or(CreditError::TaskNotFound)?;
    if task.creator_id != preview.creator_id {
        return Err(CreditError::TaskNotFound.into());
    }
    if task.creator_id == auth.id {
        return Err(CreditError::InvalidAction("cannot accept your own task".into()).into());
    }
    if task.status != "open" {
        return Err(CreditError::StateConflict.into());
    }
    repo::accept_task_tx(&mut tx, id, auth.id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v2/credit/tasks/{id}/action — submit/confirm/cancel/reject/delete a task.
///
/// Every transition locks the task and uses a compare-and-set write. Value
/// transitions append their release and consume the hold in the same transaction.
pub(crate) async fn action_task(
    State(state): State<CreditState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<TaskAction>,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let mut tx = state.app.db.begin().await?;
    lock_actor_for_write(&state, &mut tx, auth.id).await?;
    let task =
        repo::find_task_for_update_tx(&mut tx, id).await?.ok_or(CreditError::TaskNotFound)?;

    match body.action.as_str() {
        "submit" => {
            if auth.id != task.acceptor_id.unwrap_or(-1) {
                return Err(CreditError::InvalidAction("only acceptor can submit".into()).into());
            }
            if task.status != "in_progress" {
                return Err(CreditError::InvalidAction("task is not in_progress".into()).into());
            }
            repo::transition_task_status_tx(&mut tx, id, "in_progress", "submitted", false).await?;
        }
        "confirm" => {
            if auth.id != task.creator_id {
                return Err(CreditError::InvalidAction("only creator can confirm".into()).into());
            }
            if task.status != "submitted" {
                return Err(CreditError::InvalidAction("task is not submitted".into()).into());
            }

            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                state.wallet_key_resolver.as_ref(),
                &headers,
                auth.id,
                "credit.task.action",
                &request,
            )
            .await?;
            let hold_tx = task.hold_tx_id.as_deref().ok_or(CreditError::StateConflict)?;
            let acceptor_id = task.acceptor_id.ok_or(CreditError::StateConflict)?;
            let metadata = serde_json::json!({ "task_id": id.to_string(), "hold_tx": hold_tx });
            append_system_release(&mut tx, &state.app, acceptor_id, task.reward_amount, &metadata)
                .await?;
            repo::transition_task_status_tx(&mut tx, id, "submitted", "completed", true).await?;
        }
        "cancel" | "reject" => {
            let allowed = if body.action == "cancel" {
                auth.id == task.creator_id
            } else {
                auth.id == task.acceptor_id.unwrap_or(-1)
            };
            if !allowed {
                return Err(CreditError::InvalidAction("unauthorized action".into()).into());
            }
            if task.status == "completed" || task.status == "cancelled" {
                return Err(CreditError::InvalidAction("task already resolved".into()).into());
            }

            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                state.wallet_key_resolver.as_ref(),
                &headers,
                auth.id,
                "credit.task.action",
                &request,
            )
            .await?;
            let hold_tx = task.hold_tx_id.as_deref().ok_or(CreditError::StateConflict)?;
            let metadata = serde_json::json!(
                { "task_id": id.to_string(), "hold_tx": hold_tx, "reason": &body.action }
            );
            append_system_release(
                &mut tx,
                &state.app,
                task.creator_id,
                task.reward_amount,
                &metadata,
            )
            .await?;
            repo::transition_task_status_tx(&mut tx, id, &task.status, "cancelled", true).await?;
        }
        "delete" => {
            if auth.id != task.creator_id {
                return Err(CreditError::InvalidAction("only creator can delete".into()).into());
            }
            if task.status != "open" && task.status != "cancelled" {
                return Err(CreditError::InvalidAction(
                    "can only delete open/cancelled tasks".into(),
                )
                .into());
            }

            if task.status == "open" {
                let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
                crate::signing::consume_intent(
                    &mut tx,
                    state.wallet_key_resolver.as_ref(),
                    &headers,
                    auth.id,
                    "credit.task.action",
                    &request,
                )
                .await?;
                let hold_tx = task.hold_tx_id.as_deref().ok_or(CreditError::StateConflict)?;
                let metadata = serde_json::json!(
                    { "task_id": id.to_string(), "hold_tx": hold_tx, "reason": "delete" }
                );
                append_system_release(
                    &mut tx,
                    &state.app,
                    task.creator_id,
                    task.reward_amount,
                    &metadata,
                )
                .await?;
            } else if task.hold_tx_id.is_some() {
                return Err(CreditError::StateConflict.into());
            }

            repo::delete_task_tx(&mut tx, id, &task.status).await?;
        }
        _ => {
            return Err(
                CreditError::InvalidAction(format!("unknown action: {}", body.action)).into()
            );
        }
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Products (escrow market — marketplace-style)
// ---------------------------------------------------------------------------

/// Query parameters for GET /api/v2/credit/products.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductsQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn product_status_filter(status: Option<&str>) -> AppResult<Option<&str>> {
    match status {
        None | Some("all") => Ok(None),
        Some(status @ ("on_sale" | "off_sale" | "sold_out")) => Ok(Some(status)),
        Some(_) => Err(shared::AppError::BadRequest("invalid product status".into())),
    }
}

/// GET /api/v2/credit/products
pub async fn list_products(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ProductsQuery>,
) -> AppResult<Json<Page<ProductDto>>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let _ = auth;

    let (cursor, limit) = parse_pagination(params.cursor.as_deref(), params.limit)?;
    let status = product_status_filter(params.status.as_deref())?;
    let page = repo::list_products(&state.db, status, cursor, limit).await?;

    let items: Vec<ProductDto> = page
        .items
        .into_iter()
        .map(|r| ProductDto {
            id: r.id.to_string(),
            seller_id: r.seller_id.to_string(),
            title: r.title,
            description: r.description,
            price: r.price,
            stock: r.stock,
            status: r.status,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/credit/products — list a new product.
pub(crate) async fn create_product(
    State(state): State<CreditState>,
    headers: HeaderMap,
    Json(body): Json<ProductInput>,
) -> AppResult<(StatusCode, Json<ProductDto>)> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    shared::ratelimit::check_token_bucket(
        state.app.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;
    if body.price <= 0 {
        return Err(shared::AppError::BadRequest("price must be positive".into()));
    }
    if body.stock < 0 {
        return Err(shared::AppError::BadRequest("stock must be nonnegative".into()));
    }

    let mut tx = state.app.db.begin().await?;
    lock_actor_for_write(&state, &mut tx, auth.id).await?;
    let product = repo::insert_product_tx(
        &mut tx,
        auth.id,
        &body.title,
        body.description.as_deref(),
        body.price,
        body.stock,
        body.delivery_info.as_deref(),
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(ProductDto {
            id: product.id.to_string(),
            seller_id: product.seller_id.to_string(),
            title: product.title,
            description: product.description,
            price: product.price,
            stock: product.stock,
            status: product.status,
            created_at: product.created_at.timestamp(),
        }),
    ))
}

/// POST /api/v2/credit/products/{id}/purchase
///
/// Atomic: escrow_hold + decrement_stock + status update + insert_purchase
/// in a single transaction. Requires `X-Wallet-Sig`.
#[tracing::instrument(skip(state, headers))]
pub(crate) async fn purchase_product(
    State(state): State<CreditState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<(StatusCode, Json<PurchaseDto>)> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    shared::ratelimit::check_token_bucket(
        state.app.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;

    let request = serde_json::json!({ "productId": id.to_string() });
    let preview =
        repo::find_product(&state.app.db, id).await?.ok_or(CreditError::ProductNotFound)?;
    let mut tx = state.app.db.begin().await?;
    if !lock_actor_and_counterparty(&state, &mut tx, auth.id, preview.seller_id).await? {
        return Err(CreditError::ProductNotFound.into());
    }
    let product =
        repo::find_product_for_update_tx(&mut tx, id).await?.ok_or(CreditError::ProductNotFound)?;
    if product.seller_id != preview.seller_id {
        return Err(CreditError::ProductNotFound.into());
    }
    if product.status != "on_sale" {
        return Err(CreditError::InvalidAction("product is not on_sale".into()).into());
    }
    if product.stock <= 0 {
        return Err(CreditError::InvalidAction("product is sold out".into()).into());
    }
    if product.seller_id == auth.id {
        return Err(CreditError::InvalidAction("cannot purchase your own product".into()).into());
    }
    let consumed = crate::signing::consume_intent(
        &mut tx,
        state.wallet_key_resolver.as_ref(),
        &headers,
        auth.id,
        "credit.product.purchase",
        &request,
    )
    .await?;
    let wallet_balance = repo::lock_wallet_for_debit_tx(&mut tx, auth.id).await?;
    if wallet_balance < product.price {
        return Err(CreditError::InsufficientBalance.into());
    }
    let hold_tx_id =
        consumed.ledger_entry.as_ref().ok_or(CreditError::InvalidSignature)?.tx_id.clone();
    append_consumed_user_ledger(&mut tx, auth.id, "escrow_hold", consumed).await?;

    repo::decrement_stock_tx(&mut tx, id).await?;
    if product.stock <= 1 {
        repo::update_product_status_tx(&mut tx, id, "sold_out").await?;
    }
    let purchase = repo::insert_purchase_tx(
        &mut tx,
        id,
        auth.id,
        product.seller_id,
        product.price,
        &hold_tx_id,
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(PurchaseDto {
            id: purchase.id.to_string(),
            product_id: purchase.product_id.to_string(),
            buyer_id: purchase.buyer_id.to_string(),
            seller_id: purchase.seller_id.to_string(),
            amount: purchase.amount,
            status: purchase.status,
            delivery_info: product.delivery_info,
            created_at: purchase.created_at.timestamp(),
        }),
    ))
}

/// Query for GET /api/v2/credit/purchases.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasesQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// GET /api/v2/credit/purchases
pub async fn list_purchases(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<PurchasesQuery>,
) -> AppResult<Json<Page<PurchaseDto>>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let (cursor, limit) = parse_pagination(params.cursor.as_deref(), params.limit)?;
    let page = repo::list_purchases(&state.db, auth.id, cursor, limit).await?;

    let items: Vec<PurchaseDto> = page
        .items
        .into_iter()
        .map(|r| PurchaseDto {
            id: r.id.to_string(),
            product_id: r.product_id.to_string(),
            buyer_id: r.buyer_id.to_string(),
            seller_id: r.seller_id.to_string(),
            amount: r.amount,
            status: r.status,
            delivery_info: r.delivery_info,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/credit/purchases/{id}/action — accept/deliver/confirm/cancel
///
/// Every transition locks the purchase and uses a compare-and-set write.
/// confirm/cancel append the release and consume the hold atomically.
pub(crate) async fn action_purchase(
    State(state): State<CreditState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<PurchaseAction>,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.app.db, &state.app.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let mut tx = state.app.db.begin().await?;
    lock_actor_for_write(&state, &mut tx, auth.id).await?;
    let purchase = repo::find_purchase_for_update_tx(&mut tx, id)
        .await?
        .ok_or(CreditError::PurchaseNotFound)?;
    if auth.id != purchase.buyer_id && auth.id != purchase.seller_id {
        return Err(CreditError::PurchaseNotFound.into());
    }

    match body.action.as_str() {
        "accept" => {
            if auth.id != purchase.seller_id {
                return Err(CreditError::InvalidAction("only seller can accept".into()).into());
            }
            if purchase.status != "pending" {
                return Err(CreditError::StateConflict.into());
            }
            repo::transition_purchase_status_tx(&mut tx, id, "pending", "accepted", false).await?;
        }
        "deliver" => {
            if auth.id != purchase.seller_id {
                return Err(CreditError::InvalidAction("only seller can deliver".into()).into());
            }
            if purchase.status != "accepted" {
                return Err(CreditError::StateConflict.into());
            }
            repo::transition_purchase_status_tx(&mut tx, id, "accepted", "delivered", false)
                .await?;
        }
        "confirm" => {
            if auth.id != purchase.buyer_id {
                return Err(CreditError::InvalidAction("only buyer can confirm".into()).into());
            }
            if purchase.status != "delivered" {
                return Err(CreditError::InvalidAction("purchase is not delivered".into()).into());
            }

            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                state.wallet_key_resolver.as_ref(),
                &headers,
                auth.id,
                "credit.purchase.action",
                &request,
            )
            .await?;
            let hold_tx = purchase.hold_tx_id.as_deref().ok_or(CreditError::StateConflict)?;
            let metadata = serde_json::json!({ "purchase_id": id.to_string(), "hold_tx": hold_tx });
            append_system_release(
                &mut tx,
                &state.app,
                purchase.seller_id,
                purchase.amount,
                &metadata,
            )
            .await?;
            repo::transition_purchase_status_tx(&mut tx, id, "delivered", "completed", true)
                .await?;
        }
        "cancel" => {
            if auth.id != purchase.buyer_id {
                return Err(CreditError::InvalidAction("only buyer can cancel".into()).into());
            }
            if purchase.status != "pending" && purchase.status != "accepted" {
                return Err(
                    CreditError::InvalidAction("purchase cannot be cancelled".into()).into()
                );
            }

            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                state.wallet_key_resolver.as_ref(),
                &headers,
                auth.id,
                "credit.purchase.action",
                &request,
            )
            .await?;
            let hold_tx = purchase.hold_tx_id.as_deref().ok_or(CreditError::StateConflict)?;
            let metadata = serde_json::json!(
                { "purchase_id": id.to_string(), "hold_tx": hold_tx, "reason": "cancelled" }
            );
            repo::transition_purchase_status_tx(&mut tx, id, &purchase.status, "cancelled", true)
                .await?;
            repo::restore_cancelled_purchase_stock_tx(&mut tx, purchase.product_id).await?;
            append_system_release(
                &mut tx,
                &state.app,
                purchase.buyer_id,
                purchase.amount,
                &metadata,
            )
            .await?;
        }
        _ => {
            return Err(
                CreditError::InvalidAction(format!("unknown action: {}", body.action)).into()
            );
        }
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;

    use super::map_auth_err;

    #[test]
    fn preserves_authentication_error_classification() {
        let unauthorized = map_auth_err(axum::http::StatusCode::UNAUTHORIZED.into_response());
        assert!(matches!(unauthorized, shared::AppError::Unauthorized));

        let forbidden = map_auth_err(axum::http::StatusCode::FORBIDDEN.into_response());
        assert!(matches!(forbidden, shared::AppError::Forbidden));

        let internal = map_auth_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response());
        assert!(matches!(internal, shared::AppError::Internal(_)));
    }
}

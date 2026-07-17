//! Credit domain — Web2.5 closed-loop points.
//!
//! HARD COMPLIANCE RULES (闭环虚拟权益, see `AGENTS.md` and
//! `docs/architecture/contracts-and-data.md`):
//! - NO recharge, NO withdrawal, NO fiat conversion, NO cashout.
//! - NO unrestricted peer transfer — value only moves inside controlled flows
//!   (tip / bounty / escrow). Do NOT add a free `transfer` endpoint.
//! - Points are earned by contribution only (system-signed `mint`).
//!
//! Ledger model: `credit.ledger` is append-only, monotonic `seq`, `prev_hash`
//! chained, and every entry carries an Ed25519 signature (system key for mint,
//! the account key for tip/bounty/escrow). Balance is a derived cache, never the
//! source of truth. Appends are serialized (advisory lock) to keep the chain linear.

pub mod account_eligibility;
pub mod auth;
pub mod data_export;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod ledger;
pub mod models;
pub mod reconciliation;
pub mod repo;
pub mod signing;
pub mod tip_targets;
pub mod wallet_keys;

use std::sync::Arc;

use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use shared::AppState;
use sqlx::PgPool;

/// Router state that keeps cross-domain owner APIs at the application boundary.
#[derive(Clone)]
pub(crate) struct CreditState {
    app: AppState,
    account_eligibility_resolver: Arc<dyn account_eligibility::AccountEligibilityResolver>,
    tip_target_resolver: Arc<dyn tip_targets::TipTargetResolver>,
    wallet_key_resolver: Arc<dyn wallet_keys::WalletKeyResolver>,
}

impl FromRef<CreditState> for AppState {
    fn from_ref(state: &CreditState) -> Self {
        state.app.clone()
    }
}

/// System-signed mint for contributions with idempotency protection.
///
/// Before minting, checks `SELECT seq, tx_id, type, from_account, to_account,
/// amount, nonce, metadata, signer, signature, prev_hash, hash, created_at
/// FROM credit.ledger WHERE tx_id = $1`. If a matching entry exists, returns
/// it without minting again.
///
/// # Panics
/// Never panics — the query and insert are handled via `?`.
pub async fn mint_for_contribution(
    pool: &PgPool,
    account_id: i64,
    amount: i64,
    idempotency_key: &str,
    reason: &str,
    system_seed: &[u8],
) -> shared::AppResult<crate::models::LedgerEntryRow> {
    // Check idempotency — already minted?
    let existing = sqlx::query_as::<_, crate::models::LedgerEntryRow>(
        "SELECT seq, tx_id, type, from_account, to_account, \
                amount, nonce, metadata, signer, signature, prev_hash, hash, created_at \
         FROM credit.ledger WHERE tx_id = $1",
    )
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = existing {
        return Ok(row);
    }

    // Not yet minted — mint with the idempotency key as the ledger `tx_id` so
    // the pre-check above matches on retry and the `tx_id` UNIQUE constraint is
    // a hard backstop against double-minting.
    repo::mint_points_with_tx_id(pool, account_id, amount, idempotency_key, reason, system_seed)
        .await
}

/// All routes owned by the credit domain.
pub fn routes(
    state: AppState,
    account_eligibility_resolver: Arc<dyn account_eligibility::AccountEligibilityResolver>,
    tip_target_resolver: Arc<dyn tip_targets::TipTargetResolver>,
    wallet_key_resolver: Arc<dyn wallet_keys::WalletKeyResolver>,
) -> Router {
    Router::new()
        // Wallet
        .route("/api/v2/wallet", get(handlers::get_wallet))
        .route("/api/v2/credit/signing-intents", post(handlers::create_signing_intent))
        .route("/api/v2/credit/signing-intent-outcome", post(handlers::get_signing_intent_outcome))
        // Canonical: POST /api/v2/credit/tip — alias: POST /api/v2/wallet/tip
        .route("/api/v2/credit/tip", post(handlers::tip))
        .route("/api/v2/wallet/tip", post(handlers::tip))
        .route("/api/v2/wallet/ledger", get(handlers::get_ledger))
        .route("/api/v2/wallet/ledger/verify", get(handlers::verify_ledger))
        .route(
            "/api/v2/admin/credit/reconciliations",
            get(handlers::list_reconciliation_runs).post(handlers::request_reconciliation_run),
        )
        .route("/api/v2/admin/credit/reconciliations/stats", get(handlers::reconciliation_stats))
        .route("/api/v2/admin/credit/reconciliations/{id}", get(handlers::get_reconciliation_run))
        .route(
            "/api/v2/admin/credit/reconciliations/{id}/resume",
            post(handlers::resume_reconciliation_run),
        )
        .route(
            "/api/v2/admin/credit/reconciliations/{id}/wallets",
            get(handlers::list_reconciliation_wallets),
        )
        // Tasks
        .route("/api/v2/credit/tasks", get(handlers::list_tasks).post(handlers::create_task))
        .route("/api/v2/credit/tasks/{id}/accept", post(handlers::accept_task))
        .route("/api/v2/credit/tasks/{id}/action", post(handlers::action_task))
        // Products
        .route(
            "/api/v2/credit/products",
            get(handlers::list_products).post(handlers::create_product),
        )
        .route("/api/v2/credit/products/{id}/purchase", post(handlers::purchase_product))
        // Purchases
        .route("/api/v2/credit/purchases", get(handlers::list_purchases))
        .route("/api/v2/credit/purchases/{id}/action", post(handlers::action_purchase))
        .with_state(CreditState {
            app: state,
            account_eligibility_resolver,
            tip_target_resolver,
            wallet_key_resolver,
        })
}

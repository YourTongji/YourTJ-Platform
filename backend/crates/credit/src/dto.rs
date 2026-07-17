//! Request and response types for the credit domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// Request for exact bytes authorizing one credit operation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningIntentInput {
    pub action: String,
    pub request: serde_json::Value,
}

/// One-time signing challenge bound to the authenticated account and request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningIntentOutput {
    pub intent_id: String,
    pub signing_bytes: String,
    pub expires_at: i64,
}

/// Owner request for one signing intent's bounded outcome.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigningIntentOutcomeInput {
    pub intent_id: String,
}

/// Owner-visible state of a one-time signing intent.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningIntentOutcomeDto {
    pub intent_id: String,
    pub status: SigningIntentStatus,
    pub expires_at: i64,
}

/// Stable outcome states exposed without returning signing material.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningIntentStatus {
    Pending,
    Committed,
    Expired,
}

/// GET /wallet
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDto {
    pub account_id: String,
    pub balance: i64,
    pub active_public_key: Option<String>,
}

/// A single entry in the public ledger view.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntryDto {
    pub seq: i64,
    pub tx_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub from_account: Option<String>,
    pub to_account: Option<String>,
    pub amount: i64,
    pub nonce: String,
    pub metadata: Option<serde_json::Value>,
    pub signer: String,
    pub prev_hash: String,
    pub hash: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
}

/// Returned by the ledger verify endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerVerify {
    pub ok: bool,
    pub latest_seq: Option<i64>,
    pub latest_hash: Option<String>,
}

/// Reason supplied by an administrator requesting a read-only integrity run.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReconciliationRunInput {
    pub reason: String,
}

/// Persistent state and summary metrics for one ledger reconciliation run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationRunDto {
    pub id: String,
    pub status: String,
    pub requested_by: String,
    pub reason: String,
    pub ledger_ok: Option<bool>,
    pub ledger_latest_seq: Option<i64>,
    pub ledger_latest_hash: Option<String>,
    pub ledger_failure_seq: Option<i64>,
    pub wallets_checked: i64,
    pub drifted_wallets: i64,
    pub missing_wallets: i64,
    pub balance_drifted_wallets: i64,
    pub sequence_drifted_wallets: i64,
    pub total_absolute_drift: String,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

/// One persisted wallet projection comparison from a reconciliation run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationWalletDto {
    pub account_id: String,
    pub expected_balance: String,
    pub actual_balance: Option<String>,
    pub delta: String,
    pub expected_last_seq: i64,
    pub actual_last_seq: Option<i64>,
    pub wallet_exists: bool,
    pub has_balance_drift: bool,
    pub has_sequence_drift: bool,
}

/// Aggregate health counters and the newest persisted reconciliation run.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationStatsDto {
    pub total_runs: i64,
    pub failed_runs: i64,
    pub ledger_failure_runs: i64,
    pub runs_with_drift: i64,
    pub latest_run: Option<ReconciliationRunDto>,
}

/// POST /wallet/tip
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TipInput {
    pub to_account_id: String,
    pub amount: i64,
    pub target_type: String,
    pub target_id: String,
}

/// Public task listing DTO.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub creator_id: String,
    pub acceptor_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub reward_amount: i64,
    pub contact_info: Option<String>,
    pub status: String,
    pub created_at: i64,
}

/// POST /credit/tasks
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    pub description: Option<String>,
    pub reward_amount: i64,
    pub contact_info: Option<String>,
}

/// POST /credit/tasks/{id}/action
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAction {
    pub action: String,
}

/// Public product listing DTO.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductDto {
    pub id: String,
    pub seller_id: String,
    pub title: String,
    pub description: Option<String>,
    pub price: i64,
    pub stock: i32,
    pub status: String,
    pub created_at: i64,
}

/// POST /credit/products
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub title: String,
    pub description: Option<String>,
    pub price: i64,
    pub stock: i32,
    pub delivery_info: Option<String>,
}

/// Public purchase DTO.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseDto {
    pub id: String,
    pub product_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub amount: i64,
    pub status: String,
    pub delivery_info: Option<String>,
    pub created_at: i64,
}

/// POST /credit/purchases/{id}/action
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseAction {
    pub action: String,
}

//! Axum request handlers for the credit domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use shared::{AppResult, AppState, AuthAccount, Page};

use crate::dto::{LedgerEntryDto, LedgerVerify, TipInput, WalletDto};
use crate::error::CreditError;
use crate::ledger::verify_signature;
use crate::repo;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_limit() -> i64 {
    20
}

/// Helper: convert `AuthAccount::from_headers`'s `Response` error into `AppError`.
fn map_auth_err(response: axum::response::Response) -> shared::AppError {
    if response.status() == axum::http::StatusCode::UNAUTHORIZED {
        shared::AppError::Unauthorized
    } else {
        shared::AppError::Forbidden
    }
}

// ---------------------------------------------------------------------------
// Wallet
// ---------------------------------------------------------------------------

/// GET /api/v2/wallet — authenticated wallet balance.
pub async fn get_wallet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<WalletDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let wallet = repo::get_wallet(&state.db, auth.id).await?;
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
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let cursor = params.cursor.as_deref().and_then(|c| c.parse::<i64>().ok());
    let page = repo::list_ledger(&state.db, auth.id, cursor, params.limit).await?;
    Ok(Json(page))
}

/// GET /api/v2/wallet/ledger/verify — public verification result.
pub async fn verify_ledger(State(state): State<AppState>) -> AppResult<Json<LedgerVerify>> {
    let result = repo::verify_full_ledger(&state.db).await?;
    Ok(Json(result))
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
pub async fn tip(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TipInput>,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let to_account_id: i64 = body
        .to_account_id
        .parse()
        .map_err(|_| shared::AppError::BadRequest("invalid to_account_id".into()))?;

    if body.amount <= 0 {
        return Err(shared::AppError::BadRequest("amount must be positive".into()));
    }

    // Fetch the signer's Ed25519 public key.
    let pk_row: Option<(String,)> =
        sqlx::query_as("SELECT public_key FROM identity.account_keys WHERE account_id = $1")
            .bind(auth.id)
            .fetch_optional(&state.db)
            .await?;

    let (public_key,) = pk_row.ok_or(CreditError::WalletNotBound)?;

    // Extract X-Wallet-Sig header.
    let sig_b64 = headers
        .get("x-wallet-sig")
        .and_then(|v| v.to_str().ok())
        .ok_or(CreditError::InvalidSignature)?;

    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = Utc::now().timestamp();

    // Build the canonical payload that the wallet app signed.
    let payload = serde_json::json!({
        "tx_id": tx_id,
        "type": "tip",
        "from": auth.id.to_string(),
        "to": to_account_id.to_string(),
        "amount": body.amount,
        "target_type": body.target_type,
        "target_id": body.target_id,
        "nonce": nonce,
        "timestamp": timestamp,
    });
    let canonical = crate::ledger::canonicalize(&payload);

    // Verify signature.
    if !verify_signature(&canonical, sig_b64, &public_key) {
        return Err(CreditError::InvalidSignature.into());
    }

    // Check balance.
    let wallet = repo::get_wallet(&state.db, auth.id).await?;
    if wallet.balance < body.amount {
        return Err(CreditError::InsufficientBalance.into());
    }

    // Ensure recipient wallet exists.
    repo::ensure_wallet_exists(&state.db, to_account_id).await?;

    let metadata = serde_json::json!({
        "target_type": body.target_type,
        "target_id": body.target_id,
    });

    // Append the ledger entry.
    repo::append_ledger_entry(
        &state.db,
        &tx_id,
        "tip",
        Some(auth.id),
        Some(to_account_id),
        body.amount,
        &nonce,
        Some(metadata),
        &auth.id.to_string(),
        sig_b64,
    )
    .await?;

    tracing::info!(
        from = auth.id,
        to = to_account_id,
        amount = body.amount,
        "tip processed"
    );

    Ok(StatusCode::NO_CONTENT)
}

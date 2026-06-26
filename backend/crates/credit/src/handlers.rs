//! Axum request handlers for the credit domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use shared::{AppResult, AppState, AuthAccount, Page};

use crate::dto::{LedgerEntryDto, LedgerVerify, WalletDto};
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

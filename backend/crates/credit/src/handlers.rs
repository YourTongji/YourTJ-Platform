//! Axum request handlers for the credit domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use shared::{AppResult, AppState, AuthAccount, Page};

use crate::dto::{
    LedgerEntryDto, LedgerVerify, ProductDto, ProductInput, PurchaseAction, PurchaseDto,
    TaskAction, TaskDto, TaskInput, TipInput, WalletDto,
};
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

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
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

/// GET /api/v2/credit/tasks
pub async fn list_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TasksQuery>,
) -> AppResult<Json<Page<TaskDto>>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let _ = auth; // listing is public for authenticated users

    let cursor = params.cursor.as_deref().and_then(|c| c.parse::<i64>().ok());
    let page = repo::list_tasks(&state.db, params.status.as_deref(), cursor, params.limit).await?;

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
            contact_info: r.contact_info,
            status: r.status,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/credit/tasks — create a new task with escrow_hold.
#[tracing::instrument(skip(state, headers, body))]
pub async fn create_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TaskInput>,
) -> AppResult<Json<TaskDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;

    if body.reward_amount <= 0 {
        return Err(shared::AppError::BadRequest("reward_amount must be positive".into()));
    }

    // Verify balance.
    let wallet = repo::get_wallet(&state.db, auth.id).await?;
    if wallet.balance < body.reward_amount {
        return Err(CreditError::InsufficientBalance.into());
    }

    // Escrow hold: lock the reward in the ledger.
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let metadata = serde_json::json!({
        "title": body.title,
    });

    repo::append_ledger_entry(
        &state.db,
        &tx_id,
        "escrow_hold",
        Some(auth.id),
        None,
        body.reward_amount,
        &nonce,
        Some(metadata),
        "system",
        "system-signed",
    )
    .await?;

    // Insert the task row.
    let task = repo::insert_task(
        &state.db,
        auth.id,
        &body.title,
        body.description.as_deref(),
        body.reward_amount,
        body.contact_info.as_deref(),
        &tx_id,
    )
    .await?;

    Ok(Json(TaskDto {
        id: task.id.to_string(),
        creator_id: task.creator_id.to_string(),
        acceptor_id: task.acceptor_id.map(|v| v.to_string()),
        title: task.title,
        description: task.description,
        reward_amount: task.reward_amount,
        contact_info: task.contact_info,
        status: task.status,
        created_at: task.created_at.timestamp(),
    }))
}

/// POST /api/v2/credit/tasks/{id}/accept — acceptor claims a task.
pub async fn accept_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let task = repo::find_task(&state.db, id).await?.ok_or(CreditError::TaskNotFound)?;
    if task.status != "open" {
        return Err(CreditError::InvalidAction("task is not open".into()).into());
    }

    repo::accept_task(&state.db, id, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v2/credit/tasks/{id}/action — submit/confirm/cancel/reject/delete a task.
pub async fn action_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<TaskAction>,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let task = repo::find_task(&state.db, id).await?.ok_or(CreditError::TaskNotFound)?;

    match body.action.as_str() {
        "submit" => {
            if auth.id != task.acceptor_id.unwrap_or(-1) {
                return Err(CreditError::InvalidAction("only acceptor can submit".into()).into());
            }
            if task.status != "in_progress" {
                return Err(CreditError::InvalidAction("task is not in_progress".into()).into());
            }
            repo::update_task_status(&state.db, id, "submitted", auth.id).await?;
        }
        "confirm" => {
            if auth.id != task.creator_id {
                return Err(CreditError::InvalidAction("only creator can confirm".into()).into());
            }
            if task.status != "submitted" {
                return Err(CreditError::InvalidAction("task is not submitted".into()).into());
            }

            // Release escrow: escrow_release ledger entry.
            if let Some(hold_tx) = &task.hold_tx_id {
                let release_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let metadata = serde_json::json!({ "task_id": id.to_string(), "hold_tx": hold_tx });

                repo::append_ledger_entry(
                    &state.db,
                    &release_tx_id,
                    "escrow_release",
                    None,
                    task.acceptor_id,
                    task.reward_amount,
                    &nonce,
                    Some(metadata),
                    "system",
                    "system-signed",
                )
                .await?;
            }

            repo::update_task_status(&state.db, id, "completed", auth.id).await?;
            repo::clear_task_hold(&state.db, id).await?;
        }
        "cancel" | "reject" => {
            let allowed = auth.id == task.creator_id
                || (body.action == "reject" && auth.id == task.acceptor_id.unwrap_or(-1));
            if !allowed {
                return Err(CreditError::InvalidAction("unauthorized action".into()).into());
            }
            if task.status == "completed" || task.status == "cancelled" {
                return Err(CreditError::InvalidAction("task already resolved".into()).into());
            }

            // Refund: escrow_release back to creator.
            if let Some(hold_tx) = &task.hold_tx_id {
                let refund_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let metadata = serde_json::json!({ "task_id": id.to_string(), "hold_tx": hold_tx, "reason": &body.action });

                repo::append_ledger_entry(
                    &state.db,
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(task.creator_id),
                    task.reward_amount,
                    &nonce,
                    Some(metadata),
                    "system",
                    "system-signed",
                )
                .await?;
            }

            let new_status = "cancelled";
            repo::update_task_status(&state.db, id, new_status, auth.id).await?;
            repo::clear_task_hold(&state.db, id).await?;
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
            repo::update_task_status(&state.db, id, "delete", auth.id).await?;
        }
        _ => {
            return Err(
                CreditError::InvalidAction(format!("unknown action: {}", body.action)).into()
            );
        }
    }

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

/// GET /api/v2/credit/products
pub async fn list_products(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ProductsQuery>,
) -> AppResult<Json<Page<ProductDto>>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let _ = auth;

    let cursor = params.cursor.as_deref().and_then(|c| c.parse::<i64>().ok());
    let page =
        repo::list_products(&state.db, params.status.as_deref(), cursor, params.limit).await?;

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
            delivery_info: r.delivery_info,
            status: r.status,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/credit/products — list a new product.
pub async fn create_product(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ProductInput>,
) -> AppResult<Json<ProductDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;
    if body.price <= 0 {
        return Err(shared::AppError::BadRequest("price must be positive".into()));
    }

    let product = repo::insert_product(
        &state.db,
        auth.id,
        &body.title,
        body.description.as_deref(),
        body.price,
        body.stock,
        body.delivery_info.as_deref(),
    )
    .await?;

    Ok(Json(ProductDto {
        id: product.id.to_string(),
        seller_id: product.seller_id.to_string(),
        title: product.title,
        description: product.description,
        price: product.price,
        stock: product.stock,
        delivery_info: product.delivery_info,
        status: product.status,
        created_at: product.created_at.timestamp(),
    }))
}

/// POST /api/v2/credit/products/{id}/purchase
#[tracing::instrument(skip(state, headers))]
pub async fn purchase_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<Json<PurchaseDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    // Rate-limit credit operations: 20 per 60 seconds per account.
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "transfer",
        &auth.id.to_string(),
        20,
        60,
    )
    .await?;

    let product = repo::find_product(&state.db, id).await?.ok_or(CreditError::ProductNotFound)?;

    if product.status != "on_sale" {
        return Err(CreditError::InvalidAction("product is not on_sale".into()).into());
    }
    if product.stock <= 0 {
        return Err(CreditError::InvalidAction("product is sold out".into()).into());
    }
    if product.seller_id == auth.id {
        return Err(CreditError::InvalidAction("cannot purchase your own product".into()).into());
    }

    // Check balance.
    let wallet = repo::get_wallet(&state.db, auth.id).await?;
    if wallet.balance < product.price {
        return Err(CreditError::InsufficientBalance.into());
    }

    // Escrow hold.
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let metadata = serde_json::json!({ "product_id": id.to_string(), "title": product.title });

    repo::append_ledger_entry(
        &state.db,
        &tx_id,
        "escrow_hold",
        Some(auth.id),
        None,
        product.price,
        &nonce,
        Some(metadata),
        "system",
        "system-signed",
    )
    .await?;

    repo::decrement_stock(&state.db, id).await?;
    if product.stock <= 1 {
        repo::update_product_status(&state.db, id, "sold_out").await?;
    }

    // Insert purchase row.
    let purchase =
        repo::insert_purchase(&state.db, id, auth.id, product.seller_id, product.price, &tx_id)
            .await?;

    Ok(Json(PurchaseDto {
        id: purchase.id.to_string(),
        product_id: purchase.product_id.to_string(),
        buyer_id: purchase.buyer_id.to_string(),
        seller_id: purchase.seller_id.to_string(),
        amount: purchase.amount,
        status: purchase.status,
    }))
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
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let cursor = params.cursor.as_deref().and_then(|c| c.parse::<i64>().ok());
    let page = repo::list_purchases(&state.db, auth.id, cursor, params.limit).await?;

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
        })
        .collect();

    Ok(Json(Page::new(items, page.next_cursor)))
}

/// POST /api/v2/credit/purchases/{id}/action — accept/deliver/confirm/cancel
pub async fn action_purchase(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<PurchaseAction>,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

    let purchase = repo::find_purchase(&state.db, id).await?.ok_or(CreditError::ProductNotFound)?;

    match body.action.as_str() {
        "accept" => {
            if auth.id != purchase.seller_id {
                return Err(CreditError::InvalidAction("only seller can accept".into()).into());
            }
            if purchase.status != "pending" {
                return Err(CreditError::InvalidAction("purchase is not pending".into()).into());
            }
            repo::update_purchase_status(&state.db, id, "accepted").await?;
        }
        "deliver" => {
            if auth.id != purchase.seller_id {
                return Err(CreditError::InvalidAction("only seller can deliver".into()).into());
            }
            if purchase.status != "accepted" {
                return Err(CreditError::InvalidAction("purchase is not accepted".into()).into());
            }
            repo::update_purchase_status(&state.db, id, "delivered").await?;
        }
        "confirm" => {
            if auth.id != purchase.buyer_id {
                return Err(CreditError::InvalidAction("only buyer can confirm".into()).into());
            }
            if purchase.status != "delivered" {
                return Err(CreditError::InvalidAction("purchase is not delivered".into()).into());
            }

            // Release escrow to seller.
            if let Some(hold_tx) = &purchase.hold_tx_id {
                let release_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let metadata =
                    serde_json::json!({ "purchase_id": id.to_string(), "hold_tx": hold_tx });

                repo::append_ledger_entry(
                    &state.db,
                    &release_tx_id,
                    "escrow_release",
                    None,
                    Some(purchase.seller_id),
                    purchase.amount,
                    &nonce,
                    Some(metadata),
                    "system",
                    "system-signed",
                )
                .await?;
            }

            repo::update_purchase_status(&state.db, id, "completed").await?;
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

            // Refund to buyer.
            if let Some(hold_tx) = &purchase.hold_tx_id {
                let refund_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let metadata = serde_json::json!({ "purchase_id": id.to_string(), "hold_tx": hold_tx, "reason": "cancelled" });

                repo::append_ledger_entry(
                    &state.db,
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(purchase.buyer_id),
                    purchase.amount,
                    &nonce,
                    Some(metadata),
                    "system",
                    "system-signed",
                )
                .await?;
            }

            repo::update_purchase_status(&state.db, id, "cancelled").await?;
        }
        _ => {
            return Err(
                CreditError::InvalidAction(format!("unknown action: {}", body.action)).into()
            );
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

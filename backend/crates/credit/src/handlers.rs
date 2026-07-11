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
    SigningIntentInput, SigningIntentOutput, TaskAction, TaskDto, TaskInput, TipInput, WalletDto,
};
use crate::error::CreditError;
use crate::repo;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_limit() -> i64 {
    20
}

/// Helper: convert the `Response` error from `authenticate` into `AppError`.
fn map_auth_err(response: axum::response::Response) -> shared::AppError {
    if response.status() == axum::http::StatusCode::UNAUTHORIZED {
        shared::AppError::Unauthorized
    } else {
        shared::AppError::Forbidden
    }
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

/// POST /api/v2/credit/signing-intents — return exact bytes for wallet signing.
pub async fn create_signing_intent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SigningIntentInput>,
) -> AppResult<Json<SigningIntentOutput>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|header| header.to_str().ok())
        .filter(|header| !header.is_empty())
        .ok_or(CreditError::IntentUnavailable)?;
    Ok(Json(crate::signing::create_intent(&state.db, auth.id, &body, idempotency_key).await?))
}

// ---------------------------------------------------------------------------
// Wallet
// ---------------------------------------------------------------------------

/// GET /api/v2/wallet — authenticated wallet balance.
pub async fn get_wallet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<WalletDto>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;
    let cursor = params.cursor.as_deref().and_then(|c| c.parse::<i64>().ok());
    let page = repo::list_ledger(&state.db, auth.id, cursor, params.limit).await?;
    Ok(Json(page))
}

/// GET /api/v2/wallet/ledger/verify — public verification result.
pub async fn verify_ledger(State(state): State<AppState>) -> AppResult<Json<LedgerVerify>> {
    let result = repo::verify_full_ledger(&state.db, &state.system_public_key_b64).await?;
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
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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

    let request = serde_json::to_value(&body)
        .map_err(|error| shared::AppError::Internal(anyhow::Error::new(error)))?;
    let mut tx = state.db.begin().await?;
    let consumed =
        crate::signing::consume_intent(&mut tx, &headers, auth.id, "credit.tip", &request).await?;
    let wallet_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $1 FOR UPDATE), 0)",
    )
    .bind(auth.id)
    .fetch_one(&mut *tx)
    .await?;
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

/// GET /api/v2/credit/tasks
pub async fn list_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TasksQuery>,
) -> AppResult<Json<Page<TaskDto>>> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(map_auth_err)?;

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
pub async fn create_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TaskInput>,
) -> AppResult<(StatusCode, Json<TaskDto>)> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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

    let request = serde_json::to_value(&body)
        .map_err(|error| shared::AppError::Internal(anyhow::Error::new(error)))?;
    let mut tx = state.db.begin().await?;
    let consumed =
        crate::signing::consume_intent(&mut tx, &headers, auth.id, "credit.task.create", &request)
            .await?;
    let wallet_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $1 FOR UPDATE), 0)",
    )
    .bind(auth.id)
    .fetch_one(&mut *tx)
    .await?;
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
pub async fn accept_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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
///
/// confirm/cancel/reject/delete are atomic: escrow_release + status update (or
/// row deletion) in a single transaction. Access is gated by JWT auth; ledger
/// entries are system-signed.
pub async fn action_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<TaskAction>,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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

            // Atomic: escrow_release + status update + hold clear.
            let mut tx = state.db.begin().await?;
            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                &headers,
                auth.id,
                "credit.task.action",
                &request,
            )
            .await?;

            if let Some(hold_tx) = &task.hold_tx_id {
                let release_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let created_at = Utc::now().timestamp();
                let metadata = serde_json::json!({ "task_id": id.to_string(), "hold_tx": hold_tx });

                let canonical = crate::ledger::build_ledger_canonical(
                    &release_tx_id,
                    "escrow_release",
                    None,
                    task.acceptor_id,
                    task.reward_amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    created_at,
                );
                let system_sig =
                    crate::ledger::sign_with_seed(&canonical, &state.system_private_key);

                repo::append_ledger_entry_tx(
                    &mut tx,
                    &release_tx_id,
                    "escrow_release",
                    None,
                    task.acceptor_id,
                    task.reward_amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    &system_sig,
                    created_at,
                )
                .await?;
            }

            repo::update_task_status_tx(&mut tx, id, "completed").await?;
            repo::clear_task_hold_tx(&mut tx, id).await?;
            tx.commit().await?;
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

            // Atomic: escrow_release refund + status update + hold clear.
            let mut tx = state.db.begin().await?;
            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                &headers,
                auth.id,
                "credit.task.action",
                &request,
            )
            .await?;

            if let Some(hold_tx) = &task.hold_tx_id {
                let refund_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let created_at = Utc::now().timestamp();
                let metadata = serde_json::json!(
                    { "task_id": id.to_string(), "hold_tx": hold_tx, "reason": &body.action }
                );

                let canonical = crate::ledger::build_ledger_canonical(
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(task.creator_id),
                    task.reward_amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    created_at,
                );
                let system_sig =
                    crate::ledger::sign_with_seed(&canonical, &state.system_private_key);

                repo::append_ledger_entry_tx(
                    &mut tx,
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(task.creator_id),
                    task.reward_amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    &system_sig,
                    created_at,
                )
                .await?;
            }

            repo::update_task_status_tx(&mut tx, id, "cancelled").await?;
            repo::clear_task_hold_tx(&mut tx, id).await?;
            tx.commit().await?;
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

            // `credit.task_status` has no `deleted` value — delete removes the
            // row. An `open` task still has funds in escrow, so refund the
            // creator before deleting; a `cancelled` task already had its hold
            // released and cleared, so there is nothing to refund.
            let mut tx = state.db.begin().await?;
            if task.hold_tx_id.is_some() {
                let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
                crate::signing::consume_intent(
                    &mut tx,
                    &headers,
                    auth.id,
                    "credit.task.action",
                    &request,
                )
                .await?;
            }

            if let Some(hold_tx) = &task.hold_tx_id {
                let refund_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let created_at = Utc::now().timestamp();
                let metadata = serde_json::json!(
                    { "task_id": id.to_string(), "hold_tx": hold_tx, "reason": "delete" }
                );

                let canonical = crate::ledger::build_ledger_canonical(
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(task.creator_id),
                    task.reward_amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    created_at,
                );
                let system_sig =
                    crate::ledger::sign_with_seed(&canonical, &state.system_private_key);

                repo::append_ledger_entry_tx(
                    &mut tx,
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(task.creator_id),
                    task.reward_amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    &system_sig,
                    created_at,
                )
                .await?;
            }

            repo::delete_task_tx(&mut tx, id).await?;
            tx.commit().await?;
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
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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
) -> AppResult<(StatusCode, Json<ProductDto>)> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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

    Ok((
        StatusCode::CREATED,
        Json(ProductDto {
            id: product.id.to_string(),
            seller_id: product.seller_id.to_string(),
            title: product.title,
            description: product.description,
            price: product.price,
            stock: product.stock,
            delivery_info: product.delivery_info,
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
pub async fn purchase_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> AppResult<(StatusCode, Json<PurchaseDto>)> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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

    let request = serde_json::json!({ "productId": id.to_string() });
    let mut tx = state.db.begin().await?;
    let consumed = crate::signing::consume_intent(
        &mut tx,
        &headers,
        auth.id,
        "credit.product.purchase",
        &request,
    )
    .await?;
    let product =
        repo::find_product_for_update_tx(&mut tx, id).await?.ok_or(CreditError::ProductNotFound)?;
    if product.status != "on_sale" {
        return Err(CreditError::InvalidAction("product is not on_sale".into()).into());
    }
    if product.stock <= 0 {
        return Err(CreditError::InvalidAction("product is sold out".into()).into());
    }
    if product.seller_id == auth.id {
        return Err(CreditError::InvalidAction("cannot purchase your own product".into()).into());
    }
    let wallet_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $1 FOR UPDATE), 0)",
    )
    .bind(auth.id)
    .fetch_one(&mut *tx)
    .await?;
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
///
/// confirm/cancel are atomic: escrow_release + status update in a single
/// transaction. Requires `X-Wallet-Sig` for value-moving actions.
pub async fn action_purchase(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<PurchaseAction>,
) -> AppResult<StatusCode> {
    let auth = crate::auth::authenticate(&headers, &state.db, &state.jwt_secret)
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

            // Atomic: escrow_release + status update.
            let mut tx = state.db.begin().await?;
            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                &headers,
                auth.id,
                "credit.purchase.action",
                &request,
            )
            .await?;

            if let Some(hold_tx) = &purchase.hold_tx_id {
                let release_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let created_at = Utc::now().timestamp();
                let metadata =
                    serde_json::json!({ "purchase_id": id.to_string(), "hold_tx": hold_tx });

                let canonical = crate::ledger::build_ledger_canonical(
                    &release_tx_id,
                    "escrow_release",
                    None,
                    Some(purchase.seller_id),
                    purchase.amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    created_at,
                );
                let system_sig =
                    crate::ledger::sign_with_seed(&canonical, &state.system_private_key);

                repo::append_ledger_entry_tx(
                    &mut tx,
                    &release_tx_id,
                    "escrow_release",
                    None,
                    Some(purchase.seller_id),
                    purchase.amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    &system_sig,
                    created_at,
                )
                .await?;
            }

            repo::update_purchase_status_tx(&mut tx, id, "completed").await?;
            tx.commit().await?;
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

            // Atomic: escrow_release refund + status update.
            let mut tx = state.db.begin().await?;
            let request = serde_json::json!({ "id": id.to_string(), "action": body.action });
            crate::signing::consume_intent(
                &mut tx,
                &headers,
                auth.id,
                "credit.purchase.action",
                &request,
            )
            .await?;

            if let Some(hold_tx) = &purchase.hold_tx_id {
                let refund_tx_id = uuid::Uuid::new_v4().to_string();
                let nonce = uuid::Uuid::new_v4().to_string();
                let created_at = Utc::now().timestamp();
                let metadata = serde_json::json!(
                    { "purchase_id": id.to_string(), "hold_tx": hold_tx, "reason": "cancelled" }
                );

                let canonical = crate::ledger::build_ledger_canonical(
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(purchase.buyer_id),
                    purchase.amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    created_at,
                );
                let system_sig =
                    crate::ledger::sign_with_seed(&canonical, &state.system_private_key);

                repo::append_ledger_entry_tx(
                    &mut tx,
                    &refund_tx_id,
                    "escrow_release",
                    None,
                    Some(purchase.buyer_id),
                    purchase.amount,
                    &nonce,
                    Some(&metadata),
                    "system",
                    &system_sig,
                    created_at,
                )
                .await?;
            }

            repo::update_purchase_status_tx(&mut tx, id, "cancelled").await?;
            tx.commit().await?;
        }
        _ => {
            return Err(
                CreditError::InvalidAction(format!("unknown action: {}", body.action)).into()
            );
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

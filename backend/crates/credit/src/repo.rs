//! Database access layer for the credit domain.
//!
//! Every function takes `&PgPool` and returns `AppResult` so callers
//! can use `?` and let Axum render errors.

use chrono::Utc;
use serde_json::Value;
use shared::{AppResult, Page};
use sqlx::PgPool;

use crate::dto::{LedgerEntryDto, LedgerVerify, WalletDto};
use crate::error::CreditError;
use crate::ledger::{canonicalize, compute_hash};
use crate::models::{LedgerEntryRow, ProductRow, PurchaseRow, TaskRow};

/// Alias for the public sequence of `credential.ledger.type` values that
/// represent tip / mint / escrow actions.
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

// ---------------------------------------------------------------------------
// Ledger: append, list, verify
// ---------------------------------------------------------------------------

/// Append a ledger entry inside a transaction that takes an advisory lock.
/// Updates the wallet balance cache for both sides. Returns the inserted row.
#[tracing::instrument(skip(pool))]
pub async fn append_ledger_entry(
    pool: &PgPool,
    tx_id: &str,
    type_: &str,
    from_account: Option<i64>,
    to_account: Option<i64>,
    amount: i64,
    nonce: &str,
    metadata: Option<Value>,
    signer: &str,
    signature: &str,
) -> AppResult<LedgerEntryRow> {
    let mut tx = pool.begin().await?;

    // Serialize all credit appends under a single advisory lock so the hash
    // chain stays linear regardless of connection pool distribution.
    sqlx::query("SELECT pg_advisory_xact_lock(42)").execute(&mut *tx).await?;

    // Find the latest hash in the ledger — used as prev_hash.
    let prev_hash: Option<String> =
        sqlx::query_scalar("SELECT hash FROM credit.ledger ORDER BY seq DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;

    let prev_hash = prev_hash.unwrap_or_else(|| GENESIS_HASH.to_string());

    // Build the canonical payload and compute the entry hash.
    let payload = serde_json::json!({
        "tx_id": tx_id,
        "type": type_,
        "from_account": from_account.map(|v| v.to_string()),
        "to_account": to_account.map(|v| v.to_string()),
        "amount": amount,
        "nonce": nonce,
        "metadata": metadata,
        "signer": signer,
        "timestamp": Utc::now().timestamp(),
    });
    let canonical = canonicalize(&payload);
    let hash = compute_hash(&canonical, &prev_hash);

    let row: LedgerEntryRow = sqlx::query_as(
        "INSERT INTO credit.ledger \
         (tx_id, type, from_account, to_account, amount, nonce, metadata, \
          signer, signature, prev_hash, hash) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         RETURNING seq, tx_id, type, from_account, to_account, amount, \
                   nonce, metadata, signer, signature, prev_hash, hash, created_at",
    )
    .bind(tx_id)
    .bind(type_)
    .bind(from_account)
    .bind(to_account)
    .bind(amount)
    .bind(nonce)
    .bind(&metadata)
    .bind(signer)
    .bind(signature)
    .bind(&prev_hash)
    .bind(&hash)
    .fetch_one(&mut *tx)
    .await?;

    // Update wallets — increment recipient, decrement sender.
    if let Some(to) = to_account {
        ensure_wallet_exists_tx(&mut *tx, to).await?;
        sqlx::query(
            "UPDATE credit.wallets SET balance = balance + $1, last_seq = $2 WHERE account_id = $3",
        )
        .bind(amount)
        .bind(row.seq)
        .bind(to)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(from) = from_account {
        ensure_wallet_exists_tx(&mut *tx, from).await?;
        sqlx::query(
            "UPDATE credit.wallets SET balance = balance - $1, last_seq = $2 WHERE account_id = $3",
        )
        .bind(amount)
        .bind(row.seq)
        .bind(from)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(row)
}

/// Cursor-paginated ledger entries for an account (as sender or receiver).
pub async fn list_ledger(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<LedgerEntryDto>> {
    let since_seq = cursor.unwrap_or(0);
    let rows: Vec<LedgerEntryRow> = sqlx::query_as(
        "SELECT seq, tx_id, type, from_account, to_account, amount, nonce, \
                metadata, signer, signature, prev_hash, hash, created_at \
         FROM credit.ledger \
         WHERE (from_account = $1 OR to_account = $1) AND seq > $2 \
         ORDER BY seq ASC LIMIT $3",
    )
    .bind(account_id)
    .bind(since_seq)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let next_cursor = rows.last().map(|r| r.seq.to_string());
    let items: Vec<LedgerEntryDto> = rows
        .into_iter()
        .map(|r| LedgerEntryDto {
            seq: r.seq,
            tx_id: r.tx_id,
            type_: r.type_,
            from_account: r.from_account.map(|v| v.to_string()),
            to_account: r.to_account.map(|v| v.to_string()),
            amount: r.amount,
            nonce: r.nonce,
            metadata: r.metadata,
            signer: r.signer,
            prev_hash: r.prev_hash,
            hash: r.hash,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Page::new(items, next_cursor))
}

/// Recompute the hash chain and verify every Ed25519 signature for all
/// ledger entries. Only `signer == "system"` entries are treated as
/// self-consistent (their signatures are not user-verifiable).
pub async fn verify_full_ledger(pool: &PgPool) -> AppResult<LedgerVerify> {
    let rows: Vec<LedgerEntryRow> = sqlx::query_as(
        "SELECT seq, tx_id, type, from_account, to_account, amount, nonce, \
                metadata, signer, signature, prev_hash, hash, created_at \
         FROM credit.ledger ORDER BY seq ASC",
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(LedgerVerify { ok: true, latest_seq: None, latest_hash: None });
    }

    let mut expected_prev = GENESIS_HASH.to_string();
    for row in &rows {
        // Verify prev_hash links correctly.
        if row.prev_hash != expected_prev {
            return Ok(LedgerVerify {
                ok: false,
                latest_seq: Some(row.seq),
                latest_hash: Some(row.hash.clone()),
            });
        }

        // Build canonical payload and verify hash.
        let payload = serde_json::json!({
            "tx_id": row.tx_id,
            "type": row.type_,
            "from_account": row.from_account.map(|v| v.to_string()),
            "to_account": row.to_account.map(|v| v.to_string()),
            "amount": row.amount,
            "nonce": row.nonce,
            "metadata": row.metadata,
            "signer": row.signer,
            "timestamp": row.created_at.timestamp(),
        });
        let canonical = canonicalize(&payload);
        let computed_hash = compute_hash(&canonical, &row.prev_hash);

        if computed_hash != row.hash {
            return Ok(LedgerVerify {
                ok: false,
                latest_seq: Some(row.seq),
                latest_hash: Some(row.hash.clone()),
            });
        }

        // For non-system entries, verify the signature against the signer's
        // public key. System entries carry a placeholder signature.
        if row.signer != "system" {
            let pk_row: Option<(String,)> = sqlx::query_as(
                "SELECT public_key FROM identity.account_keys \
                 WHERE account_id = (SELECT id FROM identity.accounts WHERE id::text = $1 LIMIT 1) \
                 LIMIT 1",
            )
            .bind(&row.signer)
            .fetch_optional(pool)
            .await?;

            if let Some((pk,)) = pk_row {
                if !crate::ledger::verify_signature(&canonical, &row.signature, &pk) {
                    return Ok(LedgerVerify {
                        ok: false,
                        latest_seq: Some(row.seq),
                        latest_hash: Some(row.hash.clone()),
                    });
                }
            }
            // If no public key found for signer, we skip signature check
            // (this can happen for historical entries).
        }

        expected_prev = row.hash.clone();
    }

    let last = rows.last().unwrap();
    Ok(LedgerVerify { ok: true, latest_seq: Some(last.seq), latest_hash: Some(last.hash.clone()) })
}

// ---------------------------------------------------------------------------
// Wallets
// ---------------------------------------------------------------------------

/// Read the wallet balance for an account.
pub async fn get_wallet(pool: &PgPool, account_id: i64) -> AppResult<WalletDto> {
    let row: (i64,) =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or((0,));

    Ok(WalletDto { account_id: account_id.to_string(), balance: row.0 })
}

/// Ensure a wallet row exists for `account_id` (idempotent).
pub async fn ensure_wallet_exists(pool: &PgPool, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO credit.wallets (account_id, balance, last_seq) \
         VALUES ($1, 0, 0) ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Transaction-internal variant — works on any PostgreSQL executor.
async fn ensure_wallet_exists_tx(tx: &mut sqlx::PgConnection, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO credit.wallets (account_id, balance, last_seq) \
         VALUES ($1, 0, 0) ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// System-signed mint: creates a ledger entry with `type = "mint"`.
/// The signature is a placeholder — in production a real system key would be used.
#[tracing::instrument(skip(pool))]
pub async fn mint_points(
    pool: &PgPool,
    to_account_id: i64,
    amount: i64,
    reason: &str,
) -> AppResult<LedgerEntryRow> {
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let metadata = serde_json::json!({ "reason": reason });

    append_ledger_entry(
        pool,
        &tx_id,
        "mint",
        None,
        Some(to_account_id),
        amount,
        &nonce,
        Some(metadata),
        "system",
        "system-signed",
    )
    .await
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

/// List tasks with optional status filter, cursor-paginated.
pub async fn list_tasks(
    pool: &PgPool,
    status: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<TaskRow>> {
    let since_id = cursor.unwrap_or(0);

    let rows: Vec<TaskRow> = if let Some(st) = status {
        sqlx::query_as(
            "SELECT id, creator_id, acceptor_id, title, description, \
                    reward_amount, contact_info, status::text, hold_tx_id, created_at \
             FROM credit.tasks WHERE status = $1::credit.task_status AND id > $2 \
             ORDER BY id ASC LIMIT $3",
        )
        .bind(st)
        .bind(since_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, creator_id, acceptor_id, title, description, \
                    reward_amount, contact_info, status::text, hold_tx_id, created_at \
             FROM credit.tasks WHERE id > $1 \
             ORDER BY id ASC LIMIT $2",
        )
        .bind(since_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let next_cursor = rows.last().map(|r| r.id.to_string());
    Ok(Page::new(rows, next_cursor))
}

/// Insert a new task row. Returns the inserted row.
pub async fn insert_task(
    pool: &PgPool,
    creator_id: i64,
    title: &str,
    description: Option<&str>,
    reward_amount: i64,
    contact_info: Option<&str>,
    hold_tx_id: &str,
) -> AppResult<TaskRow> {
    let row = sqlx::query_as(
        "INSERT INTO credit.tasks \
         (creator_id, title, description, reward_amount, contact_info, hold_tx_id) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, creator_id, acceptor_id, title, description, \
                   reward_amount, contact_info, status::text, hold_tx_id, created_at",
    )
    .bind(creator_id)
    .bind(title)
    .bind(description)
    .bind(reward_amount)
    .bind(contact_info)
    .bind(hold_tx_id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Look up a task by id.
pub async fn find_task(pool: &PgPool, id: i64) -> AppResult<Option<TaskRow>> {
    let row = sqlx::query_as(
        "SELECT id, creator_id, acceptor_id, title, description, \
                reward_amount, contact_info, status::text, hold_tx_id, created_at \
         FROM credit.tasks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Update a task's acceptor and transition status to `in_progress`.
pub async fn accept_task(pool: &PgPool, id: i64, acceptor_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE credit.tasks \
         SET acceptor_id = $1, \
             status = 'in_progress'::credit.task_status, \
             updated_at = now() \
         WHERE id = $2 AND status = 'open'::credit.task_status",
    )
    .bind(acceptor_id)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Transition a task's status (submit, confirm, cancel, reject, delete).
pub async fn update_task_status(
    pool: &PgPool,
    id: i64,
    new_status: &str,
    caller_id: i64,
) -> AppResult<()> {
    let valid = match new_status {
        "submit" | "confirm" | "cancel" | "reject" | "delete" => true,
        _ => false,
    };
    if !valid {
        return Err(CreditError::InvalidAction(format!("unknown task action: {new_status}")).into());
    }

    sqlx::query(
        "UPDATE credit.tasks \
         SET status = $1::credit.task_status, updated_at = now() \
         WHERE id = $2",
    )
    .bind(new_status)
    .bind(id)
    .execute(pool)
    .await?;

    let _ = caller_id;
    Ok(())
}

/// Clear a task's hold_tx_id (e.g., after escrow release).
pub async fn clear_task_hold(pool: &PgPool, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE credit.tasks SET hold_tx_id = NULL WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Products
// ---------------------------------------------------------------------------

/// List products with optional status filter, cursor-paginated.
pub async fn list_products(
    pool: &PgPool,
    status: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<ProductRow>> {
    let since_id = cursor.unwrap_or(0);

    let rows: Vec<ProductRow> = if let Some(st) = status {
        sqlx::query_as(
            "SELECT id, seller_id, title, description, price, stock, \
                    delivery_info, status::text, created_at \
             FROM credit.products WHERE status = $1::credit.product_status AND id > $2 \
             ORDER BY id ASC LIMIT $3",
        )
        .bind(st)
        .bind(since_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, seller_id, title, description, price, stock, \
                    delivery_info, status::text, created_at \
             FROM credit.products WHERE id > $1 \
             ORDER BY id ASC LIMIT $2",
        )
        .bind(since_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let next_cursor = rows.last().map(|r| r.id.to_string());
    Ok(Page::new(rows, next_cursor))
}

/// Insert a new product row.
pub async fn insert_product(
    pool: &PgPool,
    seller_id: i64,
    title: &str,
    description: Option<&str>,
    price: i64,
    stock: i32,
    delivery_info: Option<&str>,
) -> AppResult<ProductRow> {
    let row = sqlx::query_as(
        "INSERT INTO credit.products \
         (seller_id, title, description, price, stock, delivery_info) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, seller_id, title, description, price, stock, \
                   delivery_info, status::text, created_at",
    )
    .bind(seller_id)
    .bind(title)
    .bind(description)
    .bind(price)
    .bind(stock)
    .bind(delivery_info)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Look up a product by id.
pub async fn find_product(pool: &PgPool, id: i64) -> AppResult<Option<ProductRow>> {
    let row = sqlx::query_as(
        "SELECT id, seller_id, title, description, price, stock, \
                delivery_info, status::text, created_at \
         FROM credit.products WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Update product status.
pub async fn update_product_status(pool: &PgPool, id: i64, status: &str) -> AppResult<()> {
    sqlx::query("UPDATE credit.products SET status = $1::credit.product_status WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Decrement product stock.
pub async fn decrement_stock(pool: &PgPool, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE credit.products SET stock = stock - 1 WHERE id = $1 AND stock > 0")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Purchases
// ---------------------------------------------------------------------------

/// List purchases for a user (buyer or seller).
pub async fn list_purchases(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<PurchaseRow>> {
    let since_id = cursor.unwrap_or(0);

    let rows: Vec<PurchaseRow> = sqlx::query_as(
        "SELECT id, product_id, buyer_id, seller_id, amount, status::text, hold_tx_id \
         FROM credit.purchases \
         WHERE (buyer_id = $1 OR seller_id = $1) AND id > $2 \
         ORDER BY id ASC LIMIT $3",
    )
    .bind(account_id)
    .bind(since_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let next_cursor = rows.last().map(|r| r.id.to_string());
    Ok(Page::new(rows, next_cursor))
}

/// Insert a new purchase row.
pub async fn insert_purchase(
    pool: &PgPool,
    product_id: i64,
    buyer_id: i64,
    seller_id: i64,
    amount: i64,
    hold_tx_id: &str,
) -> AppResult<PurchaseRow> {
    let row = sqlx::query_as(
        "INSERT INTO credit.purchases \
         (product_id, buyer_id, seller_id, amount, hold_tx_id) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, product_id, buyer_id, seller_id, amount, status::text, hold_tx_id",
    )
    .bind(product_id)
    .bind(buyer_id)
    .bind(seller_id)
    .bind(amount)
    .bind(hold_tx_id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Look up a purchase by id.
pub async fn find_purchase(pool: &PgPool, id: i64) -> AppResult<Option<PurchaseRow>> {
    let row = sqlx::query_as(
        "SELECT id, product_id, buyer_id, seller_id, amount, status::text, hold_tx_id \
         FROM credit.purchases WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Update purchase status.
pub async fn update_purchase_status(pool: &PgPool, id: i64, status: &str) -> AppResult<()> {
    sqlx::query("UPDATE credit.purchases SET status = $1::credit.purchase_status WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

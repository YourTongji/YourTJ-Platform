//! Database access layer for the credit domain.
//!
//! Every function takes `&PgPool` and returns `AppResult` so callers
//! can use `?` and let Axum render errors. Transaction-aware `_tx`
//! variants accept `&mut PgConnection` for atomic multi-step flows.

use chrono::Utc;
use serde_json::Value;
use shared::{AppError, AppResult, Page};
use sqlx::PgPool;

use crate::dto::{LedgerEntryDto, LedgerVerify, WalletDto};
use crate::error::CreditError;
use crate::ledger::{compute_hash, sign_with_seed};
use crate::models::{LedgerEntryRow, ProductRow, PurchaseRow, TaskRow};

const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

// ---------------------------------------------------------------------------
// Ledger: append, list, verify
// ---------------------------------------------------------------------------

/// Transaction-internal variant: append a ledger entry inside an existing
/// transaction. Takes the advisory lock, computes `prev_hash`, builds the
/// canonical payload using the deterministic `created_at`, inserts the row,
/// and updates wallet balances. Does NOT commit or roll back.
#[allow(clippy::too_many_arguments)] // reason: append_ledger_entry_tx mirrors the full ledger column set; packing into a struct would obscure the required fields
pub async fn append_ledger_entry_tx(
    conn: &mut sqlx::PgConnection,
    tx_id: &str,
    type_: &str,
    from_account: Option<i64>,
    to_account: Option<i64>,
    amount: i64,
    nonce: &str,
    metadata: Option<&Value>,
    signer: &str,
    signature: &str,
    created_at: i64,
) -> AppResult<LedgerEntryRow> {
    sqlx::query("SELECT pg_advisory_xact_lock(42)").execute(&mut *conn).await?;

    let prev_hash: Option<String> =
        sqlx::query_scalar("SELECT hash FROM credit.ledger ORDER BY seq DESC LIMIT 1")
            .fetch_optional(&mut *conn)
            .await?;

    let prev_hash = prev_hash.unwrap_or_else(|| GENESIS_HASH.to_string());

    let canonical = crate::ledger::build_ledger_canonical(
        tx_id,
        type_,
        from_account,
        to_account,
        amount,
        nonce,
        metadata,
        signer,
        created_at,
    );
    let hash = compute_hash(&canonical, &prev_hash);

    // Persist the exact `created_at` that was hashed/signed (seconds precision),
    // not the DB `now()`. Otherwise `verify_full_ledger` recomputes the canonical
    // from a different timestamp than the one baked into `hash` and fails.
    let created_at_ts = chrono::DateTime::from_timestamp(created_at, 0).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("append_ledger_entry_tx: invalid created_at timestamp"))
    })?;

    let row: LedgerEntryRow = sqlx::query_as(
        "INSERT INTO credit.ledger \
         (tx_id, type, from_account, to_account, amount, nonce, metadata, \
          signer, signature, prev_hash, hash, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING seq, tx_id, type, from_account, to_account, amount, \
                   nonce, metadata, signer, signature, prev_hash, hash, created_at",
    )
    .bind(tx_id)
    .bind(type_)
    .bind(from_account)
    .bind(to_account)
    .bind(amount)
    .bind(nonce)
    .bind(metadata)
    .bind(signer)
    .bind(signature)
    .bind(&prev_hash)
    .bind(&hash)
    .bind(created_at_ts)
    .fetch_one(&mut *conn)
    .await?;

    // Update wallets — increment recipient, decrement sender.
    if let Some(to) = to_account {
        ensure_wallet_exists_tx(conn, to).await?;
        sqlx::query(
            "UPDATE credit.wallets SET balance = balance + $1, last_seq = $2 WHERE account_id = $3",
        )
        .bind(amount)
        .bind(row.seq)
        .bind(to)
        .execute(&mut *conn)
        .await?;
    }

    if let Some(from) = from_account {
        ensure_wallet_exists_tx(conn, from).await?;
        sqlx::query(
            "UPDATE credit.wallets SET balance = balance - $1, last_seq = $2 WHERE account_id = $3",
        )
        .bind(amount)
        .bind(row.seq)
        .bind(from)
        .execute(&mut *conn)
        .await?;
    }

    Ok(row)
}

/// Append a ledger entry inside its own transaction. Uses `Utc::now()` for
/// the timestamp. Delegates to [`append_ledger_entry_tx`].
#[allow(clippy::too_many_arguments)]
// reason: ledger entries require all fields for hash-chain integrity; a builder would hide which fields are mandatory
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
    let created_at = Utc::now().timestamp();
    let mut tx = pool.begin().await?;
    let row = append_ledger_entry_tx(
        &mut tx,
        tx_id,
        type_,
        from_account,
        to_account,
        amount,
        nonce,
        metadata.as_ref(),
        signer,
        signature,
        created_at,
    )
    .await?;
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

/// Recompute the hash chain and verify every Ed25519 signature for all ledger
/// entries. System-signed entries are verified against `system_public_key_b64`;
/// user-signed entries are verified against the account's bound key
/// (`identity.account_keys`).
pub async fn verify_full_ledger(
    pool: &PgPool,
    system_public_key_b64: &str,
) -> AppResult<LedgerVerify> {
    use std::collections::HashMap;

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

    let signer_ids: Vec<i64> = rows
        .iter()
        .filter(|row| row.signer != "system")
        .filter_map(|row| row.signer.parse::<i64>().ok())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut pk_map: HashMap<i64, Vec<String>> = HashMap::new();
    if !signer_ids.is_empty() {
        let key_rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT account_id, public_key FROM identity.account_keys WHERE account_id = ANY($1)",
        )
        .bind(&signer_ids)
        .fetch_all(pool)
        .await?;
        for (account_id, public_key) in key_rows {
            pk_map.entry(account_id).or_default().push(public_key);
        }
    }

    let mut expected_prev = GENESIS_HASH.to_string();
    for row in &rows {
        if row.prev_hash != expected_prev {
            return Ok(LedgerVerify {
                ok: false,
                latest_seq: Some(row.seq),
                latest_hash: Some(row.hash.clone()),
            });
        }

        let canonical = crate::ledger::build_ledger_canonical(
            &row.tx_id,
            &row.type_,
            row.from_account,
            row.to_account,
            row.amount,
            &row.nonce,
            row.metadata.as_ref(),
            &row.signer,
            row.created_at.timestamp(),
        );
        let computed_hash = compute_hash(&canonical, &row.prev_hash);

        if computed_hash != row.hash {
            return Ok(LedgerVerify {
                ok: false,
                latest_seq: Some(row.seq),
                latest_hash: Some(row.hash.clone()),
            });
        }

        if row.signer == "system" {
            if !crate::ledger::verify_signature(&canonical, &row.signature, system_public_key_b64) {
                return Ok(LedgerVerify {
                    ok: false,
                    latest_seq: Some(row.seq),
                    latest_hash: Some(row.hash.clone()),
                });
            }
        } else if let Ok(account_id) = row.signer.parse::<i64>() {
            let public_keys = match pk_map.get(&account_id) {
                Some(public_keys) => public_keys,
                None => {
                    return Ok(LedgerVerify {
                        ok: false,
                        latest_seq: Some(row.seq),
                        latest_hash: Some(row.hash.clone()),
                    });
                }
            };
            let intent_signing_bytes: Option<String> = match row
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("signing_intent_id"))
                .and_then(Value::as_str)
            {
                Some(intent_id) => {
                    sqlx::query_scalar(
                        "SELECT signing_bytes FROM credit.signing_intents WHERE id::text = $1",
                    )
                    .bind(intent_id)
                    .fetch_optional(pool)
                    .await?
                }
                None => None,
            };
            let signed_bytes = intent_signing_bytes.as_deref().unwrap_or(&canonical);
            if !public_keys.iter().any(|public_key| {
                crate::ledger::verify_signature(signed_bytes, &row.signature, public_key)
            }) {
                return Ok(LedgerVerify {
                    ok: false,
                    latest_seq: Some(row.seq),
                    latest_hash: Some(row.hash.clone()),
                });
            }
        } else {
            return Ok(LedgerVerify {
                ok: false,
                latest_seq: Some(row.seq),
                latest_hash: Some(row.hash.clone()),
            });
        }

        expected_prev = row.hash.clone();
    }

    let last = rows.last().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("ledger verification: empty rows after non-empty check"))
    })?;
    Ok(LedgerVerify { ok: true, latest_seq: Some(last.seq), latest_hash: Some(last.hash.clone()) })
}

// ---------------------------------------------------------------------------
// Wallets
// ---------------------------------------------------------------------------

/// Read the wallet balance for an account.
pub async fn get_wallet(pool: &PgPool, account_id: i64) -> AppResult<WalletDto> {
    let balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or(0);

    Ok(WalletDto { account_id: account_id.to_string(), balance })
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
async fn ensure_wallet_exists_tx(conn: &mut sqlx::PgConnection, account_id: i64) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO credit.wallets (account_id, balance, last_seq) \
         VALUES ($1, 0, 0) ON CONFLICT (account_id) DO NOTHING",
    )
    .bind(account_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// System-signed mint: creates a ledger entry with `type = "mint"`.
/// Signs the payload using the system private key seed. Generates a fresh
/// random `tx_id`; use [`mint_points_with_tx_id`] when the caller needs the
/// `tx_id` to be a stable idempotency key.
#[tracing::instrument(skip(pool, system_seed))]
pub async fn mint_points(
    pool: &PgPool,
    to_account_id: i64,
    amount: i64,
    reason: &str,
    system_seed: &[u8],
) -> AppResult<LedgerEntryRow> {
    let tx_id = uuid::Uuid::new_v4().to_string();
    mint_points_with_tx_id(pool, to_account_id, amount, &tx_id, reason, system_seed).await
}

/// System-signed mint with a caller-supplied `tx_id`.
///
/// The ledger's `tx_id` is `UNIQUE`, so passing a stable idempotency key here
/// makes the mint safe to retry: a duplicate call either short-circuits on the
/// caller's pre-check or fails the unique constraint instead of double-minting.
#[tracing::instrument(skip(pool, system_seed))]
pub async fn mint_points_with_tx_id(
    pool: &PgPool,
    to_account_id: i64,
    amount: i64,
    tx_id: &str,
    reason: &str,
    system_seed: &[u8],
) -> AppResult<LedgerEntryRow> {
    let nonce = uuid::Uuid::new_v4().to_string();
    let metadata = serde_json::json!({ "reason": reason });
    let created_at = Utc::now().timestamp();

    let canonical = crate::ledger::build_ledger_canonical(
        tx_id,
        "mint",
        None,
        Some(to_account_id),
        amount,
        &nonce,
        Some(&metadata),
        "system",
        created_at,
    );
    let signature = sign_with_seed(&canonical, system_seed);

    let mut tx = pool.begin().await?;
    let row = append_ledger_entry_tx(
        &mut tx,
        tx_id,
        "mint",
        None,
        Some(to_account_id),
        amount,
        &nonce,
        Some(&metadata),
        "system",
        &signature,
        created_at,
    )
    .await?;
    tx.commit().await?;
    Ok(row)
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

/// Transaction-internal variant of [`insert_task`].
pub async fn insert_task_tx(
    conn: &mut sqlx::PgConnection,
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
    .fetch_one(&mut *conn)
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

/// Transition a task to a new `credit.task_status` value.
///
/// `new_status` must be a valid enum value that a caller legitimately sets
/// through this function (`submitted`, `completed`, `cancelled`); `open` and
/// `in_progress` are set by insert/accept, not here. Task removal goes through
/// [`delete_task_tx`], not a status transition — there is no `deleted` status.
pub async fn update_task_status(
    pool: &PgPool,
    id: i64,
    new_status: &str,
    caller_id: i64,
) -> AppResult<()> {
    if !matches!(new_status, "submitted" | "completed" | "cancelled") {
        return Err(CreditError::InvalidAction(format!("unknown task status: {new_status}")).into());
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

/// Transaction-internal variant of [`update_task_status`].
pub async fn update_task_status_tx(
    conn: &mut sqlx::PgConnection,
    id: i64,
    new_status: &str,
) -> AppResult<()> {
    if !matches!(new_status, "submitted" | "completed" | "cancelled") {
        return Err(CreditError::InvalidAction(format!("unknown task status: {new_status}")).into());
    }

    sqlx::query(
        "UPDATE credit.tasks \
         SET status = $1::credit.task_status, updated_at = now() \
         WHERE id = $2",
    )
    .bind(new_status)
    .bind(id)
    .execute(&mut *conn)
    .await?;

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

/// Transaction-internal variant of [`clear_task_hold`].
pub async fn clear_task_hold_tx(conn: &mut sqlx::PgConnection, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE credit.tasks SET hold_tx_id = NULL WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Transaction-internal hard delete of a task row. There is no `deleted`
/// status in `credit.task_status`; the `delete` action removes the record
/// entirely (any active escrow hold must be refunded first by the caller).
pub async fn delete_task_tx(conn: &mut sqlx::PgConnection, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM credit.tasks WHERE id = $1").bind(id).execute(&mut *conn).await?;

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

/// Transaction-internal variant of [`update_product_status`].
pub async fn update_product_status_tx(
    conn: &mut sqlx::PgConnection,
    id: i64,
    status: &str,
) -> AppResult<()> {
    sqlx::query("UPDATE credit.products SET status = $1::credit.product_status WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(&mut *conn)
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

/// Transaction-internal variant of [`decrement_stock`].
pub async fn decrement_stock_tx(conn: &mut sqlx::PgConnection, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE credit.products SET stock = stock - 1 WHERE id = $1 AND stock > 0")
        .bind(id)
        .execute(&mut *conn)
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

/// Transaction-internal variant of [`insert_purchase`].
pub async fn insert_purchase_tx(
    conn: &mut sqlx::PgConnection,
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
    .fetch_one(&mut *conn)
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

/// Transaction-internal variant of [`update_purchase_status`].
pub async fn update_purchase_status_tx(
    conn: &mut sqlx::PgConnection,
    id: i64,
    status: &str,
) -> AppResult<()> {
    sqlx::query("UPDATE credit.purchases SET status = $1::credit.purchase_status WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}

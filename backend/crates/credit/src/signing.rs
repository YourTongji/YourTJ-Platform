//! One-time authorization for user-initiated credit operations.

use axum::http::HeaderMap;
use chrono::Utc;
use serde_json::Value;
use sha2::Digest as _;
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::dto::{SigningIntentInput, SigningIntentOutput};
use crate::error::CreditError;

const INTENT_TTL_SECONDS: i64 = 300;

#[derive(sqlx::FromRow)]
struct SigningIntentRow {
    account_id: i64,
    public_key: String,
    action: String,
    request_hash: String,
    snapshot: Value,
    idempotency_key: String,
    signing_bytes: String,
    ledger_entry: Option<Value>,
    ledger_canonical: Option<String>,
    expires_at: chrono::DateTime<Utc>,
    consumed_at: Option<chrono::DateTime<Utc>>,
}

/// Exact ledger fields prepared before the wallet signs an intent.
#[derive(Debug, Clone)]
pub struct PreparedLedgerEntry {
    pub tx_id: String,
    pub type_: String,
    pub from_account: Option<i64>,
    pub to_account: Option<i64>,
    pub amount: i64,
    pub nonce: String,
    pub metadata: Option<Value>,
    pub signer: String,
    pub created_at: i64,
}

/// A consumed signing intent and any exact ledger entry it authorizes.
pub struct ConsumedIntent {
    pub signature: String,
    pub ledger_entry: Option<PreparedLedgerEntry>,
}

pub fn request_hash(request: &Value) -> String {
    hex::encode(sha2::Sha256::digest(crate::ledger::canonicalize(request).as_bytes()))
}

pub async fn create_intent(
    pool: &PgPool,
    account_id: i64,
    input: &SigningIntentInput,
    idempotency_key: &str,
) -> AppResult<SigningIntentOutput> {
    validate_action(&input.action)?;
    let public_key: String = sqlx::query_scalar(
        "SELECT public_key FROM identity.account_keys \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(CreditError::WalletNotBound)?;

    let intent_id = uuid::Uuid::new_v4();
    let normalized_request = normalize_request(&input.action, &input.request)?;
    let request_hash = request_hash(&normalized_request);
    let snapshot = build_snapshot(pool, account_id, &input.action, &normalized_request).await?;
    let ledger_entry =
        prepare_ledger_entry(account_id, intent_id, &input.action, &normalized_request, &snapshot)?;
    let ledger_canonical = ledger_entry.as_ref().map(crate::ledger::canonicalize);
    let expires_at = Utc::now().timestamp() + INTENT_TTL_SECONDS;
    let signing_bytes = crate::ledger::canonicalize(&serde_json::json!({
        "version": 1,
        "intentId": intent_id.to_string(),
        "accountId": account_id.to_string(),
        "publicKey": public_key,
        "action": input.action,
        "requestHash": request_hash,
        "snapshot": snapshot,
        "ledgerEntry": ledger_entry,
        "idempotencyKey": idempotency_key,
        "expiresAt": expires_at,
    }));

    let inserted = sqlx::query_as::<_, (uuid::Uuid, String, chrono::DateTime<Utc>)>(
        "INSERT INTO credit.signing_intents \
         (id, account_id, public_key, action, request_hash, snapshot, idempotency_key, \
          signing_bytes, ledger_entry, ledger_canonical, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, to_timestamp($11)) \
         ON CONFLICT (account_id, idempotency_key) DO NOTHING \
         RETURNING id, signing_bytes, expires_at",
    )
    .bind(intent_id)
    .bind(account_id)
    .bind(&public_key)
    .bind(&input.action)
    .bind(&request_hash)
    .bind(&snapshot)
    .bind(idempotency_key)
    .bind(&signing_bytes)
    .bind(&ledger_entry)
    .bind(&ledger_canonical)
    .bind(expires_at)
    .fetch_optional(pool)
    .await?;

    if let Some((id, bytes, expiry)) = inserted {
        return Ok(SigningIntentOutput {
            intent_id: id.to_string(),
            signing_bytes: bytes,
            expires_at: expiry.timestamp(),
        });
    }

    let existing: (
        uuid::Uuid,
        String,
        String,
        String,
        chrono::DateTime<Utc>,
        Option<chrono::DateTime<Utc>>,
    ) = sqlx::query_as(
        "SELECT id, request_hash, action, signing_bytes, expires_at, consumed_at \
             FROM credit.signing_intents WHERE account_id = $1 AND idempotency_key = $2",
    )
    .bind(account_id)
    .bind(idempotency_key)
    .fetch_one(pool)
    .await?;
    if existing.1 != request_hash || existing.2 != input.action {
        return Err(CreditError::IdempotencyConflict.into());
    }
    if existing.5.is_some() || existing.4.timestamp() <= Utc::now().timestamp() {
        return Err(CreditError::IntentUnavailable.into());
    }
    Ok(SigningIntentOutput {
        intent_id: existing.0.to_string(),
        signing_bytes: existing.3,
        expires_at: existing.4.timestamp(),
    })
}

pub async fn consume_intent(
    conn: &mut PgConnection,
    headers: &HeaderMap,
    account_id: i64,
    action: &str,
    request: &Value,
) -> AppResult<ConsumedIntent> {
    let normalized_request = normalize_request(action, request)?;
    let intent_id = required_header(headers, "x-wallet-intent")?
        .parse::<uuid::Uuid>()
        .map_err(|_| CreditError::IntentUnavailable)?;
    let signature = required_header(headers, "x-wallet-sig")?;
    let idempotency_key = required_header(headers, "idempotency-key")?;

    let intent = sqlx::query_as::<_, SigningIntentRow>(
        "SELECT account_id, public_key, action, request_hash, snapshot, idempotency_key, \
                signing_bytes, ledger_entry, ledger_canonical, expires_at, consumed_at \
         FROM credit.signing_intents WHERE id = $1 FOR UPDATE",
    )
    .bind(intent_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(CreditError::IntentUnavailable)?;
    if intent.account_id != account_id
        || intent.action != action
        || intent.request_hash != request_hash(&normalized_request)
        || intent.idempotency_key != idempotency_key
        || intent.expires_at.timestamp() <= Utc::now().timestamp()
        || intent.consumed_at.is_some()
    {
        return Err(CreditError::IntentUnavailable.into());
    }
    let current_key: Option<String> = sqlx::query_scalar(
        "SELECT public_key FROM identity.account_keys \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id)
    .fetch_optional(&mut *conn)
    .await?;
    let current_snapshot =
        build_snapshot_tx(conn, account_id, action, &normalized_request, true).await?;
    if current_key.as_deref() != Some(intent.public_key.as_str())
        || !crate::ledger::verify_signature(&intent.signing_bytes, signature, &intent.public_key)
        || current_snapshot != intent.snapshot
    {
        return Err(CreditError::InvalidSignature.into());
    }
    let ledger_entry = match (&intent.ledger_entry, &intent.ledger_canonical) {
        (Some(entry), Some(canonical)) => {
            if crate::ledger::canonicalize(entry) != *canonical {
                return Err(CreditError::InvalidSignature.into());
            }
            Some(parse_prepared_ledger(entry)?)
        }
        (None, None) => None,
        _ => return Err(CreditError::InvalidSignature.into()),
    };
    sqlx::query("UPDATE credit.signing_intents SET consumed_at = now() WHERE id = $1")
        .bind(intent_id)
        .execute(&mut *conn)
        .await?;
    Ok(ConsumedIntent { signature: signature.to_string(), ledger_entry })
}

fn normalize_request(action: &str, request: &Value) -> AppResult<Value> {
    if action == "credit.task.create" {
        let input: crate::dto::TaskInput = serde_json::from_value(request.clone())
            .map_err(|_| AppError::BadRequest("invalid task signing request".into()))?;
        serde_json::to_value(input).map_err(|error| AppError::Internal(anyhow::Error::new(error)))
    } else {
        Ok(request.clone())
    }
}

fn required_header<'a>(headers: &'a HeaderMap, name: &str) -> AppResult<&'a str> {
    headers
        .get(name)
        .and_then(|header| header.to_str().ok())
        .filter(|header| !header.is_empty())
        .ok_or_else(|| CreditError::IntentUnavailable.into())
}

fn validate_action(action: &str) -> AppResult<()> {
    if matches!(
        action,
        "credit.tip"
            | "credit.task.create"
            | "credit.task.action"
            | "credit.product.purchase"
            | "credit.purchase.action"
    ) {
        Ok(())
    } else {
        Err(AppError::BadRequest("unsupported credit signing action".into()))
    }
}

async fn build_snapshot(
    pool: &PgPool,
    account_id: i64,
    action: &str,
    request: &Value,
) -> AppResult<Value> {
    let mut conn = pool.acquire().await?;
    build_snapshot_tx(&mut conn, account_id, action, request, false).await
}

async fn build_snapshot_tx(
    conn: &mut PgConnection,
    account_id: i64,
    action: &str,
    request: &Value,
    lock_entity: bool,
) -> AppResult<Value> {
    match action {
        "credit.tip" | "credit.task.create" => {
            let balance: i64 = sqlx::query_scalar(
                "SELECT COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $1), 0)",
            )
            .bind(account_id)
            .fetch_one(&mut *conn)
            .await?;
            Ok(serde_json::json!({ "balance": balance }))
        }
        "credit.product.purchase" => {
            let product_id = request
                .get("productId")
                .and_then(Value::as_str)
                .and_then(|id| id.parse::<i64>().ok())
                .ok_or_else(|| AppError::BadRequest("productId is required".into()))?;
            let query = if lock_entity {
                "SELECT price, stock, status::text, seller_id, title \
                 FROM credit.products WHERE id = $1 FOR UPDATE"
            } else {
                "SELECT price, stock, status::text, seller_id, title \
                 FROM credit.products WHERE id = $1"
            };
            let snapshot: (i64, i32, String, i64, String) = sqlx::query_as(query)
                .bind(product_id)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(CreditError::ProductNotFound)?;
            Ok(serde_json::json!({
                "price": snapshot.0,
                "stock": snapshot.1,
                "status": snapshot.2,
                "sellerId": snapshot.3.to_string(),
                "title": snapshot.4,
            }))
        }
        "credit.task.action" => {
            entity_snapshot(conn, "credit.tasks", request, account_id, lock_entity).await
        }
        "credit.purchase.action" => {
            entity_snapshot(conn, "credit.purchases", request, account_id, lock_entity).await
        }
        _ => Err(AppError::BadRequest("unsupported credit signing action".into())),
    }
}

async fn entity_snapshot(
    conn: &mut PgConnection,
    table: &str,
    request: &Value,
    account_id: i64,
    lock_entity: bool,
) -> AppResult<Value> {
    let entity_id = request
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("id is required".into()))?;
    let query = if table == "credit.tasks" && lock_entity {
        "SELECT status::text, creator_id, COALESCE(acceptor_id, 0), reward_amount \
         FROM credit.tasks WHERE id = $1 FOR UPDATE"
    } else if table == "credit.tasks" {
        "SELECT status::text, creator_id, COALESCE(acceptor_id, 0), reward_amount FROM credit.tasks WHERE id = $1"
    } else if lock_entity {
        "SELECT status::text, buyer_id, seller_id, amount \
         FROM credit.purchases WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT status::text, buyer_id, seller_id, amount FROM credit.purchases WHERE id = $1"
    };
    let snapshot: (String, i64, i64, i64) = sqlx::query_as(query)
        .bind(entity_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(CreditError::IntentUnavailable)?;
    Ok(serde_json::json!({
        "status": snapshot.0,
        "partyA": snapshot.1.to_string(),
        "partyB": snapshot.2.to_string(),
        "amount": snapshot.3,
        "actorId": account_id.to_string(),
    }))
}

fn prepare_ledger_entry(
    account_id: i64,
    intent_id: uuid::Uuid,
    action: &str,
    request: &Value,
    snapshot: &Value,
) -> AppResult<Option<Value>> {
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = Utc::now().timestamp();
    let signer = account_id.to_string();
    let entry = match action {
        "credit.tip" => {
            let to_account = required_i64_string(request, "toAccountId")?;
            let amount = required_positive_i64(request, "amount")?;
            let target_type = required_string(request, "targetType")?;
            if !matches!(target_type, "review" | "thread" | "comment") {
                return Err(AppError::BadRequest("unsupported tip targetType".into()));
            }
            let target_id = required_string(request, "targetId")?;
            serde_json::json!({
                "tx_id": tx_id,
                "type": "tip",
                "from_account": account_id.to_string(),
                "to_account": to_account.to_string(),
                "amount": amount,
                "nonce": nonce,
                "metadata": {
                    "target_type": target_type,
                    "target_id": target_id,
                    "signing_intent_id": intent_id.to_string(),
                },
                "signer": signer,
                "timestamp": timestamp,
            })
        }
        "credit.task.create" => {
            let amount = required_positive_i64(request, "rewardAmount")?;
            let title = required_string(request, "title")?;
            serde_json::json!({
                "tx_id": tx_id,
                "type": "escrow_hold",
                "from_account": account_id.to_string(),
                "to_account": Value::Null,
                "amount": amount,
                "nonce": nonce,
                "metadata": {
                    "title": title,
                    "signing_intent_id": intent_id.to_string(),
                },
                "signer": signer,
                "timestamp": timestamp,
            })
        }
        "credit.product.purchase" => {
            let product_id = required_i64_string(request, "productId")?;
            let amount = snapshot
                .get("price")
                .and_then(Value::as_i64)
                .ok_or_else(|| AppError::BadRequest("product price is unavailable".into()))?;
            let title = snapshot
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::BadRequest("product title is unavailable".into()))?;
            serde_json::json!({
                "tx_id": tx_id,
                "type": "escrow_hold",
                "from_account": account_id.to_string(),
                "to_account": Value::Null,
                "amount": amount,
                "nonce": nonce,
                "metadata": {
                    "product_id": product_id.to_string(),
                    "title": title,
                    "signing_intent_id": intent_id.to_string(),
                },
                "signer": signer,
                "timestamp": timestamp,
            })
        }
        "credit.task.action" | "credit.purchase.action" => return Ok(None),
        _ => return Err(AppError::BadRequest("unsupported credit signing action".into())),
    };
    Ok(Some(entry))
}

fn parse_prepared_ledger(entry: &Value) -> AppResult<PreparedLedgerEntry> {
    Ok(PreparedLedgerEntry {
        tx_id: required_string(entry, "tx_id")?.to_string(),
        type_: required_string(entry, "type")?.to_string(),
        from_account: optional_i64_string(entry, "from_account")?,
        to_account: optional_i64_string(entry, "to_account")?,
        amount: required_positive_i64(entry, "amount")?,
        nonce: required_string(entry, "nonce")?.to_string(),
        metadata: entry.get("metadata").filter(|value| !value.is_null()).cloned(),
        signer: required_string(entry, "signer")?.to_string(),
        created_at: entry
            .get("timestamp")
            .and_then(Value::as_i64)
            .ok_or(CreditError::InvalidSignature)?,
    })
}

fn required_string<'a>(value: &'a Value, field: &str) -> AppResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|field_value| !field_value.is_empty())
        .ok_or_else(|| AppError::BadRequest(format!("{field} is required")))
}

fn required_positive_i64(value: &Value, field: &str) -> AppResult<i64> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .filter(|field_value| *field_value > 0)
        .ok_or_else(|| AppError::BadRequest(format!("{field} must be positive")))
}

fn required_i64_string(value: &Value, field: &str) -> AppResult<i64> {
    required_string(value, field)?
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest(format!("{field} must be an integer string")))
}

fn optional_i64_string(value: &Value, field: &str) -> AppResult<Option<i64>> {
    match value.get(field) {
        Some(Value::String(field_value)) => {
            field_value.parse::<i64>().map(Some).map_err(|_| CreditError::InvalidSignature.into())
        }
        Some(Value::Null) | None => Ok(None),
        _ => Err(CreditError::InvalidSignature.into()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{normalize_request, request_hash};

    #[test]
    fn request_hash_is_canonical() {
        assert_eq!(
            request_hash(&json!({ "amount": 10, "target": "x" })),
            request_hash(&json!({ "target": "x", "amount": 10 }))
        );
    }

    #[test]
    fn request_hash_detects_tampering() {
        assert_ne!(request_hash(&json!({ "amount": 10 })), request_hash(&json!({ "amount": 11 })));
    }

    #[test]
    fn task_request_normalization_treats_omitted_optionals_as_null() {
        let omitted = serde_json::json!({ "title": "Task", "rewardAmount": 10 });
        let explicit = serde_json::json!({
            "title": "Task",
            "rewardAmount": 10,
            "description": null,
            "contactInfo": null
        });
        assert_eq!(
            normalize_request("credit.task.create", &omitted).unwrap(),
            normalize_request("credit.task.create", &explicit).unwrap()
        );
    }
}

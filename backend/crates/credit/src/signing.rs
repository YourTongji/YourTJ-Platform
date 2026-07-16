//! One-time authorization for user-initiated credit operations.

use axum::http::HeaderMap;
use chrono::Utc;
use serde_json::Value;
use sha2::Digest as _;
use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::dto::{
    SigningIntentInput, SigningIntentOutcomeDto, SigningIntentOutput, SigningIntentStatus,
};
use crate::error::CreditError;

const INTENT_TTL_SECONDS: i64 = 300;
const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;

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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SigningEnvelope {
    version: u8,
    intent_id: String,
    account_id: String,
    public_key: String,
    action: String,
    request_hash: String,
    snapshot: Value,
    ledger_entry: Option<Value>,
    idempotency_key: String,
    expires_at: i64,
}

/// Authoritative row fields that exact signing bytes must commit to.
pub(crate) struct SigningEnvelopeExpectation<'a> {
    pub intent_id: uuid::Uuid,
    pub account_id: i64,
    pub public_key: &'a str,
    pub action: &'a str,
    pub request_hash: &'a str,
    pub snapshot: &'a Value,
    pub ledger_entry: Option<&'a Value>,
    pub idempotency_key: &'a str,
    pub expires_at: i64,
}

/// Verify that stored exact bytes are canonical and bind every persisted proof field.
pub(crate) fn signing_envelope_matches(
    signing_bytes: &str,
    expected: &SigningEnvelopeExpectation<'_>,
) -> bool {
    const REQUIRED_FIELDS: [&str; 10] = [
        "version",
        "intentId",
        "accountId",
        "publicKey",
        "action",
        "requestHash",
        "snapshot",
        "ledgerEntry",
        "idempotencyKey",
        "expiresAt",
    ];

    let Ok(value) = serde_json::from_str::<Value>(signing_bytes) else {
        return false;
    };
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != REQUIRED_FIELDS.len()
        || REQUIRED_FIELDS.iter().any(|field| !object.contains_key(*field))
        || crate::ledger::canonicalize(&value) != signing_bytes
    {
        return false;
    }
    let Ok(envelope) = serde_json::from_value::<SigningEnvelope>(value) else {
        return false;
    };
    envelope.version == 1
        && envelope.intent_id == expected.intent_id.to_string()
        && envelope.account_id == expected.account_id.to_string()
        && envelope.public_key == expected.public_key
        && envelope.action == expected.action
        && envelope.request_hash == expected.request_hash
        && envelope.snapshot == *expected.snapshot
        && envelope.ledger_entry.as_ref() == expected.ledger_entry
        && envelope.idempotency_key == expected.idempotency_key
        && envelope.expires_at == expected.expires_at
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

/// Return one owner's intent outcome after waiting for any in-flight consumer row lock.
pub async fn intent_outcome(
    pool: &PgPool,
    account_id: i64,
    intent_id: uuid::Uuid,
) -> AppResult<SigningIntentOutcomeDto> {
    let mut tx = pool.begin().await?;
    let row: Option<(uuid::Uuid, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)> =
        sqlx::query_as(
            "SELECT id, expires_at, consumed_at FROM credit.signing_intents \
         WHERE id = $1 AND account_id = $2 FOR SHARE",
        )
        .bind(intent_id)
        .bind(account_id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((id, expires_at, consumed_at)) = row else {
        return Err(AppError::NotFound);
    };
    // `now()` is fixed at transaction start, before a possible row-lock wait.
    let observed_at: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT clock_timestamp()").fetch_one(&mut *tx).await?;
    let status = if consumed_at.is_some() {
        SigningIntentStatus::Committed
    } else if expires_at <= observed_at {
        SigningIntentStatus::Expired
    } else {
        SigningIntentStatus::Pending
    };
    tx.commit().await?;
    Ok(SigningIntentOutcomeDto {
        intent_id: id.to_string(),
        status,
        expires_at: expires_at.timestamp(),
    })
}

pub async fn create_intent(
    pool: &PgPool,
    account_eligibility_resolver: &dyn crate::account_eligibility::AccountEligibilityResolver,
    wallet_key_resolver: &dyn crate::wallet_keys::WalletKeyResolver,
    account_id: i64,
    input: &SigningIntentInput,
    idempotency_key: &str,
) -> AppResult<SigningIntentOutput> {
    validate_action(&input.action)?;
    validate_idempotency_key(idempotency_key)?;
    let mut tx = pool.begin().await?;
    if !account_eligibility_resolver.are_eligible_on(&mut tx, &[account_id]).await? {
        return Err(AppError::Forbidden);
    }
    let public_key = wallet_key_resolver
        .active_public_key_on(&mut tx, account_id)
        .await?
        .ok_or(CreditError::WalletNotBound)?;

    let intent_id = uuid::Uuid::new_v4();
    let normalized_request = normalize_request(&input.action, &input.request)?;
    let request_hash = request_hash(&normalized_request);
    let snapshot =
        build_snapshot_tx(&mut tx, account_id, &input.action, &normalized_request, false).await?;
    let database_time: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT clock_timestamp()").fetch_one(&mut *tx).await?;
    let observed_at = database_time.timestamp();
    let ledger_entry = prepare_ledger_entry(
        account_id,
        intent_id,
        &input.action,
        &normalized_request,
        &snapshot,
        observed_at,
    )?;
    let ledger_canonical = ledger_entry.as_ref().map(crate::ledger::canonicalize);
    let expires_at = observed_at + INTENT_TTL_SECONDS;
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
    .fetch_optional(&mut *tx)
    .await?;

    let output = if let Some((id, bytes, expiry)) = inserted {
        SigningIntentOutput {
            intent_id: id.to_string(),
            signing_bytes: bytes,
            expires_at: expiry.timestamp(),
        }
    } else {
        let existing: (
            uuid::Uuid,
            String,
            String,
            String,
            chrono::DateTime<Utc>,
            Option<chrono::DateTime<Utc>>,
            bool,
        ) = sqlx::query_as(
            "SELECT id, request_hash, action, signing_bytes, expires_at, consumed_at, \
                        expires_at <= clock_timestamp() AS is_expired \
                 FROM credit.signing_intents WHERE account_id = $1 AND idempotency_key = $2",
        )
        .bind(account_id)
        .bind(idempotency_key)
        .fetch_one(&mut *tx)
        .await?;
        if existing.1 != request_hash || existing.2 != input.action {
            return Err(CreditError::IdempotencyConflict.into());
        }
        if existing.5.is_some() || existing.6 {
            return Err(CreditError::IntentUnavailable.into());
        }
        SigningIntentOutput {
            intent_id: existing.0.to_string(),
            signing_bytes: existing.3,
            expires_at: existing.4.timestamp(),
        }
    };
    tx.commit().await?;
    Ok(output)
}

pub async fn consume_intent(
    conn: &mut PgConnection,
    wallet_key_resolver: &dyn crate::wallet_keys::WalletKeyResolver,
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
    validate_idempotency_key(idempotency_key)?;

    let intent = sqlx::query_as::<_, SigningIntentRow>(
        "SELECT account_id, public_key, action, request_hash, snapshot, idempotency_key, \
                signing_bytes, ledger_entry, ledger_canonical, expires_at, consumed_at \
         FROM credit.signing_intents WHERE id = $1 FOR UPDATE",
    )
    .bind(intent_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(CreditError::IntentUnavailable)?;
    let observed_at: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT clock_timestamp()").fetch_one(&mut *conn).await?;
    if intent.account_id != account_id
        || intent.action != action
        || intent.request_hash != request_hash(&normalized_request)
        || intent.idempotency_key != idempotency_key
        || intent.expires_at <= observed_at
        || intent.consumed_at.is_some()
    {
        return Err(CreditError::IntentUnavailable.into());
    }
    let current_key = wallet_key_resolver.active_public_key_on(conn, account_id).await?;
    if matches!(action, "credit.tip" | "credit.task.create") {
        crate::repo::lock_wallet_for_debit_tx(conn, account_id).await?;
    }
    let current_snapshot =
        build_snapshot_tx(conn, account_id, action, &normalized_request, true).await?;
    let envelope = SigningEnvelopeExpectation {
        intent_id,
        account_id: intent.account_id,
        public_key: &intent.public_key,
        action: &intent.action,
        request_hash: &intent.request_hash,
        snapshot: &intent.snapshot,
        ledger_entry: intent.ledger_entry.as_ref(),
        idempotency_key: &intent.idempotency_key,
        expires_at: intent.expires_at.timestamp(),
    };
    if !signing_envelope_matches(&intent.signing_bytes, &envelope)
        || current_key.as_deref() != Some(intent.public_key.as_str())
        || !crate::ledger::verify_signature(&intent.signing_bytes, signature, &intent.public_key)
        || !snapshot_matches(&intent.action, &current_snapshot, &intent.snapshot)
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

fn validate_idempotency_key(idempotency_key: &str) -> AppResult<()> {
    if idempotency_key.is_empty() || idempotency_key.len() > MAX_IDEMPOTENCY_KEY_BYTES {
        return Err(AppError::BadRequest("Idempotency-Key must contain 1 to 128 bytes".into()));
    }
    Ok(())
}

fn snapshot_matches(action: &str, current: &Value, signed: &Value) -> bool {
    if action != "credit.product.purchase" {
        return current == signed;
    }
    let Some(signed_fields) = signed.as_object() else {
        return false;
    };
    if signed_fields.len() == 4 {
        return current == signed;
    }
    if signed_fields.len() != 5 || !signed_fields.get("title").is_some_and(Value::is_string) {
        return false;
    }
    let mut normalized = signed_fields.clone();
    normalized.remove("title");
    current == &Value::Object(normalized)
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
                "SELECT price, stock, status::text, seller_id \
                 FROM credit.products WHERE id = $1 FOR UPDATE"
            } else {
                "SELECT price, stock, status::text, seller_id \
                 FROM credit.products WHERE id = $1"
            };
            let snapshot: (i64, i32, String, i64) = sqlx::query_as(query)
                .bind(product_id)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(CreditError::ProductNotFound)?;
            Ok(serde_json::json!({
                "price": snapshot.0,
                "stock": snapshot.1,
                "status": snapshot.2,
                "sellerId": snapshot.3.to_string(),
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
    let snapshot: (String, i64, i64, i64) =
        sqlx::query_as(query).bind(entity_id).fetch_optional(&mut *conn).await?.ok_or_else(
            || {
                AppError::from(if table == "credit.purchases" {
                    CreditError::PurchaseNotFound
                } else {
                    CreditError::TaskNotFound
                })
            },
        )?;
    if table == "credit.tasks" {
        validate_task_signing_action(account_id, &snapshot.0, snapshot.1, snapshot.2, request)?;
    } else {
        validate_purchase_signing_action(account_id, &snapshot.0, snapshot.1, snapshot.2, request)?;
    }
    Ok(serde_json::json!({
        "status": snapshot.0,
        "partyA": snapshot.1.to_string(),
        "partyB": snapshot.2.to_string(),
        "amount": snapshot.3,
        "actorId": account_id.to_string(),
    }))
}

fn validate_task_signing_action(
    account_id: i64,
    status: &str,
    creator_id: i64,
    acceptor_id: i64,
    request: &Value,
) -> AppResult<()> {
    match required_string(request, "action")? {
        "confirm" if account_id != creator_id => {
            Err(CreditError::InvalidAction("only creator can confirm".into()).into())
        }
        "confirm" if acceptor_id == 0 || status != "submitted" => {
            Err(CreditError::StateConflict.into())
        }
        "confirm" => Ok(()),
        "cancel" if account_id != creator_id => {
            Err(CreditError::InvalidAction("only creator can cancel".into()).into())
        }
        "cancel" if !matches!(status, "open" | "in_progress" | "submitted") => {
            Err(CreditError::StateConflict.into())
        }
        "cancel" => Ok(()),
        "reject" if acceptor_id == 0 || account_id != acceptor_id => {
            Err(CreditError::InvalidAction("only acceptor can reject".into()).into())
        }
        "reject" if !matches!(status, "in_progress" | "submitted") => {
            Err(CreditError::StateConflict.into())
        }
        "reject" => Ok(()),
        "delete" if account_id != creator_id => {
            Err(CreditError::InvalidAction("only creator can delete".into()).into())
        }
        "delete" if status != "open" => Err(CreditError::StateConflict.into()),
        "delete" => Ok(()),
        action => {
            Err(CreditError::InvalidAction(format!("unsupported signed task action: {action}"))
                .into())
        }
    }
}

fn validate_purchase_signing_action(
    account_id: i64,
    status: &str,
    buyer_id: i64,
    seller_id: i64,
    request: &Value,
) -> AppResult<()> {
    if account_id != buyer_id && account_id != seller_id {
        return Err(CreditError::PurchaseNotFound.into());
    }

    match required_string(request, "action")? {
        "confirm" if account_id != buyer_id => {
            Err(CreditError::InvalidAction("only buyer can confirm".into()).into())
        }
        "confirm" if status != "delivered" => Err(CreditError::StateConflict.into()),
        "confirm" => Ok(()),
        "cancel" if account_id != buyer_id => {
            Err(CreditError::InvalidAction("only buyer can cancel".into()).into())
        }
        "cancel" if status != "pending" && status != "accepted" => {
            Err(CreditError::StateConflict.into())
        }
        "cancel" => Ok(()),
        action => {
            Err(CreditError::InvalidAction(format!("unsupported signed purchase action: {action}"))
                .into())
        }
    }
}

fn prepare_ledger_entry(
    account_id: i64,
    intent_id: uuid::Uuid,
    action: &str,
    request: &Value,
    snapshot: &Value,
    timestamp: i64,
) -> AppResult<Option<Value>> {
    let tx_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
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
            serde_json::json!({
                "tx_id": tx_id,
                "type": "escrow_hold",
                "from_account": account_id.to_string(),
                "to_account": Value::Null,
                "amount": amount,
                "nonce": nonce,
                "metadata": {
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
            serde_json::json!({
                "tx_id": tx_id,
                "type": "escrow_hold",
                "from_account": account_id.to_string(),
                "to_account": Value::Null,
                "amount": amount,
                "nonce": nonce,
                "metadata": {
                    "product_id": product_id.to_string(),
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

    use super::{
        normalize_request, prepare_ledger_entry, request_hash, snapshot_matches, INTENT_TTL_SECONDS,
    };

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

    #[test]
    fn prepared_ledger_timestamp_uses_the_intent_database_clock() {
        let observed_at = 1_725_000_000;
        let entry = prepare_ledger_entry(
            42,
            uuid::Uuid::nil(),
            "credit.task.create",
            &json!({ "title": "Task", "rewardAmount": 10 }),
            &json!({ "balance": 10 }),
            observed_at,
        )
        .unwrap()
        .expect("task creation prepares a ledger entry");

        assert_eq!(entry["timestamp"], observed_at);
        assert!(entry["timestamp"].as_i64().unwrap() <= observed_at + INTENT_TTL_SECONDS);
    }

    #[test]
    fn product_snapshot_accepts_only_the_legacy_title_extension() {
        let current = json!({
            "price": 10,
            "sellerId": "42",
            "status": "on_sale",
            "stock": 2,
        });
        let legacy = json!({
            "price": 10,
            "sellerId": "42",
            "status": "on_sale",
            "stock": 2,
            "title": "legacy product title",
        });
        assert!(snapshot_matches("credit.product.purchase", &current, &current));
        assert!(snapshot_matches("credit.product.purchase", &current, &legacy));

        let mut tampered = legacy;
        tampered["unexpected"] = json!(true);
        assert!(!snapshot_matches("credit.product.purchase", &current, &tampered));
        assert!(!snapshot_matches(
            "credit.product.purchase",
            &current,
            &json!({ "price": 10, "sellerId": "42", "status": "on_sale", "stock": 2, "title": null }),
        ));
    }
}

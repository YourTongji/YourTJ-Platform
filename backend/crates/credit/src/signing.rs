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
    expires_at: chrono::DateTime<Utc>,
    consumed_at: Option<chrono::DateTime<Utc>>,
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
         WHERE account_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(CreditError::WalletNotBound)?;

    let request_hash = request_hash(&input.request);
    let snapshot = build_snapshot(pool, account_id, &input.action, &input.request).await?;
    let intent_id = uuid::Uuid::new_v4();
    let expires_at = Utc::now().timestamp() + INTENT_TTL_SECONDS;
    let signing_bytes = crate::ledger::canonicalize(&serde_json::json!({
        "version": 1,
        "intentId": intent_id.to_string(),
        "accountId": account_id.to_string(),
        "publicKey": public_key,
        "action": input.action,
        "requestHash": request_hash,
        "snapshot": snapshot,
        "idempotencyKey": idempotency_key,
        "expiresAt": expires_at,
    }));

    let inserted = sqlx::query_as::<_, (uuid::Uuid, String, chrono::DateTime<Utc>)>(
        "INSERT INTO credit.signing_intents \
         (id, account_id, public_key, action, request_hash, snapshot, idempotency_key, signing_bytes, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9)) \
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
) -> AppResult<String> {
    let intent_id = required_header(headers, "x-wallet-intent")?
        .parse::<uuid::Uuid>()
        .map_err(|_| CreditError::IntentUnavailable)?;
    let signature = required_header(headers, "x-wallet-sig")?;
    let idempotency_key = required_header(headers, "idempotency-key")?;

    let intent = sqlx::query_as::<_, SigningIntentRow>(
        "SELECT account_id, public_key, action, request_hash, snapshot, idempotency_key, \
                signing_bytes, expires_at, consumed_at \
         FROM credit.signing_intents WHERE id = $1 FOR UPDATE",
    )
    .bind(intent_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(CreditError::IntentUnavailable)?;
    if intent.account_id != account_id
        || intent.action != action
        || intent.request_hash != request_hash(request)
        || intent.idempotency_key != idempotency_key
        || intent.expires_at.timestamp() <= Utc::now().timestamp()
        || intent.consumed_at.is_some()
    {
        return Err(CreditError::IntentUnavailable.into());
    }
    let current_key: Option<String> = sqlx::query_scalar(
        "SELECT public_key FROM identity.account_keys \
         WHERE account_id = $1 AND public_key = $2 AND revoked_at IS NULL",
    )
    .bind(account_id)
    .bind(&intent.public_key)
    .fetch_optional(&mut *conn)
    .await?;
    if current_key.is_none()
        || !crate::ledger::verify_signature(&intent.signing_bytes, signature, &intent.public_key)
        || build_snapshot_tx(conn, account_id, action, request).await? != intent.snapshot
    {
        return Err(CreditError::InvalidSignature.into());
    }
    sqlx::query("UPDATE credit.signing_intents SET consumed_at = now() WHERE id = $1")
        .bind(intent_id)
        .execute(&mut *conn)
        .await?;
    Ok(signature.to_string())
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
    build_snapshot_tx(&mut conn, account_id, action, request).await
}

async fn build_snapshot_tx(
    conn: &mut PgConnection,
    account_id: i64,
    action: &str,
    request: &Value,
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
            let snapshot: (i64, i32, String, i64) = sqlx::query_as(
                "SELECT price, stock, status::text, seller_id FROM credit.products WHERE id = $1",
            )
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
        "credit.task.action" => entity_snapshot(conn, "credit.tasks", request, account_id).await,
        "credit.purchase.action" => {
            entity_snapshot(conn, "credit.purchases", request, account_id).await
        }
        _ => Err(AppError::BadRequest("unsupported credit signing action".into())),
    }
}

async fn entity_snapshot(
    conn: &mut PgConnection,
    table: &str,
    request: &Value,
    account_id: i64,
) -> AppResult<Value> {
    let entity_id = request
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| id.parse::<i64>().ok())
        .ok_or_else(|| AppError::BadRequest("id is required".into()))?;
    let query = if table == "credit.tasks" {
        "SELECT status::text, creator_id, COALESCE(acceptor_id, 0), reward_amount FROM credit.tasks WHERE id = $1"
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::request_hash;

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
}

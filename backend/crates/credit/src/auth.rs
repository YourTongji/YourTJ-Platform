//! Local authentication helper for the credit domain.
//!
//! Exists because `identity` depends on `credit` (regular dep), so `credit`
//! cannot depend on `identity` (circular). This function duplicates the
//! JWT-verification + account-lookup logic from `identity::auth_middleware`.

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use shared::{auth, AuthAccount};
use sqlx::PgPool;

/// Resolve the authenticated account from request headers.
///
/// 1. Extracts Bearer token from `Authorization` header.
/// 2. Verifies JWT and extracts `sub` (account id).
/// 3. Looks up `status` and `role` from `identity.accounts`.
pub async fn authenticate(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
) -> Result<AuthAccount, axum::response::Response> {
    let header =
        headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or_else(auth::unauthorized)?;

    let token = header.strip_prefix("Bearer ").ok_or_else(auth::unauthorized)?;

    let claims = auth::verify_jwt(token, jwt_secret)?;
    let account_id: i64 = claims.sub.parse().map_err(|_| auth::unauthorized())?;

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(db)
            .await
            .map_err(|_| auth::internal_error())?
            .ok_or_else(auth::unauthorized)?;

    if status != "active" {
        return Err(auth::forbidden());
    }

    let role: String = sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(db)
        .await
        .map_err(|_| auth::internal_error())?;

    Ok(AuthAccount { id: account_id, role, status })
}

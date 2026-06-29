//! Authentication middleware that resolves `AuthAccount` from headers + DB.
//!
//! Call this at the start of authenticated handlers instead of
//! `AuthAccount::from_headers`. This function lives in the identity domain
//! because it queries `identity.accounts` for status/role.

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use shared::AuthAccount;
use sqlx::PgPool;

/// Resolve the authenticated account from the request headers.
///
/// 1. Extracts the Bearer token from the `Authorization` header.
/// 2. Verifies the JWT and extracts the `sub` claim (account id).
/// 3. Looks up `status` and `role` from `identity.accounts`.
///
/// Returns `Result<AuthAccount, Response>` for use with `map_err`.
#[tracing::instrument(skip(headers, db))]
pub async fn authenticate(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
) -> Result<AuthAccount, axum::response::Response> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(shared::auth::unauthorized)?;

    let token = header.strip_prefix("Bearer ").ok_or_else(shared::auth::unauthorized)?;

    let claims = shared::auth::verify_jwt(token, jwt_secret)?;
    let account_id: i64 = claims.sub.parse().map_err(|_| shared::auth::unauthorized())?;

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(db)
            .await
            .map_err(|_| shared::auth::internal_error())?
            .ok_or_else(shared::auth::unauthorized)?;

    if status != "active" {
        return Err(shared::auth::forbidden());
    }

    let role: String = sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(db)
        .await
        .map_err(|_| shared::auth::internal_error())?;

    Ok(AuthAccount { id: account_id, role, status })
}

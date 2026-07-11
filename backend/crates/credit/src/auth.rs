//! Local authentication helper for the credit domain.
//!
//! Exists because `identity` depends on `credit` (regular dep), so `credit`
//! cannot depend on `identity` (circular). This function duplicates the
//! JWT, revocable-session, account-status, and suspension checks from
//! `identity::auth_middleware`.

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use shared::{auth, AuthAccount};
use sqlx::PgPool;

#[derive(sqlx::FromRow)]
struct AccountAuthRow {
    role: String,
    status: String,
    auth_version: i64,
    legacy_access_revoked_before: chrono::DateTime<chrono::Utc>,
}

/// Resolve the authenticated account from request headers.
///
/// 1. Extracts Bearer token from `Authorization` header.
/// 2. Verifies JWT and extracts `sub` (account id).
/// 3. Enforces account status, access-session revocation, and active suspension.
pub async fn authenticate(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
) -> Result<AuthAccount, axum::response::Response> {
    let header =
        headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or_else(auth::unauthorized)?;

    let token = header.strip_prefix("Bearer ").ok_or_else(auth::unauthorized)?;

    let claims = auth::verify_jwt(token, jwt_secret)?;
    if claims.scope.is_some() {
        return Err(auth::forbidden());
    }
    let account_id: i64 = claims.sub.parse().map_err(|_| auth::unauthorized())?;

    let account = sqlx::query_as::<_, AccountAuthRow>(
        "SELECT role::text, status::text, auth_version, legacy_access_revoked_before \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(account_id)
    .fetch_optional(db)
    .await
    .map_err(|_| auth::internal_error())?
    .ok_or_else(auth::unauthorized)?;

    if account.status != "active" {
        return Err(auth::forbidden());
    }
    match (claims.sid.as_deref(), claims.ver) {
        (Some(session_id), Some(auth_version)) => {
            if account.auth_version != auth_version {
                return Err(auth::unauthorized());
            }
            let session_id = session_id.parse::<i64>().map_err(|_| auth::unauthorized())?;
            let is_active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM identity.sessions \
                 WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL AND expires_at > now())",
            )
            .bind(session_id)
            .bind(account_id)
            .fetch_one(db)
            .await
            .map_err(|_| auth::internal_error())?;
            if !is_active {
                return Err(auth::unauthorized());
            }
        }
        (None, None) => {
            let issued_at = i64::try_from(claims.iat).map_err(|_| auth::unauthorized())?;
            let issued_at =
                chrono::DateTime::from_timestamp(issued_at, 0).ok_or_else(auth::unauthorized)?;
            if account.legacy_access_revoked_before > issued_at {
                return Err(auth::unauthorized());
            }
        }
        _ => return Err(auth::unauthorized()),
    }
    let is_suspended: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'suspend' AND revoked_at IS NULL \
           AND (ends_at IS NULL OR ends_at > now()))",
    )
    .bind(account_id)
    .fetch_one(db)
    .await
    .map_err(|_| auth::internal_error())?;
    if is_suspended {
        return Err(auth::forbidden());
    }

    Ok(AuthAccount { id: account_id, role: account.role, status: account.status })
}

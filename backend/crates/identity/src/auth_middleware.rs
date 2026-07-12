//! Authentication middleware that resolves `AuthAccount` from headers + DB.
//!
//! Call this at the start of authenticated handlers instead of
//! `AuthAccount::from_headers`. This function lives in the identity domain
//! because it queries `identity.accounts` for status/role.

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use shared::AuthAccount;
use sqlx::PgPool;

/// A successful step-up remains valid for ten minutes on one revocable session family.
pub const RECENT_AUTH_WINDOW_SECONDS: i64 = 10 * 60;

/// Authenticated account plus the revocable session bound to its JWT, when present.
#[derive(Debug, Clone)]
pub struct AuthenticatedContext {
    pub account: AuthAccount,
    pub session_id: Option<i64>,
}

/// Server-side recent-auth state; JWT issuance time is intentionally absent.
#[derive(Debug, Clone)]
pub struct RecentAuthState {
    pub session_bound: bool,
    pub authenticated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub method: Option<String>,
    is_fresh: bool,
}

impl RecentAuthState {
    pub fn is_fresh(&self) -> bool {
        self.is_fresh
    }
}

#[derive(sqlx::FromRow)]
struct AccountAuthRow {
    role: String,
    status: String,
    auth_version: i64,
    legacy_access_revoked_before: chrono::DateTime<chrono::Utc>,
    onboarding_complete: bool,
}

/// Resolve the authenticated account from the request headers.
///
/// 1. Extracts the Bearer token from the `Authorization` header.
/// 2. Verifies the JWT and extracts the `sub` claim (account id).
/// 3. Looks up `status` and `role` from `identity.accounts`.
/// 4. Checks for active suspensions (cached in Redis).
///
/// Returns `Result<AuthAccount, Response>` for use with `map_err`.
#[tracing::instrument(skip(headers, db, jwt_secret, redis))]
pub async fn authenticate(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
    redis: Option<&deadpool_redis::Pool>,
) -> Result<AuthAccount, axum::response::Response> {
    authenticate_context(headers, db, jwt_secret, redis).await.map(|context| context.account)
}

/// Authenticate when a bearer header is present, preserving invalid-token failures.
pub async fn authenticate_optional(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
    redis: Option<&deadpool_redis::Pool>,
) -> Result<Option<AuthAccount>, axum::response::Response> {
    if !headers.contains_key(AUTHORIZATION) {
        return Ok(None);
    }
    authenticate(headers, db, jwt_secret, redis).await.map(Some)
}

/// Resolve the account and expose the current server-side session to identity handlers.
#[tracing::instrument(skip(headers, db, jwt_secret, redis))]
pub async fn authenticate_context(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
    redis: Option<&deadpool_redis::Pool>,
) -> Result<AuthenticatedContext, axum::response::Response> {
    authenticate_context_inner(headers, db, jwt_secret, redis, false).await
}

/// Resolve an active account for onboarding, owner security, or logout routes.
///
/// Ordinary domain authentication remains closed until onboarding has accepted the current
/// required choices; this narrower entry point must not be used by community interactions.
#[tracing::instrument(skip(headers, db, jwt_secret, redis))]
pub async fn authenticate_context_allow_incomplete_onboarding(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
    redis: Option<&deadpool_redis::Pool>,
) -> Result<AuthenticatedContext, axum::response::Response> {
    authenticate_context_inner(headers, db, jwt_secret, redis, true).await
}

async fn authenticate_context_inner(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
    redis: Option<&deadpool_redis::Pool>,
    allow_incomplete_onboarding: bool,
) -> Result<AuthenticatedContext, axum::response::Response> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(shared::auth::unauthorized)?;

    let token = header.strip_prefix("Bearer ").ok_or_else(shared::auth::unauthorized)?;

    let claims = shared::auth::verify_jwt(token, jwt_secret)?;
    if claims.scope.is_some() {
        return Err(shared::auth::forbidden());
    }
    let account_id: i64 = claims.sub.parse().map_err(|_| shared::auth::unauthorized())?;

    let account = sqlx::query_as::<_, AccountAuthRow>(
        "SELECT account.role::text, account.status::text, account.auth_version, \
                account.legacy_access_revoked_before, \
                COALESCE(onboarding.completed_at IS NOT NULL, FALSE) AS onboarding_complete \
         FROM identity.accounts account \
         LEFT JOIN identity.account_onboarding onboarding ON onboarding.account_id = account.id \
         WHERE account.id = $1",
    )
    .bind(account_id)
    .fetch_optional(db)
    .await
    .map_err(|_| shared::auth::internal_error())?
    .ok_or_else(shared::auth::unauthorized)?;

    if account.status != "active" {
        return Err(shared::auth::forbidden());
    }
    if !allow_incomplete_onboarding && !account.onboarding_complete {
        return Err(shared::auth::forbidden());
    }

    let session_id = match (claims.sid.as_deref(), claims.ver) {
        (Some(session_id), Some(auth_version)) => {
            if account.auth_version != auth_version {
                return Err(shared::auth::unauthorized());
            }
            let session_id = session_id.parse::<i64>().map_err(|_| shared::auth::unauthorized())?;
            let is_active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM identity.sessions \
                 WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL \
                   AND expires_at > now())",
            )
            .bind(session_id)
            .bind(account_id)
            .fetch_one(db)
            .await
            .map_err(|_| shared::auth::internal_error())?;
            if !is_active {
                return Err(shared::auth::unauthorized());
            }
            sqlx::query(
                "UPDATE identity.sessions SET last_used_at = now() \
                 WHERE id = $1 AND last_used_at < now() - interval '5 minutes'",
            )
            .bind(session_id)
            .execute(db)
            .await
            .map_err(|_| shared::auth::internal_error())?;
            Some(session_id)
        }
        (None, None) => {
            let issued_at = i64::try_from(claims.iat).map_err(|_| shared::auth::unauthorized())?;
            let issued_at = chrono::DateTime::from_timestamp(issued_at, 0)
                .ok_or_else(shared::auth::unauthorized)?;
            if account.legacy_access_revoked_before > issued_at {
                return Err(shared::auth::unauthorized());
            }
            None
        }
        _ => return Err(shared::auth::unauthorized()),
    };

    // Check for active suspension
    if crate::sanctions::is_suspended(redis, db, account_id)
        .await
        .map_err(|_| shared::auth::internal_error())?
    {
        return Err(shared::auth::forbidden());
    }

    Ok(AuthenticatedContext {
        account: AuthAccount { id: account_id, role: account.role, status: account.status },
        session_id,
    })
}

/// Read freshness from the active server-side session, never from JWT claims.
pub async fn recent_auth_state(
    context: &AuthenticatedContext,
    db: &PgPool,
) -> shared::AppResult<RecentAuthState> {
    let Some(session_id) = context.session_id else {
        return Ok(RecentAuthState {
            session_bound: false,
            authenticated_at: None,
            method: None,
            is_fresh: false,
        });
    };
    let state: Option<(Option<chrono::DateTime<chrono::Utc>>, Option<String>, bool)> =
        sqlx::query_as(
            "SELECT session.recent_authenticated_at, session.recent_auth_method, \
                COALESCE(session.recent_authenticated_at <= now() + interval '1 minute' \
                  AND session.recent_authenticated_at > now() - ($3::bigint * interval '1 second') \
                  AND (session.recent_auth_method <> 'password' \
                    OR session.recent_auth_credential_version = account.credential_version), false) \
                  AS is_fresh \
         FROM identity.sessions session \
         JOIN identity.accounts account ON account.id = session.account_id \
         WHERE session.id = $1 AND session.account_id = $2 \
           AND session.revoked_at IS NULL AND session.expires_at > now()",
        )
        .bind(session_id)
        .bind(context.account.id)
        .bind(RECENT_AUTH_WINDOW_SECONDS)
        .fetch_optional(db)
        .await?;
    let Some((authenticated_at, method, is_fresh)) = state else {
        return Ok(RecentAuthState {
            session_bound: false,
            authenticated_at: None,
            method: None,
            is_fresh: false,
        });
    };
    Ok(RecentAuthState { session_bound: true, authenticated_at, method, is_fresh })
}

/// Lock and require fresh session state inside the high-risk mutation transaction.
pub async fn require_recent_auth_tx(
    context: &AuthenticatedContext,
    tx: &mut sqlx::PgConnection,
) -> shared::AppResult<()> {
    let Some(session_id) = context.session_id else {
        return Err(shared::AppError::RecentAuthRequired);
    };
    let credential_version: Option<i64> = sqlx::query_scalar(
        "SELECT credential_version FROM identity.accounts WHERE id = $1 FOR SHARE",
    )
    .bind(context.account.id)
    .fetch_optional(&mut *tx)
    .await?;
    let credential_version = credential_version.ok_or(shared::AppError::RecentAuthRequired)?;
    let is_fresh: Option<bool> = sqlx::query_scalar(
        "SELECT COALESCE(recent_authenticated_at <= now() + interval '1 minute' \
                  AND recent_authenticated_at > now() - ($3::bigint * interval '1 second') \
                  AND (recent_auth_method <> 'password' \
                    OR recent_auth_credential_version = $4), false) \
         FROM identity.sessions \
         WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL AND expires_at > now() \
         FOR SHARE",
    )
    .bind(session_id)
    .bind(context.account.id)
    .bind(RECENT_AUTH_WINDOW_SECONDS)
    .bind(credential_version)
    .fetch_optional(tx)
    .await?;
    if is_fresh == Some(true) {
        Ok(())
    } else {
        Err(shared::AppError::RecentAuthRequired)
    }
}

/// Authenticate a normal account session or a short-lived appeal-only token.
///
/// Appeal-only tokens deliberately bypass an active suspension but cannot pass the regular
/// authentication middleware, so they cannot read profile, content, credit, or staff routes.
pub async fn authenticate_appeal_subject(
    headers: &HeaderMap,
    db: &PgPool,
    jwt_secret: &str,
    redis: Option<&deadpool_redis::Pool>,
) -> Result<AuthAccount, axum::response::Response> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(shared::auth::unauthorized)?;
    let token = header.strip_prefix("Bearer ").ok_or_else(shared::auth::unauthorized)?;
    let claims = shared::auth::verify_jwt(token, jwt_secret)?;
    if claims.scope.as_deref() != Some("appeal") {
        return authenticate(headers, db, jwt_secret, redis).await;
    }
    if claims.sid.is_some() || claims.ver.is_some() {
        return Err(shared::auth::unauthorized());
    }
    let account_id: i64 = claims.sub.parse().map_err(|_| shared::auth::unauthorized())?;
    let account: Option<(String, String)> =
        sqlx::query_as("SELECT role::text, status::text FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_optional(db)
            .await
            .map_err(|_| shared::auth::internal_error())?;
    let (role, status) = account.ok_or_else(shared::auth::unauthorized)?;
    if status != "active" && status != "suspended" {
        return Err(shared::auth::forbidden());
    }
    Ok(AuthAccount { id: account_id, role, status })
}

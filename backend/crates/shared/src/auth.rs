//! Authentication helpers for Axum handlers. These live in `shared` so every
//! domain crate can use them without depending on the identity crate.

use axum::http::header::AUTHORIZATION;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

/// Claims extracted from a verified JWT access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

/// An authenticated account, resolved from the bearer token in a header map.
#[derive(Debug, Clone)]
pub struct AuthAccount {
    pub id: i64,
    pub role: String,
    pub status: String,
}

impl AuthAccount {
    #[allow(clippy::result_large_err)]
    pub fn require_mod(&self) -> Result<(), Response> {
        if self.role == "mod" || self.role == "admin" {
            Ok(())
        } else {
            Err(forbidden())
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn require_admin(&self) -> Result<(), Response> {
        if self.role == "admin" {
            Ok(())
        } else {
            Err(forbidden())
        }
    }
}

/// Resolve the authenticated account from headers + DB.
/// Call this at the start of authenticated handlers:
/// ```ignore
/// let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret).await?;
/// ```
impl AuthAccount {
    pub async fn from_headers(
        headers: &axum::http::HeaderMap,
        db: &PgPool,
        jwt_secret: &str,
    ) -> Result<Self, Response> {
        let header =
            headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or_else(unauthorized)?;

        let token = header.strip_prefix("Bearer ").ok_or_else(unauthorized)?;

        let claims = verify_jwt(token, jwt_secret)?;
        let account_id: i64 = claims.sub.parse().map_err(|_| unauthorized())?;

        let status: String =
            sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
                .bind(account_id)
                .fetch_optional(db)
                .await
                .map_err(|_| internal_error())?
                .ok_or_else(unauthorized)?;

        if status != "active" {
            return Err(forbidden());
        }

        let role: String =
            sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1")
                .bind(account_id)
                .fetch_one(db)
                .await
                .map_err(|_| internal_error())?;

        Ok(AuthAccount { id: account_id, role, status })
    }
}

#[allow(clippy::result_large_err)]
fn verify_jwt(token: &str, secret: &str) -> Result<JwtClaims, Response> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    let mut v = Validation::new(jsonwebtoken::Algorithm::HS256);
    v.validate_exp = true;
    let key = DecodingKey::from_secret(secret.as_bytes());
    decode::<JwtClaims>(token, &key, &v).map(|d| d.claims).map_err(|_| unauthorized())
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error":{"code":"UNAUTHORIZED","message":"unauthorized"}})),
    )
        .into_response()
}

fn forbidden() -> Response {
    (StatusCode::FORBIDDEN, Json(json!({"error":{"code":"FORBIDDEN","message":"forbidden"}})))
        .into_response()
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error":{"code":"INTERNAL","message":"internal server error"}})),
    )
        .into_response()
}

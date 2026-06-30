//! Authentication types for Axum handlers.
//!
//! The `AuthAccount` struct and JWT verification live here so every domain crate
//! can use them without depending on the identity crate. The actual DB lookup
//! (account status / role) lives in `identity::auth_middleware`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

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

/// Verify a JWT access token and return the parsed claims.
#[allow(clippy::result_large_err)]
pub fn verify_jwt(token: &str, secret: &str) -> Result<JwtClaims, Response> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    let mut v = Validation::new(jsonwebtoken::Algorithm::HS256);
    v.validate_exp = true;
    let key = DecodingKey::from_secret(secret.as_bytes());
    decode::<JwtClaims>(token, &key, &v).map(|d| d.claims).map_err(|_| unauthorized())
}

pub fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error":{"code":"UNAUTHORIZED","message":"unauthorized"}})),
    )
        .into_response()
}

pub fn forbidden() -> Response {
    (StatusCode::FORBIDDEN, Json(json!({"error":{"code":"FORBIDDEN","message":"forbidden"}})))
        .into_response()
}

pub fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error":{"code":"INTERNAL","message":"internal server error"}})),
    )
        .into_response()
}

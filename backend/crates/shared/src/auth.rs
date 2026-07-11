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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ver: Option<i64>,
}

/// An authenticated account, resolved from the bearer token in a header map.
#[derive(Debug, Clone)]
pub struct AuthAccount {
    pub id: i64,
    pub role: String,
    pub status: String,
}

/// Named server capabilities derived from the persisted account role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    ModerateContent,
    SearchUsers,
    SilenceUsers,
    ReadAudit,
    InviteUsers,
    ChangeRoles,
    SuspendUsers,
    ManageCommunity,
    ManageCourses,
    ManagePlatform,
    ManageActivity,
    ManageAnnouncements,
    ManagePromotions,
    ManageVerifications,
    RunOperations,
    ManageCreditIntegrity,
}

impl Capability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModerateContent => "moderation.content",
            Self::SearchUsers => "users.search",
            Self::SilenceUsers => "users.silence",
            Self::ReadAudit => "audit.read",
            Self::InviteUsers => "users.invite",
            Self::ChangeRoles => "users.roles",
            Self::SuspendUsers => "users.suspend",
            Self::ManageCommunity => "community.manage",
            Self::ManageCourses => "courses.manage",
            Self::ManagePlatform => "platform.settings",
            Self::ManageActivity => "activity.policy",
            Self::ManageAnnouncements => "announcements.manage",
            Self::ManagePromotions => "promotions.manage",
            Self::ManageVerifications => "verifications.manage",
            Self::RunOperations => "operations.jobs",
            Self::ManageCreditIntegrity => "credit.integrity",
        }
    }
}

const MOD_CAPABILITIES: &[Capability] = &[
    Capability::ModerateContent,
    Capability::SearchUsers,
    Capability::SilenceUsers,
    Capability::ReadAudit,
];

const ADMIN_CAPABILITIES: &[Capability] = &[
    Capability::ModerateContent,
    Capability::SearchUsers,
    Capability::SilenceUsers,
    Capability::ReadAudit,
    Capability::InviteUsers,
    Capability::ChangeRoles,
    Capability::SuspendUsers,
    Capability::ManageCommunity,
    Capability::ManageCourses,
    Capability::ManagePlatform,
    Capability::ManageActivity,
    Capability::ManageAnnouncements,
    Capability::ManagePromotions,
    Capability::ManageVerifications,
    Capability::RunOperations,
    Capability::ManageCreditIntegrity,
];

pub fn capabilities_for_role(role: &str) -> &'static [Capability] {
    match role {
        "mod" => MOD_CAPABILITIES,
        "admin" => ADMIN_CAPABILITIES,
        _ => &[],
    }
}

pub fn capability_names_for_role(role: &str) -> Vec<String> {
    capabilities_for_role(role).iter().map(|capability| capability.as_str().to_string()).collect()
}

impl AuthAccount {
    pub fn has_capability(&self, capability: Capability) -> bool {
        capabilities_for_role(&self.role).contains(&capability)
    }

    #[allow(clippy::result_large_err)] // reason: authorization guards return tower Response directly for handler composition
    pub fn require_capability(&self, capability: Capability) -> Result<(), Response> {
        if self.has_capability(capability) {
            Ok(())
        } else {
            Err(forbidden())
        }
    }

    #[allow(clippy::result_large_err)] // reason: require_mod returns tower Response directly for middleware-like guards; boxing would add indirection without benefit
    pub fn require_mod(&self) -> Result<(), Response> {
        if self.role == "mod" || self.role == "admin" {
            Ok(())
        } else {
            Err(forbidden())
        }
    }

    #[allow(clippy::result_large_err)] // reason: require_admin returns tower Response directly for middleware-like guards; boxing would add indirection without benefit
    pub fn require_admin(&self) -> Result<(), Response> {
        if self.role == "admin" {
            Ok(())
        } else {
            Err(forbidden())
        }
    }
}

/// Verify a JWT access token and return the parsed claims.
#[allow(clippy::result_large_err)] // reason: verify_jwt returns a tower Response on failure so it can be used directly as an Axum extractor; boxing would not improve this
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

#[cfg(test)]
mod tests {
    use super::{capabilities_for_role, AuthAccount, Capability};

    #[test]
    fn moderator_has_moderation_but_not_configuration_capabilities() {
        let moderator = AuthAccount { id: 1, role: "mod".into(), status: "active".into() };
        assert!(moderator.has_capability(Capability::ModerateContent));
        assert!(moderator.has_capability(Capability::SilenceUsers));
        assert!(!moderator.has_capability(Capability::ManageActivity));
        assert!(!moderator.has_capability(Capability::ManagePromotions));
        assert!(!moderator.has_capability(Capability::ManageVerifications));
        assert!(!moderator.has_capability(Capability::ManageCreditIntegrity));
        assert!(!moderator.has_capability(Capability::ChangeRoles));
    }

    #[test]
    fn administrator_has_independent_platform_management_capabilities() {
        let administrator = AuthAccount { id: 1, role: "admin".into(), status: "active".into() };
        assert!(administrator.has_capability(Capability::ManageAnnouncements));
        assert!(administrator.has_capability(Capability::ManagePromotions));
        assert!(administrator.has_capability(Capability::ManageVerifications));
        assert!(administrator.has_capability(Capability::ManageCreditIntegrity));
    }

    #[test]
    fn ordinary_user_has_no_staff_capabilities() {
        assert!(capabilities_for_role("user").is_empty());
    }
}

//! Request and response types for the identity domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// POST /auth/email/request-code
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCodeInput {
    pub email: String,
    pub captcha_token: String,
}

/// POST /auth/email/verify
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailInput {
    pub email: String,
    pub code: String,
    pub handle: Option<String>,
    pub password: Option<String>,
}

/// POST /auth/password/login
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordLoginInput {
    pub email: String,
    pub password: String,
}

/// POST /auth/password/forgot
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordForgotInput {
    pub email: String,
    pub captcha_token: String,
}

/// POST /auth/password/reset
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetInput {
    pub email: String,
    pub code: String,
    pub new_password: String,
}

/// POST /auth/password/change
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChangeInput {
    pub current_password: String,
    pub new_password: String,
}

/// Returned on successful verification / refresh.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthTokensOutput {
    pub access_token: String,
    pub refresh_token: String,
    pub account: AccountDto,
}

/// Public-facing account DTO. The real email is *never* exposed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub id: String,
    pub handle: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub capabilities: Vec<String>,
    pub trust_level: i16,
    pub created_at: i64,
}

/// POST /auth/refresh — the client sends the refresh token in the JSON body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshInput {
    pub refresh_token: String,
}

/// PATCH /me
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeInput {
    pub handle: Option<String>,
    pub avatar_url: Option<String>,
}

/// POST /wallet/bind
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindKeyInput {
    pub public_key: String,
}

/// GET /wallet/claim-challenge response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimChallengeOutput {
    pub challenge_id: String,
    pub nonce: String,
}

/// POST /wallet/claim request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimInput {
    pub legacy_user_hash: String,
    pub challenge_id: String,
    pub signature: String,
}

/// Wallet balance DTO (replaces legacy WalletOutput).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDto {
    pub account_id: String,
    pub balance: i64,
}

/// A privacy-safe account record for the staff user directory.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDto {
    pub id: String,
    pub handle: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub status: String,
    pub trust_level: i16,
    pub last_active_at: Option<i64>,
    pub created_at: i64,
}

/// POST /admin/users provisions an unverified campus-email invitation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserInviteInput {
    pub email: String,
    pub handle: String,
    pub reason: String,
}

/// PATCH /admin/users/{id}/role changes a persisted platform role.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserRoleInput {
    pub role: String,
    pub reason: String,
}

/// A mandatory human-readable justification for a privileged action.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReasonInput {
    pub reason: String,
}

/// A reversible user sanction returned to staff.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SanctionDto {
    pub id: String,
    pub account_id: String,
    pub kind: String,
    pub reason: String,
    pub issued_by: Option<String>,
    pub starts_at: i64,
    pub ends_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

/// POST /admin/users/{id}/{silence,suspend} creates a sanction.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SanctionInput {
    pub reason: String,
    pub ends_at: Option<i64>,
}

/// POST /admin/users/{id}/unsanction revokes one active sanction.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsanctionInput {
    pub sanction_id: String,
    pub reason: String,
}

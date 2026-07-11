//! Request and response types for the identity domain.
//!
//! Every serialisable struct carries `#[serde(rename_all = "camelCase")]`
//! so the JSON wire format uses camelCase keys.

use serde::{Deserialize, Serialize};

/// Public email-code purposes. Password reset has its own dedicated endpoint.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailCodePurpose {
    Login,
    Registration,
    Appeal,
}

impl From<EmailCodePurpose> for crate::email_code::CodePurpose {
    fn from(value: EmailCodePurpose) -> Self {
        match value {
            EmailCodePurpose::Login => Self::Login,
            EmailCodePurpose::Registration => Self::Registration,
            EmailCodePurpose::Appeal => Self::Appeal,
        }
    }
}

/// POST /auth/email/request-code
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCodeInput {
    pub email: String,
    pub captcha_token: String,
    pub purpose: Option<EmailCodePurpose>,
}

/// POST /auth/email/verify
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailInput {
    pub email: String,
    pub code: String,
    pub handle: Option<String>,
    pub password: Option<String>,
    pub purpose: Option<EmailCodePurpose>,
}

/// POST /auth/password/login
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordLoginInput {
    pub email: String,
    pub password: String,
}

/// POST /auth/appeal/email/verify
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealEmailVerificationInput {
    pub email: String,
    pub code: String,
}

/// A short-lived credential accepted only by the appeal-center routes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealAccessTokenOutput {
    pub access_token: String,
    pub expires_at: i64,
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

/// One revocable device session owned by the authenticated account.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub is_current: bool,
    pub device_label: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
    pub expires_at: i64,
}

/// Server-verifiable methods that can refresh the current session's freshness.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecentAuthMethod {
    Password,
    EmailCode,
}

/// Current owner-visible step-up state without exposing an email address.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAuthStatusDto {
    pub session_bound: bool,
    pub is_fresh: bool,
    pub authenticated_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub method: Option<RecentAuthMethod>,
    pub available_methods: Vec<RecentAuthMethod>,
}

/// POST /auth/recent-auth/verify verifies exactly one method.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAuthVerifyInput {
    pub method: RecentAuthMethod,
    pub password: Option<String>,
    pub code: Option<String>,
}

/// PATCH /me
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeInput {
    pub handle: Option<String>,
}

/// Profile text and controlled media references visible to the account owner.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyProfileDto {
    pub account_id: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub avatar_asset_id: Option<String>,
    pub banner_asset_id: Option<String>,
}

/// PUT /me/profile replaces every owner-editable text field.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdateInput {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
}

/// Profile and relationship privacy settings owned by the account.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePrivacyDto {
    pub profile_visibility: String,
    pub activity_visibility: String,
    pub followers_visibility: String,
    pub following_visibility: String,
    pub discoverable: bool,
    pub dm_policy: String,
    pub mention_policy: String,
}

/// PUT /me/privacy. Optional new fields preserve older clients during a rolling deploy.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePrivacyUpdateInput {
    pub profile_visibility: String,
    pub activity_visibility: Option<String>,
    pub followers_visibility: String,
    pub following_visibility: String,
    pub discoverable: bool,
    pub dm_policy: String,
    pub mention_policy: Option<String>,
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

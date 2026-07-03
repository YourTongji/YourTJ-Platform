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
}

/// POST /auth/email/verify
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailInput {
    pub email: String,
    pub code: String,
    pub handle: Option<String>,
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

/// GET /api/v2/users/{handle} — public user profile.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileDto {
    pub handle: String,
    pub avatar_url: Option<String>,
    pub trust_level: i16,
    pub badges: Vec<UserBadgeDto>,
    pub thread_count: i32,
    pub comment_count: i32,
    pub votes_received: i32,
    pub created_at: i64,
}

/// A single badge entry inside a user profile.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBadgeDto {
    pub slug: String,
    pub name: String,
}

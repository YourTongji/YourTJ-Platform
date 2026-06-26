//! Axum request handlers for the identity domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use sha2::Digest as _;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use shared::{AppResult, AppState, AuthAccount};

use crate::auth::{create_access_token, generate_refresh_token};
use crate::dto::{
    AccountDto, AuthTokensOutput, BindKeyInput, RefreshInput, RequestCodeInput, UpdateMeInput,
    VerifyEmailInput, WalletOutput,
};
use crate::email_code::{generate_code, hash_code, verify_code};
use crate::error::IdentityError;
use crate::repo;

// ---------------------------------------------------------------------------
// Rate limiter
// ---------------------------------------------------------------------------

/// Per-email minimum interval between code requests (60 seconds).
const CODE_RATE_LIMIT: Duration = Duration::from_secs(60);

/// Thread-local rate limiter. In production this would be Redis-backed, but a
/// simple in-process map is sufficient for now.
static LAST_CODE_REQUEST: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn check_rate_limit(email: &str) -> Result<(), IdentityError> {
    let mut map = LAST_CODE_REQUEST.lock().expect("rate-limit lock poisoned");
    let now = Instant::now();
    if let Some(last) = map.get(email) {
        if now.duration_since(*last) < CODE_RATE_LIMIT {
            return Err(IdentityError::RateLimited);
        }
    }
    map.insert(email.to_lowercase(), now);
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_dto(row: &crate::models::AccountRow) -> AccountDto {
    AccountDto {
        id: row.id.to_string(),
        handle: row.handle.clone(),
        avatar_url: row.avatar_url.clone(),
        role: row.role.clone(),
        created_at: row.created_at.timestamp(),
    }
}

fn validate_handle(handle: &str) -> Result<(), IdentityError> {
    if handle.len() < 3 || handle.len() > 30 {
        return Err(IdentityError::InvalidHandle);
    }
    if !handle
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
    {
        return Err(IdentityError::InvalidHandle);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /auth/email/request-code
///
/// Rate-limited: one code per email per 60 seconds. Sends a 204 on success.
pub async fn request_code(
    State(state): State<AppState>,
    Json(body): Json<RequestCodeInput>,
) -> AppResult<StatusCode> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    check_rate_limit(&email)?;

    let code = generate_code();
    let code_hash = hash_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    repo::insert_email_code(&state.db, &email, &code_hash, expires_at).await?;

    // In production we would send the code via email here.
    tracing::info!(email = %email, "verification code generated (not sent — email SMTP not yet wired)");

    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/email/verify
///
/// Validates the code, creates or looks up the account, and returns JWT
/// access + refresh tokens.
pub async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    // Look up the live code row.
    let code_row =
        repo::find_email_code(&state.db, &email).await?.ok_or(IdentityError::CodeExpired)?;

    if code_row.attempts >= 5 {
        return Err(IdentityError::CodeExhausted.into());
    }

    let is_valid = verify_code(&body.code, &code_row.code_hash);
    repo::increment_code_attempts(&state.db, &email).await?;

    if !is_valid {
        return Err(IdentityError::InvalidCode.into());
    }

    // Find or create the account.
    let account = match repo::find_account_by_email(&state.db, &email).await? {
        Some(acct) => acct,
        None => {
            // First login — auto-provision.
            let handle_opt = body.handle.as_deref();
            // Validate handle if supplied.
            if let Some(h) = handle_opt {
                validate_handle(h)?;
            }
            repo::insert_account(&state.db, &email, handle_opt).await?
        }
    };

    // Create access + refresh tokens.
    let access_token = create_access_token(account.id, &state.jwt_secret, state.jwt_ttl)
        .map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);

    let session_id =
        repo::insert_session(&state.db, account.id, &refresh_hash, refresh_expires).await?;

    // Embed session_id so the refresh handler can look it up efficiently.
    let combined_refresh = format!("{session_id:x}:{refresh_plain}");

    Ok(Json(AuthTokensOutput {
        access_token,
        refresh_token: combined_refresh,
        account: row_to_dto(&account),
    }))
}

/// POST /auth/refresh
///
/// Accepts a refresh token, validates it, revokes the old session, and
/// returns a new token pair.
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let refresh_plain = body.refresh_token;

    // Parse session_id:random_hex
    let (sid_hex, random_part) =
        refresh_plain.split_once(':').ok_or(shared::AppError::Unauthorized)?;

    let sid = i64::from_str_radix(sid_hex, 16).map_err(|_| shared::AppError::Unauthorized)?;

    let refresh_hash = hex::encode(sha2::Sha256::digest(random_part.as_bytes()));

    let session = repo::find_session(&state.db, sid, &refresh_hash)
        .await?
        .ok_or(shared::AppError::Unauthorized)?;

    // Revoke the old session.
    repo::revoke_session(&state.db, session.id).await?;

    // Get the account.
    let account = repo::find_account_by_id(&state.db, session.account_id)
        .await?
        .ok_or(shared::AppError::Unauthorized)?;

    // Create new token pair.
    let access_token = create_access_token(account.id, &state.jwt_secret, state.jwt_ttl)
        .map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
    let (new_refresh_plain, new_refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);

    let new_sid =
        repo::insert_session(&state.db, account.id, &new_refresh_hash, refresh_expires).await?;

    let combined_refresh = format!("{new_sid:x}:{new_refresh_plain}");

    Ok(Json(AuthTokensOutput {
        access_token,
        refresh_token: combined_refresh,
        account: row_to_dto(&account),
    }))
}

/// POST /auth/logout
///
/// Revokes every active session for the authenticated account.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;
    repo::revoke_all_sessions(&state.db, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /me
pub async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;
    let account =
        repo::find_account_by_id(&state.db, auth.id).await?.ok_or(shared::AppError::NotFound)?;
    Ok(Json(row_to_dto(&account)))
}

/// PATCH /me
pub async fn update_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateMeInput>,
) -> AppResult<Json<AccountDto>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    // Validate handle if provided.
    if let Some(ref handle) = body.handle {
        validate_handle(handle)?;
        // Check uniqueness (case-insensitive).
        let existing = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE handle = $1 AND id != $2)",
        )
        .bind(handle)
        .bind(auth.id)
        .fetch_one(&state.db)
        .await?;
        if existing {
            return Err(IdentityError::HandleTaken.into());
        }
    }

    let row = repo::update_account(
        &state.db,
        auth.id,
        body.handle.as_deref(),
        body.avatar_url.as_deref(),
    )
    .await?;

    Ok(Json(row_to_dto(&row)))
}

/// GET /wallet
pub async fn get_wallet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<WalletOutput>> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    let wallet = repo::find_wallet(&state.db, auth.id)
        .await?
        .unwrap_or(crate::models::WalletRow { account_id: auth.id, balance: 0 });

    Ok(Json(WalletOutput { account_id: wallet.account_id.to_string(), balance: wallet.balance }))
}

/// POST /wallet/bind
///
/// Bind an Ed25519 public key (base64-encoded, 32 bytes) to the
/// authenticated account.
pub async fn bind_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BindKeyInput>,
) -> AppResult<StatusCode> {
    let auth = AuthAccount::from_headers(&headers, &state.db, &state.jwt_secret)
        .await
        .map_err(|_r| shared::AppError::Unauthorized)?;

    // Decode base64 and validate exactly 32 bytes.
    let key_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &body.public_key)
            .map_err(|_| IdentityError::InvalidPublicKey)?;

    if key_bytes.len() != 32 {
        return Err(IdentityError::InvalidPublicKey.into());
    }

    repo::insert_account_key(&state.db, auth.id, &body.public_key).await?;

    Ok(StatusCode::NO_CONTENT)
}

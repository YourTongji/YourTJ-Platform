//! Axum request handlers for the identity domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

pub(crate) mod admin;

use sha2::Digest as _;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use shared::{AppResult, AppState};

use crate::auth::{create_access_token, generate_refresh_token};
use crate::dto::{
    AccountDto, AuthTokensOutput, BindKeyInput, ClaimChallengeOutput, ClaimInput,
    PasswordChangeInput, PasswordForgotInput, PasswordLoginInput, PasswordResetInput, RefreshInput,
    RegisterInput, RequestCodeInput, UpdateMeInput, VerifyEmailInput, WalletDto,
};
use crate::email_code::{generate_code, hash_code, verify_code, CodePurpose};
use crate::error::IdentityError;
use crate::password;
use crate::repo;

// Rate limiting is now handled by shared::ratelimit::check_token_bucket (Redis-backed).
// When Redis is unavailable the check passes through so we never block legitimate traffic.

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn row_to_dto(row: &crate::models::AccountRow) -> AccountDto {
    AccountDto {
        id: row.id.to_string(),
        handle: row.handle.clone(),
        avatar_url: row.avatar_url.clone(),
        role: row.role.clone(),
        capabilities: shared::auth::capability_names_for_role(&row.role),
        trust_level: row.trust_level,
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

async fn ensure_login_allowed(
    state: &AppState,
    account: &crate::models::AccountRow,
) -> AppResult<()> {
    if account.status != "active"
        || crate::sanctions::is_suspended(state.redis.as_ref(), &state.db, account.id).await?
    {
        return Err(shared::AppError::Forbidden);
    }
    Ok(())
}

async fn deliver_email_code(
    state: &AppState,
    email: &str,
    code_hash: &str,
    content: &crate::email_templates::EmailContent,
) -> AppResult<()> {
    if let Err(delivery_error) = shared::email::send_email(
        &state.config,
        email,
        content.subject,
        &content.text,
        Some(&content.html),
    )
    .await
    {
        if let Err(invalidation_error) = repo::invalidate_email_code(&state.db, code_hash).await {
            tracing::warn!(
                ?invalidation_error,
                "email delivery failed and verification code invalidation also failed"
            );
            return Err(invalidation_error);
        }
        return Err(delivery_error);
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
    headers: HeaderMap,
    Json(body): Json<RequestCodeInput>,
) -> AppResult<StatusCode> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    // Validate purpose.
    let purpose = CodePurpose::from_str(&body.purpose).ok_or(IdentityError::InvalidPurpose)?;

    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "email_code",
        &body.captcha_token,
    )
    .await?;

    // Rate-limit code requests: 1 per 60 seconds per email (Redis-backed).
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "email_code", &email, 1, 60)
        .await?;
    // Rate-limit by IP as well: 5 requests per 10 minutes.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "ip_code", &ip, 5, 600).await?;

    let code = generate_code();
    let code_hash = hash_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(10);
    let request_id = uuid::Uuid::new_v4();

    repo::insert_email_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        &code_hash,
        expires_at,
        purpose.as_str(),
        request_id,
    )
    .await?;

    let email_content = crate::email_templates::login_code(&code);
    deliver_email_code(&state, &email, &code_hash, &email_content).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/email/verify
///
/// Validates a login-purpose code and returns JWT tokens for the existing
/// account. Registration is handled separately by POST /auth/register.
pub async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    // Look up the live code row for login purpose.
    let code_row =
        repo::find_email_code(&state.db, state.email_encryption.as_ref(), &email, "login")
            .await?
            .ok_or(IdentityError::CodeExpired)?;

    if code_row.attempts >= 5 {
        return Err(IdentityError::CodeExhausted.into());
    }

    let is_valid = verify_code(&body.code, &code_row.code_hash);
    repo::increment_code_attempts(&state.db, state.email_encryption.as_ref(), &email).await?;

    if !is_valid {
        return Err(IdentityError::InvalidCode.into());
    }

    // Login-only: account must already exist.
    let account = repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email)
        .await?
        .ok_or(IdentityError::AccountNotFound)?;

    repo::ensure_invitation_valid(&state.db, account.id).await?;
    repo::mark_email_verified(&state.db, account.id).await?;

    ensure_login_allowed(&state, &account).await?;

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

/// POST /auth/register
///
/// Validates a registration-purpose code, creates the account with the given
/// handle, optionally sets a password, and returns JWT tokens.
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    validate_handle(&body.handle)?;

    // Check email not already registered.
    if repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email)
        .await?
        .is_some()
    {
        return Err(IdentityError::EmailAlreadyUsed.into());
    }

    // Look up the live code row for registration purpose.
    let code_row =
        repo::find_email_code(&state.db, state.email_encryption.as_ref(), &email, "registration")
            .await?
            .ok_or(IdentityError::CodeExpired)?;

    if code_row.attempts >= 5 {
        return Err(IdentityError::CodeExhausted.into());
    }

    let is_valid = verify_code(&body.code, &code_row.code_hash);
    repo::increment_code_attempts(&state.db, state.email_encryption.as_ref(), &email).await?;

    if !is_valid {
        return Err(IdentityError::InvalidCode.into());
    }

    // Create the account.
    let account =
        repo::insert_account(&state.db, state.email_encryption.as_ref(), &email, &body.handle)
            .await?;

    // Set password if provided.
    if let Some(pw) = body.password.as_deref() {
        password::validate(pw, &email)?;
        let hash = password::hash(pw)?;
        repo::update_password_hash(&state.db, account.id, &hash).await?;
    }

    ensure_login_allowed(&state, &account).await?;

    // Create access + refresh tokens.
    let access_token = create_access_token(account.id, &state.jwt_secret, state.jwt_ttl)
        .map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);

    let session_id =
        repo::insert_session(&state.db, account.id, &refresh_hash, refresh_expires).await?;

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
    let account =
        repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), session.account_id)
            .await?
            .ok_or(shared::AppError::Unauthorized)?;
    ensure_login_allowed(&state, &account).await?;

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
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
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
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;
    let account = repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.id)
        .await?
        .ok_or(shared::AppError::NotFound)?;
    Ok(Json(row_to_dto(&account)))
}

/// PATCH /me
pub async fn update_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateMeInput>,
) -> AppResult<Json<AccountDto>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
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
        state.email_encryption.as_ref(),
        auth.id,
        body.handle.as_deref(),
        body.avatar_url.as_deref(),
    )
    .await?;

    Ok(Json(row_to_dto(&row)))
}

/// GET /wallet/claim-challenge
///
/// Creates a random challenge with 10-minute expiry that the legacy app must
/// sign to prove ownership of the legacy wallet.
pub async fn claim_challenge(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ClaimChallengeOutput>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;

    let challenge_id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    repo::insert_claim_challenge(&state.db, &challenge_id, auth.id, &nonce, expires_at).await?;

    Ok(Json(ClaimChallengeOutput { challenge_id, nonce }))
}

/// POST /wallet/claim
///
/// Claims a legacy wallet by verifying the Ed25519 signature over the
/// canonical payload `{ accountId, challengeId, legacyUserHash, nonce }`.
///
/// Runs in a single transaction that locks the challenge and legacy wallet
/// link rows, validates all conditions, then auto-assigns legacy reviews
/// by `wallet_user_hash`, mints any legacy balance into the credit ledger,
/// and commits.
///
/// NOTE: This handler directly accesses `credit.ledger`, `credit.wallets`,
/// and `reviews.reviews` — an intentional exception to the domain-boundary
/// rule. The cross-domain access is architecturally necessary because this
/// is a one-time legacy-claim flow that must atomically link legacy data
/// into the new system. The tight coupling is confined to this single
/// handler and is not used as a precedent for other identity → cross-domain
/// queries.
#[tracing::instrument(skip(state, headers))]
pub async fn claim_wallet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ClaimInput>,
) -> AppResult<Json<WalletDto>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;

    let mut tx = state.db.begin().await?;

    // Lock the challenge row.
    let challenge = sqlx::query_as::<_, crate::models::WalletClaimChallengeRow>(
        "SELECT id, account_id, nonce, expires_at, used_at, created_at \
         FROM identity.wallet_claim_challenges \
         WHERE id = $1 FOR UPDATE",
    )
    .bind(&body.challenge_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(IdentityError::ChallengeNotFound)?;

    // Validate challenge conditions.
    if challenge.account_id != auth.id {
        return Err(IdentityError::ChallengeWrongAccount.into());
    }
    if challenge.used_at.is_some() {
        return Err(IdentityError::ChallengeAlreadyUsed.into());
    }
    if challenge.expires_at < Utc::now() {
        return Err(IdentityError::ChallengeExpired.into());
    }

    // Lock the legacy wallet link row.
    let link = sqlx::query_as::<_, crate::models::LegacyWalletLinkRow>(
        "SELECT legacy_user_hash, account_id, claimed_at, legacy_public_key, \
                legacy_balance, imported_metadata \
         FROM identity.legacy_wallet_links \
         WHERE legacy_user_hash = $1 FOR UPDATE",
    )
    .bind(&body.legacy_user_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(IdentityError::LegacyLinkNotFound)?;

    // Validate link conditions.
    if link.account_id.is_some() {
        return Err(IdentityError::LegacyLinkAlreadyClaimed.into());
    }

    let legacy_pk = link.legacy_public_key.as_deref().ok_or(IdentityError::LegacyNoPublicKey)?;

    // Reconstruct the canonical payload that the legacy wallet signed.
    let payload = serde_json::json!({
        "accountId": auth.id.to_string(),
        "challengeId": body.challenge_id,
        "legacyUserHash": body.legacy_user_hash,
        "nonce": challenge.nonce,
    });
    let canonical = serde_json::to_string(&payload).map_err(|_| {
        shared::AppError::Internal(anyhow::anyhow!("failed to serialize canonical claim payload"))
    })?;

    // Verify the signature.
    if !crate::ledger::verify_signature(&canonical, &body.signature, legacy_pk) {
        return Err(IdentityError::InvalidSignature.into());
    }

    // Mark challenge used.
    sqlx::query("UPDATE identity.wallet_claim_challenges SET used_at = now() WHERE id = $1")
        .bind(&body.challenge_id)
        .execute(&mut *tx)
        .await?;

    // Claim the link.
    sqlx::query(
        "UPDATE identity.legacy_wallet_links \
         SET account_id = $2, claimed_at = now() \
         WHERE legacy_user_hash = $1",
    )
    .bind(&body.legacy_user_hash)
    .bind(auth.id)
    .execute(&mut *tx)
    .await?;

    // Auto-assign legacy reviews (by wallet_user_hash) to the claimed account.
    let claimed_review_count = sqlx::query(
        "UPDATE reviews.reviews SET account_id = $1 \
         WHERE wallet_user_hash = $2 AND account_id IS NULL",
    )
    .bind(auth.id)
    .bind(&body.legacy_user_hash)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if claimed_review_count > 0 {
        tracing::info!(account_id = auth.id, legacy_user_hash = %body.legacy_user_hash,
            count = claimed_review_count, "claimed legacy reviews");
    }

    // If there is a legacy balance, mint points into the credit ledger.
    if link.legacy_balance > 0 {
        let tx_id = format!("legacy_claim:{}", body.legacy_user_hash);
        let nonce = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now().timestamp();
        let metadata = serde_json::json!({
            "reason": "legacy_wallet_claim",
            "legacy_user_hash": body.legacy_user_hash,
        });

        // Build canonical payload and sign with system key.
        let canonical = credit::ledger::build_ledger_canonical(
            &tx_id,
            "mint",
            None,
            Some(auth.id),
            link.legacy_balance,
            &nonce,
            Some(&metadata),
            "system",
            created_at,
        );
        let signature = credit::ledger::sign_with_seed(&canonical, &state.system_private_key);

        // Append mint entry via the shared repo function.
        let prev_hash: Option<String> =
            sqlx::query_scalar("SELECT hash FROM credit.ledger ORDER BY seq DESC LIMIT 1")
                .fetch_optional(&mut *tx)
                .await?;
        let prev_hash = prev_hash.unwrap_or_else(|| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        });

        let hash = credit::ledger::compute_hash(&canonical, &prev_hash);

        sqlx::query(
            "INSERT INTO credit.ledger \
             (tx_id, type, from_account, to_account, amount, nonce, metadata, \
              signer, signature, prev_hash, hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&tx_id)
        .bind("mint")
        .bind(None::<i64>)
        .bind(auth.id)
        .bind(link.legacy_balance)
        .bind(&nonce)
        .bind(&metadata)
        .bind("system")
        .bind(&signature)
        .bind(&prev_hash)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;

        // Ensure wallet cache exists and update balance.
        sqlx::query(
            "INSERT INTO credit.wallets (account_id, balance, last_seq) \
             VALUES ($1, $2, 0) ON CONFLICT (account_id) DO UPDATE \
             SET balance = credit.wallets.balance + $2",
        )
        .bind(auth.id)
        .bind(link.legacy_balance)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Read the final wallet balance.
    let balance: i64 =
        sqlx::query_scalar("SELECT COALESCE(balance, 0) FROM credit.wallets WHERE account_id = $1")
            .bind(auth.id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(WalletDto { account_id: auth.id.to_string(), balance }))
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
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
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

// ---------------------------------------------------------------------------
// Password auth handlers
// ---------------------------------------------------------------------------

/// POST /auth/password/login
///
/// Logs in with email + password. Returns a uniform "invalid_credentials"
/// error for all failure modes (account not found, no password set, wrong
/// password) to prevent email enumeration. Rate-limited: 5 attempts per
/// email per 5 minutes.
#[tracing::instrument(skip(state))]
pub async fn password_login(
    State(state): State<AppState>,
    Json(body): Json<PasswordLoginInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    // Rate-limit: 5 login attempts per email per 5 minutes.
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "password_login", &email, 5, 300)
        .await?;

    // Look up the account and password hash.
    let (account, phc) = match (
        repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email).await?,
        repo::find_password_hash(&state.db, state.email_encryption.as_ref(), &email).await?,
    ) {
        (Some(acct), Some(hash)) => (acct, hash),
        _ => {
            // Dummy Argon2 verification — constant response and timing.
            let dummy =
                "$argon2id$v=19$m=19456,t=2,p=1$dummy$j6PJgxYhKuWrkOV+72FYV7k+GMc4PszN7YlPxJ/8rTk";
            password::verify("dummy-invalid", dummy);
            return Err(shared::AppError::Unauthorized);
        }
    };

    if !password::verify(&body.password, &phc) {
        return Err(shared::AppError::Unauthorized);
    }
    ensure_login_allowed(&state, &account).await?;

    // Issue tokens.
    let access_token = create_access_token(account.id, &state.jwt_secret, state.jwt_ttl)
        .map_err(|e| shared::AppError::Internal(anyhow::anyhow!(e)))?;
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);

    let session_id =
        repo::insert_session(&state.db, account.id, &refresh_hash, refresh_expires).await?;

    let combined_refresh = format!("{session_id:x}:{refresh_plain}");

    Ok(Json(AuthTokensOutput {
        access_token,
        refresh_token: combined_refresh,
        account: row_to_dto(&account),
    }))
}

/// POST /auth/password/forgot
///
/// Always returns 204 regardless of whether the account exists or has a
/// password set, to prevent email enumeration. Only sends a reset code
/// to accounts that actually have a password. Rate-limited: 1 request
/// per email per 60 seconds.
#[tracing::instrument(skip(state))]
pub async fn password_forgot(
    State(state): State<AppState>,
    Json(body): Json<PasswordForgotInput>,
) -> AppResult<StatusCode> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "password_forgot",
        &body.captcha_token,
    )
    .await?;

    // Rate-limit: 1 per 60 seconds per email (same bucket as request_code).
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "email_code", &email, 1, 60)
        .await?;

    // Only send code to accounts that exist and have a password set.
    let hash = repo::find_password_hash(&state.db, state.email_encryption.as_ref(), &email).await?;
    if hash.is_some() {
        let code = generate_code();
        let code_hash = hash_code(&code);
        let expires_at = Utc::now() + chrono::Duration::minutes(10);
        let request_id = uuid::Uuid::new_v4();

        repo::insert_email_code(
            &state.db,
            state.email_encryption.as_ref(),
            &email,
            &code_hash,
            expires_at,
            "password_reset",
            request_id,
        )
        .await?;

        let email_content = crate::email_templates::password_reset_code(&code);
        deliver_email_code(&state, &email, &code_hash, &email_content).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/password/reset
///
/// Verifies the code, updates the password hash, and revokes all sessions.
/// Does NOT automatically log the user in — they must use
/// /auth/password/login afterwards.
#[tracing::instrument(skip(state))]
pub async fn password_reset(
    State(state): State<AppState>,
    Json(body): Json<PasswordResetInput>,
) -> AppResult<StatusCode> {
    let email = body.email.trim().to_lowercase();
    if !email.ends_with("@tongji.edu.cn") {
        return Err(IdentityError::InvalidEmailDomain.into());
    }

    // Must have a password set to reset it (can't reset a non-existent password).
    let current_hash =
        repo::find_password_hash(&state.db, state.email_encryption.as_ref(), &email).await?;
    if current_hash.is_none() {
        return Err(shared::AppError::Unauthorized);
    }

    // Look up the live code row for password_reset purpose.
    let code_row =
        repo::find_email_code(&state.db, state.email_encryption.as_ref(), &email, "password_reset")
            .await?
            .ok_or(IdentityError::CodeExpired)?;

    if code_row.attempts >= 5 {
        return Err(IdentityError::CodeExhausted.into());
    }

    let is_valid = verify_code(&body.code, &code_row.code_hash);
    repo::increment_code_attempts(&state.db, state.email_encryption.as_ref(), &email).await?;

    if !is_valid {
        return Err(IdentityError::InvalidCode.into());
    }

    // Validate and hash the new password.
    password::validate(&body.new_password, &email)?;
    let new_hash = password::hash(&body.new_password)?;

    // Find account to get its id.
    let account = repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email)
        .await?
        .ok_or(shared::AppError::NotFound)?;

    repo::update_password_hash(&state.db, account.id, &new_hash).await?;

    // Revoke all sessions — password changed, old tokens invalid.
    repo::revoke_all_sessions(&state.db, account.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/password/change
///
/// Changes the password for the authenticated account. Requires the current
/// password and a valid Bearer token.
/// Rate-limited: 3 attempts per account per minute.
#[tracing::instrument(skip(state, headers))]
pub async fn password_change(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordChangeInput>,
) -> AppResult<StatusCode> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| shared::AppError::Unauthorized)?;

    // Rate-limit: 3 changes per account per minute.
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "password_change",
        &auth.id.to_string(),
        3,
        60,
    )
    .await?;

    // Look up account to get the email for password validation.
    let account_row = repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.id)
        .await?
        .ok_or(shared::AppError::NotFound)?;

    let phc = repo::find_password_hash_by_account_id(&state.db, auth.id)
        .await?
        .ok_or(IdentityError::NoPasswordSet)?;

    // Verify current password.
    if !password::verify(&body.current_password, &phc) {
        return Err(IdentityError::WrongPassword.into());
    }

    // Validate and set new password.
    password::validate(&body.new_password, &account_row.email)?;
    let new_hash = password::hash(&body.new_password)?;

    repo::update_password_hash(&state.db, auth.id, &new_hash).await?;

    // Revoke all sessions — password changed, old tokens invalid.
    repo::revoke_all_sessions(&state.db, auth.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

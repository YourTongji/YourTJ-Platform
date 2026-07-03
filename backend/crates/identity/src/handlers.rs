//! Axum request handlers for the identity domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

use sha2::Digest as _;

use axum::extract::Path;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use shared::{AppResult, AppState};

use crate::auth::{create_access_token, generate_refresh_token};
use crate::dto::{
    AccountDto, AuthTokensOutput, BindKeyInput, ClaimChallengeOutput, ClaimInput, RefreshInput,
    RequestCodeInput, UpdateMeInput, UserBadgeDto, UserProfileDto, VerifyEmailInput, WalletDto,
};
use crate::email_code::{generate_code, hash_code, verify_code};
use crate::error::IdentityError;
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

    repo::insert_email_code(&state.db, &email, &code_hash, expires_at).await?;

    shared::email::send_email(
        &state.config,
        &email,
        "YourTJ 验证码",
        &format!("您的验证码是：{code}，5 分钟内有效。如非本人操作，请忽略此邮件。"),
    )
    .await;

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
    let account =
        repo::find_account_by_id(&state.db, auth.id).await?.ok_or(shared::AppError::NotFound)?;
    Ok(Json(row_to_dto(&account)))
}

/// GET /api/v2/users/{handle} — public user profile (no auth required).
#[tracing::instrument(skip(state))]
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> AppResult<Json<UserProfileDto>> {
    // Look up account by handle (CITEXT, case-insensitive).
    let account = sqlx::query_as::<_, crate::models::AccountRow>(
        "SELECT id, email, handle, avatar_url, role, status, trust_level, created_at \
         FROM identity.accounts WHERE handle = $1",
    )
    .bind(&handle)
    .fetch_optional(&state.db)
    .await?
    .ok_or(shared::AppError::NotFound)?;

    // Aggregate stats from forum.user_stats (LEFT JOIN behaviour via COALESCE).
    let (thread_count, comment_count, votes_received) = sqlx::query_as::<_, (i32, i32, i32)>(
        "SELECT COALESCE(threads_created, 0), COALESCE(comments_created, 0), \
                    COALESCE(votes_received, 0) \
             FROM forum.user_stats WHERE account_id = $1",
    )
    .bind(account.id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or((0, 0, 0));

    // Badges from platform.account_badges → platform.badges.
    let badges: Vec<UserBadgeDto> = sqlx::query_as::<_, (String, String)>(
        "SELECT b.slug, b.name \
         FROM platform.account_badges ab \
         JOIN platform.badges b ON b.id = ab.badge_id \
         WHERE ab.account_id = $1",
    )
    .bind(account.id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(slug, name)| UserBadgeDto { slug, name })
    .collect();

    Ok(Json(UserProfileDto {
        handle: account.handle,
        avatar_url: account.avatar_url,
        trust_level: account.trust_level,
        badges,
        thread_count,
        comment_count,
        votes_received,
        created_at: account.created_at.timestamp(),
    }))
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
/// link rows, validates all conditions, then mints any legacy balance into
/// the credit ledger.
///
/// NOTE: This handler directly queries/inserts into `credit.ledger` and
/// `credit.wallets` — an intentional exception to the domain-boundary rule
/// that "identity must not query credit tables." The cross-domain access is
/// architecturally necessary because this is a one-time legacy-claim flow
/// that must atomically transfer balance from the old system into the new
/// ledger. Moving it to the credit crate would create circular dependencies
/// (credit needs identity data for the claim). The tight coupling here is
/// confined to this single handler and is not used as a precedent for other
/// identity → credit queries.
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

/// POST /api/v2/admin/users/{id}/silence — silence a user (cannot write)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SanctionInput {
    pub reason: String,
    pub ends_at: Option<i64>, // unix seconds, None = indefinite
}

/// POST /api/v2/admin/users/{id}/unsanction
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsanctionInput {
    pub sanction_id: String,
}

/// POST /api/v2/admin/users/{id}/silence — silence a user (cannot write)
#[tracing::instrument(skip(state, headers))]
pub async fn silence_user(
    State(state): State<AppState>,
    Path(account_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<SanctionInput>,
) -> AppResult<StatusCode> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let account_id: i64 = account_id_str.parse().map_err(|_| shared::AppError::NotFound)?;
    let ends_at = body
        .ends_at
        .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or(chrono::Utc::now()));

    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by, ends_at) \
         VALUES ($1, 'silence', $2, $3, $4)",
    )
    .bind(account_id)
    .bind(&body.reason)
    .bind(auth.id)
    .bind(ends_at)
    .execute(&state.db)
    .await?;

    // Invalidate Redis sanction cache
    if let Some(ref r) = state.redis {
        if let Ok(mut conn) = r.get().await {
            let key = format!("identity:sanction:{account_id}");
            let _: () = redis::cmd("DEL").arg(&key).query_async(&mut conn).await.unwrap_or(());
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v2/admin/users/{id}/suspend — suspend a user (cannot login)
#[tracing::instrument(skip(state, headers))]
pub async fn suspend_user(
    State(state): State<AppState>,
    Path(account_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<SanctionInput>,
) -> AppResult<StatusCode> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let account_id: i64 = account_id_str.parse().map_err(|_| shared::AppError::NotFound)?;
    let ends_at = body
        .ends_at
        .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or(chrono::Utc::now()));

    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by, ends_at) \
         VALUES ($1, 'suspend', $2, $3, $4)",
    )
    .bind(account_id)
    .bind(&body.reason)
    .bind(auth.id)
    .bind(ends_at)
    .execute(&state.db)
    .await?;

    if let Some(ref r) = state.redis {
        if let Ok(mut conn) = r.get().await {
            let key = format!("identity:sanction:{account_id}");
            let _: () = redis::cmd("DEL").arg(&key).query_async(&mut conn).await.unwrap_or(());
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v2/admin/users/{id}/unsanction — revoke a sanction
#[tracing::instrument(skip(state, headers))]
pub async fn unsanction_user(
    State(state): State<AppState>,
    Path(account_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UnsanctionInput>,
) -> AppResult<StatusCode> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let account_id: i64 = account_id_str.parse().map_err(|_| shared::AppError::NotFound)?;
    let sanction_id: i64 = body
        .sanction_id
        .parse()
        .map_err(|_| shared::AppError::BadRequest("invalid sanctionId".into()))?;

    sqlx::query(
        "UPDATE identity.sanctions SET revoked_at = now(), revoked_by = $1 \
         WHERE id = $2 AND account_id = $3 AND revoked_at IS NULL",
    )
    .bind(auth.id)
    .bind(sanction_id)
    .bind(account_id)
    .execute(&state.db)
    .await?;

    if let Some(ref r) = state.redis {
        if let Ok(mut conn) = r.get().await {
            let key = format!("identity:sanction:{account_id}");
            let _: () = redis::cmd("DEL").arg(&key).query_async(&mut conn).await.unwrap_or(());
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v2/admin/users/{id}/sanctions — list sanctions for a user
#[tracing::instrument(skip(state, headers))]
pub async fn list_user_sanctions(
    State(state): State<AppState>,
    Path(account_id_str): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| shared::AppError::Forbidden)?;

    let account_id: i64 = account_id_str.parse().map_err(|_| shared::AppError::NotFound)?;

    #[derive(Debug, sqlx::FromRow)]
    struct SanctionRow {
        id: i64,
        account_id: i64,
        kind: String,
        reason: String,
        issued_by: i64,
        starts_at: chrono::DateTime<chrono::Utc>,
        ends_at: Option<chrono::DateTime<chrono::Utc>>,
        revoked_at: Option<chrono::DateTime<chrono::Utc>>,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<SanctionRow> = sqlx::query_as(
        "SELECT id, account_id, kind, reason, issued_by, starts_at, ends_at, revoked_at, created_at \
         FROM identity.sanctions WHERE account_id = $1 ORDER BY created_at DESC",
    )
    .bind(account_id)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "accountId": r.account_id.to_string(),
                "kind": r.kind,
                "reason": r.reason,
                "issuedBy": r.issued_by.to_string(),
                "startsAt": r.starts_at.timestamp(),
                "endsAt": r.ends_at.map(|t| t.timestamp()),
                "revokedAt": r.revoked_at.map(|t| t.timestamp()),
                "createdAt": r.created_at.timestamp(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!(items)))
}

/// GET /api/v2/users/{handle}/threads — public user thread list.
pub async fn list_user_threads(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> AppResult<Json<Vec<crate::dto::UserThreadDto>>> {
    use crate::dto::UserThreadDto;

    // Find account by handle
    let account_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM identity.accounts WHERE handle = $1")
            .bind(&handle)
            .fetch_optional(&state.db)
            .await?
            .map(|id: i64| id);

    let account_id = account_id.ok_or(shared::AppError::NotFound)?;

    let rows: Vec<(i64, String, String, i32, i32, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT t.id, t.title, COALESCE(b.slug, ''), \
                t.reply_count, t.vote_count, t.created_at \
         FROM forum.threads t \
         JOIN forum.boards b ON b.id = t.board_id \
         WHERE t.author_id = $1 AND t.deleted_at IS NULL AND t.hidden_at IS NULL \
         ORDER BY t.created_at DESC LIMIT 50",
    )
    .bind(account_id)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<UserThreadDto> = rows
        .into_iter()
        .map(|(id, title, board_slug, reply_count, vote_count, created_at)| UserThreadDto {
            id: id.to_string(),
            title,
            board_slug,
            reply_count,
            vote_count,
            created_at: created_at.timestamp(),
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/v2/users/{handle}/comments — public user comment list.
pub async fn list_user_comments(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> AppResult<Json<Vec<crate::dto::UserCommentDto>>> {
    use crate::dto::UserCommentDto;

    let account_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM identity.accounts WHERE handle = $1")
            .bind(&handle)
            .fetch_optional(&state.db)
            .await?
            .map(|id: i64| id);

    let account_id = account_id.ok_or(shared::AppError::NotFound)?;

    let rows: Vec<(i64, i64, String, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT c.id, c.thread_id, COALESCE(t.title, ''), \
                LEFT(c.body, 200), c.created_at \
         FROM forum.comments c \
         JOIN forum.threads t ON t.id = c.thread_id \
         WHERE c.author_id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL \
         ORDER BY c.created_at DESC LIMIT 50",
    )
    .bind(account_id)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<UserCommentDto> = rows
        .into_iter()
        .map(|(id, thread_id, thread_title, body_excerpt, created_at)| UserCommentDto {
            id: id.to_string(),
            thread_id: thread_id.to_string(),
            thread_title,
            body_excerpt,
            created_at: created_at.timestamp(),
        })
        .collect();

    Ok(Json(items))
}

//! Axum request handlers for the identity domain.
//!
//! Every handler returns `AppResult<impl IntoResponse>` so `?` on a DB or
//! domain error automatically renders the correct error envelope.

pub(crate) mod admin;

use sha2::Digest as _;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use shared::{AppResult, AppState, Page};

use crate::auth::{
    create_appeal_access_token, create_session_access_token, generate_refresh_token,
};
use crate::dto::{
    AccountDto, AccountLifecycleDto, AccountLifecycleMutationDto, AccountLifecycleMutationInput,
    AppealAccessTokenOutput, AppealEmailVerificationInput, AuthTokensOutput, BindKeyInput,
    ClaimChallengeOutput, ClaimInput, EmailCodePurpose, MyProfileDto, OnboardingCompleteInput,
    OnboardingStateDto, PasswordChangeInput, PasswordForgotInput, PasswordLoginInput,
    PasswordResetInput, PasswordSetInput, ProfilePrivacyDto, ProfilePrivacyUpdateInput,
    ProfileUpdateInput, RecentAuthMethod, RecentAuthStatusDto, RecentAuthVerifyInput,
    RecoveryCredentialDto, RecoveryEmailVerificationInput, RefreshInput, RequestCodeInput,
    SessionDto, UpdateMeInput, VerifyEmailInput, WalletDto,
};
use crate::email_code::{generate_code, hash_code, CodePurpose};
use crate::error::IdentityError;
use crate::password;
use crate::repo;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

// Rate limiting is handled by shared::ratelimit::check_token_bucket (Redis-backed).
// A configured Redis outage returns a stable 503 so abuse controls cannot be bypassed.

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn row_to_dto(state: &AppState, row: &crate::models::AccountRow) -> AppResult<AccountDto> {
    Ok(AccountDto {
        id: row.id.to_string(),
        handle: row.handle.clone(),
        avatar_url: row.avatar_url.clone(),
        role: row.role.clone(),
        capabilities: shared::auth::capability_names_for_role(&row.role),
        trust_level: row.trust_level,
        has_password: row.password_hash.is_some(),
        onboarding_required: crate::onboarding::is_required(&state.db, row.id).await?,
        created_at: row.created_at.timestamp(),
    })
}

fn lifecycle_to_dto(lifecycle: crate::lifecycle::LifecycleRecord) -> AccountLifecycleDto {
    AccountLifecycleDto {
        state: lifecycle.state,
        deactivated_at: lifecycle.deactivated_at.map(|value| value.timestamp()),
        deletion_requested_at: lifecycle.deletion_requested_at.map(|value| value.timestamp()),
        recover_until: lifecycle.recover_until.map(|value| value.timestamp()),
        deleted_at: lifecycle.deleted_at.map(|value| value.timestamp()),
        purged_at: lifecycle.purged_at.map(|value| value.timestamp()),
        lifecycle_version: lifecycle.lifecycle_version,
    }
}

fn recovery_to_dto(
    credential: crate::lifecycle::IssuedRecoveryCredential,
) -> RecoveryCredentialDto {
    RecoveryCredentialDto {
        recovery_token: credential.token,
        expires_at: credential.expires_at.timestamp(),
        lifecycle: lifecycle_to_dto(credential.lifecycle),
    }
}

fn onboarding_to_dto(state: crate::onboarding::OnboardingState) -> OnboardingStateDto {
    OnboardingStateDto {
        required: state.required,
        current_terms_version: state.current_terms_version,
        accepted_terms_version: state.accepted_terms_version,
        handle: state.handle,
        display_name: state.display_name,
        bio: state.bio,
        profile_visibility: state.profile_visibility,
        activity_visibility: state.activity_visibility,
        discoverable: state.discoverable,
        completed_at: state.completed_at.map(|value| value.timestamp()),
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

fn normalize_profile_text(
    value: Option<&str>,
    field_name: &str,
    max_chars: usize,
    allows_line_breaks: bool,
) -> AppResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_chars
        || value.chars().any(|character| {
            character.is_control()
                && !(allows_line_breaks && matches!(character, '\n' | '\t' | '\r'))
        })
    {
        return Err(shared::AppError::BadRequest(format!("invalid {field_name}")));
    }
    Ok(Some(value.to_string()))
}

fn normalize_website(value: Option<&str>) -> AppResult<Option<String>> {
    let website = normalize_profile_text(value, "website", 2048, false)?;
    let Some(website) = website else {
        return Ok(None);
    };
    let authority = website
        .strip_prefix("https://")
        .and_then(|remainder| remainder.split(['/', '?', '#']).next())
        .filter(|authority| !authority.is_empty() && !authority.contains('@'))
        .ok_or_else(|| shared::AppError::BadRequest("website must be an HTTPS URL".into()))?;
    if authority.starts_with('.') || authority.ends_with('.') {
        return Err(shared::AppError::BadRequest("website must be an HTTPS URL".into()));
    }
    Ok(Some(website))
}

fn profile_to_dto(profile: crate::profiles::ProfileRecord) -> MyProfileDto {
    MyProfileDto {
        account_id: profile.account_id.to_string(),
        display_name: profile.display_name,
        school: profile.school,
        bio: profile.bio,
        website: profile.website,
        avatar_asset_id: profile.avatar_asset_id.map(|id| id.to_string()),
        banner_asset_id: profile.banner_asset_id.map(|id| id.to_string()),
    }
}

fn privacy_to_dto(privacy: crate::profiles::ProfilePrivacyRecord) -> ProfilePrivacyDto {
    ProfilePrivacyDto {
        profile_visibility: privacy.profile_visibility,
        activity_visibility: privacy.activity_visibility,
        followers_visibility: privacy.followers_visibility,
        following_visibility: privacy.following_visibility,
        discoverable: privacy.discoverable,
        dm_policy: privacy.dm_policy,
        mention_policy: privacy.mention_policy,
    }
}

fn validate_privacy(input: &ProfilePrivacyUpdateInput) -> AppResult<()> {
    if !matches!(input.profile_visibility.as_str(), "public" | "campus" | "only_me")
        || input
            .activity_visibility
            .as_deref()
            .is_some_and(|value| !matches!(value, "public" | "campus" | "only_me"))
        || !matches!(
            input.followers_visibility.as_str(),
            "public" | "campus" | "followers" | "only_me"
        )
        || !matches!(
            input.following_visibility.as_str(),
            "public" | "campus" | "followers" | "only_me"
        )
        || !matches!(input.dm_policy.as_str(), "everyone" | "following" | "nobody")
        || input
            .mention_policy
            .as_deref()
            .is_some_and(|value| !matches!(value, "everyone" | "following" | "nobody"))
    {
        return Err(shared::AppError::BadRequest("invalid profile privacy policy".into()));
    }
    Ok(())
}

fn normalize_campus_email(value: &str) -> Result<String, IdentityError> {
    let email = value.trim().to_lowercase();
    let mut parts = email.split('@');
    let local_part = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if email.len() > 254
        || local_part.is_empty()
        || domain != "tongji.edu.cn"
        || parts.next().is_some()
    {
        return Err(IdentityError::InvalidEmailDomain);
    }
    Ok(email)
}

fn validate_email_code(code: &str) -> Result<(), IdentityError> {
    if code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(IdentityError::InvalidCode)
    }
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
    request_id: uuid::Uuid,
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
        if let Err(invalidation_error) = repo::invalidate_email_code(&state.db, request_id).await {
            tracing::warn!(
                ?invalidation_error,
                "email delivery failed and verification code invalidation also failed"
            );
            return Err(invalidation_error);
        }
        return Err(delivery_error);
    }
    repo::mark_email_code_delivered(&state.db, request_id).await?;
    Ok(())
}

fn device_label(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(200).collect())
}

fn parse_client_installation_id(value: Option<&str>) -> AppResult<Option<uuid::Uuid>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let installation_id = uuid::Uuid::parse_str(value).map_err(|_| {
        shared::AppError::BadRequest("clientInstallationId must be a UUID v4".into())
    })?;
    if installation_id.get_version() != Some(uuid::Version::Random) {
        return Err(shared::AppError::BadRequest("clientInstallationId must be a UUID v4".into()));
    }
    Ok(Some(installation_id))
}

fn forwarded_client_address(headers: &HeaderMap) -> &str {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
}

async fn check_unauthenticated_credential_limits(
    state: &AppState,
    headers: &HeaderMap,
    route: &str,
    per_network_limit: u64,
    global_limit: u64,
) -> AppResult<()> {
    let client_address = forwarded_client_address(headers);
    let network_subject = crate::rate_limit_subject::network_rate_limit_subject(
        &state.jwt_secret,
        route,
        client_address,
    )?;
    let global_subject =
        crate::rate_limit_subject::network_rate_limit_subject(&state.jwt_secret, route, "global")?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        &format!("{route}_network"),
        &network_subject,
        if client_address == "unknown" { global_limit } else { per_network_limit },
        300,
    )
    .await?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        &format!("{route}_global"),
        &global_subject,
        global_limit,
        60,
    )
    .await
}

async fn issue_tokens(
    state: &AppState,
    account: &crate::models::AccountRow,
    user_agent: Option<&str>,
    client_installation_id: Option<uuid::Uuid>,
) -> AppResult<AuthTokensOutput> {
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);
    let (session_id, auth_version) = repo::insert_session(
        &state.db,
        account.id,
        &refresh_hash,
        refresh_expires,
        user_agent,
        client_installation_id,
    )
    .await?;
    let access_token = create_session_access_token(
        account.id,
        session_id,
        auth_version,
        &state.jwt_secret,
        state.jwt_ttl,
    )
    .map_err(|error| shared::AppError::Internal(anyhow::anyhow!(error)))?;
    Ok(AuthTokensOutput {
        access_token,
        refresh_token: format!("{session_id:x}:{refresh_plain}"),
        account: row_to_dto(state, account).await?,
    })
}

async fn credential_mutation_tokens(
    state: &AppState,
    mutation: repo::CredentialMutation,
    refresh_plain: &str,
) -> AppResult<AuthTokensOutput> {
    let access_token = create_session_access_token(
        mutation.account.id,
        mutation.session_id,
        mutation.auth_version,
        &state.jwt_secret,
        state.jwt_ttl,
    )
    .map_err(|error| shared::AppError::Internal(anyhow::anyhow!(error)))?;
    Ok(AuthTokensOutput {
        access_token,
        refresh_token: format!("{:x}:{refresh_plain}", mutation.session_id),
        account: row_to_dto(state, &mutation.account).await?,
    })
}

fn issue_appeal_access(
    state: &AppState,
    account: &crate::models::AccountRow,
) -> AppResult<AppealAccessTokenOutput> {
    if !matches!(account.status.as_str(), "active" | "suspended") {
        return Err(shared::AppError::Forbidden);
    }
    let ttl = 60 * 60;
    let access_token = create_appeal_access_token(account.id, &state.jwt_secret, ttl)
        .map_err(|error| shared::AppError::Internal(anyhow::anyhow!(error)))?;
    Ok(AppealAccessTokenOutput { access_token, expires_at: Utc::now().timestamp() + ttl as i64 })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /auth/email/request-code
///
/// Rate-limited: one code per email per 60 seconds. Sends a 204 on success.
#[tracing::instrument(skip_all)]
pub async fn request_code(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RequestCodeInput>,
) -> AppResult<StatusCode> {
    let email = normalize_campus_email(&body.email)?;
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "email_code",
        &body.captcha_token,
    )
    .await?;

    // Rate-limit code requests: 1 per 60 seconds per email (Redis-backed).
    let email_subject = crate::rate_limit_subject::email_rate_limit_subject(
        state.email_encryption.as_ref(),
        &state.jwt_secret,
        &email,
    )?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "email_code",
        &email_subject,
        1,
        60,
    )
    .await?;
    // Rate-limit by IP as well: 5 requests per 10 minutes.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    shared::ratelimit::check_token_bucket(state.redis.as_ref(), "ip_code", &ip, 5, 600).await?;

    let account =
        repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email).await?;
    let purpose = body.purpose.map(Into::into).unwrap_or(if account.is_some() {
        CodePurpose::Login
    } else {
        CodePurpose::Registration
    });
    if purpose == CodePurpose::Recovery {
        let can_recover = match account.as_ref() {
            Some(account) => crate::lifecycle::can_recover(&state.db, account.id).await?,
            None => false,
        };
        if !can_recover {
            return Ok(StatusCode::NO_CONTENT);
        }
    }
    let code = generate_code();
    let code_hash = hash_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    let request_id = repo::insert_email_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        purpose,
        &code_hash,
        expires_at,
        None,
    )
    .await?;

    let email_content = match purpose {
        CodePurpose::Appeal => crate::email_templates::appeal_code(&code),
        CodePurpose::Recovery => crate::email_templates::account_recovery_code(&code),
        CodePurpose::Login | CodePurpose::Registration => crate::email_templates::login_code(&code),
        CodePurpose::PasswordReset | CodePurpose::RecentAuth => {
            return Err(shared::AppError::BadRequest("invalid code purpose".into()))
        }
    };
    deliver_email_code(&state, &email, request_id, &email_content).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/email/verify
///
/// Validates the code, creates or looks up the account, and returns JWT
/// access + refresh tokens.
#[tracing::instrument(skip_all)]
pub async fn verify_email(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<VerifyEmailInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = normalize_campus_email(&body.email)?;
    let client_installation_id =
        parse_client_installation_id(body.client_installation_id.as_deref())?;
    validate_email_code(&body.code)?;
    check_unauthenticated_credential_limits(&state, &headers, "email_verify", 30, 1_000).await?;

    let existing =
        repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email).await?;
    if existing.is_none() {
        let handle = body.handle.as_deref().ok_or_else(|| {
            shared::AppError::BadRequest("registration requires an explicit handle".into())
        })?;
        validate_handle(handle)?;
        let handle_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE handle = $1::citext)",
        )
        .bind(handle)
        .fetch_one(&state.db)
        .await?;
        if handle_exists {
            return Err(IdentityError::HandleTaken.into());
        }
    }

    let account = if let Some(acct) = existing {
        if let Some(candidate_password) = body.password.as_deref() {
            let is_login_proof = matches!(body.purpose, None | Some(EmailCodePurpose::Login));
            if acct.password_hash.is_none() && is_login_proof {
                repo::preflight_email_code(
                    &state.db,
                    state.email_encryption.as_ref(),
                    &email,
                    Some(CodePurpose::Login),
                    &body.code,
                )
                .await?;
                password::validate(candidate_password, &email)?;
                let password_hash = password::hash(candidate_password).await?;
                let (refresh_plain, refresh_hash) = generate_refresh_token();
                let refresh_expires =
                    Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);
                let mutation = repo::set_password_with_login_code(
                    &state.db,
                    state.email_encryption.as_ref(),
                    acct.id,
                    &email,
                    &body.code,
                    &password_hash,
                    &refresh_hash,
                    refresh_expires,
                    device_label(&headers).as_deref(),
                    client_installation_id,
                )
                .await?;
                ensure_login_allowed(&state, &mutation.account).await?;
                crate::public_search::reconcile_user_in_background(&state, mutation.account.id);
                return Ok(Json(
                    credential_mutation_tokens(&state, mutation, &refresh_plain).await?,
                ));
            }
        }
        let code_purpose = repo::consume_email_code(
            &state.db,
            state.email_encryption.as_ref(),
            &email,
            body.purpose.map(Into::into),
            &body.code,
        )
        .await?;
        match code_purpose {
            CodePurpose::Login => {
                repo::ensure_invitation_valid(&state.db, acct.id).await?;
                repo::mark_email_verified(&state.db, acct.id).await?;
                acct
            }
            CodePurpose::PasswordReset | CodePurpose::Appeal | CodePurpose::Recovery => {
                return Err(IdentityError::InvalidCode.into());
            }
            CodePurpose::Registration | CodePurpose::RecentAuth => {
                return Err(shared::AppError::Conflict(
                    "account state changed; request a new code".into(),
                ));
            }
        }
    } else {
        let handle = body.handle.as_deref().ok_or_else(|| {
            shared::AppError::Internal(anyhow::anyhow!("validated registration lost handle"))
        })?;
        let password_hash = if let Some(password) = body.password.as_deref() {
            let proof = repo::preflight_email_code(
                &state.db,
                state.email_encryption.as_ref(),
                &email,
                body.purpose.map(Into::into),
                &body.code,
            )
            .await?;
            if proof != CodePurpose::Registration {
                return Err(shared::AppError::Conflict(
                    "account state changed; request a new code".into(),
                ));
            }
            password::validate(password, &email)?;
            Some(password::hash(password).await?)
        } else {
            None
        };
        repo::register_account_with_code(
            &state.db,
            state.email_encryption.as_ref(),
            &email,
            body.purpose.map(Into::into),
            &body.code,
            handle,
            password_hash.as_deref(),
        )
        .await?
    };

    ensure_login_allowed(&state, &account).await?;
    crate::public_search::reconcile_user_in_background(&state, account.id);

    Ok(Json(
        issue_tokens(&state, &account, device_label(&headers).as_deref(), client_installation_id)
            .await?,
    ))
}

/// POST /auth/refresh
///
/// Accepts a refresh token, validates it, revokes the old session, and
/// returns a new token pair.
#[tracing::instrument(skip_all)]
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RefreshInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let refresh_plain = body.refresh_token;
    if refresh_plain.len() > 256 {
        return Err(shared::AppError::Unauthorized);
    }

    // Parse session_id:random_hex
    let (sid_hex, random_part) =
        refresh_plain.split_once(':').ok_or(shared::AppError::Unauthorized)?;
    if sid_hex.is_empty()
        || sid_hex.len() > 16
        || !sid_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        || random_part.len() != 64
        || !random_part.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(shared::AppError::Unauthorized);
    }

    let sid = i64::from_str_radix(sid_hex, 16).map_err(|_| shared::AppError::Unauthorized)?;

    let refresh_hash = hex::encode(sha2::Sha256::digest(random_part.as_bytes()));

    let (new_refresh_plain, new_refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);
    let rotation = repo::rotate_session(
        &state.db,
        sid,
        &refresh_hash,
        &new_refresh_hash,
        refresh_expires,
        device_label(&headers).as_deref(),
    )
    .await?;
    let account =
        repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), rotation.account_id)
            .await?
            .ok_or(shared::AppError::Unauthorized)?;
    if let Err(error) = ensure_login_allowed(&state, &account).await {
        repo::revoke_all_sessions(&state.db, account.id).await?;
        return Err(error);
    }
    let access_token = create_session_access_token(
        account.id,
        rotation.session_id,
        rotation.auth_version,
        &state.jwt_secret,
        state.jwt_ttl,
    )
    .map_err(|error| shared::AppError::Internal(anyhow::anyhow!(error)))?;

    Ok(Json(AuthTokensOutput {
        access_token,
        refresh_token: format!("{:x}:{new_refresh_plain}", rotation.session_id),
        account: row_to_dto(&state, &account).await?,
    }))
}

async fn authenticated_context(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<crate::auth_middleware::AuthenticatedContext> {
    crate::auth_middleware::authenticate_context_allow_incomplete_onboarding(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)
}

/// GET /auth/recent-auth — describe the current session's server-side freshness.
#[tracing::instrument(skip_all)]
pub async fn recent_auth_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<RecentAuthStatusDto>> {
    let auth = authenticated_context(&state, &headers).await?;
    let recent = crate::auth_middleware::recent_auth_state(&auth, &state.db).await?;
    let has_password =
        repo::find_password_hash_by_account_id(&state.db, auth.account.id).await?.is_some();
    let authenticated_at = recent.authenticated_at.map(|value| value.timestamp());
    let expires_at = recent
        .authenticated_at
        .map(|value| value.timestamp() + crate::auth_middleware::RECENT_AUTH_WINDOW_SECONDS);
    let method = match recent.method.as_deref() {
        Some("password") => Some(RecentAuthMethod::Password),
        Some("email_code") => Some(RecentAuthMethod::EmailCode),
        Some(_) => {
            return Err(shared::AppError::Internal(anyhow::anyhow!(
                "invalid persisted recent-auth method"
            )))
        }
        None => None,
    };
    let mut available_methods = Vec::with_capacity(2);
    if recent.session_bound && has_password {
        available_methods.push(RecentAuthMethod::Password);
    }
    if recent.session_bound {
        available_methods.push(RecentAuthMethod::EmailCode);
    }
    Ok(Json(RecentAuthStatusDto {
        session_bound: recent.session_bound,
        is_fresh: recent.is_fresh(),
        authenticated_at,
        expires_at,
        method,
        available_methods,
    }))
}

/// POST /auth/recent-auth/email/request-code — send to the authenticated account's email.
#[tracing::instrument(skip_all)]
pub async fn request_recent_auth_code(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = authenticated_context(&state, &headers).await?;
    let session_id = auth.session_id.ok_or(shared::AppError::RecentAuthRequired)?;
    let recent = crate::auth_middleware::recent_auth_state(&auth, &state.db).await?;
    if !recent.session_bound {
        return Err(shared::AppError::RecentAuthRequired);
    }
    let rate_limit_key = format!("{}:{session_id}", auth.account.id);
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "recent_auth_email",
        &rate_limit_key,
        1,
        60,
    )
    .await?;
    let account =
        repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.account.id)
            .await?
            .ok_or(shared::AppError::Unauthorized)?;
    let code = generate_code();
    let request_id = repo::insert_email_code(
        &state.db,
        state.email_encryption.as_ref(),
        &account.email,
        CodePurpose::RecentAuth,
        &hash_code(&code),
        Utc::now() + chrono::Duration::minutes(10),
        None,
    )
    .await?;
    let content = crate::email_templates::recent_auth_code(&code);
    deliver_email_code(&state, &account.email, request_id, &content).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/recent-auth/verify — refresh only the current active session.
#[tracing::instrument(skip_all)]
pub async fn verify_recent_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RecentAuthVerifyInput>,
) -> AppResult<Json<RecentAuthStatusDto>> {
    let auth = authenticated_context(&state, &headers).await?;
    let session_id = auth.session_id.ok_or(shared::AppError::RecentAuthRequired)?;
    let rate_limit_key = format!("{}:{session_id}", auth.account.id);
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "recent_auth_verify",
        &rate_limit_key,
        5,
        300,
    )
    .await?;
    let account =
        repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.account.id)
            .await?
            .ok_or(shared::AppError::Unauthorized)?;
    match body.method {
        RecentAuthMethod::Password => {
            let has_valid_password = matches!(body.password.as_deref(), Some(password) if (1..=128).contains(&password.len()));
            if body.code.is_some() || !has_valid_password {
                return Err(shared::AppError::BadRequest(
                    "password verification requires only password".into(),
                ));
            }
            let password_state =
                repo::find_password_state_by_account_id(&state.db, account.id).await?;
            let password_matches = password::verify_or_dummy(
                body.password.as_deref().unwrap_or_default(),
                password_state.as_ref().map(|(hash, _)| hash.as_str()),
            )
            .await?;
            if !password_matches {
                return Err(IdentityError::RecentAuthFailed.into());
            }
            let expected_credential_version = password_state
                .map(|(_, version)| version)
                .ok_or(IdentityError::RecentAuthFailed)?;
            repo::mark_recent_auth_password(
                &state.db,
                account.id,
                session_id,
                expected_credential_version,
            )
            .await?;
        }
        RecentAuthMethod::EmailCode => {
            let has_valid_code =
                matches!(body.code.as_deref(), Some(code) if validate_email_code(code).is_ok());
            if body.password.is_some() || !has_valid_code {
                return Err(shared::AppError::BadRequest(
                    "email-code verification requires only a six-digit code".into(),
                ));
            }
            repo::consume_recent_auth_code(
                &state.db,
                state.email_encryption.as_ref(),
                account.id,
                session_id,
                &account.email,
                body.code.as_deref().unwrap_or_default(),
            )
            .await?;
        }
    }
    recent_auth_status(State(state), headers).await
}

/// POST /auth/logout
///
/// Revokes the authenticated device session.
#[tracing::instrument(skip_all)]
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<StatusCode> {
    let auth = authenticated_context(&state, &headers).await?;
    if let Some(session_id) = auth.session_id {
        repo::revoke_account_session(&state.db, auth.account.id, session_id).await?;
    } else {
        repo::revoke_all_sessions(&state.db, auth.account.id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/logout-all
#[tracing::instrument(skip_all)]
pub async fn logout_all(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = authenticated_context(&state, &headers).await?;
    repo::revoke_all_sessions(&state.db, auth.account.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /me/sessions
pub async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SessionQuery>,
) -> AppResult<Json<Page<SessionDto>>> {
    let auth = authenticated_context(&state, &headers).await?;
    let cursor = query
        .cursor
        .as_deref()
        .map(str::parse::<i64>)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid session cursor".into()))?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let rows = repo::list_sessions(&state.db, auth.account.id, cursor, limit).await?;
    let has_more = rows.len() > limit as usize;
    let visible_rows = if has_more { &rows[..limit as usize] } else { &rows };
    let next_cursor = has_more.then(|| visible_rows.last().map(|row| row.id.to_string())).flatten();
    let items = visible_rows
        .iter()
        .map(|row| SessionDto {
            id: row.id.to_string(),
            is_current: auth.session_id == Some(row.id),
            device_label: row.user_agent.clone(),
            created_at: row.created_at.timestamp(),
            last_used_at: row.last_used_at.timestamp(),
            expires_at: row.expires_at.timestamp(),
        })
        .collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// DELETE /me/sessions/{id}
pub async fn revoke_named_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> AppResult<StatusCode> {
    let auth = authenticated_context(&state, &headers).await?.account;
    let session_id = session_id
        .parse::<i64>()
        .map_err(|_| shared::AppError::BadRequest("invalid session id".into()))?;
    if !repo::revoke_account_session(&state.db, auth.id, session_id).await? {
        return Err(shared::AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /me/sessions/revoke-others
pub async fn revoke_other_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let auth = authenticated_context(&state, &headers).await?;
    let session_id = auth.session_id.ok_or(shared::AppError::Unauthorized)?;
    repo::revoke_other_sessions(&state.db, auth.account.id, session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /me
pub async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountDto>> {
    let auth = authenticated_context(&state, &headers).await?.account;
    let account = repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.id)
        .await?
        .ok_or(shared::AppError::NotFound)?;
    Ok(Json(row_to_dto(&state, &account).await?))
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
    )
    .await?;
    crate::public_search::reconcile_user_in_background(&state, auth.id);

    Ok(Json(row_to_dto(&state, &row).await?))
}

/// GET /api/v2/me/onboarding
pub async fn get_onboarding(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<OnboardingStateDto>> {
    let auth = authenticated_context(&state, &headers).await?;
    let onboarding = crate::onboarding::get(&state.db, auth.account.id).await?;
    Ok(Json(onboarding_to_dto(onboarding)))
}

/// PUT /api/v2/me/onboarding
pub async fn complete_onboarding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<OnboardingCompleteInput>,
) -> AppResult<Json<OnboardingStateDto>> {
    let auth = authenticated_context(&state, &headers).await?;
    validate_handle(&body.handle)?;
    let display_name =
        normalize_profile_text(body.display_name.as_deref(), "displayName", 50, false)?;
    let bio = normalize_profile_text(body.bio.as_deref(), "bio", 500, true)?;
    if !matches!(body.profile_visibility.as_str(), "public" | "campus" | "only_me")
        || !matches!(body.activity_visibility.as_str(), "public" | "campus" | "only_me")
    {
        return Err(shared::AppError::BadRequest("invalid onboarding privacy policy".into()));
    }
    let onboarding = crate::onboarding::complete(
        &state.db,
        auth.account.id,
        crate::onboarding::OnboardingChoices {
            handle: &body.handle,
            display_name: display_name.as_deref(),
            bio: bio.as_deref(),
            profile_visibility: &body.profile_visibility,
            activity_visibility: &body.activity_visibility,
            discoverable: body.discoverable,
            accepted_terms_version: &body.accepted_terms_version,
        },
    )
    .await?;
    crate::public_search::reconcile_user_in_background(&state, auth.account.id);
    Ok(Json(onboarding_to_dto(onboarding)))
}

/// GET /api/v2/me/lifecycle
pub async fn get_lifecycle(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountLifecycleDto>> {
    let auth = authenticated_context(&state, &headers).await?;
    Ok(Json(lifecycle_to_dto(crate::lifecycle::get(&state.db, auth.account.id).await?)))
}

/// POST /api/v2/me/lifecycle/deactivate
pub async fn deactivate_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AccountLifecycleMutationInput>,
) -> AppResult<Json<AccountLifecycleMutationDto>> {
    if body.confirmation != "DEACTIVATE" {
        return Err(shared::AppError::BadRequest("confirmation must be DEACTIVATE".into()));
    }
    let auth = authenticated_context(&state, &headers).await?;
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| shared::AppError::BadRequest("Idempotency-Key is required".into()))?;
    let credential = crate::lifecycle::deactivate(&state.db, &auth, idempotency_key).await?;
    crate::public_search::reconcile_user_in_background(&state, auth.account.id);
    let lifecycle = lifecycle_to_dto(credential.lifecycle.clone());
    Ok(Json(AccountLifecycleMutationDto { lifecycle, recovery: recovery_to_dto(credential) }))
}

/// POST /api/v2/me/lifecycle/delete
pub async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AccountLifecycleMutationInput>,
) -> AppResult<(StatusCode, Json<AccountLifecycleMutationDto>)> {
    if body.confirmation != "DELETE" {
        return Err(shared::AppError::BadRequest("confirmation must be DELETE".into()));
    }
    let auth = authenticated_context(&state, &headers).await?;
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| shared::AppError::BadRequest("Idempotency-Key is required".into()))?;
    let credential = crate::lifecycle::request_deletion(&state.db, &auth, idempotency_key).await?;
    crate::public_search::reconcile_user_in_background(&state, auth.account.id);
    let lifecycle = lifecycle_to_dto(credential.lifecycle.clone());
    Ok((
        StatusCode::ACCEPTED,
        Json(AccountLifecycleMutationDto { lifecycle, recovery: recovery_to_dto(credential) }),
    ))
}

/// GET /api/v2/me/profile
pub async fn get_my_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<MyProfileDto>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    let profile = crate::profiles::get_or_create_profile(&state.db, auth.id).await?;
    Ok(Json(profile_to_dto(profile)))
}

/// PUT /api/v2/me/profile
pub async fn replace_my_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ProfileUpdateInput>,
) -> AppResult<Json<MyProfileDto>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    let display_name =
        normalize_profile_text(body.display_name.as_deref(), "displayName", 50, false)?;
    let school = match body.school.as_deref() {
        Some(value) => Some(
            normalize_profile_text(Some(value), "school", 100, false)?
                .ok_or_else(|| shared::AppError::BadRequest("school cannot be empty".into()))?,
        ),
        None => None,
    };
    let bio = normalize_profile_text(body.bio.as_deref(), "bio", 500, true)?;
    let website = normalize_website(body.website.as_deref())?;
    let profile = crate::profiles::replace_profile_text(
        &state.db,
        auth.id,
        display_name.as_deref(),
        school.as_deref(),
        bio.as_deref(),
        website.as_deref(),
    )
    .await?;
    crate::public_search::reconcile_user_in_background(&state, auth.id);
    Ok(Json(profile_to_dto(profile)))
}

/// GET /api/v2/me/privacy
pub async fn get_my_privacy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ProfilePrivacyDto>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    let privacy = crate::profiles::get_or_create_privacy(&state.db, auth.id).await?;
    Ok(Json(privacy_to_dto(privacy)))
}

/// PUT /api/v2/me/privacy
pub async fn replace_my_privacy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ProfilePrivacyUpdateInput>,
) -> AppResult<Json<ProfilePrivacyDto>> {
    let auth = crate::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| shared::AppError::Unauthorized)?;
    validate_privacy(&body)?;
    let privacy = crate::profiles::replace_privacy(
        &state.db,
        auth.id,
        &body.profile_visibility,
        body.activity_visibility.as_deref(),
        &body.followers_visibility,
        &body.following_visibility,
        body.discoverable,
        &body.dm_policy,
        body.mention_policy.as_deref(),
    )
    .await?;
    crate::public_search::reconcile_user_in_background(&state, auth.id);
    Ok(Json(privacy_to_dto(privacy)))
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
/// Logs in with email + password. Returns the same JWT token pair as
/// email-code login. Rate-limited: 5 attempts per email per 5 minutes.
#[tracing::instrument(skip_all)]
pub async fn password_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordLoginInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = normalize_campus_email(&body.email)?;
    let client_installation_id =
        parse_client_installation_id(body.client_installation_id.as_deref())?;
    check_unauthenticated_credential_limits(&state, &headers, "password_login", 30, 1_000).await?;

    // Rate-limit: 5 login attempts per email per 5 minutes.
    let email_subject = crate::rate_limit_subject::email_rate_limit_subject(
        state.email_encryption.as_ref(),
        &state.jwt_secret,
        &email,
    )?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "password_login",
        &email_subject,
        5,
        300,
    )
    .await?;

    let account =
        repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email).await?;
    let password_hash = account.as_ref().and_then(|account| account.password_hash.as_deref());
    let has_valid_password_length = (1..=128).contains(&body.password.len());
    let password_for_verification =
        if has_valid_password_length { body.password.as_str() } else { "invalid-password-input" };
    let password_matches =
        password::verify_or_dummy(password_for_verification, password_hash).await?;
    if !has_valid_password_length
        || !password_matches
        || password_hash.is_none()
        || account.is_none()
    {
        return Err(IdentityError::WrongPassword.into());
    }
    let account = account.ok_or(shared::AppError::Unauthorized)?;
    match ensure_login_allowed(&state, &account).await {
        Ok(()) => {}
        Err(shared::AppError::Forbidden) => {
            tracing::warn!(account_id = account.id, "password login rejected by account state");
            return Err(IdentityError::WrongPassword.into());
        }
        Err(error) => return Err(error),
    }

    Ok(Json(
        issue_tokens(&state, &account, device_label(&headers).as_deref(), client_installation_id)
            .await?,
    ))
}

/// POST /auth/appeal/password — issue a short-lived credential usable only for appeals.
#[tracing::instrument(skip_all)]
pub async fn appeal_password_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordLoginInput>,
) -> AppResult<Json<AppealAccessTokenOutput>> {
    let email = normalize_campus_email(&body.email)?;
    check_unauthenticated_credential_limits(&state, &headers, "appeal_password_login", 20, 500)
        .await?;
    let email_subject = crate::rate_limit_subject::email_rate_limit_subject(
        state.email_encryption.as_ref(),
        &state.jwt_secret,
        &email,
    )?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "appeal_password_login",
        &email_subject,
        5,
        300,
    )
    .await?;
    let account =
        repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email).await?;
    let password_hash = account.as_ref().and_then(|account| account.password_hash.as_deref());
    let valid_length = (1..=128).contains(&body.password.len());
    let candidate = if valid_length { body.password.as_str() } else { "invalid-password-input" };
    let matches = password::verify_or_dummy(candidate, password_hash).await?;
    if !valid_length || !matches || password_hash.is_none() || account.is_none() {
        return Err(IdentityError::WrongPassword.into());
    }
    let account = account.ok_or(shared::AppError::Unauthorized)?;
    Ok(Json(issue_appeal_access(&state, &account)?))
}

/// POST /auth/appeal/email/verify — consume an appeal-purpose code without opening a full session.
#[tracing::instrument(skip_all)]
pub async fn appeal_email_verify(
    State(state): State<AppState>,
    Json(body): Json<AppealEmailVerificationInput>,
) -> AppResult<Json<AppealAccessTokenOutput>> {
    let email = normalize_campus_email(&body.email)?;
    validate_email_code(&body.code)?;
    repo::consume_email_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        Some(CodePurpose::Appeal),
        &body.code,
    )
    .await?;
    let account = repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email)
        .await?
        .ok_or(IdentityError::InvalidCode)?;
    Ok(Json(issue_appeal_access(&state, &account)?))
}

/// POST /auth/recovery/password — prove account control without creating a session.
#[tracing::instrument(skip_all)]
pub async fn recovery_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordLoginInput>,
) -> AppResult<Json<RecoveryCredentialDto>> {
    let email = normalize_campus_email(&body.email)?;
    check_unauthenticated_credential_limits(&state, &headers, "account_recovery_password", 20, 500)
        .await?;
    let email_subject = crate::rate_limit_subject::email_rate_limit_subject(
        state.email_encryption.as_ref(),
        &state.jwt_secret,
        &email,
    )?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "account_recovery_password",
        &email_subject,
        5,
        300,
    )
    .await?;
    let account =
        repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email).await?;
    let password_hash = account.as_ref().and_then(|account| account.password_hash.as_deref());
    let valid_length = (1..=128).contains(&body.password.len());
    let candidate = if valid_length { body.password.as_str() } else { "invalid-password-input" };
    let matches = password::verify_or_dummy(candidate, password_hash).await?;
    if !valid_length || !matches || password_hash.is_none() || account.is_none() {
        return Err(IdentityError::WrongPassword.into());
    }
    let account = account.ok_or(shared::AppError::Unauthorized)?;
    if !crate::lifecycle::can_recover(&state.db, account.id).await? {
        return Err(shared::AppError::Unauthorized);
    }
    let credential =
        crate::lifecycle::issue_recovery_credential(&state.db, account.id, "password").await?;
    Ok(Json(recovery_to_dto(credential)))
}

/// POST /auth/recovery/email/verify — consume only a recovery-purpose code.
#[tracing::instrument(skip_all)]
pub async fn recovery_email_verify(
    State(state): State<AppState>,
    Json(body): Json<RecoveryEmailVerificationInput>,
) -> AppResult<Json<RecoveryCredentialDto>> {
    let email = normalize_campus_email(&body.email)?;
    validate_email_code(&body.code)?;
    repo::consume_email_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        Some(CodePurpose::Recovery),
        &body.code,
    )
    .await?;
    let account = repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email)
        .await?
        .ok_or(IdentityError::InvalidCode)?;
    let credential =
        crate::lifecycle::issue_recovery_credential(&state.db, account.id, "email_code").await?;
    Ok(Json(recovery_to_dto(credential)))
}

fn recovery_token(headers: &HeaderMap) -> AppResult<&str> {
    headers
        .get("X-Recovery-Token")
        .and_then(|value| value.to_str().ok())
        .ok_or(shared::AppError::Unauthorized)
}

/// GET /auth/recovery
pub async fn inspect_recovery(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountLifecycleDto>> {
    let lifecycle =
        crate::lifecycle::inspect_recovery(&state.db, recovery_token(&headers)?).await?;
    Ok(Json(lifecycle_to_dto(lifecycle)))
}

/// POST /auth/recovery — consume the recovery credential and require a normal login next.
pub async fn reactivate_account(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AccountLifecycleDto>> {
    let token = recovery_token(&headers)?;
    let lifecycle = crate::lifecycle::reactivate(&state.db, token).await?;
    let account_id: i64 = sqlx::query_scalar(
        "SELECT credential.account_id FROM identity.account_recovery_credentials credential \
         WHERE credential.token_hash = $1",
    )
    .bind(hex::encode(sha2::Sha256::digest(token.as_bytes())))
    .fetch_one(&state.db)
    .await?;
    crate::public_search::reconcile_user_in_background(&state, account_id);
    Ok(Json(lifecycle_to_dto(lifecycle)))
}

/// POST /auth/password/forgot
///
/// Sends a 6-digit verification code to the email for password reset.
/// Rate-limited: 1 request per email per 60 seconds (reuses email_code bucket).
#[tracing::instrument(skip_all)]
pub async fn password_forgot(
    State(state): State<AppState>,
    Json(body): Json<PasswordForgotInput>,
) -> AppResult<StatusCode> {
    let email = normalize_campus_email(&body.email)?;
    shared::captcha::require_captcha(
        state.captcha_verifier.as_deref(),
        state.redis.as_ref(),
        "password_forgot",
        &body.captcha_token,
    )
    .await?;

    // Rate-limit: 1 per 60 seconds per email (same bucket as request_code).
    let email_subject = crate::rate_limit_subject::email_rate_limit_subject(
        state.email_encryption.as_ref(),
        &state.jwt_secret,
        &email,
    )?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "email_code",
        &email_subject,
        1,
        60,
    )
    .await?;

    let credential_version =
        repo::find_password_reset_version(&state.db, state.email_encryption.as_ref(), &email)
            .await?;
    let Some(credential_version) = credential_version else {
        return Ok(StatusCode::NO_CONTENT);
    };

    let code = generate_code();
    let code_hash = hash_code(&code);
    let expires_at = Utc::now() + chrono::Duration::minutes(10);

    let request_id = repo::insert_email_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        CodePurpose::PasswordReset,
        &code_hash,
        expires_at,
        Some(credential_version),
    )
    .await?;

    let email_content = crate::email_templates::password_reset_code(&code);
    if let Err(error) = deliver_email_code(&state, &email, request_id, &email_content).await {
        tracing::warn!(?error, "password reset email delivery was not accepted");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /auth/password/reset
///
/// Verifies the code, replaces every prior session, and returns one new token pair.
#[tracing::instrument(skip_all)]
pub async fn password_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordResetInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let email = normalize_campus_email(&body.email)?;
    let client_installation_id =
        parse_client_installation_id(body.client_installation_id.as_deref())?;
    validate_email_code(&body.code)?;
    check_unauthenticated_credential_limits(&state, &headers, "password_reset", 10, 300).await?;

    repo::preflight_password_reset_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        &body.code,
    )
    .await?;
    password::validate(&body.new_password, &email)?;
    let new_hash = password::hash(&body.new_password).await?;
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);
    let mutation = repo::reset_password_with_code(
        &state.db,
        state.email_encryption.as_ref(),
        &email,
        &body.code,
        &new_hash,
        &refresh_hash,
        refresh_expires,
        device_label(&headers).as_deref(),
        client_installation_id,
    )
    .await?;
    if let Err(error) = ensure_login_allowed(&state, &mutation.account).await {
        repo::revoke_all_sessions(&state.db, mutation.account.id).await?;
        return Err(error);
    }
    Ok(Json(credential_mutation_tokens(&state, mutation, &refresh_plain).await?))
}

/// POST /auth/password/set — establish the first password after fresh mailbox proof.
#[tracing::instrument(skip_all)]
pub async fn password_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordSetInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let auth = authenticated_context(&state, &headers).await?;
    let client_installation_id =
        parse_client_installation_id(body.client_installation_id.as_deref())?;
    let account =
        repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.account.id)
            .await?
            .ok_or(shared::AppError::NotFound)?;
    password::validate(&body.new_password, &account.email)?;
    let new_hash = password::hash(&body.new_password).await?;
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);
    let mutation = repo::set_password_and_replace_sessions(
        &state.db,
        &auth,
        &new_hash,
        &refresh_hash,
        refresh_expires,
        device_label(&headers).as_deref(),
        client_installation_id,
        state.email_encryption.as_ref(),
    )
    .await?;
    Ok(Json(credential_mutation_tokens(&state, mutation, &refresh_plain).await?))
}

/// POST /auth/password/change
///
/// Changes the password and replaces every prior refresh family with one new session.
/// Rate-limited: 3 attempts per account per minute.
#[tracing::instrument(skip_all)]
pub async fn password_change(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PasswordChangeInput>,
) -> AppResult<Json<AuthTokensOutput>> {
    let auth = authenticated_context(&state, &headers).await?;
    let client_installation_id =
        parse_client_installation_id(body.client_installation_id.as_deref())?;

    // Rate-limit: 3 changes per account per minute.
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "password_change",
        &auth.account.id.to_string(),
        3,
        60,
    )
    .await?;

    let account_row =
        repo::find_account_by_id(&state.db, state.email_encryption.as_ref(), auth.account.id)
            .await?
            .ok_or(shared::AppError::NotFound)?;

    let (phc, expected_credential_version) =
        repo::find_password_state_by_account_id(&state.db, auth.account.id)
            .await?
            .ok_or(IdentityError::NoPasswordSet)?;

    if !password::verify(&body.current_password, &phc).await? {
        return Err(IdentityError::WrongPassword.into());
    }

    password::validate(&body.new_password, &account_row.email)?;
    let new_hash = password::hash(&body.new_password).await?;
    let current_session_id = auth.session_id.ok_or(shared::AppError::Unauthorized)?;
    let (refresh_plain, refresh_hash) = generate_refresh_token();
    let refresh_expires = Utc::now() + chrono::Duration::seconds(state.refresh_ttl as i64);
    let mutation = repo::change_password_and_replace_sessions(
        &state.db,
        auth.account.id,
        current_session_id,
        expected_credential_version,
        &new_hash,
        &refresh_hash,
        refresh_expires,
        device_label(&headers).as_deref(),
        client_installation_id,
        state.email_encryption.as_ref(),
    )
    .await?;
    Ok(Json(credential_mutation_tokens(&state, mutation, &refresh_plain).await?))
}

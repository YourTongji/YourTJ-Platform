//! Staff-issued identity and special verifications.
//!
//! Achievement badges remain owned by their contribution rules. This module owns only typed,
//! reasoned credentials that can expire or be revoked. Public projections intentionally omit
//! issuer, reason, and the private evidence reference.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::{AuthAccount, Capability};
use shared::{AppError, AppResult, AppState, Page};
use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};

use crate::auth::staff_account;
use crate::validation::{optional_text, parse_id, reason, required_text, timestamp};

#[derive(Debug, FromRow)]
struct VerificationTypeRecord {
    id: i64,
    slug: String,
    category: String,
    label: String,
    description: Option<String>,
    icon: String,
    badge_variant: String,
    allows_public_display: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct VerificationGrantRecord {
    id: i64,
    account_id: i64,
    verification_type_id: i64,
    slug: String,
    category: String,
    label: String,
    icon: String,
    badge_variant: String,
    display_on_profile: bool,
    evidence_reference: Option<String>,
    issue_reason: String,
    issued_by: Option<i64>,
    issued_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    revoked_by: Option<i64>,
    revoked_at: Option<DateTime<Utc>>,
    revoke_reason: Option<String>,
}

struct ValidatedVerificationType {
    slug: String,
    category: VerificationCategory,
    label: String,
    description: Option<String>,
    icon: VerificationIcon,
    badge_variant: VerificationBadgeVariant,
    allows_public_display: bool,
}

enum VerificationCategory {
    Identity,
    Special,
}

impl VerificationCategory {
    fn parse(value: &str) -> AppResult<Self> {
        match value {
            "identity" => Ok(Self::Identity),
            "special" => Ok(Self::Special),
            _ => Err(AppError::BadRequest("invalid verification category".into())),
        }
    }

    const fn as_str(&self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Special => "special",
        }
    }
}

enum VerificationIcon {
    BadgeCheck,
    Building,
    ShieldCheck,
    Sparkles,
}

impl VerificationIcon {
    fn parse(value: &str) -> AppResult<Self> {
        match value {
            "badge-check" => Ok(Self::BadgeCheck),
            "building-2" => Ok(Self::Building),
            "shield-check" => Ok(Self::ShieldCheck),
            "sparkles" => Ok(Self::Sparkles),
            _ => Err(AppError::BadRequest("invalid verification icon token".into())),
        }
    }

    const fn as_str(&self) -> &'static str {
        match self {
            Self::BadgeCheck => "badge-check",
            Self::Building => "building-2",
            Self::ShieldCheck => "shield-check",
            Self::Sparkles => "sparkles",
        }
    }
}

enum VerificationBadgeVariant {
    Default,
    Secondary,
    Outline,
}

impl VerificationBadgeVariant {
    fn parse(value: &str) -> AppResult<Self> {
        match value {
            "default" => Ok(Self::Default),
            "secondary" => Ok(Self::Secondary),
            "outline" => Ok(Self::Outline),
            _ => Err(AppError::BadRequest("invalid verification badge variant".into())),
        }
    }

    const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Secondary => "secondary",
            Self::Outline => "outline",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum VerificationGrantStatus {
    Active,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicVerificationDto {
    pub slug: String,
    pub category: String,
    pub label: String,
    pub description: Option<String>,
    pub icon: String,
    pub badge_variant: String,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificationTypeDto {
    id: String,
    slug: String,
    category: String,
    label: String,
    description: Option<String>,
    icon: String,
    badge_variant: String,
    allows_public_display: bool,
    created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerificationGrantDto {
    id: String,
    account_id: String,
    verification_type_id: String,
    slug: String,
    category: String,
    label: String,
    icon: String,
    badge_variant: String,
    display_on_profile: bool,
    status: VerificationGrantStatus,
    issued_by: Option<String>,
    issued_at: i64,
    expires_at: Option<i64>,
    issue_reason: String,
    has_evidence: bool,
    revoked_by: Option<String>,
    revoked_at: Option<i64>,
    revoke_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VerificationTypeInput {
    slug: String,
    category: String,
    label: String,
    description: Option<String>,
    icon: String,
    badge_variant: String,
    allows_public_display: bool,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VerificationGrantInput {
    verification_type_id: String,
    display_on_profile: bool,
    expires_at: Option<i64>,
    evidence_reference: Option<String>,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VerificationRevokeInput {
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

fn verification_type_dto(record: VerificationTypeRecord) -> VerificationTypeDto {
    VerificationTypeDto {
        id: record.id.to_string(),
        slug: record.slug,
        category: record.category,
        label: record.label,
        description: record.description,
        icon: record.icon,
        badge_variant: record.badge_variant,
        allows_public_display: record.allows_public_display,
        created_at: record.created_at.timestamp(),
    }
}

fn verification_grant_dto(record: VerificationGrantRecord) -> VerificationGrantDto {
    let now = Utc::now();
    let status = if record.revoked_at.is_some() {
        VerificationGrantStatus::Revoked
    } else if record.expires_at.is_some_and(|expires_at| expires_at <= now) {
        VerificationGrantStatus::Expired
    } else {
        VerificationGrantStatus::Active
    };
    VerificationGrantDto {
        id: record.id.to_string(),
        account_id: record.account_id.to_string(),
        verification_type_id: record.verification_type_id.to_string(),
        slug: record.slug,
        category: record.category,
        label: record.label,
        icon: record.icon,
        badge_variant: record.badge_variant,
        display_on_profile: record.display_on_profile,
        status,
        issued_by: record.issued_by.map(|value| value.to_string()),
        issued_at: record.issued_at.timestamp(),
        expires_at: record.expires_at.map(|value| value.timestamp()),
        issue_reason: record.issue_reason,
        has_evidence: record.evidence_reference.is_some(),
        revoked_by: record.revoked_by.map(|value| value.to_string()),
        revoked_at: record.revoked_at.map(|value| value.timestamp()),
        revoke_reason: record.revoke_reason,
    }
}

fn validate_slug(value: &str) -> AppResult<String> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.split('-').all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        });
    if !valid {
        return Err(AppError::BadRequest(
            "slug must contain lowercase ASCII letters, digits, and single hyphens".into(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_single_line(value: String, field: &str) -> AppResult<String> {
    if value.chars().any(|character| character.is_control() || matches!(character, '<' | '>')) {
        return Err(AppError::BadRequest(format!("{field} must be plain single-line text")));
    }
    Ok(value)
}

fn validate_description(value: String) -> AppResult<String> {
    if value.chars().any(|character| {
        matches!(character, '<' | '>')
            || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    }) {
        return Err(AppError::BadRequest("description must be plain text".into()));
    }
    Ok(value)
}

fn validate_definition(input: &VerificationTypeInput) -> AppResult<ValidatedVerificationType> {
    Ok(ValidatedVerificationType {
        slug: validate_slug(&input.slug)?,
        category: VerificationCategory::parse(&input.category)?,
        label: validate_single_line(required_text(&input.label, 80, "label")?, "label")?,
        description: optional_text(input.description.as_deref(), 240, "description")?
            .map(validate_description)
            .transpose()?,
        icon: VerificationIcon::parse(&input.icon)?,
        badge_variant: VerificationBadgeVariant::parse(&input.badge_variant)?,
        allows_public_display: input.allows_public_display,
    })
}

fn validate_evidence_reference(value: Option<&str>) -> AppResult<Option<String>> {
    let value = optional_text(value, 128, "evidenceReference")?;
    if value.as_deref().is_some_and(|value| {
        value.contains("://")
            || !value.chars().enumerate().all(|(index, character)| {
                character.is_ascii_alphanumeric()
                    || (index > 0 && matches!(character, '.' | '_' | ':' | '/' | '-'))
            })
    }) {
        return Err(AppError::BadRequest(
            "evidenceReference must be an opaque internal reference".into(),
        ));
    }
    Ok(value)
}

fn role_rank(role: &str) -> Option<u8> {
    match role {
        "user" => Some(0),
        "mod" => Some(1),
        "admin" => Some(2),
        _ => None,
    }
}

async fn require_lower_target(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthAccount,
    account_id: i64,
) -> AppResult<identity::public_accounts::AccountAuthorizationState> {
    let target = identity::public_accounts::find_account_authorization_state_by_id(tx, account_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let actor_rank = role_rank(&actor.role).ok_or(AppError::Forbidden)?;
    let target_rank = role_rank(&target.role).ok_or(AppError::Forbidden)?;
    if actor.id == account_id || actor_rank <= target_rank {
        return Err(AppError::Forbidden);
    }
    Ok(target)
}

async fn find_grant_for_update(
    tx: &mut Transaction<'_, Postgres>,
    grant_id: i64,
) -> AppResult<VerificationGrantRecord> {
    sqlx::query_as::<_, VerificationGrantRecord>(
        "SELECT credential.id, credential.account_id, credential.verification_type_id, definition.slug, \
                definition.category, definition.label, definition.icon, definition.badge_variant, \
                credential.display_on_profile, credential.evidence_reference, credential.issue_reason, \
                credential.issued_by, credential.issued_at, credential.expires_at, credential.revoked_by, \
                credential.revoked_at, credential.revoke_reason \
         FROM platform.verification_grants credential \
         JOIN platform.verification_types definition ON definition.id = credential.verification_type_id \
         WHERE credential.id = $1 FOR UPDATE OF credential",
    )
    .bind(grant_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

/// Lock an expiry notification's grant and confirm that expiry, rather than revocation, won.
pub async fn lock_current_expiry_notification(
    connection: &mut PgConnection,
    grant_id: i64,
) -> AppResult<bool> {
    let is_current = sqlx::query_scalar(
        "SELECT revoked_at IS NULL AND expires_at IS NOT NULL AND expires_at <= now() \
         FROM platform.verification_grants WHERE id = $1 FOR SHARE",
    )
    .bind(grant_id)
    .fetch_optional(connection)
    .await?
    .unwrap_or(false);
    Ok(is_current)
}

/// Return only active, explicitly public verification labels for a public account surface.
pub async fn list_public_account_verifications(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<Vec<PublicVerificationDto>> {
    #[derive(Debug, FromRow)]
    struct PublicRecord {
        slug: String,
        category: String,
        label: String,
        description: Option<String>,
        icon: String,
        badge_variant: String,
        issued_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
    }

    let records = sqlx::query_as::<_, PublicRecord>(
        "SELECT definition.slug, definition.category, definition.label, definition.description, \
                definition.icon, definition.badge_variant, credential.issued_at, credential.expires_at \
         FROM platform.verification_grants credential \
         JOIN platform.verification_types definition ON definition.id = credential.verification_type_id \
         WHERE credential.account_id = $1 AND definition.allows_public_display \
           AND credential.display_on_profile AND credential.revoked_at IS NULL \
           AND (credential.expires_at IS NULL OR credential.expires_at > now()) \
         ORDER BY CASE definition.category WHEN 'identity' THEN 0 ELSE 1 END, \
                  credential.issued_at DESC, definition.id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(records
        .into_iter()
        .map(|record| PublicVerificationDto {
            slug: record.slug,
            category: record.category,
            label: record.label,
            description: record.description,
            icon: record.icon,
            badge_variant: record.badge_variant,
            issued_at: record.issued_at.timestamp(),
            expires_at: record.expires_at.map(|value| value.timestamp()),
        })
        .collect())
}

async fn admin_list_types(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<VerificationTypeDto>>> {
    staff_account(&headers, &state, Capability::ManageVerifications).await?;
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let records = sqlx::query_as::<_, VerificationTypeRecord>(
        "SELECT id, slug, category, label, description, icon, badge_variant, \
                allows_public_display, created_at \
         FROM platform.verification_types WHERE ($1::bigint IS NULL OR id < $1) \
         ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = records.len() > limit as usize;
    let visible = records.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor =
        has_more.then(|| visible.last().map(|record| record.id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(verification_type_dto).collect(), next_cursor)))
}

async fn admin_create_type(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<VerificationTypeInput>,
) -> AppResult<(StatusCode, Json<VerificationTypeDto>)> {
    let actor = staff_account(&headers, &state, Capability::ManageVerifications).await?;
    let values = validate_definition(&input)?;
    let audit_reason = reason(&input.reason)?;
    let mut tx = state.db.begin().await?;
    let record = sqlx::query_as::<_, VerificationTypeRecord>(
        "INSERT INTO platform.verification_types \
         (slug, category, label, description, icon, badge_variant, allows_public_display, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, slug, category, label, description, icon, badge_variant, \
                   allows_public_display, created_at",
    )
    .bind(&values.slug)
    .bind(values.category.as_str())
    .bind(&values.label)
    .bind(&values.description)
    .bind(values.icon.as_str())
    .bind(values.badge_variant.as_str())
    .bind(values.allows_public_display)
    .bind(actor.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|database| database.constraint() == Some("verification_types_slug_key"))
        {
            AppError::Conflict("verification slug already exists".into())
        } else {
            AppError::from(error)
        }
    })?;
    let metadata = serde_json::json!({
        "slug": record.slug,
        "category": record.category,
        "allowsPublicDisplay": record.allows_public_display,
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.verification_type.created",
        "verification_type",
        &record.id.to_string(),
        audit_reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(verification_type_dto(record))))
}

async fn admin_list_account_grants(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<VerificationGrantDto>>> {
    let actor = staff_account(&headers, &state, Capability::ManageVerifications).await?;
    let account_id = parse_id(&account_id, "account id")?;
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let mut tx = state.db.begin().await?;
    require_lower_target(&mut tx, &actor, account_id).await?;
    let records = sqlx::query_as::<_, VerificationGrantRecord>(
        "SELECT credential.id, credential.account_id, credential.verification_type_id, definition.slug, \
                definition.category, definition.label, definition.icon, definition.badge_variant, \
                credential.display_on_profile, credential.evidence_reference, credential.issue_reason, \
                credential.issued_by, credential.issued_at, credential.expires_at, credential.revoked_by, \
                credential.revoked_at, credential.revoke_reason \
         FROM platform.verification_grants credential \
         JOIN platform.verification_types definition ON definition.id = credential.verification_type_id \
         WHERE credential.account_id = $1 AND ($2::bigint IS NULL OR credential.id < $2) \
         ORDER BY credential.id DESC LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let has_more = records.len() > limit as usize;
    let visible = records.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor =
        has_more.then(|| visible.last().map(|record| record.id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(verification_grant_dto).collect(), next_cursor)))
}

async fn admin_grant(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<VerificationGrantInput>,
) -> AppResult<(StatusCode, Json<VerificationGrantDto>)> {
    let actor = staff_account(&headers, &state, Capability::ManageVerifications).await?;
    let account_id = parse_id(&account_id, "account id")?;
    let verification_type_id = parse_id(&input.verification_type_id, "verification type id")?;
    let audit_reason = reason(&input.reason)?;
    let evidence_reference = validate_evidence_reference(input.evidence_reference.as_deref())?;
    let expires_at = timestamp(input.expires_at, "expiresAt")?;
    let now = Utc::now();
    if expires_at.is_some_and(|value| value <= now) {
        return Err(AppError::BadRequest("expiresAt must be in the future".into()));
    }

    let mut tx = state.db.begin().await?;
    let target = require_lower_target(&mut tx, &actor, account_id).await?;
    if target.status != "active" {
        return Err(AppError::Conflict("verification target must be active".into()));
    }
    let definition = sqlx::query_as::<_, VerificationTypeRecord>(
        "SELECT id, slug, category, label, description, icon, badge_variant, \
                allows_public_display, created_at \
         FROM platform.verification_types WHERE id = $1 FOR UPDATE",
    )
    .bind(verification_type_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if input.display_on_profile && !definition.allows_public_display {
        return Err(AppError::BadRequest(
            "this verification type does not allow public display".into(),
        ));
    }
    let already_active = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS( \
           SELECT 1 FROM platform.verification_grants \
           WHERE account_id = $1 AND verification_type_id = $2 AND revoked_at IS NULL \
             AND (expires_at IS NULL OR expires_at > $3) \
         )",
    )
    .bind(account_id)
    .bind(verification_type_id)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    if already_active {
        return Err(AppError::Conflict("account already has an active verification".into()));
    }
    let grant_id: i64 = sqlx::query_scalar(
        "INSERT INTO platform.verification_grants \
         (account_id, verification_type_id, display_on_profile, evidence_reference, issue_reason, \
          issued_by, issued_at, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(account_id)
    .bind(verification_type_id)
    .bind(input.display_on_profile)
    .bind(&evidence_reference)
    .bind(audit_reason)
    .bind(actor.id)
    .bind(now)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;
    let record = find_grant_for_update(&mut tx, grant_id).await?;
    let metadata = serde_json::json!({
        "accountId": account_id.to_string(),
        "typeSlug": definition.slug,
        "displayOnProfile": input.display_on_profile,
        "expiresAt": expires_at.map(|value| value.timestamp()),
        "hasEvidence": evidence_reference.is_some(),
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.verification.granted",
        "account_verification",
        &grant_id.to_string(),
        audit_reason,
        Some(&metadata),
    )
    .await?;
    crate::outbox::enqueue_notification_tx(
        &mut tx,
        &format!("verification-grant:{grant_id}:granted"),
        account_id,
        None,
        "verification_granted",
        &serde_json::json!({
            "verificationGrantId": grant_id.to_string(),
            "verificationTypeId": verification_type_id.to_string(),
            "verificationSlug": &definition.slug,
            "verificationLabel": &definition.label,
            "title": "你的账号获得了一项认证",
            "expiresAt": expires_at.map(|value| value.timestamp()),
        }),
        None,
        None,
    )
    .await?;
    if let Some(expires_at) = expires_at {
        crate::outbox::enqueue_notification_tx(
            &mut tx,
            &format!("verification-grant:{grant_id}:expired"),
            account_id,
            None,
            "verification_expired",
            &serde_json::json!({
                "verificationGrantId": grant_id.to_string(),
                "verificationTypeId": verification_type_id.to_string(),
                "verificationSlug": &definition.slug,
                "verificationLabel": &definition.label,
                "title": "你的一项账号认证已到期",
            }),
            None,
            Some(expires_at),
        )
        .await?;
    }
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(verification_grant_dto(record))))
}

async fn admin_revoke(
    State(state): State<AppState>,
    Path(grant_id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<VerificationRevokeInput>,
) -> AppResult<Json<VerificationGrantDto>> {
    let actor = staff_account(&headers, &state, Capability::ManageVerifications).await?;
    let grant_id = parse_id(&grant_id, "verification grant id")?;
    let audit_reason = reason(&input.reason)?;
    let mut tx = state.db.begin().await?;
    let current = find_grant_for_update(&mut tx, grant_id).await?;
    require_lower_target(&mut tx, &actor, current.account_id).await?;
    let now = Utc::now();
    if current.revoked_at.is_some()
        || current.expires_at.is_some_and(|expires_at| expires_at <= now)
    {
        return Err(AppError::Conflict("verification grant is not active".into()));
    }
    let affected = sqlx::query(
        "UPDATE platform.verification_grants \
         SET revoked_by = $1, revoked_at = $2, revoke_reason = $3 \
         WHERE id = $4 AND revoked_at IS NULL \
           AND (expires_at IS NULL OR expires_at > $2)",
    )
    .bind(actor.id)
    .bind(now)
    .bind(audit_reason)
    .bind(grant_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("verification grant is not active".into()));
    }
    let record = find_grant_for_update(&mut tx, grant_id).await?;
    let metadata = serde_json::json!({
        "accountId": record.account_id.to_string(),
        "typeSlug": record.slug,
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.verification.revoked",
        "account_verification",
        &grant_id.to_string(),
        audit_reason,
        Some(&metadata),
    )
    .await?;
    crate::outbox::cancel_queued_event_tx(
        &mut tx,
        &format!("verification-grant:{grant_id}:expired"),
    )
    .await?;
    crate::outbox::enqueue_notification_tx(
        &mut tx,
        &format!("verification-grant:{grant_id}:revoked"),
        record.account_id,
        None,
        "verification_revoked",
        &serde_json::json!({
            "verificationGrantId": grant_id.to_string(),
            "verificationTypeId": record.verification_type_id.to_string(),
            "verificationSlug": &record.slug,
            "verificationLabel": &record.label,
            "title": "你的一项账号认证已被撤销",
        }),
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(verification_grant_dto(record)))
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v2/admin/verifications/types", get(admin_list_types).post(admin_create_type))
        .route(
            "/api/v2/admin/users/{account_id}/verifications",
            get(admin_list_account_grants).post(admin_grant),
        )
        .route("/api/v2/admin/verifications/grants/{grant_id}/revoke", post(admin_revoke))
}

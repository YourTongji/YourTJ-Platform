//! Contribution achievements and their staff operations.
//!
//! Achievements describe contribution milestones. They are intentionally separate from staff
//! roles and identity verifications. Automatic awards may enqueue a contribution mint; manual
//! awards never mint points.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::{AuthAccount, Capability};
use shared::{AppError, AppResult, AppState, Page};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

use crate::auth::staff_account;
use crate::validation::{optional_text, parse_id, reason, required_text};

const STANDARD_ACHIEVEMENTS: &[(&str, &str, &str, &str, i64)] = &[
    ("first-thread", "首次发帖", "发表你的第一个主题", "award", 5),
    ("quality-author", "优质作者", "你的主题被标记为精选", "star", 10),
    ("first-comment", "首次评论", "发表你的第一条评论", "message-circle-heart", 2),
];

#[derive(Debug, FromRow)]
struct AchievementRecord {
    id: i64,
    slug: String,
    name: String,
    description: Option<String>,
    icon_token: String,
    status: String,
    mint_amount: i64,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct AchievementGrantRecord {
    account_id: i64,
    badge_id: i64,
    slug: String,
    name: String,
    icon_token: String,
    definition_status: String,
    award_reason: Option<String>,
    awarded_at: DateTime<Utc>,
    awarded_by: i64,
    revoked_at: Option<DateTime<Utc>>,
    revoked_by: Option<i64>,
    revoke_reason: Option<String>,
}

#[derive(Debug, FromRow)]
struct AchievementEventRecord {
    id: i64,
    badge_id: i64,
    slug: String,
    name: String,
    action: String,
    source: String,
    actor_id: Option<i64>,
    reason: String,
    created_at: DateTime<Utc>,
}

/// Minimal achievement label safe for public profiles.
#[derive(Debug, Clone)]
pub struct PublicAchievement {
    pub slug: String,
    pub name: String,
}

/// Result returned to a contribution rule after an automatic award attempt.
#[derive(Debug)]
pub struct AutomaticAwardResult {
    pub newly_awarded: bool,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AchievementDto {
    id: String,
    slug: String,
    name: String,
    description: Option<String>,
    icon: String,
    status: String,
    mint_amount: i64,
    version: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AchievementGrantDto {
    account_id: String,
    achievement_id: String,
    slug: String,
    name: String,
    icon: String,
    definition_status: String,
    status: String,
    award_reason: Option<String>,
    awarded_at: i64,
    awarded_by: String,
    revoked_at: Option<i64>,
    revoked_by: Option<String>,
    revoke_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AchievementEventDto {
    id: String,
    achievement_id: String,
    slug: String,
    name: String,
    action: String,
    source: String,
    actor_id: Option<String>,
    reason: String,
    created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AchievementCreateInput {
    slug: String,
    name: String,
    description: Option<String>,
    icon: String,
    mint_amount: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AchievementUpdateInput {
    expected_version: i64,
    name: String,
    description: Option<String>,
    icon: String,
    status: String,
    mint_amount: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AchievementGrantInput {
    achievement_id: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AchievementRevokeInput {
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

fn achievement_dto(record: AchievementRecord) -> AchievementDto {
    AchievementDto {
        id: record.id.to_string(),
        slug: record.slug,
        name: record.name,
        description: record.description,
        icon: record.icon_token,
        status: record.status,
        mint_amount: record.mint_amount,
        version: record.version,
        created_at: record.created_at.timestamp(),
        updated_at: record.updated_at.timestamp(),
    }
}

fn achievement_grant_dto(record: AchievementGrantRecord) -> AchievementGrantDto {
    AchievementGrantDto {
        account_id: record.account_id.to_string(),
        achievement_id: record.badge_id.to_string(),
        slug: record.slug,
        name: record.name,
        icon: record.icon_token,
        definition_status: record.definition_status,
        status: if record.revoked_at.is_some() { "revoked".into() } else { "active".into() },
        award_reason: record.award_reason,
        awarded_at: record.awarded_at.timestamp(),
        awarded_by: record.awarded_by.to_string(),
        revoked_at: record.revoked_at.map(|value| value.timestamp()),
        revoked_by: record.revoked_by.map(|value| value.to_string()),
        revoke_reason: record.revoke_reason,
    }
}

fn achievement_event_dto(record: AchievementEventRecord) -> AchievementEventDto {
    AchievementEventDto {
        id: record.id.to_string(),
        achievement_id: record.badge_id.to_string(),
        slug: record.slug,
        name: record.name,
        action: record.action,
        source: record.source,
        actor_id: record.actor_id.map(|value| value.to_string()),
        reason: record.reason,
        created_at: record.created_at.timestamp(),
    }
}

fn validate_slug(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value.split('-').all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
    {
        return Err(AppError::BadRequest(
            "slug must contain lowercase ASCII letters, digits, and single hyphens".into(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_icon(value: &str) -> AppResult<&str> {
    match value {
        "award" | "book-open-check" | "message-circle-heart" | "star" => Ok(value),
        _ => Err(AppError::BadRequest("invalid achievement icon token".into())),
    }
}

fn validate_status(value: &str) -> AppResult<&str> {
    match value {
        "active" | "retired" => Ok(value),
        _ => Err(AppError::BadRequest("invalid achievement status".into())),
    }
}

fn validate_mint_amount(value: i64) -> AppResult<i64> {
    if (0..=100_000).contains(&value) {
        Ok(value)
    } else {
        Err(AppError::BadRequest("mintAmount must be between 0 and 100000".into()))
    }
}

fn validate_name(value: String) -> AppResult<String> {
    if value.chars().any(|character| character.is_control() || matches!(character, '<' | '>')) {
        return Err(AppError::BadRequest("name must be plain single-line text".into()));
    }
    Ok(value)
}

fn validate_description(value: Option<String>) -> AppResult<Option<String>> {
    if value.as_deref().is_some_and(|value| {
        value.chars().any(|character| {
            matches!(character, '<' | '>')
                || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        })
    }) {
        return Err(AppError::BadRequest("description must be plain text".into()));
    }
    Ok(value)
}

fn list_limit(value: Option<i64>) -> AppResult<i64> {
    let value = value.unwrap_or(30);
    if (1..=100).contains(&value) {
        Ok(value)
    } else {
        Err(AppError::BadRequest("limit must be between 1 and 100".into()))
    }
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
    transaction: &mut Transaction<'_, Postgres>,
    actor: &AuthAccount,
    account_id: i64,
) -> AppResult<identity::public_accounts::AccountAuthorizationState> {
    let target =
        identity::public_accounts::find_account_authorization_state_by_id(transaction, account_id)
            .await?
            .ok_or(AppError::NotFound)?;
    let actor_rank = role_rank(&actor.role).ok_or(AppError::Forbidden)?;
    let target_rank = role_rank(&target.role).ok_or(AppError::Forbidden)?;
    if actor.id == account_id || actor_rank <= target_rank {
        return Err(AppError::Forbidden);
    }
    Ok(target)
}

async fn achievement_record_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    achievement_id: i64,
) -> AppResult<AchievementRecord> {
    sqlx::query_as::<_, AchievementRecord>(
        "SELECT id, slug, name, description, icon_token, status, mint_amount, version, \
                created_at, updated_at \
         FROM platform.badges WHERE id = $1 FOR UPDATE",
    )
    .bind(achievement_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(AppError::NotFound)
}

async fn grant_record(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    achievement_id: i64,
) -> AppResult<AchievementGrantRecord> {
    sqlx::query_as::<_, AchievementGrantRecord>(
        "SELECT account_badge.account_id, account_badge.badge_id, badge.slug, badge.name, \
                badge.icon_token, badge.status AS definition_status, account_badge.award_reason, \
                account_badge.awarded_at, account_badge.awarded_by, account_badge.revoked_at, \
                account_badge.revoked_by, account_badge.revoke_reason \
         FROM platform.account_badges account_badge \
         JOIN platform.badges badge ON badge.id = account_badge.badge_id \
         WHERE account_badge.account_id = $1 AND account_badge.badge_id = $2",
    )
    .bind(account_id)
    .bind(achievement_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(AppError::from)
}

async fn insert_event(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    achievement_id: i64,
    action: &str,
    source: &str,
    actor_id: Option<i64>,
    event_reason: &str,
) -> AppResult<i64> {
    let event_id = sqlx::query_scalar(
        "INSERT INTO platform.achievement_events \
         (account_id, badge_id, action, source, actor_id, reason) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(account_id)
    .bind(achievement_id)
    .bind(action)
    .bind(source)
    .bind(actor_id)
    .bind(event_reason)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(event_id)
}

/// Seed the first-party automatic achievement definitions without overwriting operator changes.
pub async fn seed_achievements(pool: &PgPool) -> AppResult<()> {
    for (slug, name, description, icon, mint_amount) in STANDARD_ACHIEVEMENTS {
        sqlx::query(
            "INSERT INTO platform.badges \
             (slug, name, description, icon_token, mint_amount, status) \
             VALUES ($1, $2, $3, $4, $5, 'active') \
             ON CONFLICT (slug) DO NOTHING",
        )
        .bind(slug)
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(mint_amount)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// List active account awards for public profile composition.
pub async fn list_public_account_achievements(
    pool: &PgPool,
    account_id: i64,
) -> AppResult<Vec<PublicAchievement>> {
    let achievements = sqlx::query_as::<_, (String, String)>(
        "SELECT badge.slug, badge.name \
         FROM platform.account_badges account_badge \
         JOIN platform.badges badge ON badge.id = account_badge.badge_id \
         WHERE account_badge.account_id = $1 AND account_badge.revoked_at IS NULL \
         ORDER BY account_badge.awarded_at DESC, badge.id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(achievements.into_iter().map(|(slug, name)| PublicAchievement { slug, name }).collect())
}

async fn award_achievement_tx(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: i64,
    slug: &str,
    awarded_by: i64,
    award_reason: &str,
) -> AppResult<Option<AutomaticAwardResult>> {
    let award_reason = reason(award_reason)?;
    let achievement = sqlx::query_as::<_, AchievementRecord>(
        "SELECT id, slug, name, description, icon_token, status, mint_amount, version, \
                created_at, updated_at \
         FROM platform.badges WHERE slug = $1 AND status = 'active' FOR UPDATE",
    )
    .bind(slug)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(achievement) = achievement else {
        return Ok(None);
    };
    let inserted = sqlx::query(
        "INSERT INTO platform.account_badges \
         (account_id, badge_id, awarded_by, award_reason) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (account_id, badge_id) DO NOTHING",
    )
    .bind(account_id)
    .bind(achievement.id)
    .bind(awarded_by)
    .bind(award_reason)
    .execute(&mut **transaction)
    .await?
    .rows_affected()
        == 1;
    if inserted {
        let achievement_event_id = insert_event(
            transaction,
            account_id,
            achievement.id,
            "awarded",
            "automatic",
            Some(awarded_by),
            award_reason,
        )
        .await?;
        if achievement.mint_amount > 0 {
            sqlx::query(
                "INSERT INTO platform.pending_mints \
                 (account_id, amount, idempotency_key, badge_slug) \
                 VALUES ($1, $2, $3, $4) ON CONFLICT (idempotency_key) DO NOTHING",
            )
            .bind(account_id)
            .bind(achievement.mint_amount)
            .bind(format!("badge:{}:{account_id}", achievement.slug))
            .bind(&achievement.slug)
            .execute(&mut **transaction)
            .await?;
        }
        crate::outbox::enqueue_notification_tx(
            transaction,
            &format!("achievement-event:{achievement_event_id}:notification"),
            account_id,
            None,
            "achievement_awarded",
            &serde_json::json!({
                "achievementId": achievement.id.to_string(),
                "badgeSlug": &achievement.slug,
                "badgeName": &achievement.name,
                "title": "你获得了新的社区成就",
            }),
            None,
            None,
        )
        .await?;
    }
    Ok(Some(AutomaticAwardResult { newly_awarded: inserted, name: achievement.name }))
}

/// Apply one automatic contribution award and enqueue its notification and mint exactly once.
pub async fn award_achievement_by_slug(
    pool: &PgPool,
    account_id: i64,
    slug: &str,
    awarded_by: i64,
    award_reason: &str,
) -> AppResult<Option<AutomaticAwardResult>> {
    let mut transaction = pool.begin().await?;
    let result =
        award_achievement_tx(&mut transaction, account_id, slug, awarded_by, award_reason).await?;
    transaction.commit().await?;
    Ok(result)
}

/// Consume one claimed automatic-achievement event and complete it with the award atomically.
pub async fn deliver_automatic_award(
    pool: &PgPool,
    event: &crate::outbox::OutboxEvent,
) -> AppResult<()> {
    if event.topic != "achievement_award" {
        return Err(AppError::Internal(
            std::io::Error::other("achievement consumer received a different outbox topic").into(),
        ));
    }
    let award_reason =
        event.payload.get("awardReason").and_then(serde_json::Value::as_str).ok_or_else(|| {
            AppError::Internal(std::io::Error::other("achievement event has no reason").into())
        })?;
    let awarded_by = event.actor_account_id.ok_or_else(|| {
        AppError::Internal(
            std::io::Error::other("achievement event has no contribution actor").into(),
        )
    })?;
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("achievement-outbox:{}", event.id))
        .execute(&mut *transaction)
        .await?;
    if !crate::outbox::lock_claim_tx(&mut transaction, event.id, event.claimed_by).await? {
        transaction.rollback().await?;
        return Ok(());
    }
    award_achievement_tx(
        &mut transaction,
        event.recipient_account_id,
        &event.event_type,
        awarded_by,
        award_reason,
    )
    .await?;
    if !crate::outbox::mark_succeeded_tx(&mut transaction, event.id, event.claimed_by).await? {
        return Err(AppError::Internal(
            std::io::Error::other("locked outbox claim changed").into(),
        ));
    }
    transaction.commit().await?;
    Ok(())
}

async fn admin_list_achievements(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<AchievementDto>>> {
    staff_account(&headers, &state, Capability::ManageBadges).await?;
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = list_limit(query.limit)?;
    let records = sqlx::query_as::<_, AchievementRecord>(
        "SELECT id, slug, name, description, icon_token, status, mint_amount, version, \
                created_at, updated_at \
         FROM platform.badges WHERE ($1::bigint IS NULL OR id < $1) \
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
    Ok(Json(Page::new(visible.into_iter().map(achievement_dto).collect(), next_cursor)))
}

async fn admin_create_achievement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AchievementCreateInput>,
) -> AppResult<(StatusCode, Json<AchievementDto>)> {
    let actor = staff_account(&headers, &state, Capability::ManageBadges).await?;
    let slug = validate_slug(&input.slug)?;
    let name = validate_name(required_text(&input.name, 100, "name")?)?;
    let description =
        validate_description(optional_text(input.description.as_deref(), 240, "description")?)?;
    let icon = validate_icon(&input.icon)?;
    let mint_amount = validate_mint_amount(input.mint_amount)?;
    let audit_reason = reason(&input.reason)?;
    let mut transaction = state.db.begin().await?;
    let record = sqlx::query_as::<_, AchievementRecord>(
        "INSERT INTO platform.badges \
         (slug, name, description, icon_token, mint_amount, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, slug, name, description, icon_token, status, mint_amount, version, \
                   created_at, updated_at",
    )
    .bind(&slug)
    .bind(&name)
    .bind(&description)
    .bind(icon)
    .bind(mint_amount)
    .bind(actor.id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|database| database.constraint() == Some("badges_slug_key"))
        {
            AppError::Conflict("achievement slug already exists".into())
        } else {
            AppError::from(error)
        }
    })?;
    governance::record_account_event_tx(
        &mut transaction,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.achievement.created",
        "achievement",
        &record.id.to_string(),
        audit_reason,
        Some(&serde_json::json!({ "slug": record.slug })),
    )
    .await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(achievement_dto(record))))
}

async fn admin_update_achievement(
    State(state): State<AppState>,
    Path(achievement_id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<AchievementUpdateInput>,
) -> AppResult<Json<AchievementDto>> {
    let actor = staff_account(&headers, &state, Capability::ManageBadges).await?;
    let achievement_id = parse_id(&achievement_id, "achievement id")?;
    if input.expected_version < 1 {
        return Err(AppError::BadRequest("expectedVersion must be positive".into()));
    }
    let name = validate_name(required_text(&input.name, 100, "name")?)?;
    let description =
        validate_description(optional_text(input.description.as_deref(), 240, "description")?)?;
    let icon = validate_icon(&input.icon)?;
    let status = validate_status(&input.status)?;
    let mint_amount = validate_mint_amount(input.mint_amount)?;
    let audit_reason = reason(&input.reason)?;
    let mut transaction = state.db.begin().await?;
    let current = achievement_record_for_update(&mut transaction, achievement_id).await?;
    if current.version != input.expected_version {
        return Err(AppError::Conflict("achievement definition changed".into()));
    }
    let record = sqlx::query_as::<_, AchievementRecord>(
        "UPDATE platform.badges SET name = $1, description = $2, icon_token = $3, status = $4, \
                mint_amount = $5, version = version + 1, updated_at = now() \
         WHERE id = $6 AND version = $7 \
         RETURNING id, slug, name, description, icon_token, status, mint_amount, version, \
                   created_at, updated_at",
    )
    .bind(&name)
    .bind(&description)
    .bind(icon)
    .bind(status)
    .bind(mint_amount)
    .bind(achievement_id)
    .bind(input.expected_version)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::Conflict("achievement definition changed".into()))?;
    governance::record_account_event_tx(
        &mut transaction,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.achievement.updated",
        "achievement",
        &achievement_id.to_string(),
        audit_reason,
        Some(&serde_json::json!({
            "fromVersion": current.version,
            "toVersion": record.version,
            "status": record.status,
        })),
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(achievement_dto(record)))
}

async fn admin_list_account_achievements(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<AchievementGrantDto>>> {
    let actor = staff_account(&headers, &state, Capability::ManageBadges).await?;
    let account_id = parse_id(&account_id, "account id")?;
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = list_limit(query.limit)?;
    let mut transaction = state.db.begin().await?;
    require_lower_target(&mut transaction, &actor, account_id).await?;
    let records = sqlx::query_as::<_, AchievementGrantRecord>(
        "SELECT account_badge.account_id, account_badge.badge_id, badge.slug, badge.name, \
                badge.icon_token, badge.status AS definition_status, account_badge.award_reason, \
                account_badge.awarded_at, account_badge.awarded_by, account_badge.revoked_at, \
                account_badge.revoked_by, account_badge.revoke_reason \
         FROM platform.account_badges account_badge \
         JOIN platform.badges badge ON badge.id = account_badge.badge_id \
         WHERE account_badge.account_id = $1 AND ($2::bigint IS NULL OR badge.id < $2) \
         ORDER BY badge.id DESC LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    let has_more = records.len() > limit as usize;
    let visible = records.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor =
        has_more.then(|| visible.last().map(|record| record.badge_id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(achievement_grant_dto).collect(), next_cursor)))
}

async fn admin_grant_achievement(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<AchievementGrantInput>,
) -> AppResult<(StatusCode, Json<AchievementGrantDto>)> {
    let actor = staff_account(&headers, &state, Capability::ManageBadges).await?;
    let account_id = parse_id(&account_id, "account id")?;
    let achievement_id = parse_id(&input.achievement_id, "achievement id")?;
    let audit_reason = reason(&input.reason)?;
    let mut transaction = state.db.begin().await?;
    let target = require_lower_target(&mut transaction, &actor, account_id).await?;
    if target.status != "active" {
        return Err(AppError::Conflict("achievement target must be active".into()));
    }
    let achievement = achievement_record_for_update(&mut transaction, achievement_id).await?;
    if achievement.status != "active" {
        return Err(AppError::Conflict("retired achievements cannot be awarded".into()));
    }
    let current_revoked_at: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
        "SELECT revoked_at FROM platform.account_badges \
         WHERE account_id = $1 AND badge_id = $2 FOR UPDATE",
    )
    .bind(account_id)
    .bind(achievement_id)
    .fetch_optional(&mut *transaction)
    .await?;
    match current_revoked_at {
        Some(None) => {
            return Err(AppError::Conflict("account already has this achievement".into()))
        }
        Some(Some(_)) => {
            sqlx::query(
                "UPDATE platform.account_badges \
                 SET awarded_at = now(), awarded_by = $1, award_reason = $2, revoked_at = NULL, \
                     revoked_by = NULL, revoke_reason = NULL, updated_at = now() \
                 WHERE account_id = $3 AND badge_id = $4",
            )
            .bind(actor.id)
            .bind(audit_reason)
            .bind(account_id)
            .bind(achievement_id)
            .execute(&mut *transaction)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO platform.account_badges \
                 (account_id, badge_id, awarded_by, award_reason) VALUES ($1, $2, $3, $4)",
            )
            .bind(account_id)
            .bind(achievement_id)
            .bind(actor.id)
            .bind(audit_reason)
            .execute(&mut *transaction)
            .await?;
        }
    }
    let achievement_event_id = insert_event(
        &mut transaction,
        account_id,
        achievement_id,
        "awarded",
        "manual",
        Some(actor.id),
        audit_reason,
    )
    .await?;
    crate::outbox::enqueue_notification_tx(
        &mut transaction,
        &format!("achievement-event:{achievement_event_id}:notification"),
        account_id,
        None,
        "achievement_awarded",
        &serde_json::json!({
            "achievementId": achievement_id.to_string(),
            "badgeSlug": &achievement.slug,
            "badgeName": &achievement.name,
            "title": "你获得了新的社区成就",
        }),
        None,
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.achievement.awarded",
        "account_achievement",
        &format!("{account_id}:{achievement_id}"),
        audit_reason,
        Some(&serde_json::json!({
            "accountId": account_id.to_string(),
            "achievementSlug": achievement.slug,
            "manualMint": false,
        })),
    )
    .await?;
    let record = grant_record(&mut transaction, account_id, achievement_id).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(achievement_grant_dto(record))))
}

async fn admin_revoke_achievement(
    State(state): State<AppState>,
    Path((account_id, achievement_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(input): Json<AchievementRevokeInput>,
) -> AppResult<Json<AchievementGrantDto>> {
    let actor = staff_account(&headers, &state, Capability::ManageBadges).await?;
    let account_id = parse_id(&account_id, "account id")?;
    let achievement_id = parse_id(&achievement_id, "achievement id")?;
    let audit_reason = reason(&input.reason)?;
    let mut transaction = state.db.begin().await?;
    require_lower_target(&mut transaction, &actor, account_id).await?;
    let achievement = achievement_record_for_update(&mut transaction, achievement_id).await?;
    let current = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT revoked_at FROM platform.account_badges \
         WHERE account_id = $1 AND badge_id = $2 FOR UPDATE",
    )
    .bind(account_id)
    .bind(achievement_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(AppError::NotFound)?;
    if current.is_some() {
        return Err(AppError::Conflict("achievement award is already revoked".into()));
    }
    sqlx::query(
        "UPDATE platform.account_badges SET revoked_at = now(), revoked_by = $1, \
                revoke_reason = $2, updated_at = now() \
         WHERE account_id = $3 AND badge_id = $4 AND revoked_at IS NULL",
    )
    .bind(actor.id)
    .bind(audit_reason)
    .bind(account_id)
    .bind(achievement_id)
    .execute(&mut *transaction)
    .await?;
    let achievement_event_id = insert_event(
        &mut transaction,
        account_id,
        achievement_id,
        "revoked",
        "manual",
        Some(actor.id),
        audit_reason,
    )
    .await?;
    crate::outbox::enqueue_notification_tx(
        &mut transaction,
        &format!("achievement-event:{achievement_event_id}:notification"),
        account_id,
        None,
        "achievement_revoked",
        &serde_json::json!({
            "achievementId": achievement_id.to_string(),
            "badgeSlug": &achievement.slug,
            "badgeName": &achievement.name,
            "title": "一项社区成就已被撤销",
        }),
        None,
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        AccountActor { account_id: actor.id, role: &actor.role },
        "platform.achievement.revoked",
        "account_achievement",
        &format!("{account_id}:{achievement_id}"),
        audit_reason,
        Some(&serde_json::json!({
            "accountId": account_id.to_string(),
            "achievementSlug": achievement.slug,
            "mintReversed": false,
        })),
    )
    .await?;
    let record = grant_record(&mut transaction, account_id, achievement_id).await?;
    transaction.commit().await?;
    Ok(Json(achievement_grant_dto(record)))
}

async fn admin_list_achievement_events(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<AchievementEventDto>>> {
    let actor = staff_account(&headers, &state, Capability::ManageBadges).await?;
    let account_id = parse_id(&account_id, "account id")?;
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = list_limit(query.limit)?;
    let mut transaction = state.db.begin().await?;
    require_lower_target(&mut transaction, &actor, account_id).await?;
    let records = sqlx::query_as::<_, AchievementEventRecord>(
        "SELECT event.id, event.badge_id, badge.slug, badge.name, event.action, event.source, \
                event.actor_id, event.reason, event.created_at \
         FROM platform.achievement_events event \
         JOIN platform.badges badge ON badge.id = event.badge_id \
         WHERE event.account_id = $1 AND ($2::bigint IS NULL OR event.id < $2) \
         ORDER BY event.id DESC LIMIT $3",
    )
    .bind(account_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;
    let has_more = records.len() > limit as usize;
    let visible = records.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor =
        has_more.then(|| visible.last().map(|record| record.id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(achievement_event_dto).collect(), next_cursor)))
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v2/admin/achievements",
            get(admin_list_achievements).post(admin_create_achievement),
        )
        .route(
            "/api/v2/admin/achievements/{achievement_id}",
            axum::routing::patch(admin_update_achievement),
        )
        .route(
            "/api/v2/admin/users/{account_id}/achievements",
            get(admin_list_account_achievements).post(admin_grant_achievement),
        )
        .route(
            "/api/v2/admin/users/{account_id}/achievements/{achievement_id}/revoke",
            post(admin_revoke_achievement),
        )
        .route(
            "/api/v2/admin/users/{account_id}/achievement-events",
            get(admin_list_achievement_events),
        )
        .route(
            "/api/v2/admin/platform/badges",
            get(admin_list_achievements).post(admin_create_achievement),
        )
}

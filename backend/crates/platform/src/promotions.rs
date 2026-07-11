use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, Page};
use sqlx::{FromRow, Postgres, Transaction};

use crate::auth::{is_staff, optional_account, staff_account};
use crate::validation::{optional_text, parse_id, reason, required_text, schedule, timestamp};

#[derive(Debug, Clone, FromRow)]
struct PromotionRecord {
    id: i64,
    placement: String,
    title: String,
    body: Option<String>,
    cta_label: Option<String>,
    target_url: String,
    asset_id: Option<i64>,
    status: String,
    priority: i32,
    audience: String,
    version: i64,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromotionDto {
    id: String,
    placement: String,
    title: String,
    body: Option<String>,
    cta_label: Option<String>,
    target_url: String,
    asset_id: Option<String>,
    status: String,
    effective_state: String,
    priority: i32,
    audience: String,
    version: i64,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    archived_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromotionCreateInput {
    placement: String,
    title: String,
    body: Option<String>,
    cta_label: Option<String>,
    target_url: String,
    asset_id: Option<String>,
    status: String,
    priority: i32,
    audience: String,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromotionUpdateInput {
    placement: String,
    title: String,
    body: Option<String>,
    cta_label: Option<String>,
    target_url: String,
    asset_id: Option<String>,
    status: String,
    priority: i32,
    audience: String,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    reason: String,
    expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveInput {
    expected_version: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicQuery {
    placement: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdminListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

struct ValidatedPromotion {
    placement: String,
    title: String,
    body: Option<String>,
    cta_label: Option<String>,
    target_url: String,
    asset_id: Option<i64>,
    status: String,
    priority: i32,
    audience: String,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
}

fn validate_target_url(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > 2048
        || !value.starts_with('/')
        || value.starts_with("//")
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || value == "/api"
        || value.starts_with("/api/")
    {
        return Err(AppError::BadRequest(
            "targetUrl must be a safe same-origin application path".into(),
        ));
    }
    Ok(value.to_owned())
}

fn parse_asset_id(value: Option<&str>) -> AppResult<Option<i64>> {
    value.map(|value| parse_id(value, "asset id")).transpose()
}

fn validate_fields(
    input: &PromotionCreateInput,
    now: DateTime<Utc>,
) -> AppResult<ValidatedPromotion> {
    if !matches!(input.placement.as_str(), "home-left-primary" | "home-left-secondary") {
        return Err(AppError::BadRequest("invalid promotion placement".into()));
    }
    if !matches!(input.status.as_str(), "draft" | "scheduled" | "published" | "paused") {
        return Err(AppError::BadRequest("invalid promotion status".into()));
    }
    if !matches!(input.audience.as_str(), "all" | "authenticated" | "staff") {
        return Err(AppError::BadRequest("invalid promotion audience".into()));
    }
    if !(-1000..=1000).contains(&input.priority) {
        return Err(AppError::BadRequest("priority must be between -1000 and 1000".into()));
    }
    let starts_at = timestamp(input.starts_at, "startsAt")?;
    let ends_at = timestamp(input.ends_at, "endsAt")?;
    schedule(&input.status, starts_at, ends_at, now)?;
    Ok(ValidatedPromotion {
        placement: input.placement.clone(),
        title: required_text(&input.title, 120, "title")?,
        body: optional_text(input.body.as_deref(), 500, "body")?,
        cta_label: optional_text(input.cta_label.as_deref(), 40, "ctaLabel")?,
        target_url: validate_target_url(&input.target_url)?,
        asset_id: parse_asset_id(input.asset_id.as_deref())?,
        status: input.status.clone(),
        priority: input.priority,
        audience: input.audience.clone(),
        starts_at,
        ends_at,
    })
}

fn effective_state(record: &PromotionRecord, now: DateTime<Utc>) -> String {
    if record.status == "archived" {
        return "archived".into();
    }
    if record.status == "paused" {
        return "paused".into();
    }
    if record.ends_at.is_some_and(|ends_at| ends_at <= now) {
        return "expired".into();
    }
    if record.status == "draft" {
        return "draft".into();
    }
    if record.starts_at.is_some_and(|starts_at| starts_at > now) {
        return "scheduled".into();
    }
    "active".into()
}

fn dto(record: PromotionRecord) -> PromotionDto {
    PromotionDto {
        id: record.id.to_string(),
        effective_state: effective_state(&record, Utc::now()),
        placement: record.placement,
        title: record.title,
        body: record.body,
        cta_label: record.cta_label,
        target_url: record.target_url,
        asset_id: record.asset_id.map(|value| value.to_string()),
        status: record.status,
        priority: record.priority,
        audience: record.audience,
        version: record.version,
        starts_at: record.starts_at.map(|value| value.timestamp()),
        ends_at: record.ends_at.map(|value| value.timestamp()),
        archived_at: record.archived_at.map(|value| value.timestamp()),
        created_at: record.created_at.timestamp(),
        updated_at: record.updated_at.timestamp(),
    }
}

fn transition_allowed(current: &str, next: &str) -> bool {
    match current {
        "draft" => matches!(next, "draft" | "scheduled" | "published" | "paused"),
        "scheduled" => matches!(next, "draft" | "scheduled" | "published" | "paused"),
        "published" => matches!(next, "published" | "paused"),
        "paused" => matches!(next, "paused" | "scheduled" | "published"),
        "archived" => next == "archived",
        _ => false,
    }
}

async fn require_authorized_asset(
    state: &AppState,
    asset_id: Option<i64>,
    account_id: i64,
) -> AppResult<()> {
    if let Some(asset_id) = asset_id {
        if !media::is_clean_image_owned_by(&state.db, asset_id, account_id).await? {
            return Err(AppError::BadRequest(
                "assetId must reference one of the operator's clean image uploads".into(),
            ));
        }
    }
    Ok(())
}

async fn find_record(
    tx: &mut Transaction<'_, Postgres>,
    promotion_id: i64,
) -> AppResult<PromotionRecord> {
    sqlx::query_as::<_, PromotionRecord>(
        "SELECT id, placement, title, body, cta_label, target_url, asset_id, status, priority, \
                audience, version, starts_at, ends_at, created_at, updated_at, archived_at \
         FROM platform.promotions WHERE id = $1 FOR UPDATE",
    )
    .bind(promotion_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

async fn list_active(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PublicQuery>,
) -> AppResult<Json<Vec<PromotionDto>>> {
    if query
        .placement
        .as_deref()
        .is_some_and(|placement| !matches!(placement, "home-left-primary" | "home-left-secondary"))
    {
        return Err(AppError::BadRequest("invalid promotion placement".into()));
    }
    let account = optional_account(&headers, &state).await?;
    let account_id = account.as_ref().map(|account| account.id);
    let staff = is_staff(account.as_ref());
    let rows = sqlx::query_as::<_, PromotionRecord>(
        "SELECT id, placement, title, body, cta_label, target_url, asset_id, status, priority, \
                audience, version, starts_at, ends_at, created_at, updated_at, archived_at \
         FROM platform.promotions \
         WHERE status IN ('scheduled', 'published') \
           AND (starts_at IS NULL OR starts_at <= now()) \
           AND (ends_at IS NULL OR ends_at > now()) \
           AND ($1::text IS NULL OR placement = $1) \
           AND (audience = 'all' OR ($2::bigint IS NOT NULL AND audience = 'authenticated') \
                OR ($3::boolean AND audience = 'staff')) \
         ORDER BY CASE placement WHEN 'home-left-primary' THEN 0 ELSE 1 END, \
                  priority DESC, starts_at DESC NULLS LAST, id \
         LIMIT 10",
    )
    .bind(query.placement)
    .bind(account_id)
    .bind(staff)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(dto).collect()))
}

async fn admin_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminListQuery>,
) -> AppResult<Json<Page<PromotionDto>>> {
    staff_account(&headers, &state, Capability::ManagePromotions).await?;
    let cursor = query.cursor.as_deref().map(|cursor| parse_id(cursor, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let rows = sqlx::query_as::<_, PromotionRecord>(
        "SELECT id, placement, title, body, cta_label, target_url, asset_id, status, priority, \
                audience, version, starts_at, ends_at, created_at, updated_at, archived_at \
         FROM platform.promotions WHERE ($1::bigint IS NULL OR id < $1) \
         ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = has_more.then(|| visible.last().map(|row| row.id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(dto).collect(), next_cursor)))
}

async fn admin_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PromotionCreateInput>,
) -> AppResult<(StatusCode, Json<PromotionDto>)> {
    let account = staff_account(&headers, &state, Capability::ManagePromotions).await?;
    let now = Utc::now();
    let values = validate_fields(&input, now)?;
    let reason = reason(&input.reason)?;
    require_authorized_asset(&state, values.asset_id, account.id).await?;
    let mut tx = state.db.begin().await?;
    let record = sqlx::query_as::<_, PromotionRecord>(
        "INSERT INTO platform.promotions \
         (placement, title, body, cta_label, target_url, asset_id, status, priority, audience, \
          starts_at, ends_at, created_by, updated_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13, $13) \
         RETURNING id, placement, title, body, cta_label, target_url, asset_id, status, priority, \
                   audience, version, starts_at, ends_at, created_at, updated_at, archived_at",
    )
    .bind(&values.placement)
    .bind(&values.title)
    .bind(&values.body)
    .bind(&values.cta_label)
    .bind(&values.target_url)
    .bind(values.asset_id)
    .bind(&values.status)
    .bind(values.priority)
    .bind(&values.audience)
    .bind(values.starts_at)
    .bind(values.ends_at)
    .bind(account.id)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    let metadata = serde_json::json!({
        "placement": record.placement,
        "status": record.status,
        "hasAsset": record.asset_id.is_some(),
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.promotion.created",
        "promotion",
        &record.id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(dto(record))))
}

async fn admin_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(promotion_id): Path<String>,
    Json(input): Json<PromotionUpdateInput>,
) -> AppResult<Json<PromotionDto>> {
    let account = staff_account(&headers, &state, Capability::ManagePromotions).await?;
    let promotion_id = parse_id(&promotion_id, "promotion id")?;
    if input.expected_version < 1 {
        return Err(AppError::BadRequest("expectedVersion must be positive".into()));
    }
    let create_shape = PromotionCreateInput {
        placement: input.placement,
        title: input.title,
        body: input.body,
        cta_label: input.cta_label,
        target_url: input.target_url,
        asset_id: input.asset_id,
        status: input.status,
        priority: input.priority,
        audience: input.audience,
        starts_at: input.starts_at,
        ends_at: input.ends_at,
        reason: input.reason,
    };
    let now = Utc::now();
    let values = validate_fields(&create_shape, now)?;
    let reason = reason(&create_shape.reason)?;
    let snapshot = sqlx::query_as::<_, PromotionRecord>(
        "SELECT id, placement, title, body, cta_label, target_url, asset_id, status, priority, \
                audience, version, starts_at, ends_at, created_at, updated_at, archived_at \
         FROM platform.promotions WHERE id = $1",
    )
    .bind(promotion_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if snapshot.version != input.expected_version {
        return Err(AppError::Conflict("promotion was changed by another operator".into()));
    }
    if values.asset_id != snapshot.asset_id {
        require_authorized_asset(&state, values.asset_id, account.id).await?;
    }
    let mut tx = state.db.begin().await?;
    let current = find_record(&mut tx, promotion_id).await?;
    if current.version != input.expected_version {
        return Err(AppError::Conflict("promotion was changed by another operator".into()));
    }
    if !transition_allowed(&current.status, &values.status) {
        return Err(AppError::Conflict(format!(
            "promotion cannot transition from {} to {}",
            current.status, values.status
        )));
    }
    let record = sqlx::query_as::<_, PromotionRecord>(
        "UPDATE platform.promotions SET placement = $1, title = $2, body = $3, cta_label = $4, \
                target_url = $5, asset_id = $6, status = $7, priority = $8, audience = $9, \
                starts_at = $10, ends_at = $11, updated_by = $12, updated_at = $13, \
                archived_at = NULL, version = version + 1 \
         WHERE id = $14 AND version = $15 \
         RETURNING id, placement, title, body, cta_label, target_url, asset_id, status, priority, \
                   audience, version, starts_at, ends_at, created_at, updated_at, archived_at",
    )
    .bind(&values.placement)
    .bind(&values.title)
    .bind(&values.body)
    .bind(&values.cta_label)
    .bind(&values.target_url)
    .bind(values.asset_id)
    .bind(&values.status)
    .bind(values.priority)
    .bind(&values.audience)
    .bind(values.starts_at)
    .bind(values.ends_at)
    .bind(account.id)
    .bind(now)
    .bind(promotion_id)
    .bind(input.expected_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("promotion was changed by another operator".into()))?;
    let metadata = serde_json::json!({
        "oldStatus": current.status,
        "newStatus": record.status,
        "oldPlacement": current.placement,
        "newPlacement": record.placement,
        "version": record.version,
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.promotion.updated",
        "promotion",
        &promotion_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(dto(record)))
}

async fn admin_archive(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(promotion_id): Path<String>,
    Json(input): Json<ArchiveInput>,
) -> AppResult<StatusCode> {
    let account = staff_account(&headers, &state, Capability::ManagePromotions).await?;
    let promotion_id = parse_id(&promotion_id, "promotion id")?;
    let reason = reason(&input.reason)?;
    let mut tx = state.db.begin().await?;
    let current = find_record(&mut tx, promotion_id).await?;
    if current.version != input.expected_version {
        return Err(AppError::Conflict("promotion was changed by another operator".into()));
    }
    if current.status == "archived" {
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    let affected = sqlx::query(
        "UPDATE platform.promotions SET status = 'archived', archived_at = now(), \
                updated_at = now(), updated_by = $1, version = version + 1 \
         WHERE id = $2 AND version = $3",
    )
    .bind(account.id)
    .bind(promotion_id)
    .bind(input.expected_version)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict("promotion was changed by another operator".into()));
    }
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.promotion.archived",
        "promotion",
        &promotion_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v2/promotions", get(list_active))
        .route("/api/v2/admin/promotions", get(admin_list).post(admin_create))
        .route("/api/v2/admin/promotions/{id}", patch(admin_update).delete(admin_archive))
}

#[cfg(test)]
mod tests {
    use super::{transition_allowed, validate_target_url};

    #[test]
    fn promotion_links_are_same_origin_paths_only() {
        assert!(validate_target_url("/forum/threads/42").is_ok());
        assert!(validate_target_url("https://example.com").is_err());
        assert!(validate_target_url("//example.com/path").is_err());
        assert!(validate_target_url("/api/admin/promotions").is_err());
    }

    #[test]
    fn archived_promotions_are_terminal() {
        assert!(!transition_allowed("archived", "published"));
        assert!(transition_allowed("published", "paused"));
    }
}

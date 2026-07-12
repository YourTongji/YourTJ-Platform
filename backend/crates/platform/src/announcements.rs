use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::{Deserialize, Serialize};
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, AuthAccount, Page};
use sqlx::{FromRow, Postgres, Transaction};

use crate::auth::{is_staff, optional_account, required_account, staff_account};
use crate::validation::{optional_text, parse_id, reason, required_text, schedule, timestamp};

#[derive(Debug, Clone, FromRow)]
struct AnnouncementRecord {
    id: i64,
    title: String,
    body: Option<String>,
    status: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    version: i64,
    revision: i64,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct AnnouncementListRow {
    #[sqlx(flatten)]
    announcement: AnnouncementRecord,
    first_seen_at: Option<DateTime<Utc>>,
    dismissed_at: Option<DateTime<Utc>>,
    acknowledged_at: Option<DateTime<Utc>>,
    seen_count: Option<i64>,
    dismissed_count: Option<i64>,
    acknowledged_count: Option<i64>,
}

#[derive(Debug, FromRow)]
struct ReceiptRow {
    revision: i64,
    first_seen_at: Option<DateTime<Utc>>,
    dismissed_at: Option<DateTime<Utc>>,
    acknowledged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct AnnouncementRevisionRow {
    announcement_id: i64,
    version: i64,
    revision: i64,
    title: String,
    body: Option<String>,
    status: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementReceiptDto {
    revision: i64,
    first_seen_at: Option<i64>,
    dismissed_at: Option<i64>,
    acknowledged_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementReceiptSummaryDto {
    seen_count: i64,
    dismissed_count: i64,
    acknowledged_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementDto {
    id: String,
    title: String,
    body: Option<String>,
    status: String,
    effective_state: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    version: i64,
    revision: i64,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    published_at: Option<i64>,
    archived_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
    receipt: Option<AnnouncementReceiptDto>,
    receipt_summary: Option<AnnouncementReceiptSummaryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementRevisionDto {
    announcement_id: String,
    version: i64,
    revision: i64,
    title: String,
    body: Option<String>,
    status: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementCreateInput {
    title: String,
    body: Option<String>,
    status: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementUpdateInput {
    title: String,
    body: Option<String>,
    status: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    starts_at: Option<i64>,
    ends_at: Option<i64>,
    reason: String,
    expected_version: i64,
    bump_revision: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveInput {
    expected_version: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptInput {
    revision: i64,
    action: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

struct ValidatedAnnouncement {
    title: String,
    body: Option<String>,
    status: String,
    presentation: String,
    severity: String,
    priority: i32,
    audience: String,
    requires_ack: bool,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
}

fn validate_fields(
    input: &AnnouncementCreateInput,
    now: DateTime<Utc>,
) -> AppResult<ValidatedAnnouncement> {
    if !matches!(input.status.as_str(), "draft" | "scheduled" | "published") {
        return Err(AppError::BadRequest("invalid announcement status".into()));
    }
    if !matches!(input.presentation.as_str(), "card" | "banner") {
        return Err(AppError::BadRequest("invalid announcement presentation".into()));
    }
    if !matches!(input.severity.as_str(), "info" | "success" | "warning" | "critical") {
        return Err(AppError::BadRequest("invalid announcement severity".into()));
    }
    if !matches!(input.audience.as_str(), "all" | "authenticated" | "staff") {
        return Err(AppError::BadRequest("invalid announcement audience".into()));
    }
    if !(-1000..=1000).contains(&input.priority) {
        return Err(AppError::BadRequest("priority must be between -1000 and 1000".into()));
    }
    let starts_at = timestamp(input.starts_at, "startsAt")?;
    let ends_at = timestamp(input.ends_at, "endsAt")?;
    schedule(&input.status, starts_at, ends_at, now)?;
    Ok(ValidatedAnnouncement {
        title: required_text(&input.title, 200, "title")?,
        body: optional_text(input.body.as_deref(), 20_000, "body")?,
        status: input.status.clone(),
        presentation: input.presentation.clone(),
        severity: input.severity.clone(),
        priority: input.priority,
        audience: input.audience.clone(),
        requires_ack: input.requires_ack,
        starts_at,
        ends_at,
    })
}

fn effective_state(record: &AnnouncementRecord, now: DateTime<Utc>) -> String {
    if record.status == "archived" {
        return "archived".into();
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

fn receipt_dto(row: &ReceiptRow) -> AnnouncementReceiptDto {
    AnnouncementReceiptDto {
        revision: row.revision,
        first_seen_at: row.first_seen_at.map(|value| value.timestamp()),
        dismissed_at: row.dismissed_at.map(|value| value.timestamp()),
        acknowledged_at: row.acknowledged_at.map(|value| value.timestamp()),
    }
}

fn dto(
    record: AnnouncementRecord,
    receipt: Option<AnnouncementReceiptDto>,
    receipt_summary: Option<AnnouncementReceiptSummaryDto>,
) -> AnnouncementDto {
    AnnouncementDto {
        id: record.id.to_string(),
        effective_state: effective_state(&record, Utc::now()),
        title: record.title,
        body: record.body,
        status: record.status,
        presentation: record.presentation,
        severity: record.severity,
        priority: record.priority,
        audience: record.audience,
        requires_ack: record.requires_ack,
        version: record.version,
        revision: record.revision,
        starts_at: record.starts_at.map(|value| value.timestamp()),
        ends_at: record.ends_at.map(|value| value.timestamp()),
        published_at: record.published_at.map(|value| value.timestamp()),
        archived_at: record.archived_at.map(|value| value.timestamp()),
        created_at: record.created_at.timestamp(),
        updated_at: record.updated_at.timestamp(),
        receipt,
        receipt_summary,
    }
}

fn list_row_dto(row: AnnouncementListRow) -> AnnouncementDto {
    let receipt = (row.first_seen_at.is_some()
        || row.dismissed_at.is_some()
        || row.acknowledged_at.is_some())
    .then(|| AnnouncementReceiptDto {
        revision: row.announcement.revision,
        first_seen_at: row.first_seen_at.map(|value| value.timestamp()),
        dismissed_at: row.dismissed_at.map(|value| value.timestamp()),
        acknowledged_at: row.acknowledged_at.map(|value| value.timestamp()),
    });
    let summary = row.seen_count.map(|seen_count| AnnouncementReceiptSummaryDto {
        seen_count,
        dismissed_count: row.dismissed_count.unwrap_or(0),
        acknowledged_count: row.acknowledged_count.unwrap_or(0),
    });
    dto(row.announcement, receipt, summary)
}

fn revision_dto(row: AnnouncementRevisionRow) -> AnnouncementRevisionDto {
    AnnouncementRevisionDto {
        announcement_id: row.announcement_id.to_string(),
        version: row.version,
        revision: row.revision,
        title: row.title,
        body: row.body,
        status: row.status,
        presentation: row.presentation,
        severity: row.severity,
        priority: row.priority,
        audience: row.audience,
        requires_ack: row.requires_ack,
        starts_at: row.starts_at.map(|value| value.timestamp()),
        ends_at: row.ends_at.map(|value| value.timestamp()),
        created_at: row.created_at.timestamp(),
    }
}

fn audience_allows(audience: &str, account: Option<&AuthAccount>) -> bool {
    match audience {
        "all" => true,
        "authenticated" => account.is_some(),
        "staff" => is_staff(account),
        _ => false,
    }
}

fn is_active(record: &AnnouncementRecord, now: DateTime<Utc>) -> bool {
    matches!(record.status.as_str(), "published" | "scheduled")
        && record.starts_at.is_none_or(|starts_at| starts_at <= now)
        && record.ends_at.is_none_or(|ends_at| ends_at > now)
}

fn transition_allowed(current: &str, next: &str) -> bool {
    match current {
        "draft" => matches!(next, "draft" | "scheduled" | "published"),
        "scheduled" => matches!(next, "draft" | "scheduled" | "published"),
        "published" => next == "published",
        "archived" => next == "archived",
        _ => false,
    }
}

async fn find_record(
    tx: &mut Transaction<'_, Postgres>,
    announcement_id: i64,
) -> AppResult<AnnouncementRecord> {
    sqlx::query_as::<_, AnnouncementRecord>(
        "SELECT id, title, body, status, presentation, severity, priority, audience, \
                requires_ack, version, revision, starts_at, ends_at, published_at, archived_at, \
                created_at, updated_at \
         FROM platform.announcements WHERE id = $1 FOR UPDATE",
    )
    .bind(announcement_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

async fn insert_revision(
    tx: &mut Transaction<'_, Postgres>,
    record: &AnnouncementRecord,
    account_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO platform.announcement_revisions \
         (announcement_id, version, revision, title, body, status, presentation, severity, priority, \
          audience, requires_ack, starts_at, ends_at, changed_by, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(record.id)
    .bind(record.version)
    .bind(record.revision)
    .bind(&record.title)
    .bind(&record.body)
    .bind(&record.status)
    .bind(&record.presentation)
    .bind(&record.severity)
    .bind(record.priority)
    .bind(&record.audience)
    .bind(record.requires_ack)
    .bind(record.starts_at)
    .bind(record.ends_at)
    .bind(account_id)
    .bind(record.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn list_active(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AnnouncementDto>>> {
    let account = optional_account(&headers, &state).await?;
    let account_id = account.as_ref().map(|account| account.id);
    let staff = is_staff(account.as_ref());
    let rows = sqlx::query_as::<_, AnnouncementListRow>(
        "SELECT a.id, a.title, a.body, a.status, a.presentation, a.severity, a.priority, \
                a.audience, a.requires_ack, a.version, a.revision, a.starts_at, a.ends_at, \
                a.published_at, a.archived_at, a.created_at, a.updated_at, \
                receipt.first_seen_at, receipt.dismissed_at, receipt.acknowledged_at, \
                NULL::bigint AS seen_count, NULL::bigint AS dismissed_count, \
                NULL::bigint AS acknowledged_count \
         FROM platform.announcements a \
         LEFT JOIN platform.announcement_receipts receipt \
           ON receipt.account_id = $1 AND receipt.announcement_id = a.id \
          AND receipt.revision = a.revision \
         WHERE a.status IN ('scheduled', 'published') \
           AND (a.starts_at IS NULL OR a.starts_at <= now()) \
           AND (a.ends_at IS NULL OR a.ends_at > now()) \
           AND (a.audience = 'all' OR ($1::bigint IS NOT NULL AND a.audience = 'authenticated') \
                OR ($2::boolean AND a.audience = 'staff')) \
         ORDER BY a.priority DESC, COALESCE(a.published_at, a.starts_at, a.created_at) DESC, a.id \
         LIMIT 50",
    )
    .bind(account_id)
    .bind(staff)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(list_row_dto).collect()))
}

async fn list_unread(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<AnnouncementDto>>> {
    let account = required_account(&headers, &state).await?;
    let staff = is_staff(Some(&account));
    let rows = sqlx::query_as::<_, AnnouncementListRow>(
        "SELECT a.id, a.title, a.body, a.status, a.presentation, a.severity, a.priority, \
                a.audience, a.requires_ack, a.version, a.revision, a.starts_at, a.ends_at, \
                a.published_at, a.archived_at, a.created_at, a.updated_at, \
                NULL::timestamptz AS first_seen_at, NULL::timestamptz AS dismissed_at, \
                NULL::timestamptz AS acknowledged_at, NULL::bigint AS seen_count, \
                NULL::bigint AS dismissed_count, NULL::bigint AS acknowledged_count \
         FROM platform.announcements a \
         WHERE a.status IN ('scheduled', 'published') \
           AND (a.starts_at IS NULL OR a.starts_at <= now()) \
           AND (a.ends_at IS NULL OR a.ends_at > now()) \
           AND (a.audience = 'all' OR a.audience = 'authenticated' \
                OR ($2::boolean AND a.audience = 'staff')) \
           AND NOT EXISTS ( \
             SELECT 1 FROM platform.announcement_receipts receipt \
             WHERE receipt.account_id = $1 AND receipt.announcement_id = a.id \
               AND receipt.revision = a.revision AND receipt.first_seen_at IS NOT NULL \
           ) \
         ORDER BY a.priority DESC, COALESCE(a.published_at, a.starts_at, a.created_at) DESC, a.id \
         LIMIT 20",
    )
    .bind(account.id)
    .bind(staff)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(list_row_dto).collect()))
}

async fn record_receipt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(announcement_id): Path<String>,
    Json(input): Json<ReceiptInput>,
) -> AppResult<Json<AnnouncementReceiptDto>> {
    let account = required_account(&headers, &state).await?;
    let announcement_id = parse_id(&announcement_id, "announcement id")?;
    let record = sqlx::query_as::<_, AnnouncementRecord>(
        "SELECT id, title, body, status, presentation, severity, priority, audience, \
                requires_ack, version, revision, starts_at, ends_at, published_at, archived_at, \
                created_at, updated_at FROM platform.announcements WHERE id = $1",
    )
    .bind(announcement_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if !is_active(&record, Utc::now()) || !audience_allows(&record.audience, Some(&account)) {
        return Err(AppError::NotFound);
    }
    if input.revision != record.revision {
        return Err(AppError::Conflict("announcement revision changed".into()));
    }
    if !matches!(input.action.as_str(), "seen" | "dismiss" | "acknowledge") {
        return Err(AppError::BadRequest("invalid receipt action".into()));
    }
    if input.action == "acknowledge" && !record.requires_ack {
        return Err(AppError::BadRequest("announcement does not require acknowledgement".into()));
    }
    let row = match input.action.as_str() {
        "seen" => {
            sqlx::query_as::<_, ReceiptRow>(
                "INSERT INTO platform.announcement_receipts \
                 (account_id, announcement_id, revision, first_seen_at) VALUES ($1, $2, $3, now()) \
                 ON CONFLICT (account_id, announcement_id, revision) DO UPDATE \
                 SET first_seen_at = COALESCE(platform.announcement_receipts.first_seen_at, now()) \
                 RETURNING revision, first_seen_at, dismissed_at, acknowledged_at",
            )
            .bind(account.id)
            .bind(announcement_id)
            .bind(input.revision)
            .fetch_one(&state.db)
            .await?
        }
        "dismiss" => {
            sqlx::query_as::<_, ReceiptRow>(
                "INSERT INTO platform.announcement_receipts \
                 (account_id, announcement_id, revision, first_seen_at, dismissed_at) \
                 VALUES ($1, $2, $3, now(), now()) \
                 ON CONFLICT (account_id, announcement_id, revision) DO UPDATE \
                 SET first_seen_at = COALESCE(platform.announcement_receipts.first_seen_at, now()), \
                     dismissed_at = COALESCE(platform.announcement_receipts.dismissed_at, now()) \
                 RETURNING revision, first_seen_at, dismissed_at, acknowledged_at",
            )
            .bind(account.id)
            .bind(announcement_id)
            .bind(input.revision)
            .fetch_one(&state.db)
            .await?
        }
        "acknowledge" => {
            sqlx::query_as::<_, ReceiptRow>(
                "INSERT INTO platform.announcement_receipts \
                 (account_id, announcement_id, revision, first_seen_at, acknowledged_at) \
                 VALUES ($1, $2, $3, now(), now()) \
                 ON CONFLICT (account_id, announcement_id, revision) DO UPDATE \
                 SET first_seen_at = COALESCE(platform.announcement_receipts.first_seen_at, now()), \
                     acknowledged_at = COALESCE(platform.announcement_receipts.acknowledged_at, now()) \
                 RETURNING revision, first_seen_at, dismissed_at, acknowledged_at",
            )
            .bind(account.id)
            .bind(announcement_id)
            .bind(input.revision)
            .fetch_one(&state.db)
            .await?
        }
        _ => return Err(AppError::BadRequest("invalid receipt action".into())),
    };
    Ok(Json(receipt_dto(&row)))
}

async fn admin_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<AnnouncementDto>>> {
    staff_account(&headers, &state, Capability::ManageAnnouncements).await?;
    let cursor = query.cursor.as_deref().map(|cursor| parse_id(cursor, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let rows = sqlx::query_as::<_, AnnouncementListRow>(
        "SELECT a.id, a.title, a.body, a.status, a.presentation, a.severity, a.priority, \
                a.audience, a.requires_ack, a.version, a.revision, a.starts_at, a.ends_at, \
                a.published_at, a.archived_at, a.created_at, a.updated_at, \
                NULL::timestamptz AS first_seen_at, NULL::timestamptz AS dismissed_at, \
                NULL::timestamptz AS acknowledged_at, \
                (SELECT count(*) FROM platform.announcement_receipts receipt \
                  WHERE receipt.announcement_id = a.id AND receipt.revision = a.revision \
                    AND receipt.first_seen_at IS NOT NULL) AS seen_count, \
                (SELECT count(*) FROM platform.announcement_receipts receipt \
                  WHERE receipt.announcement_id = a.id AND receipt.revision = a.revision \
                    AND receipt.dismissed_at IS NOT NULL) AS dismissed_count, \
                (SELECT count(*) FROM platform.announcement_receipts receipt \
                  WHERE receipt.announcement_id = a.id AND receipt.revision = a.revision \
                    AND receipt.acknowledged_at IS NOT NULL) AS acknowledged_count \
         FROM platform.announcements a \
         WHERE ($1::bigint IS NULL OR a.id < $1) ORDER BY a.id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor =
        has_more.then(|| visible.last().map(|row| row.announcement.id.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(list_row_dto).collect(), next_cursor)))
}

async fn admin_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<AnnouncementCreateInput>,
) -> AppResult<(StatusCode, Json<AnnouncementDto>)> {
    let account = staff_account(&headers, &state, Capability::ManageAnnouncements).await?;
    let now = Utc::now();
    let values = validate_fields(&input, now)?;
    let reason = reason(&input.reason)?;
    let published_at = (values.status == "published").then_some(now);
    let mut tx = state.db.begin().await?;
    let record = sqlx::query_as::<_, AnnouncementRecord>(
        "INSERT INTO platform.announcements \
         (title, body, status, presentation, severity, priority, audience, requires_ack, \
          starts_at, ends_at, published_at, created_by, updated_by, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13) \
         RETURNING id, title, body, status, presentation, severity, priority, audience, \
                   requires_ack, version, revision, starts_at, ends_at, published_at, archived_at, \
                   created_at, updated_at",
    )
    .bind(&values.title)
    .bind(&values.body)
    .bind(&values.status)
    .bind(&values.presentation)
    .bind(&values.severity)
    .bind(values.priority)
    .bind(&values.audience)
    .bind(values.requires_ack)
    .bind(values.starts_at)
    .bind(values.ends_at)
    .bind(published_at)
    .bind(account.id)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;
    insert_revision(&mut tx, &record, account.id).await?;
    let metadata = serde_json::json!({ "status": record.status, "revision": record.revision });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.announcement.created",
        "announcement",
        &record.id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(dto(record, None, None))))
}

async fn admin_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(announcement_id): Path<String>,
    Json(input): Json<AnnouncementUpdateInput>,
) -> AppResult<Json<AnnouncementDto>> {
    let account = staff_account(&headers, &state, Capability::ManageAnnouncements).await?;
    let announcement_id = parse_id(&announcement_id, "announcement id")?;
    if input.expected_version < 1 {
        return Err(AppError::BadRequest("expectedVersion must be positive".into()));
    }
    let create_shape = AnnouncementCreateInput {
        title: input.title,
        body: input.body,
        status: input.status,
        presentation: input.presentation,
        severity: input.severity,
        priority: input.priority,
        audience: input.audience,
        requires_ack: input.requires_ack,
        starts_at: input.starts_at,
        ends_at: input.ends_at,
        reason: input.reason,
    };
    let now = Utc::now();
    let values = validate_fields(&create_shape, now)?;
    let reason = reason(&create_shape.reason)?;
    let mut tx = state.db.begin().await?;
    let current = find_record(&mut tx, announcement_id).await?;
    if current.version != input.expected_version {
        return Err(AppError::Conflict("announcement was changed by another operator".into()));
    }
    if !transition_allowed(&current.status, &values.status) {
        return Err(AppError::Conflict(format!(
            "announcement cannot transition from {} to {}",
            current.status, values.status
        )));
    }
    let revision = current.revision + i64::from(input.bump_revision);
    let published_at =
        if values.status == "published" { current.published_at.or(Some(now)) } else { None };
    let record = sqlx::query_as::<_, AnnouncementRecord>(
        "UPDATE platform.announcements SET \
           title = $1, body = $2, status = $3, presentation = $4, severity = $5, priority = $6, \
           audience = $7, requires_ack = $8, starts_at = $9, ends_at = $10, published_at = $11, \
           archived_at = NULL, updated_by = $12, updated_at = $13, version = version + 1, \
           revision = $14 \
         WHERE id = $15 AND version = $16 \
         RETURNING id, title, body, status, presentation, severity, priority, audience, \
                   requires_ack, version, revision, starts_at, ends_at, published_at, archived_at, \
                   created_at, updated_at",
    )
    .bind(&values.title)
    .bind(&values.body)
    .bind(&values.status)
    .bind(&values.presentation)
    .bind(&values.severity)
    .bind(values.priority)
    .bind(&values.audience)
    .bind(values.requires_ack)
    .bind(values.starts_at)
    .bind(values.ends_at)
    .bind(published_at)
    .bind(account.id)
    .bind(now)
    .bind(revision)
    .bind(announcement_id)
    .bind(input.expected_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("announcement was changed by another operator".into()))?;
    insert_revision(&mut tx, &record, account.id).await?;
    let metadata = serde_json::json!({
        "oldStatus": current.status,
        "newStatus": record.status,
        "oldVersion": current.version,
        "newVersion": record.version,
        "revision": record.revision,
        "rePresented": input.bump_revision,
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.announcement.updated",
        "announcement",
        &announcement_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(dto(record, None, None)))
}

async fn admin_archive(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(announcement_id): Path<String>,
    Json(input): Json<ArchiveInput>,
) -> AppResult<StatusCode> {
    let account = staff_account(&headers, &state, Capability::ManageAnnouncements).await?;
    let announcement_id = parse_id(&announcement_id, "announcement id")?;
    let reason = reason(&input.reason)?;
    let mut tx = state.db.begin().await?;
    let current = find_record(&mut tx, announcement_id).await?;
    if current.version != input.expected_version {
        return Err(AppError::Conflict("announcement was changed by another operator".into()));
    }
    if current.status == "archived" {
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    let record = sqlx::query_as::<_, AnnouncementRecord>(
        "UPDATE platform.announcements SET status = 'archived', archived_at = now(), \
                updated_at = now(), updated_by = $1, version = version + 1 \
         WHERE id = $2 AND version = $3 \
         RETURNING id, title, body, status, presentation, severity, priority, audience, \
                   requires_ack, version, revision, starts_at, ends_at, published_at, archived_at, \
                   created_at, updated_at",
    )
    .bind(account.id)
    .bind(announcement_id)
    .bind(input.expected_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Conflict("announcement was changed by another operator".into()))?;
    insert_revision(&mut tx, &record, account.id).await?;
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: account.id, role: &account.role },
        "platform.announcement.archived",
        "announcement",
        &announcement_id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_revisions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(announcement_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Page<AnnouncementRevisionDto>>> {
    staff_account(&headers, &state, Capability::ManageAnnouncements).await?;
    let announcement_id = parse_id(&announcement_id, "announcement id")?;
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM platform.announcements WHERE id = $1)",
    )
    .bind(announcement_id)
    .fetch_one(&state.db)
    .await?;
    if !exists {
        return Err(AppError::NotFound);
    }
    let cursor = query.cursor.as_deref().map(|cursor| parse_id(cursor, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let rows = sqlx::query_as::<_, AnnouncementRevisionRow>(
        "SELECT announcement_id, version, revision, title, body, status, presentation, severity, \
                priority, audience, requires_ack, starts_at, ends_at, created_at \
         FROM platform.announcement_revisions \
         WHERE announcement_id = $1 AND ($2::bigint IS NULL OR version < $2) \
         ORDER BY version DESC LIMIT $3",
    )
    .bind(announcement_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible = rows.into_iter().take(limit as usize).collect::<Vec<_>>();
    let next_cursor = has_more.then(|| visible.last().map(|row| row.version.to_string())).flatten();
    Ok(Json(Page::new(visible.into_iter().map(revision_dto).collect(), next_cursor)))
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v2/announcements", get(list_active))
        .route("/api/v2/announcements/unread", get(list_unread))
        .route("/api/v2/announcements/{id}/receipt", post(record_receipt))
        .route("/api/v2/admin/announcements", get(admin_list).post(admin_create))
        .route("/api/v2/admin/announcements/{id}", patch(admin_update).delete(admin_archive))
        .route("/api/v2/admin/announcements/{id}/revisions", get(admin_revisions))
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use shared::AuthAccount;

    use super::{audience_allows, transition_allowed, AnnouncementRecord};

    fn record(status: &str, starts_at: Option<chrono::DateTime<Utc>>) -> AnnouncementRecord {
        let now = Utc::now();
        AnnouncementRecord {
            id: 1,
            title: "Notice".into(),
            body: None,
            status: status.into(),
            presentation: "card".into(),
            severity: "info".into(),
            priority: 0,
            audience: "all".into(),
            requires_ack: false,
            version: 1,
            revision: 1,
            starts_at,
            ends_at: None,
            published_at: None,
            archived_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn audience_policy_does_not_expose_staff_announcements_to_users() {
        let user = AuthAccount { id: 1, role: "user".into(), status: "active".into() };
        let moderator = AuthAccount { id: 2, role: "mod".into(), status: "active".into() };
        assert!(!audience_allows("staff", None));
        assert!(!audience_allows("staff", Some(&user)));
        assert!(audience_allows("staff", Some(&moderator)));
    }

    #[test]
    fn published_announcements_cannot_return_to_draft() {
        assert!(!transition_allowed("published", "draft"));
        assert!(transition_allowed("scheduled", "draft"));
    }

    #[test]
    fn future_scheduled_record_is_not_active() {
        let announcement = record("scheduled", Some(Utc::now() + Duration::hours(1)));
        assert!(!super::is_active(&announcement, Utc::now()));
    }
}

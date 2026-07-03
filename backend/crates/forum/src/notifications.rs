//! Notifications: list and mark-read for the current user.
//!
//! The backing table is `forum.notifications`.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};
use sqlx::FromRow;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// DB row
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct NotificationRow {
    pub id: i64,
    pub account_id: i64,
    pub r#type: String,
    pub payload: Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDto {
    pub id: String,
    pub r#type: String,
    pub payload: Value,
    pub read: bool,
    pub read_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkReadInput {
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationListQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

// ---------------------------------------------------------------------------
// Repo helpers
// ---------------------------------------------------------------------------

/// List notifications for an account, cursor-paginated by created_at DESC.
pub async fn list_notifications(
    pool: &PgPool,
    account_id: i64,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<NotificationRow>, Option<i64>)> {
    let rows = if let Some(cursor_id) = cursor {
        sqlx::query_as::<_, NotificationRow>(
            "SELECT id, account_id, type, payload, read_at, created_at \
             FROM forum.notifications \
             WHERE account_id = $1 AND id < $2 \
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(account_id)
        .bind(cursor_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, NotificationRow>(
            "SELECT id, account_id, type, payload, read_at, created_at \
             FROM forum.notifications \
             WHERE account_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(account_id)
        .bind(limit + 1)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let next_cursor = if has_more { rows.get(limit as usize).map(|r| r.id) } else { None };

    let truncated: Vec<NotificationRow> =
        if has_more { rows.into_iter().take(limit as usize).collect() } else { rows };

    Ok((truncated, next_cursor))
}

/// Mark notifications as read. Only touches notifications belonging to the
/// given account, silently skipping any `ids` that belong to another account.
pub async fn mark_read(pool: &PgPool, account_id: i64, notification_ids: &[i64]) -> AppResult<()> {
    if notification_ids.is_empty() {
        return Ok(());
    }

    // sqlx does not support array binding natively, so we build IN ($1, $2, ...).
    let placeholders: Vec<String> =
        notification_ids.iter().enumerate().map(|(i, _)| format!("${}", i + 2)).collect();

    let sql = format!(
        "UPDATE forum.notifications SET read_at = now() \
         WHERE account_id = $1 AND id IN ({}) AND read_at IS NULL",
        placeholders.join(", ")
    );

    let mut q = sqlx::query(&sql).bind(account_id);
    for id in notification_ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/notifications — auth required
pub async fn list_notifications_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<NotificationListQuery>,
) -> AppResult<Json<Page<NotificationDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let cursor_id: Option<i64> = q
        .cursor
        .as_deref()
        .map(|c| c.parse::<i64>().map_err(|_| AppError::BadRequest("invalid cursor".into())))
        .transpose()?;

    let (rows, next_cursor) = list_notifications(&state.db, auth.id, cursor_id, q.limit).await?;

    let items: Vec<NotificationDto> = rows
        .into_iter()
        .map(|r| NotificationDto {
            id: r.id.to_string(),
            r#type: r.r#type,
            payload: r.payload,
            read: r.read_at.is_some(),
            read_at: r.read_at.map(|t| t.timestamp()),
            created_at: r.created_at.timestamp(),
        })
        .collect();

    let next_str = next_cursor.map(|c| c.to_string());
    Ok(Json(Page::new(items, next_str)))
}

/// GET /api/v2/notifications/unread-count — auth required
pub async fn unread_count_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(auth.id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "count": count })))
}

/// POST /api/v2/notifications/read — auth required
pub async fn mark_read_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MarkReadInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let ids: Result<Vec<i64>, _> = body.ids.iter().map(|s| s.parse::<i64>()).collect();

    let ids = ids.map_err(|_| AppError::BadRequest("invalid notification id".into()))?;

    mark_read(&state.db, auth.id, &ids).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

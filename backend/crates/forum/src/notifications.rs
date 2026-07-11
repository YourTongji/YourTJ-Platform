//! Notifications: list and mark-read for the current user.
//!
//! The backing table is `forum.notifications`.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::pagination::Page;
use shared::{AppError, AppResult, AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDto {
    pub id: String,
    pub r#type: String,
    pub payload: Value,
    pub target_url: Option<String>,
    pub read: bool,
    pub read_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountDto {
    pub count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkReadInput {
    #[serde(default)]
    pub ids: Option<Vec<String>>,
    #[serde(default)]
    pub all: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationListQuery {
    pub cursor: Option<String>,
    #[serde(default)]
    pub unread: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

fn numeric_payload_id(payload: &Value, field: &str) -> Option<i64> {
    payload.get(field)?.as_str()?.parse().ok().filter(|id| *id > 0)
}

fn notification_target_url(notification_type: &str, payload: &Value) -> Option<String> {
    let explicit_target = payload.get("targetUrl").and_then(Value::as_str).filter(|target| {
        target.starts_with('/')
            && !target.starts_with("//")
            && !target.contains('\\')
            && !target.chars().any(char::is_control)
    });
    if let Some(target) = explicit_target {
        return Some(target.to_owned());
    }

    if matches!(notification_type, "dm" | "dm_request" | "dm_request_accepted") {
        return numeric_payload_id(payload, "conversationId").map(|id| {
            if notification_type == "dm_request" {
                format!("/messages?view=requests&conversation={id}")
            } else {
                format!("/messages?conversation={id}")
            }
        });
    }

    if let Some(thread_id) = numeric_payload_id(payload, "threadId") {
        return Some(format!("/forum/threads/{thread_id}"));
    }

    if notification_type == "vote"
        && payload.get("postType").and_then(Value::as_str) == Some("thread")
    {
        return numeric_payload_id(payload, "postId").map(|id| format!("/forum/threads/{id}"));
    }

    None
}

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

    if !(1..=100).contains(&q.limit) {
        return Err(AppError::BadRequest("limit must be between 1 and 100".into()));
    }

    let (rows, next_cursor) =
        crate::repo::list_notifications(&state.db, auth.id, cursor_id, q.unread, q.limit).await?;

    let items: Vec<NotificationDto> = rows
        .into_iter()
        .map(|row| NotificationDto {
            id: row.id.to_string(),
            target_url: notification_target_url(&row.r#type, &row.payload),
            r#type: row.r#type,
            payload: row.payload,
            read: row.read_at.is_some(),
            read_at: row.read_at.map(|timestamp| timestamp.timestamp()),
            created_at: row.created_at.timestamp(),
        })
        .collect();

    let next_str = next_cursor.map(|c| c.to_string());
    Ok(Json(Page::new(items, next_str)))
}

/// GET /api/v2/notifications/unread-count — auth required
pub async fn unread_count_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<UnreadCountDto>> {
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

    Ok(Json(UnreadCountDto { count }))
}

/// POST /api/v2/notifications/read — auth required.
pub async fn mark_read_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MarkReadInput>,
) -> AppResult<StatusCode> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    match (body.ids, body.all) {
        (Some(ids), None) => {
            if ids.is_empty() || ids.len() > 100 {
                return Err(AppError::BadRequest(
                    "ids must contain between 1 and 100 items".into(),
                ));
            }
            let parsed_ids = ids
                .iter()
                .map(|id| id.parse::<i64>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| AppError::BadRequest("invalid notification id".into()))?;
            if parsed_ids.iter().any(|id| *id <= 0) {
                return Err(AppError::BadRequest("invalid notification id".into()));
            }
            crate::repo::mark_read(&state.db, auth.id, &parsed_ids).await?;
        }
        (None, Some(true) | None) => {
            crate::repo::mark_all_read(&state.db, auth.id).await?;
        }
        (None, Some(false)) => return Err(AppError::BadRequest("all must be true".into())),
        (Some(_), Some(_)) => {
            return Err(AppError::BadRequest("all and ids cannot be combined".into()));
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::notification_target_url;

    #[test]
    fn builds_only_internal_notification_targets() {
        assert_eq!(
            notification_target_url("reply", &json!({ "threadId": "42" })),
            Some("/forum/threads/42".into())
        );
        assert_eq!(
            notification_target_url("dm", &json!({ "conversationId": "7" })),
            Some("/messages?conversation=7".into())
        );
        assert_eq!(
            notification_target_url("dm_request", &json!({ "conversationId": "8" })),
            Some("/messages?view=requests&conversation=8".into())
        );
        assert_eq!(
            notification_target_url("system", &json!({ "targetUrl": "/settings" })),
            Some("/settings".into())
        );
        assert_eq!(
            notification_target_url("system", &json!({ "targetUrl": "//attacker.example" })),
            None
        );
        assert_eq!(
            notification_target_url("system", &json!({ "targetUrl": "/\\attacker.example" })),
            None
        );
    }
}

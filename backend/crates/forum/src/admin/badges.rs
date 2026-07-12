//! Staff thread featuring and its forum-specific achievement trigger.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeatureThreadInput {
    featured: bool,
    reason: String,
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
}

/// Feature or unfeature a thread with a reasoned moderation event.
pub async fn feature_thread(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<FeatureThreadInput>,
) -> AppResult<Json<Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let thread_id = id.parse::<i64>().map_err(|_| AppError::NotFound)?;
    let reason = validate_reason(&body.reason)?;
    let mut transaction = state.db.begin().await?;
    let thread: (i64, i64) =
        sqlx::query_as("SELECT author_id, board_id FROM forum.threads WHERE id = $1 FOR UPDATE")
            .bind(thread_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(AppError::NotFound)?;
    super::require_lower_author_role(&mut transaction, &auth, Some(thread.0)).await?;

    let action = if body.featured { "feature" } else { "unfeature" };
    let newly_featured = if body.featured {
        sqlx::query(
            "UPDATE forum.threads SET featured_at = now() \
             WHERE id = $1 AND featured_at IS NULL",
        )
        .bind(thread_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            == 1
    } else {
        sqlx::query("UPDATE forum.threads SET featured_at = NULL WHERE id = $1")
            .bind(thread_id)
            .execute(&mut *transaction)
            .await?;
        false
    };

    crate::repo::insert_mod_action(
        &mut *transaction,
        auth.id,
        action,
        "thread",
        thread_id,
        Some(reason),
        None,
    )
    .await?;
    governance::record_account_event_tx(
        &mut transaction,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        &format!("forum.thread.{action}"),
        "thread",
        &thread_id.to_string(),
        reason,
        None,
    )
    .await?;
    if newly_featured {
        platform::outbox::enqueue_achievement_award_tx(
            &mut transaction,
            &format!("forum-thread:{thread_id}:achievement:quality-author"),
            thread.0,
            auth.id,
            "quality-author",
            "staff featured a forum thread",
        )
        .await?;
    }
    transaction.commit().await?;

    crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), thread_id, thread.1).await;
    Ok(Json(json!({ "ok": true })))
}

//! User-facing forum report submission and automatic safety thresholds.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppError, AppResult, AppState};
use sqlx::PgConnection;

const AUTO_SILENCE_REASON: &str = "two content auto-hides within 24 hours";

async fn apply_auto_silence(connection: &mut PgConnection, account_id: i64) -> AppResult<bool> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("forum:auto_silence:{account_id}"))
        .execute(&mut *connection)
        .await?;

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.flags flag \
         WHERE flag.auto_hidden_at > now() - interval '24 hours' \
           AND ( \
             (flag.target_type = 'thread' AND EXISTS ( \
               SELECT 1 FROM forum.threads WHERE id = flag.target_id AND author_id = $1 \
             )) OR \
             (flag.target_type = 'comment' AND EXISTS ( \
               SELECT 1 FROM forum.comments WHERE id = flag.target_id AND author_id = $1 \
             )) \
           )",
    )
    .bind(account_id)
    .fetch_one(&mut *connection)
    .await?;
    if count < 2 {
        return Ok(false);
    }

    let metadata = serde_json::json!({ "durationHours": 24, "autoHideCount": count });
    identity::sanctions::issue_system_silence_tx(
        connection,
        account_id,
        AUTO_SILENCE_REASON,
        chrono::Utc::now() + chrono::Duration::hours(24),
        Some(&metadata),
    )
    .await
}

fn validate_input(body: &crate::dto::FlagInput) -> AppResult<(&str, Option<&str>, &str)> {
    if !matches!(body.reason.as_str(), "spam" | "abuse" | "off_topic" | "illegal" | "other") {
        return Err(AppError::BadRequest("invalid flag reason".into()));
    }
    let note = body.note.as_deref().map(str::trim).filter(|note| !note.is_empty());
    if note.is_some_and(|note| note.chars().count() > 1000) {
        return Err(AppError::BadRequest("flag note must not exceed 1000 characters".into()));
    }
    if !matches!(body.post_type.as_str(), "thread" | "comment") {
        return Err(AppError::BadRequest("postType must be thread/comment".into()));
    }
    Ok((&body.reason, note, &body.post_type))
}

/// POST /api/v2/forum/posts/{id}/flag.
pub async fn flag_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::FlagInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    if matches!(auth.role.as_str(), "mod" | "admin") {
        return Err(AppError::Forbidden);
    }
    let trust_level = crate::trust_levels::get_trust_level(&state.db, auth.id).await?;
    let (bucket, capacity) = if trust_level == 0 { ("flag_tl0", 5) } else { ("flag", 15) };
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        bucket,
        &auth.id.to_string(),
        capacity,
        86_400,
    )
    .await?;

    let post_id: i64 = id.parse().map_err(|_| AppError::NotFound)?;
    let (reason, note, target_type) = validate_input(&body)?;
    let weight = match trust_level {
        0 => 0.5,
        1 => 1.0,
        2 => 1.5,
        3 => 2.0,
        _ => 1.0,
    };

    let mut tx = state.db.begin().await?;
    let outcome =
        crate::repo::insert_flag(&mut tx, target_type, post_id, auth.id, reason, note, weight)
            .await?;
    let auto_silenced = if outcome.auto_hidden {
        if let Some(author_id) = outcome.author_id {
            apply_auto_silence(&mut tx, author_id).await?
        } else {
            false
        }
    } else {
        false
    };
    tx.commit().await?;

    if outcome.auto_hidden {
        crate::cache::invalidate_thread_surfaces(
            state.redis.as_ref(),
            outcome.thread_id,
            outcome.board_id,
        )
        .await;
        crate::meili::reconcile_thread_in_background(&state, outcome.thread_id);
        if let Some(author_id) = outcome.author_id.filter(|_| auto_silenced) {
            identity::sanctions::invalidate_silence_cache(state.redis.as_ref(), author_id).await;
        }
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "autoHidden": outcome.auto_hidden,
        "autoSilenced": auto_silenced,
    })))
}

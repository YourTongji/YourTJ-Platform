//! User-facing flag handler.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppError, AppResult, AppState};

/// POST /api/v2/forum/posts/{id}/flag
pub async fn flag_post(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
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

    let tl = crate::trust_levels::get_trust_level(state.redis.as_ref(), &state.db, auth.id).await?;
    if tl == 0 {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "flag_tl0",
            &auth.id.to_string(),
            5,
            86400,
        )
        .await?;
    } else {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "flag",
            &auth.id.to_string(),
            15,
            86400,
        )
        .await?;
    }

    let post_id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Get reporter's trust level for weight
    let weight: f32 = match auth.role.as_str() {
        "admin" | "mod" => 3.0,
        _ => {
            let tl: i16 = sqlx::query_scalar(
                "SELECT COALESCE(trust_level, 0) FROM identity.accounts WHERE id = $1",
            )
            .bind(auth.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
            match tl {
                0 => 0.5,
                1 => 1.0,
                2 => 1.5,
                3 => 2.0,
                _ => 1.0,
            }
        }
    };

    let threshold_reached = crate::repo::insert_flag(
        &state.db,
        &body.post_type,
        post_id,
        auth.id,
        &body.reason,
        body.note.as_deref(),
        weight,
    )
    .await?;

    if threshold_reached {
        // Notify target author about auto-hide (fire-and-forget)
        let pool = state.db.clone();
        let post_type = body.post_type.clone();
        let flag_post_id = post_id;
        tokio::spawn(async move {
            let author_id: Option<i64> = sqlx::query_scalar(if post_type == "thread" {
                "SELECT author_id FROM forum.threads WHERE id = $1"
            } else {
                "SELECT author_id FROM forum.comments WHERE id = $1"
            })
            .bind(flag_post_id)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();

            if let Some(target_author_id) = author_id {
                crate::notification_hooks::create_notification(
                    &pool,
                    target_author_id,
                    "flag_auto_hide",
                    serde_json::json!({
                        "postType": post_type,
                        "postId": flag_post_id.to_string(),
                    }),
                    None,
                )
                .await;
            }
        });
    }

    Ok(Json(serde_json::json!({"ok": true, "autoHidden": threshold_reached})))
}

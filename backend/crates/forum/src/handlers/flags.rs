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

    // Derive post_type by checking which table the ID exists in
    let exists_thread: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.threads WHERE id = $1)")
            .bind(post_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(false);

    let target_type = if exists_thread { "thread" } else { "comment" };

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

    let (threshold_reached, author_id) = crate::repo::insert_flag(
        &state.db,
        target_type,
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
        let post_type = target_type.to_string();
        let flag_post_id = post_id;
        if let Some(target_author_id) = author_id {
            tokio::spawn(async move {
                crate::notification_hooks::create_notification(
                    &pool,
                    target_author_id,
                    "flag_auto_hide",
                    serde_json::json!({
                        "postType": post_type,
                        "postId": flag_post_id.to_string(),
                    }),
                    None,
                    None,
                )
                .await;
            });

            // Auto-silence check (G3): if author has ≥2 auto-hides in 24h, silence them.
            let pool = state.db.clone();
            let redis_pool = state.redis.clone();
            let silenced_author_id = target_author_id;
            tokio::spawn(async move {
                // Skip auto-silence for mods/admins
                let target_role: Option<String> =
                    sqlx::query_scalar("SELECT role FROM identity.accounts WHERE id = $1")
                        .bind(silenced_author_id)
                        .fetch_optional(&pool)
                        .await
                        .unwrap_or(None);
                if matches!(target_role.as_deref(), Some("mod") | Some("admin")) {
                    return;
                }

                // Count recent auto-hides (including this one since the INSERT already ran)
                let count = match crate::repo::count_recent_auto_hides(&pool, silenced_author_id)
                    .await
                {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!(error = %e, author_id = silenced_author_id, "auto-silence count failed");
                        return;
                    }
                };

                if count >= 2 {
                    // Check if already silenced
                    let already_silenced: bool = sqlx::query_scalar(
                        "SELECT EXISTS( \
                         SELECT 1 FROM identity.sanctions \
                         WHERE account_id = $1 AND kind = 'silence' \
                         AND revoked_at IS NULL \
                         AND (ends_at IS NULL OR ends_at > now()) \
                        )",
                    )
                    .bind(silenced_author_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(false);

                    if already_silenced {
                        return;
                    }

                    // Insert silence sanction (24h, issued_by = 0 for system)
                    let _ = sqlx::query(
                        "INSERT INTO identity.sanctions \
                         (account_id, kind, reason, issued_by, ends_at) \
                         VALUES ($1, 'silence', 'auto-silence: ≥2 auto-hides in 24h', 0, now() + interval '24 hours')",
                    )
                    .bind(silenced_author_id)
                    .execute(&pool)
                    .await;

                    // Log mod_action (actor = 0 for system)
                    let _ = crate::repo::insert_mod_action(
                        &pool,
                        0,
                        "auto_silence",
                        "account",
                        silenced_author_id,
                        Some("自动禁言：24小时内被自动隐藏≥2次"),
                        None,
                    )
                    .await;

                    // Invalidate Redis cache (best-effort)
                    if let Some(ref rp) = redis_pool {
                        if let Ok(mut conn) = rp.get().await {
                            let _: () = redis::cmd("DEL")
                                .arg(format!("identity:sanction:{silenced_author_id}"))
                                .query_async(&mut *conn)
                                .await
                                .unwrap_or(());
                        }
                    }

                    // Notify the silenced user
                    crate::notification_hooks::create_notification(
                        &pool,
                        silenced_author_id,
                        "mod_action",
                        serde_json::json!({
                            "action": "auto_silence",
                            "reason": "24小时内内容被自动隐藏达到2次",
                            "duration": "24h",
                        }),
                        None,
                        None,
                    )
                    .await;
                }
            });
        }
    }

    Ok(Json(serde_json::json!({"ok": true, "autoHidden": threshold_reached})))
}

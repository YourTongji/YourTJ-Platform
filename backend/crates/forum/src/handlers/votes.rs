use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppError, AppResult, AppState};

use crate::repo;

/// POST /api/v2/forum/posts/{post_id}/vote — auth required
pub async fn vote_post(
    State(state): State<AppState>,
    Path(post_id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<crate::dto::VoteInput>,
) -> AppResult<Json<serde_json::Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_r| AppError::Unauthorized)?;

    let tl = crate::trust_levels::get_trust_level(state.redis.as_ref(), &state.db, auth.id).await?;
    if tl == 0 {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "vote_tl0",
            &auth.id.to_string(),
            30,
            60,
        )
        .await?;
    } else {
        shared::ratelimit::check_token_bucket(
            state.redis.as_ref(),
            "vote",
            &auth.id.to_string(),
            60,
            60,
        )
        .await?;
    }

    let post_id: i64 = post_id_str.parse().map_err(|_| AppError::NotFound)?;

    // The URL carries only the post id; `postType` is optional per the contract.
    // Trust it when it is a valid value, otherwise infer whether the id is a
    // thread or a comment — returning 404 when it is neither.
    let post_type = match body.post_type.as_deref() {
        Some("thread") => "thread".to_string(),
        Some("comment") => "comment".to_string(),
        _ => {
            let is_thread: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.threads WHERE id = $1)")
                    .bind(post_id)
                    .fetch_one(&state.db)
                    .await?;
            if is_thread {
                "thread".to_string()
            } else {
                let is_comment: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM forum.comments WHERE id = $1)")
                        .bind(post_id)
                        .fetch_one(&state.db)
                        .await?;
                if is_comment {
                    "comment".to_string()
                } else {
                    return Err(AppError::NotFound);
                }
            }
        }
    };

    let vote_count = repo::vote_post(&state.db, &post_type, post_id, auth.id, &body.value).await?;

    // Update user_stats: votes_cast +1 (the voter) — best-effort.
    let _ = sqlx::query(
        "INSERT INTO forum.user_stats (account_id, votes_cast) \
         VALUES ($1, 1) \
         ON CONFLICT (account_id) \
         DO UPDATE SET votes_cast = forum.user_stats.votes_cast + 1",
    )
    .bind(auth.id)
    .execute(&state.db)
    .await;

    // Look up post author for notification.
    let post_author_id: Option<i64> = sqlx::query_scalar(if post_type == "thread" {
        "SELECT author_id FROM forum.threads WHERE id = $1"
    } else {
        "SELECT author_id FROM forum.comments WHERE id = $1"
    })
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?;

    // Notify post author of upvote (fire-and-forget).
    if let Some(author_id) = post_author_id {
        if author_id != auth.id && body.value == "up" {
            let pool = state.db.clone();
            let vote_type = post_type.clone();
            let vote_post_id = post_id;
            let voter_id = auth.id;
            tokio::spawn(async move {
                crate::notification_hooks::create_notification(
                    &pool,
                    author_id,
                    "vote",
                    serde_json::json!({
                        "postType": vote_type,
                        "postId": vote_post_id.to_string(),
                        "voterHandle": "",
                    }),
                    None,
                    Some(voter_id),
                )
                .await;
            });
        }
    }

    // Update user_stats: votes_received +1 (the post author) — best-effort.
    if let Some(author_id) = post_author_id {
        if author_id != auth.id {
            let _ = sqlx::query(
                "INSERT INTO forum.user_stats (account_id, votes_received) \
                 VALUES ($1, 1) \
                 ON CONFLICT (account_id) \
                 DO UPDATE SET votes_received = forum.user_stats.votes_received + 1",
            )
            .bind(author_id)
            .execute(&state.db)
            .await;
        }
    }

    // Bump board cache version.
    shared::cache::bump_version_opt(state.redis.as_ref(), "board", &post_id.to_string()).await.ok();

    Ok(Json(serde_json::json!({"ok": true, "vote_count": vote_count})))
}

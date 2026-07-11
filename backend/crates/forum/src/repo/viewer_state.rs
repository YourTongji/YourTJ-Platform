//! Viewer-specific interaction projections for public forum posts.

use std::collections::HashMap;

use shared::AppResult;
use sqlx::PgPool;

/// Active interaction state for one post and one account.
#[derive(Debug, Clone, Default)]
pub struct PostViewerState {
    pub vote: Option<String>,
    pub is_bookmarked: bool,
}

/// Batch viewer votes and bookmarks without making list handlers issue N+1 queries.
pub async fn get_post_viewer_states(
    pool: &PgPool,
    account_id: i64,
    post_type: &str,
    post_ids: &[i64],
) -> AppResult<HashMap<i64, PostViewerState>> {
    if !matches!(post_type, "thread" | "comment") {
        return Err(shared::AppError::BadRequest("post type must be thread/comment".into()));
    }
    if post_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut states = post_ids
        .iter()
        .copied()
        .map(|post_id| (post_id, PostViewerState::default()))
        .collect::<HashMap<_, _>>();
    let votes: Vec<(i64, i16)> = sqlx::query_as(
        "SELECT post_id, value FROM forum.votes \
         WHERE account_id = $1 AND post_type = $2 AND post_id = ANY($3)",
    )
    .bind(account_id)
    .bind(post_type)
    .bind(post_ids)
    .fetch_all(pool)
    .await?;
    for (post_id, value) in votes {
        if let Some(state) = states.get_mut(&post_id) {
            state.vote = match value {
                1 => Some("up".to_owned()),
                -1 => Some("down".to_owned()),
                _ => None,
            };
        }
    }

    let bookmarked_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT target_id FROM forum.bookmarks \
         WHERE account_id = $1 AND target_type = $2 AND target_id = ANY($3)",
    )
    .bind(account_id)
    .bind(post_type)
    .bind(post_ids)
    .fetch_all(pool)
    .await?;
    for post_id in bookmarked_ids {
        if let Some(state) = states.get_mut(&post_id) {
            state.is_bookmarked = true;
        }
    }

    Ok(states)
}

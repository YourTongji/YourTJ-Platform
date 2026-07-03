//! Public tag listing handler.

use axum::extract::State;
use axum::Json;
use shared::{AppResult, AppState};

/// GET /api/v2/forum/tags — public
pub async fn list_tags_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::dto::TagDto>>> {
    let rows = crate::repo::list_tags(&state.db).await?;
    let items: Vec<crate::dto::TagDto> = rows
        .into_iter()
        .map(|r| crate::dto::TagDto {
            id: r.id.to_string(),
            slug: r.slug,
            name: r.name,
            description: r.description,
            thread_count: r.thread_count,
            created_at: r.created_at.timestamp(),
        })
        .collect();
    Ok(Json(items))
}

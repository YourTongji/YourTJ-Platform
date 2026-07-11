use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use shared::{AppResult, AppState};

use crate::dto::BoardDto;
use crate::repo;

use super::board_to_dto;

/// GET /api/v2/forum/boards — public
pub async fn list_boards(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<BoardDto>>> {
    let account = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .ok();
    let actor = if let Some(account) = account {
        Some(repo::boards::BoardPostingActor {
            trust_level: crate::trust_levels::get_trust_level(&state.db, account.id).await?,
            can_bypass_board_gates: account
                .has_capability(shared::auth::Capability::ModerateContent),
        })
    } else {
        None
    };
    let rows = repo::list_boards(&state.db).await?;
    Ok(Json(rows.iter().map(|row| board_to_dto(row, actor)).collect()))
}

use axum::extract::State;
use axum::Json;
use shared::{AppResult, AppState};

use crate::dto::BoardDto;
use crate::repo;

use super::board_to_dto;

/// GET /api/v2/forum/boards — public
pub async fn list_boards(State(state): State<AppState>) -> AppResult<Json<Vec<BoardDto>>> {
    let rows = repo::list_boards(&state.db).await?;
    Ok(Json(rows.iter().map(board_to_dto).collect()))
}

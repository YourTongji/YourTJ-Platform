use shared::{AppError, AppResult};
use sqlx::{PgConnection, PgPool};

use crate::models::BoardRow;

/// Viewer facts used to enforce a board's posting gates.
#[derive(Debug, Clone, Copy)]
pub struct BoardPostingActor {
    pub trust_level: i16,
    pub can_bypass_board_gates: bool,
}

/// The first board gate preventing a viewer from creating a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoardPostingRestriction {
    LoginRequired,
    BoardLocked,
    TrustLevel,
}

impl BoardPostingRestriction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LoginRequired => "login_required",
            Self::BoardLocked => "board_locked",
            Self::TrustLevel => "trust_level",
        }
    }
}

/// Evaluate posting access from canonical board fields and viewer facts.
pub(crate) fn posting_restriction(
    board: &BoardRow,
    actor: Option<BoardPostingActor>,
) -> Option<BoardPostingRestriction> {
    let actor = match actor {
        Some(actor) => actor,
        None => return Some(BoardPostingRestriction::LoginRequired),
    };
    if actor.can_bypass_board_gates {
        return None;
    }
    if board.is_locked {
        return Some(BoardPostingRestriction::BoardLocked);
    }
    if actor.trust_level < board.min_trust_to_post {
        return Some(BoardPostingRestriction::TrustLevel);
    }
    None
}

fn require_posting_access(board: &BoardRow, actor: BoardPostingActor) -> AppResult<()> {
    match posting_restriction(board, Some(actor)) {
        None => Ok(()),
        Some(BoardPostingRestriction::BoardLocked) => Err(AppError::Forbidden),
        Some(BoardPostingRestriction::TrustLevel) => Err(AppError::Forbidden),
        Some(BoardPostingRestriction::LoginRequired) => Err(AppError::Unauthorized),
    }
}

/// List all boards.
pub async fn list_boards(pool: &PgPool) -> AppResult<Vec<BoardRow>> {
    let rows = sqlx::query_as::<_, BoardRow>(
        "SELECT id, slug, name, parent_id, description, position, is_locked, \
                is_qa, min_trust_to_post, thread_count \
         FROM forum.boards ORDER BY position, id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find a single board by id.
pub async fn find_board(pool: &PgPool, id: i64) -> AppResult<Option<BoardRow>> {
    let row = sqlx::query_as::<_, BoardRow>(
        "SELECT id, slug, name, parent_id, description, position, is_locked, \
                is_qa, min_trust_to_post, thread_count \
         FROM forum.boards WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Check board posting policy before consuming rate-limit capacity.
pub(crate) async fn authorize_board_posting(
    pool: &PgPool,
    board_id: i64,
    actor: BoardPostingActor,
) -> AppResult<()> {
    let board = find_board(pool, board_id).await?.ok_or(AppError::NotFound)?;
    require_posting_access(&board, actor)
}

/// Lock a board and enforce its current posting gates in the write transaction.
pub(crate) async fn lock_board_for_posting(
    connection: &mut PgConnection,
    board_id: i64,
    actor: BoardPostingActor,
) -> AppResult<Vec<i64>> {
    let board = sqlx::query_as::<_, BoardRow>(
        "SELECT id, slug, name, parent_id, description, position, is_locked, \
                is_qa, min_trust_to_post, thread_count \
         FROM forum.boards WHERE id = $1 FOR UPDATE",
    )
    .bind(board_id)
    .fetch_optional(connection)
    .await?
    .ok_or(AppError::NotFound)?;
    require_posting_access(&board, actor)?;
    Ok(vec![board.id])
}

/// Lock boards in a stable order before a thread visibility transition updates their counters.
pub(crate) async fn lock_boards_for_thread_count(
    connection: &mut PgConnection,
    board_ids: &[i64],
) -> AppResult<Vec<i64>> {
    let mut unique_ids = board_ids.to_vec();
    unique_ids.sort_unstable();
    unique_ids.dedup();
    if unique_ids.is_empty() {
        return Ok(unique_ids);
    }

    let locked_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM forum.boards WHERE id = ANY($1) ORDER BY id FOR UPDATE")
            .bind(&unique_ids)
            .fetch_all(connection)
            .await?;
    if locked_ids.len() != unique_ids.len() {
        return Err(AppError::NotFound);
    }
    Ok(unique_ids)
}

/// Recalculate visible thread counters for boards already locked by the caller.
pub(crate) async fn refresh_board_thread_counts(
    connection: &mut PgConnection,
    board_ids: &[i64],
) -> AppResult<()> {
    if board_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE forum.boards board SET thread_count = ( \
           SELECT COUNT(*)::int FROM forum.threads thread \
           WHERE thread.board_id = board.id AND thread.status = 'visible' \
             AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
             AND thread.archived_at IS NULL \
         ) WHERE board.id = ANY($1)",
    )
    .bind(board_ids)
    .execute(connection)
    .await?;
    Ok(())
}

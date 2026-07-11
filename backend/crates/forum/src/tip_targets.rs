//! Read-only forum target resolution for controlled credit tips.

use shared::AppResult;
use sqlx::PgConnection;

/// A visible forum target and its author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipTarget {
    pub canonical_type: &'static str,
    pub canonical_id: i64,
    pub author_id: i64,
}

/// Resolve a visible, non-archived forum target while holding share locks until
/// the caller's transaction commits.
pub async fn resolve_tip_target(
    conn: &mut PgConnection,
    target_type: &str,
    target_id: i64,
) -> AppResult<Option<TipTarget>> {
    let author_id = match target_type {
        "thread" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT thread.author_id \
                 FROM forum.threads thread \
                 WHERE thread.id = $1 AND thread.status = 'visible' \
                   AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
                   AND thread.archived_at IS NULL AND thread.author_id IS NOT NULL \
                 FOR SHARE OF thread",
            )
            .bind(target_id)
            .fetch_optional(&mut *conn)
            .await?
        }
        "comment" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT comment.author_id \
                 FROM forum.comments comment \
                 JOIN forum.threads thread ON thread.id = comment.thread_id \
                 WHERE comment.id = $1 \
                   AND comment.deleted_at IS NULL AND comment.hidden_at IS NULL \
                   AND thread.status = 'visible' AND thread.deleted_at IS NULL \
                   AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
                   AND comment.author_id IS NOT NULL \
                 FOR SHARE OF comment, thread",
            )
            .bind(target_id)
            .fetch_optional(&mut *conn)
            .await?
        }
        _ => return Ok(None),
    };

    Ok(author_id.map(|author_id| TipTarget {
        canonical_type: if target_type == "thread" { "thread" } else { "comment" },
        canonical_id: target_id,
        author_id,
    }))
}

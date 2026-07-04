//! Flag (report) CRUD, threshold checks, and admin resolution.

use crate::models::FlagRow;
use shared::AppResult;
use sqlx::PgPool;

/// Insert a flag.
///
/// Returns `Ok((true, Some(author_id)))` when threshold (>= 3.0) is reached,
/// `Ok((false, None))` otherwise. The `author_id` is the target content author.
pub async fn insert_flag(
    pool: &PgPool,
    target_type: &str,
    target_id: i64,
    reporter_id: i64,
    reason: &str,
    note: Option<&str>,
    weight: f32,
) -> AppResult<(bool, Option<i64>)> {
    // UPSERT — one vote per user
    sqlx::query(
        "INSERT INTO forum.flags (target_type, target_id, reporter_id, reason, note, weight) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (target_type, target_id, reporter_id) \
         DO UPDATE SET reason = EXCLUDED.reason, note = EXCLUDED.note, \
                        weight = EXCLUDED.weight, status = 'open'",
    )
    .bind(target_type)
    .bind(target_id)
    .bind(reporter_id)
    .bind(reason)
    .bind(note)
    .bind(weight)
    .execute(pool)
    .await?;

    // Compute weighted score
    let score: Option<f32> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(weight), 0.0) FROM forum.flags \
         WHERE target_type = $1 AND target_id = $2 AND status = 'open'",
    )
    .bind(target_type)
    .bind(target_id)
    .fetch_one(pool)
    .await?;

    let threshold_reached = score.map(|s| s >= 3.0).unwrap_or(false);

    let author_id: Option<i64> = if threshold_reached {
        // Auto-hide the target
        if target_type == "thread" {
            sqlx::query(
                "UPDATE forum.threads SET hidden_at = now() WHERE id = $1 AND hidden_at IS NULL",
            )
            .bind(target_id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE forum.comments SET hidden_at = now() WHERE id = $1 AND hidden_at IS NULL",
            )
            .bind(target_id)
            .execute(pool)
            .await?;
        }

        // Look up the author of the hidden content
        let aid: Option<i64> = if target_type == "thread" {
            sqlx::query_scalar("SELECT author_id FROM forum.threads WHERE id = $1")
                .bind(target_id)
                .fetch_optional(pool)
                .await?
                .flatten()
        } else {
            sqlx::query_scalar("SELECT author_id FROM forum.comments WHERE id = $1")
                .bind(target_id)
                .fetch_optional(pool)
                .await?
                .flatten()
        };
        aid
    } else {
        None
    };

    Ok((threshold_reached, author_id))
}

/// Count how many times an author's content has been auto-hidden in the last 24 hours.
pub async fn count_recent_auto_hides(pool: &PgPool, author_id: i64) -> AppResult<i64> {
    let thread_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.threads \
         WHERE author_id = $1 AND hidden_at IS NOT NULL \
         AND hidden_at > now() - interval '24 hours'",
    )
    .bind(author_id)
    .fetch_one(pool)
    .await?;

    let comment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.comments \
         WHERE author_id = $1 AND hidden_at IS NOT NULL \
         AND hidden_at > now() - interval '24 hours'",
    )
    .bind(author_id)
    .fetch_one(pool)
    .await?;

    Ok(thread_count + comment_count)
}

/// List flags grouped by target, with weighted score.
pub async fn list_flag_queue(
    pool: &PgPool,
    status: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<(Vec<FlagRow>, Option<i64>)> {
    let since_id = cursor.unwrap_or(0);
    let status_filter = status.unwrap_or("open");

    let rows: Vec<FlagRow> = sqlx::query_as(
        "SELECT id, target_type, target_id, reporter_id, reason, note, weight, status, \
                handled_by, handled_at, created_at \
         FROM forum.flags \
         WHERE status = $1 AND id > $2 \
         ORDER BY id ASC \
         LIMIT $3",
    )
    .bind(status_filter)
    .bind(since_id)
    .bind(limit + 1)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = items.last().map(|r| r.id);

    Ok((items, next_cursor))
}

/// Resolve a flag.
///
/// `_note` is reserved for future mod-action logging.
pub async fn resolve_flag(
    pool: &PgPool,
    flag_id: i64,
    action: &str,
    handled_by: i64,
    _note: Option<&str>,
) -> AppResult<()> {
    // Get the flag to find target
    let flag: Option<FlagRow> = sqlx::query_as(
        "SELECT id, target_type, target_id, reporter_id, reason, note, weight, status, \
                handled_by, handled_at, created_at \
         FROM forum.flags WHERE id = $1",
    )
    .bind(flag_id)
    .fetch_optional(pool)
    .await?;

    let flag = flag.ok_or(shared::AppError::NotFound)?;

    match action {
        "uphold" => {
            // Soft-delete target
            if flag.target_type == "thread" {
                sqlx::query(
                    "UPDATE forum.threads SET deleted_at = now(), deleted_by = $1 WHERE id = $2",
                )
                .bind(handled_by)
                .bind(flag.target_id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE forum.comments SET deleted_at = now(), deleted_by = $1 WHERE id = $2",
                )
                .bind(handled_by)
                .bind(flag.target_id)
                .execute(pool)
                .await?;
            }

            // Increment flags_upheld on reporter
            sqlx::query(
                "UPDATE forum.user_stats SET flags_upheld = flags_upheld + 1 WHERE account_id = $1",
            )
            .bind(flag.reporter_id)
            .execute(pool)
            .await
            .ok();

            // Increment flagged_upheld on the target content's author
            let target_author: Option<i64> = if flag.target_type == "thread" {
                sqlx::query_scalar("SELECT author_id FROM forum.threads WHERE id = $1")
                    .bind(flag.target_id)
                    .fetch_optional(pool)
                    .await?
                    .flatten()
            } else {
                sqlx::query_scalar("SELECT author_id FROM forum.comments WHERE id = $1")
                    .bind(flag.target_id)
                    .fetch_optional(pool)
                    .await?
                    .flatten()
            };

            if let Some(author_id) = target_author {
                sqlx::query(
                    "INSERT INTO forum.user_stats (account_id, flagged_upheld) \
                     VALUES ($1, 1) \
                     ON CONFLICT (account_id) \
                     DO UPDATE SET flagged_upheld = forum.user_stats.flagged_upheld + 1",
                )
                .bind(author_id)
                .execute(pool)
                .await?;
            }
        }
        "reject" => {
            // Clear hidden_at if it was auto-hidden
            if flag.target_type == "thread" {
                sqlx::query("UPDATE forum.threads SET hidden_at = NULL WHERE id = $1")
                    .bind(flag.target_id)
                    .execute(pool)
                    .await?;
            } else {
                sqlx::query("UPDATE forum.comments SET hidden_at = NULL WHERE id = $1")
                    .bind(flag.target_id)
                    .execute(pool)
                    .await?;
            }
            // Clear all open flags on this target
            sqlx::query(
                "UPDATE forum.flags SET status = 'rejected', handled_by = $1, handled_at = now() \
                 WHERE target_type = $2 AND target_id = $3 AND status = 'open'",
            )
            .bind(handled_by)
            .bind(&flag.target_type)
            .bind(flag.target_id)
            .execute(pool)
            .await?;
            return Ok(());
        }
        "ignore" => {
            // Clear hidden
            if flag.target_type == "thread" {
                sqlx::query("UPDATE forum.threads SET hidden_at = NULL WHERE id = $1")
                    .bind(flag.target_id)
                    .execute(pool)
                    .await?;
            } else {
                sqlx::query("UPDATE forum.comments SET hidden_at = NULL WHERE id = $1")
                    .bind(flag.target_id)
                    .execute(pool)
                    .await?;
            }
        }
        _ => {
            return Err(shared::AppError::BadRequest("action must be uphold/reject/ignore".into()))
        }
    }

    // Update flag status
    sqlx::query(
        "UPDATE forum.flags SET status = $1, handled_by = $2, handled_at = now() WHERE id = $3",
    )
    .bind(action)
    .bind(handled_by)
    .bind(flag_id)
    .execute(pool)
    .await?;

    Ok(())
}

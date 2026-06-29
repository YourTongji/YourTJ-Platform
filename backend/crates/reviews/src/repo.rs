//! Database access layer for the reviews domain.
//!
//! Every function takes `&PgPool` and returns `AppResult` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use shared::pagination::Page;
use shared::AppResult;
use sqlx::PgPool;

use crate::dto::ReviewDto;
use crate::error::ReviewsError;
use crate::models::{ReviewReportRow, ReviewRow, ReviewWithAuthorRow};

/// Default page size for list queries.
const DEFAULT_LIMIT: i64 = 20;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate rating is in the allowed range.
fn check_rating(rating: i32) -> Result<(), ReviewsError> {
    if !(0..=5).contains(&rating) {
        return Err(ReviewsError::InvalidRating);
    }
    Ok(())
}

/// Build a ReviewDto from a joined row.
fn row_to_dto(row: &ReviewWithAuthorRow) -> ReviewDto {
    ReviewDto {
        id: row.id.to_string(),
        course_id: row.course_id.to_string(),
        rating: row.rating,
        comment: row.comment.clone(),
        score: row.score.clone(),
        semester: row.semester.clone(),
        author_handle: row.handle.clone(),
        author_avatar: row.avatar_url.clone(),
        approve_count: row.approve_count,
        status: row.status.clone(),
        created_at: row.created_at.timestamp(),
    }
}

/// Look up author handle and avatar, then build a DTO from a raw ReviewRow.
fn row_to_dto_with_author(row: &ReviewRow, handle: &str, avatar_url: Option<&str>) -> ReviewDto {
    ReviewDto {
        id: row.id.to_string(),
        course_id: row.course_id.to_string(),
        rating: row.rating,
        comment: row.comment.clone(),
        score: row.score.clone(),
        semester: row.semester.clone(),
        author_handle: handle.to_string(),
        author_avatar: avatar_url.map(|a| a.to_string()),
        approve_count: row.approve_count,
        status: row.status.clone(),
        created_at: row.created_at.timestamp(),
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// List reviews for a course with cursor pagination.
///
/// `sort` = `"hot"` → order by `approve_count DESC, created_at DESC`.
/// `sort` = `"new"` (default) → order by `created_at DESC`.
/// Returns a `Page<ReviewDto>` with cursor-based continuation.
pub async fn list_reviews(
    pool: &PgPool,
    course_id: i64,
    sort: Option<&str>,
    cursor: Option<i64>,
    limit: Option<i64>,
) -> AppResult<Page<ReviewDto>> {
    let fetch_limit = limit.unwrap_or(DEFAULT_LIMIT).min(100) + 1;

    let rows = if sort == Some("hot") {
        if let Some(c) = cursor {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.course_id = $1 AND r.status = 'visible' \
                   AND (r.approve_count, r.created_at) < ( \
                     SELECT rr.approve_count, rr.created_at \
                     FROM reviews.reviews rr WHERE rr.id = $2 \
                   ) \
                 ORDER BY r.approve_count DESC, r.created_at DESC \
                 LIMIT $3",
            )
            .bind(course_id)
            .bind(c)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.course_id = $1 AND r.status = 'visible' \
                 ORDER BY r.approve_count DESC, r.created_at DESC \
                 LIMIT $2",
            )
            .bind(course_id)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        }
    } else {
        if let Some(c) = cursor {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.course_id = $1 AND r.status = 'visible' AND r.id < $2 \
                 ORDER BY r.created_at DESC \
                 LIMIT $3",
            )
            .bind(course_id)
            .bind(c)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.course_id = $1 AND r.status = 'visible' \
                 ORDER BY r.created_at DESC \
                 LIMIT $2",
            )
            .bind(course_id)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        }
    };

    let actual_limit = fetch_limit - 1;
    let has_more = rows.len() > actual_limit as usize;
    let items: Vec<ReviewDto> = if has_more {
        rows[..actual_limit as usize].iter().map(row_to_dto).collect()
    } else {
        rows.iter().map(row_to_dto).collect()
    };
    let next_cursor =
        if has_more { items.last().map(|r| r.id.parse::<i64>().unwrap_or(0)) } else { None };

    Ok(Page::new(items, next_cursor.map(|c| c.to_string())))
}

/// Create a new review and update the course aggregate stats in a transaction.
pub async fn create_review(
    pool: &PgPool,
    course_id: i64,
    account_id: i64,
    rating: i32,
    comment: Option<&str>,
    semester: Option<&str>,
    score: Option<&str>,
) -> AppResult<ReviewDto> {
    check_rating(rating)?;

    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, ReviewRow>(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, semester, score) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING *",
    )
    .bind(course_id)
    .bind(account_id)
    .bind(rating)
    .bind(comment)
    .bind(semester)
    .bind(score)
    .fetch_one(&mut *tx)
    .await?;

    // Fetch author handle + avatar for the DTO.
    let (handle, avatar_url): (String, Option<String>) =
        sqlx::query_as("SELECT handle, avatar_url FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&mut *tx)
            .await?;

    // Incrementally update course stats.
    sqlx::query(
        "UPDATE courses.courses \
         SET review_count = review_count + 1, \
             review_avg = ((review_avg * (review_count - 1) + $2) / review_count::float8) \
         WHERE id = $1",
    )
    .bind(course_id)
    .bind(rating as f64)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(row_to_dto_with_author(&row, &handle, avatar_url.as_deref()))
}

/// Update a review. Only the original author may update.
pub async fn update_review(
    pool: &PgPool,
    review_id: i64,
    account_id: i64,
    rating: i32,
    comment: Option<&str>,
    semester: Option<&str>,
    score: Option<&str>,
) -> AppResult<ReviewDto> {
    check_rating(rating)?;

    let mut tx = pool.begin().await?;

    // Fetch the existing review with row lock to verify ownership.
    let existing =
        sqlx::query_as::<_, ReviewRow>("SELECT * FROM reviews.reviews WHERE id = $1 FOR UPDATE")
            .bind(review_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(ReviewsError::ReviewNotFound)?;

    if existing.account_id != account_id {
        return Err(ReviewsError::NotOwnReview.into());
    }

    let old_rating = existing.rating;
    let course_id = existing.course_id;

    let row = sqlx::query_as::<_, ReviewRow>(
        "UPDATE reviews.reviews \
         SET rating = $2, comment = $3, semester = $4, score = $5, updated_at = now() \
         WHERE id = $1 \
         RETURNING *",
    )
    .bind(review_id)
    .bind(rating)
    .bind(comment)
    .bind(semester)
    .bind(score)
    .fetch_one(&mut *tx)
    .await?;

    // Update course aggregate stats in the same transaction.
    if old_rating != rating {
        sqlx::query(
            "UPDATE courses.courses \
             SET review_avg = ((review_avg * review_count::float8 - $2 + $3) / review_count::float8) \
             WHERE id = $1",
        )
        .bind(course_id)
        .bind(old_rating as f64)
        .bind(rating as f64)
        .execute(&mut *tx)
        .await?;
    }

    let (handle, avatar_url): (String, Option<String>) =
        sqlx::query_as("SELECT handle, avatar_url FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;

    Ok(row_to_dto_with_author(&row, &handle, avatar_url.as_deref()))
}

/// Like a review. Idempotent — no-op if already liked.
pub async fn like_review(pool: &PgPool, review_id: i64, account_id: i64) -> AppResult<()> {
    let inserted: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO reviews.review_likes (review_id, account_id) \
         VALUES ($1, $2) \
         ON CONFLICT (review_id, account_id) DO NOTHING \
         RETURNING review_id",
    )
    .bind(review_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;

    if inserted.is_some() {
        sqlx::query("UPDATE reviews.reviews SET approve_count = approve_count + 1 WHERE id = $1")
            .bind(review_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Unlike a review. No-op if not previously liked.
pub async fn unlike_review(pool: &PgPool, review_id: i64, account_id: i64) -> AppResult<()> {
    let deleted =
        sqlx::query("DELETE FROM reviews.review_likes WHERE review_id = $1 AND account_id = $2")
            .bind(review_id)
            .bind(account_id)
            .execute(pool)
            .await?;

    if deleted.rows_affected() > 0 {
        sqlx::query("UPDATE reviews.reviews SET approve_count = approve_count - 1 WHERE id = $1")
            .bind(review_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Report a review. Idempotent — returns conflict error if already reported.
pub async fn report_review(
    pool: &PgPool,
    review_id: i64,
    reporter_account_id: i64,
    reason: &str,
) -> AppResult<()> {
    // Verify the review exists.
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reviews.reviews WHERE id = $1)")
            .bind(review_id)
            .fetch_one(pool)
            .await?;

    if !exists {
        return Err(ReviewsError::ReviewNotFound.into());
    }

    let inserted = sqlx::query(
        "INSERT INTO reviews.review_reports (review_id, reporter_account_id, reason) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (review_id, reporter_account_id) DO NOTHING",
    )
    .bind(review_id)
    .bind(reporter_account_id)
    .bind(reason)
    .execute(pool)
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(ReviewsError::AlreadyReported.into());
    }

    Ok(())
}

/// Look up a single review by id (for admin or ownership checks).
#[allow(dead_code)]
pub async fn find_review_by_id(pool: &PgPool, review_id: i64) -> AppResult<Option<ReviewRow>> {
    let row = sqlx::query_as::<_, ReviewRow>("SELECT * FROM reviews.reviews WHERE id = $1")
        .bind(review_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// List all reviews (admin view), optionally filtered by status.
/// Uses cursor-based pagination ordered by `created_at DESC, id DESC`.
pub async fn admin_list_reviews(
    pool: &PgPool,
    status_filter: Option<&str>,
    cursor: Option<i64>,
    limit: Option<i64>,
) -> AppResult<Page<ReviewDto>> {
    let fetch_limit = limit.unwrap_or(DEFAULT_LIMIT).min(100) + 1;

    let rows = if let Some(s) = status_filter {
        if let Some(c) = cursor {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.status = $1::reviews.review_status AND r.id < $2 \
                 ORDER BY r.id DESC \
                 LIMIT $3",
            )
            .bind(s)
            .bind(c)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.status = $1::reviews.review_status \
                 ORDER BY r.id DESC \
                 LIMIT $2",
            )
            .bind(s)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        }
    } else {
        if let Some(c) = cursor {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 WHERE r.id < $1 \
                 ORDER BY r.id DESC \
                 LIMIT $2",
            )
            .bind(c)
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.*, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 JOIN identity.accounts a ON a.id = r.account_id \
                 ORDER BY r.id DESC \
                 LIMIT $1",
            )
            .bind(fetch_limit)
            .fetch_all(pool)
            .await?
        }
    };

    let actual_limit = fetch_limit - 1;
    let has_more = rows.len() > actual_limit as usize;
    let items: Vec<ReviewDto> = if has_more {
        rows[..actual_limit as usize].iter().map(row_to_dto).collect()
    } else {
        rows.iter().map(row_to_dto).collect()
    };
    let next_cursor =
        if has_more { items.last().map(|r| r.id.parse::<i64>().unwrap_or(0)) } else { None };

    Ok(Page::new(items, next_cursor.map(|c| c.to_string())))
}

/// Admin: soft-delete a review and update course aggregates.
pub async fn admin_soft_delete_review(pool: &PgPool, review_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let existing =
        sqlx::query_as::<_, ReviewRow>("SELECT * FROM reviews.reviews WHERE id = $1 FOR UPDATE")
            .bind(review_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(ReviewsError::ReviewNotFound)?;

    if existing.status == "visible" {
        // Decrement review_count and recompute average.
        sqlx::query(
            "UPDATE courses.courses \
             SET review_count = GREATEST(review_count - 1, 0), \
                 review_avg = CASE \
                   WHEN review_count <= 1 THEN 0 \
                   ELSE ((review_avg * review_count::float8 - $2) / (review_count - 1)::float8) \
                 END \
             WHERE id = $1",
        )
        .bind(existing.course_id)
        .bind(existing.rating as f64)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE reviews.reviews SET status = 'hidden' WHERE id = $1")
        .bind(review_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Admin: toggle a review between visible and hidden, updating course aggregates.
pub async fn admin_toggle_review_visibility(pool: &PgPool, review_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let existing =
        sqlx::query_as::<_, ReviewRow>("SELECT * FROM reviews.reviews WHERE id = $1 FOR UPDATE")
            .bind(review_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(ReviewsError::ReviewNotFound)?;

    let new_status = if existing.status == "visible" { "hidden" } else { "visible" };

    if existing.status == "visible" {
        // Hiding: decrement count, remove rating from average.
        sqlx::query(
            "UPDATE courses.courses \
             SET review_count = GREATEST(review_count - 1, 0), \
                 review_avg = CASE \
                   WHEN review_count <= 1 THEN 0 \
                   ELSE ((review_avg * review_count::float8 - $2) / (review_count - 1)::float8) \
                 END \
             WHERE id = $1",
        )
        .bind(existing.course_id)
        .bind(existing.rating as f64)
        .execute(&mut *tx)
        .await?;
    } else if existing.status == "hidden" && new_status == "visible" {
        // Showing: increment count, add rating to average.
        sqlx::query(
            "UPDATE courses.courses \
             SET review_count = review_count + 1, \
                 review_avg = ((review_avg * (review_count - 1)::float8 + $2) / review_count::float8) \
             WHERE id = $1",
        )
        .bind(existing.course_id)
        .bind(existing.rating as f64)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE reviews.reviews SET status = $1::reviews.review_status WHERE id = $2")
        .bind(new_status)
        .bind(review_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Admin: edit any review with transactional course aggregate update.
pub async fn admin_edit_review(
    pool: &PgPool,
    review_id: i64,
    rating: i32,
    comment: Option<&str>,
    semester: Option<&str>,
    score: Option<&str>,
) -> AppResult<ReviewDto> {
    check_rating(rating)?;

    let mut tx = pool.begin().await?;

    let existing =
        sqlx::query_as::<_, ReviewRow>("SELECT * FROM reviews.reviews WHERE id = $1 FOR UPDATE")
            .bind(review_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(ReviewsError::ReviewNotFound)?;

    let old_rating = existing.rating;
    let account_id = existing.account_id;
    let course_id = existing.course_id;

    let row = sqlx::query_as::<_, ReviewRow>(
        "UPDATE reviews.reviews \
         SET rating = $2, comment = $3, semester = $4, score = $5, updated_at = now() \
         WHERE id = $1 \
         RETURNING *",
    )
    .bind(review_id)
    .bind(rating)
    .bind(comment)
    .bind(semester)
    .bind(score)
    .fetch_one(&mut *tx)
    .await?;

    if old_rating != rating {
        sqlx::query(
            "UPDATE courses.courses \
             SET review_avg = ((review_avg * review_count::float8 - $2 + $3) / review_count::float8) \
             WHERE id = $1",
        )
        .bind(course_id)
        .bind(old_rating as f64)
        .bind(rating as f64)
        .execute(&mut *tx)
        .await?;
    }

    let (handle, avatar_url): (String, Option<String>) =
        sqlx::query_as("SELECT handle, avatar_url FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;

    Ok(row_to_dto_with_author(&row, &handle, avatar_url.as_deref()))
}

/// List reports, optionally filtered by status.
pub async fn list_reports(
    pool: &PgPool,
    status_filter: Option<&str>,
) -> AppResult<Vec<crate::dto::ReportDto>> {
    let rows = if let Some(s) = status_filter {
        sqlx::query_as::<_, ReviewReportRow>(
            "SELECT * FROM reviews.review_reports WHERE status = $1 ORDER BY created_at DESC",
        )
        .bind(s)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ReviewReportRow>(
            "SELECT * FROM reviews.review_reports ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|r| crate::dto::ReportDto {
            id: r.id.to_string(),
            review_id: r.review_id.to_string(),
            reason: r.reason,
            status: r.status,
            created_at: r.created_at.timestamp(),
        })
        .collect())
}

/// Resolve a report (mark as resolved with optional note).
pub async fn resolve_report(pool: &PgPool, report_id: i64, note: Option<&str>) -> AppResult<()> {
    let affected = sqlx::query(
        "UPDATE reviews.review_reports \
         SET status = 'resolved', admin_note = $2, resolved_at = now() \
         WHERE id = $1",
    )
    .bind(report_id)
    .bind(note)
    .execute(pool)
    .await?;

    if affected.rows_affected() == 0 {
        return Err(shared::AppError::NotFound);
    }
    Ok(())
}

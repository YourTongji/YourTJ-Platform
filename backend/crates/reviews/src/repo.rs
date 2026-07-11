//! Database access layer for the reviews domain.
//!
//! Every function takes `&PgPool` and returns `AppResult` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

use shared::pagination::Page;
use shared::AppResult;
use sqlx::{PgConnection, PgPool};

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

fn role_rank(role: &str) -> i8 {
    match role {
        "admin" => 2,
        "mod" => 1,
        _ => 0,
    }
}

async fn require_can_moderate_author(
    tx: &mut PgConnection,
    actor_role: &str,
    author_id: Option<i64>,
) -> AppResult<()> {
    let Some(author_id) = author_id else {
        return Ok(());
    };
    let author_role = identity::public_accounts::find_account_role_by_id(tx, author_id)
        .await?
        .ok_or_else(|| {
            shared::AppError::Internal(
                std::io::Error::other("review author account is missing").into(),
            )
        })?;
    if role_rank(actor_role) <= role_rank(&author_role) {
        return Err(shared::AppError::Forbidden);
    }
    Ok(())
}

pub(crate) async fn set_review_status_tx(
    tx: &mut PgConnection,
    review: &ReviewRow,
    new_status: &str,
) -> AppResult<()> {
    if review.status == new_status {
        return Ok(());
    }
    if review.status == "visible" && new_status != "visible" {
        sqlx::query(
            "UPDATE courses.courses \
             SET review_count = GREATEST(review_count - 1, 0), \
                 review_avg = CASE \
                   WHEN review_count <= 1 THEN 0 \
                   ELSE ((review_avg * review_count::float8 - $2) / (review_count - 1)::float8) \
                 END \
             WHERE id = $1",
        )
        .bind(review.course_id)
        .bind(review.rating as f64)
        .execute(&mut *tx)
        .await?;
        project_review_likes_for_visibility(tx, review.id, false).await?;
    } else if review.status != "visible" && new_status == "visible" {
        sqlx::query(
            "UPDATE courses.courses \
             SET review_count = review_count + 1, \
                 review_avg = (review_avg * review_count + $2) / (review_count + 1)::float8 \
             WHERE id = $1",
        )
        .bind(review.course_id)
        .bind(review.rating as f64)
        .execute(&mut *tx)
        .await?;
        project_review_likes_for_visibility(tx, review.id, true).await?;
    }
    sqlx::query("UPDATE reviews.reviews SET status = $1::reviews.review_status WHERE id = $2")
        .bind(new_status)
        .bind(review.id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

async fn project_review_likes_for_visibility(
    tx: &mut PgConnection,
    review_id: i64,
    is_visible: bool,
) -> AppResult<()> {
    let likes: Vec<(i64, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT account_id, created_at FROM reviews.review_likes \
         WHERE review_id = $1 ORDER BY account_id",
    )
    .bind(review_id)
    .fetch_all(&mut *tx)
    .await?;
    for (account_id, created_at) in likes {
        let source_key = format!("review_like:{review_id}:{account_id}");
        if is_visible {
            activity::contributions::activate_contribution(
                tx,
                account_id,
                activity::contributions::ActivityKind::Like,
                &source_key,
                created_at,
            )
            .await?;
        } else {
            activity::contributions::deactivate_contribution(tx, &source_key, chrono::Utc::now())
                .await?;
        }
    }
    Ok(())
}

/// Build a ReviewDto from a joined row.
///
/// Falls back to `reviewer_name` and `reviewer_avatar` for legacy reviews
/// that have no matching account (NULL `account_id`).
fn row_to_dto(row: &ReviewWithAuthorRow) -> ReviewDto {
    ReviewDto {
        id: row.id.to_string(),
        course_id: row.course_id.to_string(),
        rating: row.rating,
        comment: row.comment.clone(),
        score: row.score.clone(),
        semester: row.semester.clone(),
        author_handle: row.handle.clone().or_else(|| row.reviewer_name.clone()).unwrap_or_default(),
        author_avatar: row.avatar_url.clone().or_else(|| row.reviewer_avatar.clone()),
        approve_count: row.approve_count,
        status: row.status.clone(),
        created_at: row.created_at.timestamp(),
    }
}

fn report_to_dto(row: ReviewReportRow) -> crate::dto::ReportDto {
    crate::dto::ReportDto {
        id: row.id.to_string(),
        review_id: row.review_id.to_string(),
        reason: row.reason,
        status: row.status,
        course_id: row.course_id.map(|course_id| course_id.to_string()),
        review_author_handle: row.review_author_handle,
        review_rating: row.review_rating,
        review_status: row.review_status,
        review_excerpt: row.review_excerpt,
        created_at: row.created_at.timestamp(),
    }
}

/// Look up author handle and avatar, then build a DTO from a raw ReviewRow.
///
/// Falls back to `reviewer_name` / `reviewer_avatar` when the review has no
/// linked account (legacy reviews, NULL `account_id`).
fn row_to_dto_with_author(row: &ReviewRow, handle: &str, avatar_url: Option<&str>) -> ReviewDto {
    let effective_handle =
        if handle.is_empty() { row.reviewer_name.as_deref().unwrap_or("") } else { handle };
    let effective_avatar =
        if avatar_url.is_none() { row.reviewer_avatar.as_deref() } else { avatar_url };
    ReviewDto {
        id: row.id.to_string(),
        course_id: row.course_id.to_string(),
        rating: row.rating,
        comment: row.comment.clone(),
        score: row.score.clone(),
        semester: row.semester.clone(),
        author_handle: effective_handle.to_string(),
        author_avatar: effective_avatar.map(|a| a.to_string()),
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
    let mut tx = pool.begin().await?;
    let dto =
        create_review_tx(&mut tx, course_id, account_id, rating, comment, semester, score).await?;
    tx.commit().await?;
    Ok(dto)
}

/// Create a review inside an existing transaction.
#[allow(clippy::too_many_arguments)] // reason: review fields remain explicit at the transactional boundary
pub async fn create_review_tx(
    tx: &mut PgConnection,
    course_id: i64,
    account_id: i64,
    rating: i32,
    comment: Option<&str>,
    semester: Option<&str>,
    score: Option<&str>,
) -> AppResult<ReviewDto> {
    check_rating(rating)?;

    let row = sqlx::query_as::<_, ReviewRow>(
        "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, semester, score) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, course_id, account_id, rating, comment, score, semester, \
                   approve_count, disapprove_count, status::text, created_at, updated_at, \
                   reviewer_name, reviewer_avatar",
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
    // In a single UPDATE the right-hand side sees the pre-increment
    // `review_count`, so compute the new average from the OLD count/avg:
    // new_avg = (old_avg * old_count + rating) / (old_count + 1). Using the
    // post-increment count in the denominator would divide by zero on the
    // first review of a course.
    sqlx::query(
        "UPDATE courses.courses \
         SET review_count = review_count + 1, \
             review_avg = (review_avg * review_count + $2) / (review_count + 1)::float8 \
         WHERE id = $1",
    )
    .bind(course_id)
    .bind(rating as f64)
    .execute(&mut *tx)
    .await?;

    Ok(row_to_dto_with_author(&row, &handle, avatar_url.as_deref()))
}

/// Serialize creation attempts sharing one account-scoped idempotency key.
pub async fn lock_review_create_idempotency(
    tx: &mut PgConnection,
    account_id: i64,
    idempotency_key: &str,
) -> AppResult<()> {
    let lock_key = format!("review_create:{account_id}:{idempotency_key}");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(lock_key)
        .execute(tx)
        .await?;
    Ok(())
}

/// Return the completed result for a matching idempotent request.
pub async fn find_review_create_replay(
    tx: &mut PgConnection,
    account_id: i64,
    idempotency_key: &str,
    request_hash: &str,
) -> AppResult<Option<ReviewDto>> {
    let existing: Option<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT request_hash, response \
         FROM reviews.review_create_idempotency \
         WHERE account_id = $1 AND idempotency_key = $2",
    )
    .bind(account_id)
    .bind(idempotency_key)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((stored_hash, response)) = existing else {
        return Ok(None);
    };
    if stored_hash != request_hash {
        return Err(shared::AppError::Conflict(
            "idempotency key was already used for another review request".into(),
        ));
    }
    let dto = serde_json::from_value(response)
        .map_err(|error| shared::AppError::Internal(error.into()))?;
    Ok(Some(dto))
}

/// Persist a completed idempotent review request in the creation transaction.
pub async fn record_review_create_idempotency(
    tx: &mut PgConnection,
    account_id: i64,
    idempotency_key: &str,
    request_hash: &str,
    review_id: i64,
    response: &ReviewDto,
) -> AppResult<()> {
    let response =
        serde_json::to_value(response).map_err(|error| shared::AppError::Internal(error.into()))?;
    sqlx::query(
        "INSERT INTO reviews.review_create_idempotency \
         (account_id, idempotency_key, request_hash, review_id, response) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(account_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(review_id)
    .bind(response)
    .execute(tx)
    .await?;
    Ok(())
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
    let existing = sqlx::query_as::<_, ReviewRow>(
        "SELECT id, course_id, account_id, rating, comment, score, semester, \
                approve_count, disapprove_count, status::text, created_at, updated_at, \
                reviewer_name, reviewer_avatar \
         FROM reviews.reviews WHERE id = $1 FOR UPDATE",
    )
    .bind(review_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ReviewsError::ReviewNotFound)?;

    // Legacy reviews (NULL account_id) cannot be edited by anyone via this path.
    if existing.account_id != Some(account_id) {
        return Err(ReviewsError::NotOwnReview.into());
    }

    let old_rating = existing.rating;
    let course_id = existing.course_id;

    let row = sqlx::query_as::<_, ReviewRow>(
        "UPDATE reviews.reviews \
         SET rating = $2, comment = $3, semester = $4, score = $5, updated_at = now() \
         WHERE id = $1 \
         RETURNING id, course_id, account_id, rating, comment, score, semester, \
                   approve_count, disapprove_count, status::text, created_at, updated_at, \
                   reviewer_name, reviewer_avatar",
    )
    .bind(review_id)
    .bind(rating)
    .bind(comment)
    .bind(semester)
    .bind(score)
    .fetch_one(&mut *tx)
    .await?;

    // Update course aggregate stats in the same transaction.
    if old_rating != rating && existing.status == "visible" {
        let updated = sqlx::query(
            "UPDATE courses.courses \
             SET review_avg = ((review_avg * review_count::float8 - $2 + $3) / review_count::float8) \
             WHERE id = $1 AND review_count > 0",
        )
        .bind(course_id)
        .bind(old_rating as f64)
        .bind(rating as f64)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(shared::AppError::Internal(
                std::io::Error::other("visible review is missing its course aggregate").into(),
            ));
        }
    }

    let (handle, avatar_url): (String, Option<String>) =
        sqlx::query_as("SELECT handle, avatar_url FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&mut *tx)
            .await?;

    tx.commit().await?;

    Ok(row_to_dto_with_author(&row, &handle, avatar_url.as_deref()))
}

/// Like a review and project the positive activity transition atomically.
///
/// Returns `true` only when the relationship was newly inserted.
pub async fn like_review(pool: &PgPool, review_id: i64, account_id: i64) -> AppResult<bool> {
    let mut tx = pool.begin().await?;
    let review_author_id: Option<i64> = sqlx::query_scalar(
        "SELECT account_id FROM reviews.reviews WHERE id = $1 AND status = 'visible'",
    )
    .bind(review_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ReviewsError::ReviewNotFound)?;
    if review_author_id == Some(account_id) {
        return Err(shared::AppError::BadRequest("cannot like your own review".into()));
    }
    let source_key = format!("review_like:{review_id}:{account_id}");
    activity::contributions::lock_contribution_source(&mut tx, &source_key).await?;
    let inserted: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO reviews.review_likes (review_id, account_id) \
         VALUES ($1, $2) \
         ON CONFLICT (review_id, account_id) DO NOTHING \
         RETURNING review_id",
    )
    .bind(review_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;

    if inserted.is_some() {
        sqlx::query("UPDATE reviews.reviews SET approve_count = approve_count + 1 WHERE id = $1")
            .bind(review_id)
            .execute(&mut *tx)
            .await?;
        activity::contributions::activate_contribution(
            &mut tx,
            account_id,
            activity::contributions::ActivityKind::Like,
            &source_key,
            chrono::Utc::now(),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(inserted.is_some())
}

/// Unlike a review and reverse the original activity date atomically.
///
/// Returns `true` only when an existing relationship was removed.
pub async fn unlike_review(pool: &PgPool, review_id: i64, account_id: i64) -> AppResult<bool> {
    let mut tx = pool.begin().await?;
    let source_key = format!("review_like:{review_id}:{account_id}");
    activity::contributions::lock_contribution_source(&mut tx, &source_key).await?;
    let deleted =
        sqlx::query("DELETE FROM reviews.review_likes WHERE review_id = $1 AND account_id = $2")
            .bind(review_id)
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

    if deleted.rows_affected() > 0 {
        sqlx::query(
            "UPDATE reviews.reviews \
             SET approve_count = GREATEST(approve_count - 1, 0) WHERE id = $1",
        )
        .bind(review_id)
        .execute(&mut *tx)
        .await?;
        activity::contributions::deactivate_contribution(&mut tx, &source_key, chrono::Utc::now())
            .await?;
    }

    tx.commit().await?;
    Ok(deleted.rows_affected() > 0)
}

/// Report a review. Idempotent — returns conflict error if already reported.
pub async fn report_review(
    pool: &PgPool,
    review_id: i64,
    reporter_account_id: i64,
    reason: &str,
) -> AppResult<()> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(shared::AppError::BadRequest("report reason must be 3–500 characters".into()));
    }
    let mut tx = pool.begin().await?;
    let review_author: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT account_id FROM reviews.reviews \
         WHERE id = $1 AND status = 'visible' FOR SHARE",
    )
    .bind(review_id)
    .fetch_optional(&mut *tx)
    .await?;
    let review_author = review_author.ok_or(ReviewsError::ReviewNotFound)?.0;
    if review_author == Some(reporter_account_id) {
        return Err(shared::AppError::BadRequest("cannot report your own review".into()));
    }

    let inserted = sqlx::query(
        "INSERT INTO reviews.review_reports (review_id, reporter_account_id, reason) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (review_id, reporter_account_id) WHERE status = 'open' DO NOTHING",
    )
    .bind(review_id)
    .bind(reporter_account_id)
    .bind(reason)
    .execute(&mut *tx)
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(ReviewsError::AlreadyReported.into());
    }

    tx.commit().await?;
    Ok(())
}

/// Look up a single review by id (for admin or ownership checks).
#[allow(dead_code)]
pub async fn find_review_by_id(pool: &PgPool, review_id: i64) -> AppResult<Option<ReviewRow>> {
    let row = sqlx::query_as::<_, ReviewRow>(
        "SELECT id, course_id, account_id, rating, comment, score, semester, \
                approve_count, disapprove_count, status::text, created_at, updated_at, \
                reviewer_name, reviewer_avatar \
         FROM reviews.reviews WHERE id = $1",
    )
    .bind(review_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Return the course owning a review, including reviews hidden from public lists.
pub async fn review_course_id(pool: &PgPool, review_id: i64) -> AppResult<Option<i64>> {
    let course_id = sqlx::query_scalar("SELECT course_id FROM reviews.reviews WHERE id = $1")
        .bind(review_id)
        .fetch_optional(pool)
        .await?;
    Ok(course_id)
}

/// List all reviews (admin view), optionally filtered by status.
/// Uses cursor-based pagination ordered by `created_at DESC, id DESC`.
pub async fn admin_list_reviews(
    pool: &PgPool,
    status_filter: Option<&str>,
    cursor: Option<i64>,
    limit: Option<i64>,
) -> AppResult<Page<ReviewDto>> {
    let requested_limit = limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=100).contains(&requested_limit) {
        return Err(shared::AppError::BadRequest("limit must be between 1 and 100".into()));
    }
    if cursor.is_some_and(|cursor| cursor <= 0) {
        return Err(shared::AppError::BadRequest("cursor must be a positive integer".into()));
    }
    if status_filter.is_some_and(|status| !matches!(status, "visible" | "hidden" | "pending")) {
        return Err(shared::AppError::BadRequest("invalid review status".into()));
    }
    let fetch_limit = requested_limit + 1;

    let rows = if let Some(s) = status_filter {
        if let Some(c) = cursor {
            sqlx::query_as::<_, ReviewWithAuthorRow>(
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
                "SELECT r.id, r.course_id, r.account_id, r.rating, r.comment, r.score, \
                 r.semester, r.approve_count, r.disapprove_count, \
                 r.status::text, r.created_at, r.updated_at, \
                 r.reviewer_name, r.reviewer_avatar, a.handle, a.avatar_url \
                 FROM reviews.reviews r \
                 LEFT JOIN identity.accounts a ON a.id = r.account_id \
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
pub async fn admin_soft_delete_review(
    pool: &PgPool,
    review_id: i64,
    actor_id: i64,
    actor_role: &str,
    reason: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let existing = sqlx::query_as::<_, ReviewRow>(
        "SELECT id, course_id, account_id, rating, comment, score, semester, \
                approve_count, disapprove_count, status::text, created_at, updated_at, \
                reviewer_name, reviewer_avatar \
         FROM reviews.reviews WHERE id = $1 FOR UPDATE",
    )
    .bind(review_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ReviewsError::ReviewNotFound)?;

    require_can_moderate_author(&mut tx, actor_role, existing.account_id).await?;
    if existing.status == "hidden" {
        return Err(shared::AppError::Conflict("review is already hidden".into()));
    }
    set_review_status_tx(&mut tx, &existing, "hidden").await?;
    let metadata = serde_json::json!({ "oldStatus": existing.status, "newStatus": "hidden" });
    let governance_event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        governance::AccountActor { account_id: actor_id, role: actor_role },
        "reviews.review.hidden",
        "review",
        &review_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    if let Some(account_id) = existing.account_id {
        governance::notices::create_notice_tx(
            &mut tx,
            account_id,
            "content_restricted",
            &format!("audit:{governance_event_id}:review"),
            Some(governance_event_id),
            None,
            "review",
            &review_id.to_string(),
            "你的课评已被隐藏，可在申诉中心查看并申请复核。",
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Admin: toggle a review between visible and hidden, updating course aggregates.
pub async fn admin_toggle_review_visibility(
    pool: &PgPool,
    review_id: i64,
    actor_id: i64,
    actor_role: &str,
    reason: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let existing = sqlx::query_as::<_, ReviewRow>(
        "SELECT id, course_id, account_id, rating, comment, score, semester, \
                approve_count, disapprove_count, status::text, created_at, updated_at, \
                reviewer_name, reviewer_avatar \
         FROM reviews.reviews WHERE id = $1 FOR UPDATE",
    )
    .bind(review_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ReviewsError::ReviewNotFound)?;

    require_can_moderate_author(&mut tx, actor_role, existing.account_id).await?;
    let new_status = if existing.status == "visible" { "hidden" } else { "visible" };

    set_review_status_tx(&mut tx, &existing, new_status).await?;
    let metadata = serde_json::json!({ "oldStatus": existing.status, "newStatus": new_status });
    let governance_event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        governance::AccountActor { account_id: actor_id, role: actor_role },
        if new_status == "visible" { "reviews.review.restored" } else { "reviews.review.hidden" },
        "review",
        &review_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    if new_status == "hidden" {
        if let Some(account_id) = existing.account_id {
            governance::notices::create_notice_tx(
                &mut tx,
                account_id,
                "content_restricted",
                &format!("audit:{governance_event_id}:review"),
                Some(governance_event_id),
                None,
                "review",
                &review_id.to_string(),
                "你的课评已被隐藏，可在申诉中心查看并申请复核。",
            )
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// List reports, optionally filtered by status.
pub async fn list_reports(
    pool: &PgPool,
    status_filter: Option<&str>,
    cursor: Option<i64>,
    limit: i64,
) -> AppResult<Page<crate::dto::ReportDto>> {
    let page_size = limit.clamp(1, 100);
    let rows = sqlx::query_as::<_, ReviewReportRow>(
        "SELECT report.id, report.review_id, report.reporter_account_id, report.reason, \
                report.status, report.admin_note, report.created_at, \
                review.course_id, \
                COALESCE(author.handle::text, review.reviewer_name) AS review_author_handle, \
                review.rating AS review_rating, review.status::text AS review_status, \
                NULLIF(left(COALESCE(review.comment, ''), 1000), '') AS review_excerpt \
         FROM reviews.review_reports report \
         JOIN reviews.reviews review ON review.id = report.review_id \
         LEFT JOIN identity.accounts author ON author.id = review.account_id \
         WHERE ($1::text IS NULL OR report.status = $1) \
           AND ($2::bigint IS NULL OR report.id < $2) \
         ORDER BY report.id DESC LIMIT $3",
    )
    .bind(status_filter)
    .bind(cursor)
    .bind(page_size + 1)
    .fetch_all(pool)
    .await?;
    let has_more = rows.len() > page_size as usize;
    let visible_count = if has_more { page_size as usize } else { rows.len() };
    let items: Vec<_> = rows.into_iter().take(visible_count).map(report_to_dto).collect();
    let next_cursor = has_more.then(|| items.last().map(|item| item.id.clone())).flatten();
    Ok(Page::new(items, next_cursor))
}

/// Apply an explicit report decision and its resulting content action atomically.
#[allow(clippy::too_many_arguments)] // reason: audit actor and decision fields stay explicit at the transaction boundary
pub async fn resolve_report(
    pool: &PgPool,
    report_id: i64,
    action: &str,
    note: &str,
    actor_id: i64,
    actor_role: &str,
) -> AppResult<crate::dto::ReportDto> {
    let decision = match action {
        "uphold" => "upheld",
        "reject" => "rejected",
        "ignore" => "ignored",
        _ => return Err(shared::AppError::BadRequest("invalid review report decision".into())),
    };
    let mut tx = pool.begin().await?;
    let report = sqlx::query_as::<_, ReviewReportRow>(
        "SELECT report.id, report.review_id, report.reporter_account_id, report.reason, \
                report.status, report.admin_note, report.created_at, \
                review.course_id, \
                COALESCE(author.handle::text, review.reviewer_name) AS review_author_handle, \
                review.rating AS review_rating, review.status::text AS review_status, \
                NULLIF(left(COALESCE(review.comment, ''), 1000), '') AS review_excerpt \
         FROM reviews.review_reports report \
         JOIN reviews.reviews review ON review.id = report.review_id \
         LEFT JOIN identity.accounts author ON author.id = review.account_id \
         WHERE report.id = $1 FOR UPDATE OF report",
    )
    .bind(report_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    if report.status != "open" {
        return Err(shared::AppError::Conflict("review report is already decided".into()));
    }
    let review = sqlx::query_as::<_, ReviewRow>(
        "SELECT id, course_id, account_id, rating, comment, score, semester, \
                approve_count, disapprove_count, status::text, created_at, updated_at, \
                reviewer_name, reviewer_avatar \
         FROM reviews.reviews WHERE id = $1 FOR UPDATE",
    )
    .bind(report.review_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ReviewsError::ReviewNotFound)?;
    require_can_moderate_author(&mut tx, actor_role, review.account_id).await?;
    let content_changed = decision == "upheld" && review.status != "hidden";
    if decision == "upheld" {
        set_review_status_tx(&mut tx, &review, "hidden").await?;
    }
    sqlx::query(
        "UPDATE reviews.review_reports \
         SET status = $1, admin_note = $2, resolved_at = now() \
         WHERE id = $3",
    )
    .bind(decision)
    .bind(note)
    .bind(report_id)
    .execute(&mut *tx)
    .await?;
    let old_status = review.status.as_str();
    let metadata = serde_json::json!({
        "decision": decision,
        "reviewId": report.review_id,
        "oldStatus": old_status,
        "newStatus": if decision == "upheld" { "hidden" } else { old_status },
        "contentChanged": content_changed,
    });
    let governance_event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        governance::AccountActor { account_id: actor_id, role: actor_role },
        "reviews.report.decided",
        "review_report",
        &report_id.to_string(),
        note,
        Some(&metadata),
    )
    .await?;
    if decision == "upheld" && content_changed {
        if let Some(account_id) = review.account_id {
            governance::notices::create_notice_tx(
                &mut tx,
                account_id,
                "content_restricted",
                &format!("audit:{governance_event_id}:review-report"),
                Some(governance_event_id),
                None,
                "review",
                &review.id.to_string(),
                "你的课评在举报复核后被隐藏，可在申诉中心查看并申请复核。",
            )
            .await?;
        }
    }
    let updated = sqlx::query_as::<_, ReviewReportRow>(
        "SELECT report.id, report.review_id, report.reporter_account_id, report.reason, \
                report.status, report.admin_note, report.created_at, \
                review.course_id, \
                COALESCE(author.handle::text, review.reviewer_name) AS review_author_handle, \
                review.rating AS review_rating, review.status::text AS review_status, \
                NULLIF(left(COALESCE(review.comment, ''), 1000), '') AS review_excerpt \
         FROM reviews.review_reports report \
         JOIN reviews.reviews review ON review.id = report.review_id \
         LEFT JOIN identity.accounts author ON author.id = review.account_id \
         WHERE report.id = $1",
    )
    .bind(report_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(report_to_dto(updated))
}

/// Claim legacy reviews by linking them to an account.
///
/// Updates all reviews whose `wallet_user_hash` matches and whose `account_id`
/// is still NULL, setting them to the provided `account_id`. Returns the number
/// of reviews that were claimed.
///
/// This is designed to be called after a successful `/wallet/claim` flow so
/// legacy reviews originally associated with an anonymous wallet hash become
/// properly linked to the user's account.
#[tracing::instrument(skip(executor), fields(account_id, wallet_user_hash))]
pub async fn claim_legacy_reviews(
    executor: impl sqlx::PgExecutor<'_>,
    account_id: i64,
    wallet_user_hash: &str,
) -> AppResult<u64> {
    let rows = sqlx::query(
        "UPDATE reviews.reviews SET account_id = $1 \
         WHERE wallet_user_hash = $2 AND account_id IS NULL",
    )
    .bind(account_id)
    .bind(wallet_user_hash)
    .execute(executor)
    .await?;

    let count = rows.rows_affected();
    tracing::info!(account_id, wallet_user_hash, count, "claimed legacy reviews");
    Ok(count)
}

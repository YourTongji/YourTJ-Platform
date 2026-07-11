//! Review-owned projection for account export and private-data purge.

use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewsExport {
    authored_reviews: Vec<ExportReview>,
    liked_review_ids: Vec<String>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportReview {
    id: i64,
    course_id: i64,
    rating: i32,
    comment: Option<String>,
    score: Option<String>,
    semester: Option<String>,
    status: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    updated_at: DateTime<Utc>,
}

pub async fn snapshot(pool: &PgPool, account_id: i64) -> AppResult<ReviewsExport> {
    let authored_reviews = sqlx::query_as::<_, ExportReview>(
        "SELECT id, course_id, rating, comment, score, semester, status::text AS status, \
                created_at, updated_at FROM reviews.reviews WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    let liked_review_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT review_id FROM reviews.review_likes WHERE account_id = $1 ORDER BY review_id",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;
    Ok(ReviewsExport {
        authored_reviews,
        liked_review_ids: liked_review_ids.into_iter().map(|id| id.to_string()).collect(),
    })
}

/// Remove mutable owner-only review projections while preserving authored public content and reports.
pub async fn purge_account_private_data(pool: &PgPool, account_id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE reviews.reviews review \
         SET approve_count = GREATEST(review.approve_count - 1, 0) \
         WHERE review.id IN (SELECT review_id FROM reviews.review_likes WHERE account_id = $1)",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM reviews.review_likes WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM reviews.review_create_idempotency WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

//! Review search indexing and public result reconstruction.

use std::collections::HashMap;

use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

#[derive(Debug, FromRow)]
struct ReviewSearchRow {
    id: i64,
    course_id: i64,
    rating: i32,
    comment: Option<String>,
    approve_count: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Canonical visible review result returned by federated search.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSearchHit {
    pub id: String,
    pub course_id: String,
    pub course_name: String,
    pub rating: i32,
    pub comment: Option<String>,
    pub approve_count: i32,
    pub created_at: i64,
}

impl ReviewSearchHit {
    fn from_row(row: ReviewSearchRow, course_name: String) -> Self {
        Self {
            id: row.id.to_string(),
            course_id: row.course_id.to_string(),
            course_name,
            rating: row.rating,
            comment: row.comment,
            approve_count: row.approve_count,
            created_at: row.created_at.timestamp(),
        }
    }
}

async fn load_visible_rows(pool: &PgPool, review_ids: &[i64]) -> AppResult<Vec<ReviewSearchRow>> {
    if review_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, ReviewSearchRow>(
        "SELECT review.id, review.course_id, review.rating, review.comment, \
                review.approve_count, review.created_at \
         FROM reviews.reviews review \
         WHERE review.id = ANY($1) AND review.status = 'visible'",
    )
    .bind(review_ids)
    .fetch_all(pool)
    .await?)
}

/// Reconstructs ranked review candidates while enforcing current visibility.
pub async fn load_review_hits(
    pool: &PgPool,
    candidate_ids: &[i64],
    limit: usize,
) -> AppResult<Vec<ReviewSearchHit>> {
    if candidate_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let rows = load_visible_rows(pool, candidate_ids).await?;
    let course_ids: Vec<i64> = rows.iter().map(|row| row.course_id).collect();
    let course_hits =
        courses::public_search::load_course_hits(pool, &course_ids, course_ids.len()).await?;
    let mut courses_by_id: HashMap<i64, courses::public_search::CourseSearchHit> = course_hits
        .into_iter()
        .filter_map(|course| course.id.parse().ok().map(|id| (id, course)))
        .collect();
    let mut hits_by_id: HashMap<i64, ReviewSearchHit> = rows
        .into_iter()
        .filter_map(|row| {
            let course = courses_by_id.remove(&row.course_id)?;
            Some((row.id, ReviewSearchHit::from_row(row, course.name)))
        })
        .collect();
    Ok(candidate_ids.iter().filter_map(|id| hits_by_id.remove(id)).take(limit).collect())
}

/// Searches review candidates and rehydrates only currently visible rows.
pub async fn search_reviews(
    pool: &PgPool,
    meili_url: &str,
    meili_key: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<ReviewSearchHit>> {
    let candidate_ids = courses::meili::search_document_ids(
        meili_url,
        meili_key,
        query,
        courses::meili::SearchDocumentKind::Review,
        limit,
    )
    .await?;
    load_review_hits(pool, &candidate_ids, limit).await
}

/// Reconciles one review's search document against its current visibility.
pub async fn sync_search_document(meili_url: &str, meili_key: &str, review_id: i64, pool: &PgPool) {
    let rows = match load_visible_rows(pool, &[review_id]).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(%error, review_id, "failed to build review search document");
            return;
        }
    };
    let document = if let Some(row) = rows.into_iter().next() {
        match courses::public_search::load_course_hits(pool, &[row.course_id], 1).await {
            Ok(mut courses) => courses.pop().map(|course| {
                courses::meili::ReviewDocument::new(row.id, course.code, course.name, row.comment)
            }),
            Err(error) => {
                tracing::warn!(%error, review_id, "failed to load review course for search");
                return;
            }
        }
    } else {
        None
    };
    courses::meili::sync_review_document_to_meili(meili_url, meili_key, review_id, document).await;
}

/// Reconciles every review row without exposing review SQL to the gateway.
pub async fn reindex_search_documents(
    pool: &PgPool,
    meili_url: &str,
    meili_key: &str,
) -> AppResult<usize> {
    let review_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM reviews.reviews ORDER BY id").fetch_all(pool).await?;
    let count = review_ids.len();
    for review_id in review_ids {
        sync_search_document(meili_url, meili_key, review_id, pool).await;
    }
    Ok(count)
}

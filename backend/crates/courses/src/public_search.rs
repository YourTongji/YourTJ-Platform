//! Public course search projection reconstructed from PostgreSQL candidates.

use std::collections::HashMap;

use serde::Serialize;
use shared::AppResult;
use sqlx::{FromRow, PgPool};

use crate::meili::{self, SearchDocumentKind};

#[derive(Debug, FromRow)]
struct CourseSearchRow {
    id: i64,
    code: String,
    name: String,
    credit: Option<f64>,
    department: Option<String>,
    teacher_name: Option<String>,
    review_count: i32,
    review_avg: Option<f64>,
}

/// Canonical course result returned by federated search.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSearchHit {
    pub id: String,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_name: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
}

impl From<CourseSearchRow> for CourseSearchHit {
    fn from(row: CourseSearchRow) -> Self {
        Self {
            id: row.id.to_string(),
            code: row.code,
            name: row.name,
            credit: row.credit,
            department: row.department,
            teacher_name: row.teacher_name,
            review_count: row.review_count,
            review_avg: row.review_avg,
        }
    }
}

/// Reconstructs ranked course candidates from the catalogue source of truth.
pub async fn load_course_hits(
    pool: &PgPool,
    candidate_ids: &[i64],
    limit: usize,
) -> AppResult<Vec<CourseSearchHit>> {
    if candidate_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, CourseSearchRow>(
        "SELECT course.id, course.code, course.name, course.credit, course.department, \
                teacher.name AS teacher_name, course.review_count, \
                CASE WHEN course.review_count > 0 THEN course.review_avg END AS review_avg \
         FROM courses.courses course \
         LEFT JOIN courses.teachers teacher ON teacher.id = course.teacher_id \
         WHERE course.id = ANY($1)",
    )
    .bind(candidate_ids)
    .fetch_all(pool)
    .await?;

    let mut hits_by_id: HashMap<i64, CourseSearchHit> =
        rows.into_iter().map(|row| (row.id, row.into())).collect();
    Ok(candidate_ids.iter().filter_map(|id| hits_by_id.remove(id)).take(limit).collect())
}

/// Searches course candidates and rehydrates them from PostgreSQL.
pub async fn search_courses(
    pool: &PgPool,
    meili_url: &str,
    meili_key: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<CourseSearchHit>> {
    if !meili::projection_is_ready(pool, "catalogue").await? {
        return Err(shared::AppError::ServiceUnavailable);
    }
    let candidate_ids =
        meili::search_document_ids(meili_url, meili_key, query, SearchDocumentKind::Course, limit)
            .await?;
    load_course_hits(pool, &candidate_ids, limit).await
}

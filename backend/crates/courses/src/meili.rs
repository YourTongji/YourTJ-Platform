//! Meilisearch integration for courses search, index setup, and document sync.
//!
//! All functions gracefully degrade when Meilisearch is unreachable:
//! they log a warning and return empty results or skip the operation.

use meilisearch_sdk::client::Client;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

/// Minimal search result returned to clients.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub kind: String,
}

/// Setup the Meilisearch "courses" index with searchable, filterable, and
/// sortable attributes.
pub async fn setup_course_index(url: &str, api_key: &str) -> Result<(), String> {
    let client = Client::new(url, Some(api_key)).map_err(|e| format!("Meili client: {e}"))?;

    let index = client
        .create_index("courses", None)
        .await
        .map_err(|e| format!("Meili create index: {e}"))?;

    // Wait for creation to complete
    let _ = index
        .wait_for_completion(&client, None, None)
        .await
        .map_err(|e| format!("Meili wait: {e}"))?;

    let index = client.index("courses");

    index
        .set_searchable_attributes(&[
            "name",
            "code",
            "pinyin",
            "initials",
            "aliases",
            "teacherName",
            "department",
        ])
        .await
        .map_err(|e| format!("Meili searchable attrs: {e}"))?;

    index
        .set_filterable_attributes(&["department", "kind"])
        .await
        .map_err(|e| format!("Meili filterable attrs: {e}"))?;

    index
        .set_sortable_attributes(&["reviewCount", "reviewAvg"])
        .await
        .map_err(|e| format!("Meili sortable attrs: {e}"))?;

    Ok(())
}

/// Document shape indexed in Meilisearch.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseDocument {
    pub id: String,
    pub name: String,
    pub code: String,
    pub pinyin: Option<String>,
    pub initials: Option<String>,
    pub aliases: Vec<String>,
    pub teacher_name: Option<String>,
    pub department: Option<String>,
    pub credit: Option<f64>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
    pub kind: String,
}

/// Sync a course to Meilisearch by id. Call via `tokio::spawn`.
pub async fn sync_course_to_meili(url: &str, api_key: &str, course_id: i64, pool: &PgPool) {
    let doc = match build_course_document(course_id, pool).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            tracing::warn!(course_id, "course not found for Meili sync");
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, course_id, "failed to build course doc for Meili sync");
            return;
        }
    };

    let record = serde_json::to_value(&doc).unwrap_or_default();

    match Client::new(url, Some(api_key)) {
        Ok(client) => {
            let index = client.index("courses");
            if let Err(e) = index.add_documents(&[record], Some("id")).await {
                tracing::warn!(error = %e, "Meili add_documents failed");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Meili client creation failed during course sync");
        }
    }
}

/// Search both courses and reviews in Meilisearch. Returns empty Vec on failure.
pub async fn search_courses_and_reviews(
    url: &str,
    api_key: &str,
    q: &str,
    limit: usize,
) -> Vec<SearchResult> {
    let client = match Client::new(url, Some(api_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Meili client failed — search returning empty");
            return Vec::new();
        }
    };

    let index = client.index("courses");
    match index.search().with_query(q).with_limit(limit).execute::<SearchResult>().await {
        Ok(results) => results.hits.into_iter().map(|h| h.result).collect(),
        Err(e) => {
            tracing::warn!(error = %e, query = %q, "Meili search failed — returning empty");
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Selection course index
// ---------------------------------------------------------------------------

/// Setup the Meilisearch "selection_courses" index with searchable attributes.
pub async fn setup_selection_index(url: &str, api_key: &str) -> Result<(), String> {
    let client = Client::new(url, Some(api_key)).map_err(|e| format!("Meili client: {e}"))?;

    match client.create_index("selection_courses", None).await {
        Ok(index) => {
            let _ = index.wait_for_completion(&client, None, None).await;
        }
        Err(e) if e.to_string().contains("index_already_exists") => {
            // Index already exists — continue
        }
        Err(e) => return Err(format!("Meili create index: {e}")),
    }

    let index = client.index("selection_courses");
    index
        .set_searchable_attributes(&["code", "name", "teacherName"])
        .await
        .map_err(|e| format!("Meili searchable attrs: {e}"))?;

    index
        .set_filterable_attributes(&["natureId", "campusId"])
        .await
        .map_err(|e| format!("Meili filterable attrs: {e}"))?;

    Ok(())
}

/// Document shape for a selection course in Meilisearch.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCourseDocument {
    pub id: String,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub nature_id: Option<i64>,
    pub campus_id: Option<i64>,
    pub teacher_name: Option<String>,
    pub kind: String,
}

/// Search selection courses via Meilisearch. Returns empty Vec on failure.
pub async fn search_selection_courses(
    url: &str,
    api_key: &str,
    q: &str,
    limit: usize,
) -> Vec<SelectionCourseDocument> {
    let client = match Client::new(url, Some(api_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Meili client failed — selection search returning empty");
            return Vec::new();
        }
    };

    let index = client.index("selection_courses");
    match index.search().with_query(q).with_limit(limit).execute::<SelectionCourseDocument>().await
    {
        Ok(results) => results.hits.into_iter().map(|h| h.result).collect(),
        Err(e) => {
            tracing::warn!(error = %e, query = %q, "Meili selection search failed");
            Vec::new()
        }
    }
}

/// Row type for selection course sync.
#[derive(Debug, FromRow)]
struct SelectionCourseRow {
    id: i64,
    code: String,
    name: String,
    credit: Option<f64>,
    nature_id: Option<i64>,
    campus_id: Option<i64>,
    teacher_name: Option<String>,
}

/// Sync all selection courses to Meilisearch.
pub async fn sync_selection_courses_to_meili(url: &str, api_key: &str, pool: &PgPool) {
    let rows: Vec<SelectionCourseRow> = match sqlx::query_as::<_, SelectionCourseRow>(
        "SELECT id, code, name, credit, nature_id, campus_id, teacher_name \
         FROM selection.courses ORDER BY id",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to fetch selection courses for Meili sync");
            return;
        }
    };

    let docs: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "code": r.code,
                "name": r.name,
                "credit": r.credit,
                "natureId": r.nature_id,
                "campusId": r.campus_id,
                "teacherName": r.teacher_name,
                "kind": "selection_course",
            })
        })
        .collect();

    let client = match Client::new(url, Some(api_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Meili client failed during selection sync");
            return;
        }
    };

    if let Err(e) = client.index("selection_courses").add_documents(&docs, Some("id")).await {
        tracing::warn!(error = %e, "Meili add_documents failed for selection sync");
    }
}

// ---------------------------------------------------------------------------
// Internal row types & doc builders
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct CourseSyncRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
    pub name_pinyin: Option<String>,
    pub name_initials: Option<String>,
    pub teacher_name: Option<String>,
}

async fn build_course_document(
    course_id: i64,
    pool: &PgPool,
) -> Result<Option<CourseDocument>, sqlx::Error> {
    let row = sqlx::query_as::<_, CourseSyncRow>(
        "SELECT c.id, c.code, c.name, c.credit, c.department, \
         c.review_count, c.review_avg, c.name_pinyin, c.name_initials, \
         t.name AS teacher_name \
         FROM courses.courses c \
         LEFT JOIN courses.teachers t ON c.teacher_id = t.id \
         WHERE c.id = $1",
    )
    .bind(course_id)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let aliases: Vec<(String,)> =
        sqlx::query_as("SELECT alias FROM courses.course_aliases WHERE course_id = $1")
            .bind(course_id)
            .fetch_all(pool)
            .await?;

    let aliases: Vec<String> = aliases.into_iter().map(|(a,)| a).collect();

    Ok(Some(CourseDocument {
        id: format!("course:{course_id}"),
        name: row.name,
        code: row.code,
        pinyin: row.name_pinyin,
        initials: row.name_initials,
        aliases,
        teacher_name: row.teacher_name,
        department: row.department,
        credit: row.credit,
        review_count: row.review_count,
        review_avg: row.review_avg,
        kind: "course".into(),
    }))
}

/// Sync a review to Meilisearch.
pub async fn sync_review_to_meili(url: &str, api_key: &str, review_id: i64, pool: &PgPool) {
    let record = match build_review_document(review_id, pool).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            tracing::warn!(review_id, "review not found for Meili sync");
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, review_id, "failed to build review doc for Meili sync");
            return;
        }
    };

    match Client::new(url, Some(api_key)) {
        Ok(client) => {
            let index = client.index("courses");
            if let Err(e) = index.add_documents(&[record], Some("id")).await {
                tracing::warn!(error = %e, review_id, "Meili add_documents failed");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Meili client creation failed during review sync");
        }
    }
}

async fn build_review_document(
    review_id: i64,
    pool: &PgPool,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    #[derive(Debug, sqlx::FromRow)]
    #[allow(dead_code)]
    struct ReviewSyncRow {
        pub id: i64,
        pub comment: Option<String>,
        pub course_name: String,
        pub course_code: String,
    }

    let row = sqlx::query_as::<_, ReviewSyncRow>(
        "SELECT r.id, r.comment, c.name AS course_name, c.code AS course_code \
         FROM reviews.reviews r \
         JOIN courses.courses c ON c.id = r.course_id \
         WHERE r.id = $1",
    )
    .bind(review_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(serde_json::json!({
            "id": format!("review:{review_id}"),
            "name": format!("Review: {}", r.course_name),
            "code": r.course_code,
            "courseName": r.course_name,
            "kind": "review",
        }))),
        None => Ok(None),
    }
}

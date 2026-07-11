//! Meilisearch integration for courses search, index setup, and document sync.
//!
//! Background synchronization degrades safely when Meilisearch is unreachable.
//! Federated candidate reads return an error so callers can distinguish an
//! unavailable section from a genuine empty result.

use std::collections::HashSet;
use std::time::Duration;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::documents::DocumentDeletionQuery;
use meilisearch_sdk::errors::{Error as MeiliError, ErrorCode};
use meilisearch_sdk::task_info::TaskInfo;
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgPool};

const TASK_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TASK_TIMEOUT: Duration = Duration::from_secs(120);

fn meili_api_key(api_key: &str) -> Option<&str> {
    let api_key = api_key.trim();
    (!api_key.is_empty()).then_some(api_key)
}

fn is_index_not_found(error: &MeiliError) -> bool {
    matches!(
        error,
        MeiliError::Meilisearch(error) if error.error_code == ErrorCode::IndexNotFound
    )
}

async fn wait_for_task(
    client: &Client,
    task: TaskInfo,
    operation: &'static str,
) -> Result<(), String> {
    let task = task
        .wait_for_completion(client, Some(TASK_POLL_INTERVAL), Some(TASK_TIMEOUT))
        .await
        .map_err(|error| format!("{operation}: {error}"))?;
    if task.is_failure() {
        return Err(format!("{operation}: {}", task.unwrap_failure()));
    }
    if !task.is_success() {
        return Err(format!("{operation}: task did not finish successfully"));
    }
    Ok(())
}

fn meili_app_failure(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::warn!(%error, operation, "course meilisearch operation failed");
    AppError::Internal(anyhow::anyhow!("course meilisearch {operation} failed"))
}

/// Document category stored in the shared catalogue index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDocumentKind {
    Course,
    Review,
}

impl SearchDocumentKind {
    fn value(self) -> &'static str {
        match self {
            Self::Course => "course",
            Self::Review => "review",
        }
    }

    fn document_prefix(self) -> &'static str {
        match self {
            Self::Course => "course-",
            Self::Review => "review-",
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchCandidate {
    id: String,
}

/// Setup the Meilisearch "courses" index with searchable, filterable, and
/// sortable attributes.
pub async fn setup_course_index(url: &str, api_key: &str) -> Result<(), String> {
    let client =
        Client::new(url, meili_api_key(api_key)).map_err(|e| format!("Meili client: {e}"))?;

    match client.get_index("courses").await {
        Ok(_) => {}
        Err(error) if is_index_not_found(&error) => {
            let task = client
                .create_index("courses", Some("id"))
                .await
                .map_err(|error| format!("course index creation: {error}"))?;
            wait_for_task(&client, task, "course index creation").await?;
        }
        Err(error) => return Err(format!("course index lookup: {error}")),
    }

    let index = client.index("courses");

    let searchable_task = index
        .set_searchable_attributes(&[
            "name",
            "code",
            "pinyin",
            "initials",
            "aliases",
            "teacherName",
            "department",
            "courseName",
            "comment",
        ])
        .await
        .map_err(|error| format!("course searchable attributes: {error}"))?;
    wait_for_task(&client, searchable_task, "course searchable attributes").await?;

    let filterable_task = index
        .set_filterable_attributes(&["department", "kind"])
        .await
        .map_err(|error| format!("course filterable attributes: {error}"))?;
    wait_for_task(&client, filterable_task, "course filterable attributes").await?;

    let sortable_task = index
        .set_sortable_attributes(&["reviewCount", "reviewAvg"])
        .await
        .map_err(|error| format!("course sortable attributes: {error}"))?;
    wait_for_task(&client, sortable_task, "course sortable attributes").await?;

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

/// Reconciles one course document and confirms the index task completed.
pub async fn reconcile_course_document(
    url: &str,
    api_key: &str,
    course_id: i64,
    pool: &PgPool,
) -> AppResult<()> {
    let document = build_course_document(course_id, pool).await?;
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_app_failure("client creation", error))?;
    let index = client.index("courses");
    let task = match document {
        Some(document) => index
            .add_documents(&[document], Some("id"))
            .await
            .map_err(|error| meili_app_failure("course upsert enqueue", error))?,
        None => index
            .delete_document(format!("course-{course_id}"))
            .await
            .map_err(|error| meili_app_failure("course deletion enqueue", error))?,
    };
    wait_for_task(&client, task, "course reconciliation")
        .await
        .map_err(|error| meili_app_failure("course reconciliation", error))
}

/// Reconciles one course after its database transaction commits.
pub fn reconcile_course_in_background(state: &AppState, course_id: i64) {
    let pool = state.db.clone();
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    tokio::spawn(async move {
        if let Err(error) =
            reconcile_course_document(&meili_url, &meili_key, course_id, &pool).await
        {
            tracing::warn!(%error, course_id, "course search reconciliation failed");
        }
    });
}

/// Replaces every course document while preserving review documents in the shared index.
pub async fn reindex_course_documents(pool: &PgPool, url: &str, api_key: &str) -> AppResult<usize> {
    let course_ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM courses.courses ORDER BY id").fetch_all(pool).await?;
    let mut documents = Vec::with_capacity(course_ids.len());
    for course_id in course_ids {
        if let Some(document) = build_course_document(course_id, pool).await? {
            documents.push(document);
        }
    }

    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_app_failure("client creation", error))?;
    let index = client.index("courses");
    let mut deletion = DocumentDeletionQuery::new(&index);
    let deletion_task = deletion
        .with_filter("kind = course")
        .execute::<serde_json::Value>()
        .await
        .map_err(|error| meili_app_failure("course index clear enqueue", error))?;
    wait_for_task(&client, deletion_task, "course index clear")
        .await
        .map_err(|error| meili_app_failure("course index clear", error))?;

    if !documents.is_empty() {
        let addition_task = index
            .add_documents(&documents, Some("id"))
            .await
            .map_err(|error| meili_app_failure("course index addition enqueue", error))?;
        wait_for_task(&client, addition_task, "course index addition")
            .await
            .map_err(|error| meili_app_failure("course index addition", error))?;
    }
    Ok(documents.len())
}

fn ranked_candidate_ids(
    candidates: impl IntoIterator<Item = SearchCandidate>,
    kind: SearchDocumentKind,
) -> Vec<i64> {
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter_map(|candidate| {
            let id = candidate.id.strip_prefix(kind.document_prefix())?.parse::<i64>().ok()?;
            (id > 0 && seen.insert(id)).then_some(id)
        })
        .collect()
}

/// Returns ranked candidate IDs for one document kind.
///
/// Callers must reconstruct every result from their owning database table before
/// exposing it; Meilisearch is never the visibility authority.
pub async fn search_document_ids(
    url: &str,
    api_key: &str,
    q: &str,
    kind: SearchDocumentKind,
    limit: usize,
) -> AppResult<Vec<i64>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_app_failure("search client creation", error))?;

    let filter = format!("kind = {}", kind.value());
    let candidate_limit = limit.saturating_mul(4).min(1_000);
    let results = client
        .index("courses")
        .search()
        .with_query(q)
        .with_filter(&filter)
        .with_limit(candidate_limit)
        .execute::<SearchCandidate>()
        .await
        .map_err(|error| meili_app_failure("candidate search", error))?;
    Ok(ranked_candidate_ids(results.hits.into_iter().map(|hit| hit.result), kind))
}

// ---------------------------------------------------------------------------
// Selection course index
// ---------------------------------------------------------------------------

/// Setup the Meilisearch "selection_courses" index with searchable attributes.
pub async fn setup_selection_index(url: &str, api_key: &str) -> Result<(), String> {
    let client =
        Client::new(url, meili_api_key(api_key)).map_err(|e| format!("Meili client: {e}"))?;

    match client.get_index("selection_courses").await {
        Ok(_) => {}
        Err(error) if is_index_not_found(&error) => {
            let task = client
                .create_index("selection_courses", Some("id"))
                .await
                .map_err(|error| format!("selection index creation: {error}"))?;
            wait_for_task(&client, task, "selection index creation").await?;
        }
        Err(error) => return Err(format!("selection index lookup: {error}")),
    }

    let index = client.index("selection_courses");
    let searchable_task = index
        .set_searchable_attributes(&["code", "name", "teacherName", "teacherNames"])
        .await
        .map_err(|error| format!("selection searchable attributes: {error}"))?;
    wait_for_task(&client, searchable_task, "selection searchable attributes").await?;

    let filterable_task = index
        .set_filterable_attributes(&["natureId", "campusId"])
        .await
        .map_err(|error| format!("selection filterable attributes: {error}"))?;
    wait_for_task(&client, filterable_task, "selection filterable attributes").await?;

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
    pub teacher_names: Option<Vec<String>>,
    pub kind: String,
}

/// Search selection courses via Meilisearch. Returns empty Vec on failure.
pub async fn search_selection_courses(
    url: &str,
    api_key: &str,
    q: &str,
    limit: usize,
) -> Vec<SelectionCourseDocument> {
    let client = match Client::new(url, meili_api_key(api_key)) {
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
    teacher_names: Option<Vec<String>>,
}

/// Sync all selection courses to Meilisearch.
pub async fn sync_selection_courses_to_meili(url: &str, api_key: &str, pool: &PgPool) {
    let rows: Vec<SelectionCourseRow> = match sqlx::query_as::<_, SelectionCourseRow>(
        "SELECT id, code, name, credit, nature_id, campus_id, teacher_name, teacher_names \
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
                "teacherNames": r.teacher_names.unwrap_or_default(),
                "kind": "selection_course",
            })
        })
        .collect();

    let client = match Client::new(url, meili_api_key(api_key)) {
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
        id: format!("course-{course_id}"),
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

/// Review document accepted by the shared catalogue index.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDocument {
    id: String,
    name: String,
    code: String,
    course_name: String,
    comment: Option<String>,
    kind: String,
}

impl ReviewDocument {
    /// Builds the stable review document owned by the catalogue index.
    pub fn new(
        review_id: i64,
        course_code: String,
        course_name: String,
        comment: Option<String>,
    ) -> Self {
        Self {
            id: format!("review-{review_id}"),
            name: format!("Review: {course_name}"),
            code: course_code,
            course_name,
            comment,
            kind: "review".into(),
        }
    }
}

/// Upserts or removes a review document supplied by the reviews domain.
pub async fn sync_review_document_to_meili(
    url: &str,
    api_key: &str,
    review_id: i64,
    document: Option<ReviewDocument>,
) {
    match Client::new(url, meili_api_key(api_key)) {
        Ok(client) => {
            let index = client.index("courses");
            let result = match document {
                Some(document) => index.add_documents(&[document], Some("id")).await,
                None => index.delete_document(format!("review-{review_id}")).await,
            };
            if let Err(error) = result {
                tracing::warn!(%error, review_id, "review search document reconciliation failed");
            }
        }
        Err(error) => {
            tracing::warn!(%error, review_id, "review search client creation failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ranked_candidate_ids, SearchCandidate, SearchDocumentKind};

    #[test]
    fn candidate_ids_require_the_requested_prefix_and_remain_ranked() {
        let candidates = [
            SearchCandidate { id: "course-9".into() },
            SearchCandidate { id: "review-8".into() },
            SearchCandidate { id: "course-9".into() },
            SearchCandidate { id: "course-3".into() },
            SearchCandidate { id: "course-invalid".into() },
        ];

        assert_eq!(ranked_candidate_ids(candidates, SearchDocumentKind::Course), vec![9, 3]);
    }
}

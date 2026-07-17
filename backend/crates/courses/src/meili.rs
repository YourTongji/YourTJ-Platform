//! Meilisearch integration for courses search, index setup, and document sync.
//!
//! Search reads fail as unavailable when Meilisearch or the corresponding
//! PostgreSQL-tracked projection is not ready. Index setup and synchronization
//! propagate failures so an operator cannot mistake an empty or failed reindex
//! for a valid zero-result search.
//! Federated candidate reads return an error so callers can distinguish an
//! unavailable section from a genuine empty result.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::documents::DocumentDeletionQuery;
use meilisearch_sdk::errors::{Error as MeiliError, ErrorCode};
use meilisearch_sdk::task_info::TaskInfo;
use serde::{Deserialize, Serialize};
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const TASK_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TASK_TIMEOUT: Duration = Duration::from_secs(120);
const PROJECTION_RECONCILE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, FromRow)]
struct SearchProjectionState {
    source_generation: i64,
    indexed_generation: Option<i64>,
    source_rows: i64,
    indexed_rows: Option<i64>,
    status: String,
}

/// Database-backed generation fence for a durable selection sync worker.
pub(crate) struct SelectionSyncFence<'a> {
    pool: &'a PgPool,
    job_id: Uuid,
    lease_token: Uuid,
}

impl<'a> SelectionSyncFence<'a> {
    pub(crate) fn new(pool: &'a PgPool, job_id: Uuid, lease_token: Uuid) -> Self {
        Self { pool, job_id, lease_token }
    }

    pub(crate) async fn assert_current(&self) -> AppResult<()> {
        let is_current: bool = sqlx::query_scalar(
            "SELECT EXISTS (\
                 SELECT 1 FROM selection.sync_jobs \
                 WHERE id = $1 AND status = 'running' AND lease_token = $2 \
                   AND lease_expires_at > now()\
             )",
        )
        .bind(self.job_id)
        .bind(self.lease_token)
        .fetch_one(self.pool)
        .await?;
        if !is_current {
            return Err(AppError::Conflict("selection sync job lease was lost".into()));
        }
        Ok(())
    }
}

async fn assert_sync_fence(fence: Option<&SelectionSyncFence<'_>>) -> AppResult<()> {
    if let Some(fence) = fence {
        fence.assert_current().await?;
    }
    Ok(())
}

async fn projection_state(pool: &PgPool, projection: &str) -> AppResult<SearchProjectionState> {
    Ok(sqlx::query_as::<_, SearchProjectionState>(
        "SELECT source_generation, indexed_generation, source_rows, indexed_rows, status \
         FROM courses.search_projection_state WHERE projection = $1",
    )
    .bind(projection)
    .fetch_one(pool)
    .await?)
}

async fn reconcile_projection_source_rows(pool: &PgPool, projection: &str) -> AppResult<()> {
    let source_rows: i64 = match projection {
        "catalogue" => {
            sqlx::query_scalar("SELECT COUNT(*) FROM courses.courses").fetch_one(pool).await?
        }
        "selection" => {
            sqlx::query_scalar("SELECT COUNT(*) FROM selection.courses").fetch_one(pool).await?
        }
        _ => return Err(AppError::NotFound),
    };
    sqlx::query(
        "UPDATE courses.search_projection_state \
         SET source_generation = source_generation + 1, source_rows = $2, \
             indexed_generation = NULL, indexed_rows = NULL, status = 'stale', \
             updated_at = now() \
         WHERE projection = $1 AND source_rows <> $2",
    )
    .bind(projection)
    .bind(source_rows)
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_projection_rebuilding(pool: &PgPool, projection: &str) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "UPDATE courses.search_projection_state \
         SET status = 'rebuilding', indexed_generation = NULL, indexed_rows = NULL, \
             updated_at = now() \
         WHERE projection = $1 RETURNING source_generation",
    )
    .bind(projection)
    .fetch_one(pool)
    .await?)
}

async fn mark_projection_ready(
    pool: &PgPool,
    projection: &str,
    source_generation: i64,
    indexed_rows: usize,
) -> AppResult<()> {
    let indexed_rows = i64::try_from(indexed_rows)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    let affected = sqlx::query(
        "UPDATE courses.search_projection_state \
         SET indexed_generation = $3, indexed_rows = $2, status = 'ready', \
             updated_at = now() \
         WHERE projection = $1 AND source_rows = $2 AND source_generation = $3",
    )
    .bind(projection)
    .bind(indexed_rows)
    .bind(source_generation)
    .execute(pool)
    .await?
    .rows_affected();
    if affected != 1 {
        return Err(AppError::Conflict(format!(
            "{projection} search source changed during reindex"
        )));
    }
    Ok(())
}

/// Returns whether a PostgreSQL source generation has a complete search projection.
pub async fn projection_is_ready(pool: &PgPool, projection: &str) -> AppResult<bool> {
    let state = projection_state(pool, projection).await?;
    Ok(state.status == "ready"
        && state.indexed_generation == Some(state.source_generation)
        && state.indexed_rows == Some(state.source_rows))
}

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

fn meili_read_unavailable(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::warn!(%error, operation, "course meilisearch read is unavailable");
    AppError::ServiceUnavailable
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
    reindex_course_documents_inner(pool, url, api_key, None).await
}

pub(crate) async fn reindex_course_documents_fenced(
    pool: &PgPool,
    url: &str,
    api_key: &str,
    fence: &SelectionSyncFence<'_>,
) -> AppResult<usize> {
    reindex_course_documents_inner(pool, url, api_key, Some(fence)).await
}

async fn reindex_course_documents_inner(
    pool: &PgPool,
    url: &str,
    api_key: &str,
    fence: Option<&SelectionSyncFence<'_>>,
) -> AppResult<usize> {
    reconcile_projection_source_rows(pool, "catalogue").await?;
    let source_generation = mark_projection_rebuilding(pool, "catalogue").await?;
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
    assert_sync_fence(fence).await?;
    let mut deletion = DocumentDeletionQuery::new(&index);
    let deletion_task = deletion
        .with_filter("kind = course")
        .execute::<serde_json::Value>()
        .await
        .map_err(|error| meili_app_failure("course index clear enqueue", error))?;
    wait_for_task(&client, deletion_task, "course index clear")
        .await
        .map_err(|error| meili_app_failure("course index clear", error))?;
    assert_sync_fence(fence).await?;

    if !documents.is_empty() {
        assert_sync_fence(fence).await?;
        let addition_task = index
            .add_documents(&documents, Some("id"))
            .await
            .map_err(|error| meili_app_failure("course index addition enqueue", error))?;
        wait_for_task(&client, addition_task, "course index addition")
            .await
            .map_err(|error| meili_app_failure("course index addition", error))?;
        assert_sync_fence(fence).await?;
    }
    mark_projection_ready(pool, "catalogue", source_generation, documents.len()).await?;
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
        .map_err(|error| meili_read_unavailable("catalogue search client creation", error))?;

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
        .map_err(|error| meili_read_unavailable("catalogue candidate search", error))?;
    Ok(ranked_candidate_ids(results.hits.into_iter().map(|hit| hit.result), kind))
}

// ---------------------------------------------------------------------------
// Selection course index
// ---------------------------------------------------------------------------

/// Setup the dedicated teaching-class offering index.
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
        .set_searchable_attributes(&[
            "code",
            "teachingClassCode",
            "name",
            "pinyin",
            "initials",
            "teacherName",
            "teacherNames",
            "teacherPinyin",
            "teacherInitials",
        ])
        .await
        .map_err(|error| format!("selection searchable attributes: {error}"))?;
    wait_for_task(&client, searchable_task, "selection searchable attributes").await?;

    let filterable_task = index
        .set_filterable_attributes(&[
            "calendarId",
            "natureId",
            "campusId",
            "courseCode",
            "majorIds",
            "grades",
            "scheduleUnknown",
            "status",
            "slotKeys",
        ])
        .await
        .map_err(|error| format!("selection filterable attributes: {error}"))?;
    wait_for_task(&client, filterable_task, "selection filterable attributes").await?;

    Ok(())
}

async fn course_index_document_count(url: &str, api_key: &str) -> AppResult<usize> {
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_read_unavailable("catalogue readiness client", error))?;
    let index = client.index("courses");
    let mut search = index.search();
    search.with_query("").with_filter("kind = course").with_limit(1);
    let result = search
        .execute::<SearchCandidate>()
        .await
        .map_err(|error| meili_read_unavailable("catalogue readiness count", error))?;
    Ok(result.estimated_total_hits.or(result.total_hits).unwrap_or(result.hits.len()))
}

async fn selection_index_document_count(url: &str, api_key: &str) -> AppResult<usize> {
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_read_unavailable("selection readiness client", error))?;
    let stats = client
        .index("selection_courses")
        .get_stats()
        .await
        .map_err(|error| meili_read_unavailable("selection readiness count", error))?;
    Ok(stats.number_of_documents)
}

/// Rebuilds configured indexes when their stored generation or live document count is stale.
pub async fn reconcile_search_projections(
    pool: &PgPool,
    url: &str,
    api_key: &str,
) -> AppResult<()> {
    setup_course_index(url, api_key)
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;
    setup_selection_index(url, api_key)
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;

    reconcile_projection_source_rows(pool, "catalogue").await?;
    let catalogue = projection_state(pool, "catalogue").await?;
    let indexed_catalogue = course_index_document_count(url, api_key).await?;
    let indexed_catalogue = i64::try_from(indexed_catalogue)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    let catalogue_ready = catalogue.status == "ready"
        && catalogue.indexed_generation == Some(catalogue.source_generation)
        && catalogue.indexed_rows == Some(catalogue.source_rows)
        && indexed_catalogue == catalogue.source_rows;
    if !catalogue_ready {
        reindex_course_documents(pool, url, api_key).await?;
    }

    reconcile_projection_source_rows(pool, "selection").await?;
    let selection = projection_state(pool, "selection").await?;
    let indexed_selection = selection_index_document_count(url, api_key).await?;
    let indexed_selection = i64::try_from(indexed_selection)
        .map_err(|error| AppError::Internal(anyhow::Error::new(error)))?;
    let selection_ready = selection.status == "ready"
        && selection.indexed_generation == Some(selection.source_generation)
        && selection.indexed_rows == Some(selection.source_rows)
        && indexed_selection == selection.source_rows;
    if !selection_ready {
        sync_selection_courses_to_meili(url, api_key, pool).await?;
    }
    Ok(())
}

/// Periodically detects external index loss and reconciles from PostgreSQL.
pub async fn run_search_projection_reconciler(state: AppState) {
    loop {
        let has_active_sync: Result<bool, sqlx::Error> = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM selection.sync_jobs \
             WHERE status IN ('queued', 'running'))",
        )
        .fetch_one(&state.db)
        .await;
        match has_active_sync {
            Ok(false) => {
                if let Err(error) = reconcile_search_projections(
                    &state.db,
                    &state.meili_url,
                    &state.meili_master_key,
                )
                .await
                {
                    tracing::warn!(?error, "course search projection reconciliation failed");
                }
            }
            Ok(true) => {}
            Err(error) => {
                tracing::warn!(?error, "course search projection readiness check failed");
            }
        }
        tokio::time::sleep(PROJECTION_RECONCILE_INTERVAL).await;
    }
}

/// Document shape for a selection course in Meilisearch.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCourseDocument {
    pub id: String,
    pub code: String,
    pub course_code: String,
    pub teaching_class_code: Option<String>,
    pub name: String,
    pub pinyin: String,
    pub initials: String,
    pub credit: Option<f64>,
    pub nature_id: Option<i64>,
    pub calendar_id: i64,
    pub campus_id: Option<i64>,
    pub teacher_name: Option<String>,
    pub teacher_names: Vec<String>,
    pub teacher_pinyin: String,
    pub teacher_initials: String,
    pub major_ids: Vec<i64>,
    pub grades: Vec<String>,
    pub schedule_unknown: bool,
    pub status: String,
    pub slot_keys: Vec<String>,
}

fn quote_meili_filter(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn selection_filter_expression(filter: &crate::selection_repo::OfferingFilter) -> Option<String> {
    let mut clauses = Vec::new();
    if let Some(value) = filter.calendar_id {
        clauses.push(format!("calendarId = {value}"));
    }
    if let Some(value) = filter.major_id {
        clauses.push(format!("majorIds = {value}"));
    }
    if let Some(value) = filter.grade.as_deref() {
        clauses.push(format!("grades = {}", quote_meili_filter(value)));
    }
    if let Some(value) = filter.nature_id {
        clauses.push(format!("natureId = {value}"));
    }
    if let Some(value) = filter.campus_id {
        clauses.push(format!("campusId = {value}"));
    }
    if let Some(value) = filter.course_code.as_deref() {
        clauses.push(format!("courseCode = {}", quote_meili_filter(value)));
    }
    if let (Some(weekday), Some(start_slot), Some(end_slot)) =
        (filter.weekday, filter.start_slot, filter.end_slot)
    {
        let mut keys = Vec::new();
        for slot in start_slot..=end_slot {
            if let Some(week) = filter.week {
                keys.push(quote_meili_filter(&format!("d{weekday}s{slot}w{week}")));
                if filter.include_unknown_schedule {
                    keys.push(quote_meili_filter(&format!("d{weekday}s{slot}wu")));
                }
            } else {
                keys.push(quote_meili_filter(&format!("d{weekday}s{slot}")));
                if filter.include_unknown_schedule {
                    keys.push(quote_meili_filter(&format!("d{weekday}s{slot}wu")));
                }
            }
        }
        let slot_clause = format!("slotKeys IN [{}]", keys.join(", "));
        clauses.push(if filter.include_unknown_schedule {
            format!("({slot_clause} OR scheduleUnknown = true)")
        } else {
            format!("(scheduleUnknown = false AND {slot_clause})")
        });
    }
    (!clauses.is_empty()).then(|| clauses.join(" AND "))
}

pub struct SelectionSearchCandidates {
    pub ids: Vec<i64>,
    pub consumed_through: Vec<usize>,
    pub consumed: usize,
    pub has_more: bool,
}

fn ranked_selection_candidate_ids(
    candidates: impl IntoIterator<Item = SearchCandidate>,
) -> (Vec<i64>, Vec<usize>) {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    let mut consumed_through = Vec::new();
    for (position, candidate) in candidates.into_iter().enumerate() {
        let Ok(id) = candidate.id.parse::<i64>() else {
            continue;
        };
        if id > 0 && seen.insert(id) {
            ids.push(id);
            consumed_through.push(position + 1);
        }
    }
    (ids, consumed_through)
}

/// Return ranked candidate offering ids. PostgreSQL rehydration is mandatory.
pub async fn search_selection_offering_ids(
    url: &str,
    api_key: &str,
    q: &str,
    filter: &crate::selection_repo::OfferingFilter,
    offset: usize,
    limit: usize,
) -> AppResult<SelectionSearchCandidates> {
    if limit == 0 {
        return Ok(SelectionSearchCandidates {
            ids: Vec::new(),
            consumed_through: Vec::new(),
            consumed: 0,
            has_more: false,
        });
    }
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_read_unavailable("selection search client creation", error))?;
    let index = client.index("selection_courses");
    let mut search = index.search();
    search.with_query(q).with_offset(offset).with_limit(limit);
    let filter_expression = selection_filter_expression(filter);
    if let Some(expression) = filter_expression.as_deref() {
        search.with_filter(expression);
    }
    let results = search
        .execute::<SearchCandidate>()
        .await
        .map_err(|error| meili_read_unavailable("selection candidate search", error))?;
    let consumed = results.hits.len();
    let total_hits = results.estimated_total_hits.or(results.total_hits);
    let has_more = total_hits
        .map(|total| offset.saturating_add(consumed) < total)
        .unwrap_or(consumed == limit);
    let (ids, consumed_through) =
        ranked_selection_candidate_ids(results.hits.into_iter().map(|hit| hit.result));
    Ok(SelectionSearchCandidates { ids, consumed_through, consumed, has_more })
}

/// Row type for selection course sync.
#[derive(Debug, FromRow)]
struct SelectionCourseRow {
    id: i64,
    code: String,
    teaching_class_code: Option<String>,
    name: String,
    credit: Option<f64>,
    nature_id: Option<i64>,
    calendar_id: i64,
    campus_id: Option<i64>,
    teacher_name: Option<String>,
    teacher_names: Option<Vec<String>>,
    major_ids: Vec<i64>,
    grades: Vec<String>,
    schedule_unknown: bool,
    status: String,
}

#[derive(Debug, FromRow)]
struct SelectionTimeslotIndexRow {
    course_id: i64,
    weekday: i32,
    start_slot: i32,
    end_slot: i32,
    week_numbers: Vec<i32>,
    weeks_unknown: bool,
}

fn slot_keys(rows: &[SelectionTimeslotIndexRow]) -> Vec<String> {
    let mut keys = HashSet::new();
    for row in rows {
        for slot in row.start_slot..=row.end_slot {
            if row.weeks_unknown {
                keys.insert(format!("d{}s{slot}wu", row.weekday));
            } else {
                keys.insert(format!("d{}s{slot}", row.weekday));
                for week in &row.week_numbers {
                    keys.insert(format!("d{}s{slot}w{week}", row.weekday));
                }
            }
        }
    }
    let mut keys: Vec<String> = keys.into_iter().collect();
    keys.sort();
    keys
}

/// Replace the rebuildable selection index and await every task.
pub async fn sync_selection_courses_to_meili(
    url: &str,
    api_key: &str,
    pool: &PgPool,
) -> AppResult<usize> {
    sync_selection_courses_to_meili_inner(url, api_key, pool, None).await
}

pub(crate) async fn sync_selection_courses_to_meili_fenced(
    url: &str,
    api_key: &str,
    pool: &PgPool,
    fence: &SelectionSyncFence<'_>,
) -> AppResult<usize> {
    sync_selection_courses_to_meili_inner(url, api_key, pool, Some(fence)).await
}

async fn sync_selection_courses_to_meili_inner(
    url: &str,
    api_key: &str,
    pool: &PgPool,
    fence: Option<&SelectionSyncFence<'_>>,
) -> AppResult<usize> {
    reconcile_projection_source_rows(pool, "selection").await?;
    let source_generation = mark_projection_rebuilding(pool, "selection").await?;
    let rows = sqlx::query_as::<_, SelectionCourseRow>(
        "SELECT course.id, course.code, course.teaching_class_code, course.name, course.credit, \
                course.nature_id, course.calendar_id, course.campus_id, course.teacher_name, \
                course.teacher_names, course.schedule_unknown, course.status, \
                ARRAY(SELECT DISTINCT binding.major_id FROM selection.major_courses AS binding \
                      WHERE binding.course_id = course.id ORDER BY binding.major_id) AS major_ids, \
                ARRAY(SELECT DISTINCT binding.grade FROM selection.major_courses AS binding \
                      WHERE binding.course_id = course.id AND binding.grade IS NOT NULL \
                      ORDER BY binding.grade) AS grades \
         FROM selection.courses AS course ORDER BY course.id",
    )
    .fetch_all(pool)
    .await?;
    let time_rows = sqlx::query_as::<_, SelectionTimeslotIndexRow>(
        "SELECT course_id, weekday, start_slot, end_slot, week_numbers, weeks_unknown \
         FROM selection.timeslots ORDER BY course_id, weekday, start_slot, end_slot",
    )
    .fetch_all(pool)
    .await?;
    let mut timeslots: HashMap<i64, Vec<SelectionTimeslotIndexRow>> = HashMap::new();
    for row in time_rows {
        timeslots.entry(row.course_id).or_default().push(row);
    }

    let documents: Vec<SelectionCourseDocument> = rows
        .into_iter()
        .map(|row| {
            let teacher_names = row.teacher_names.unwrap_or_default();
            let (pinyin, initials) = crate::pinyin::to_pinyin(&row.name);
            let teacher_text = teacher_names.join(" ");
            let (teacher_pinyin, teacher_initials) = crate::pinyin::to_pinyin(&teacher_text);
            SelectionCourseDocument {
                id: row.id.to_string(),
                course_code: row.code.clone(),
                code: row.code,
                teaching_class_code: row.teaching_class_code,
                name: row.name,
                pinyin,
                initials,
                credit: row.credit,
                nature_id: row.nature_id,
                calendar_id: row.calendar_id,
                campus_id: row.campus_id,
                teacher_name: row.teacher_name,
                teacher_names,
                teacher_pinyin,
                teacher_initials,
                major_ids: row.major_ids,
                grades: row.grades,
                schedule_unknown: row.schedule_unknown,
                status: row.status,
                slot_keys: slot_keys(timeslots.get(&row.id).map(Vec::as_slice).unwrap_or(&[])),
            }
        })
        .collect();
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_app_failure("selection sync client creation", error))?;
    let index = client.index("selection_courses");
    assert_sync_fence(fence).await?;
    let deletion_task = index
        .delete_all_documents()
        .await
        .map_err(|error| meili_app_failure("selection index clear enqueue", error))?;
    wait_for_task(&client, deletion_task, "selection index clear")
        .await
        .map_err(|error| meili_app_failure("selection index clear", error))?;
    assert_sync_fence(fence).await?;
    for batch in documents.chunks(1_000) {
        assert_sync_fence(fence).await?;
        let addition_task = index
            .add_documents(batch, Some("id"))
            .await
            .map_err(|error| meili_app_failure("selection index addition enqueue", error))?;
        wait_for_task(&client, addition_task, "selection index addition")
            .await
            .map_err(|error| meili_app_failure("selection index addition", error))?;
        assert_sync_fence(fence).await?;
    }
    mark_projection_ready(pool, "selection", source_generation, documents.len()).await?;
    Ok(documents.len())
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
         c.review_count, CASE WHEN c.review_count > 0 THEN c.review_avg END AS review_avg, \
         c.name_pinyin, c.name_initials, \
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
    use super::{
        ranked_candidate_ids, ranked_selection_candidate_ids, search_document_ids,
        search_selection_offering_ids, selection_filter_expression, slot_keys, SearchCandidate,
        SearchDocumentKind, SelectionTimeslotIndexRow,
    };
    use crate::selection_repo::OfferingFilter;
    use shared::AppError;

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

    #[test]
    fn selection_candidate_offsets_track_raw_hits_after_invalid_and_duplicate_entries() {
        let candidates = [
            SearchCandidate { id: "invalid".into() },
            SearchCandidate { id: "1".into() },
            SearchCandidate { id: "1".into() },
            SearchCandidate { id: "2".into() },
            SearchCandidate { id: "3".into() },
        ];

        let (ids, consumed_through) = ranked_selection_candidate_ids(candidates);

        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(consumed_through, vec![2, 4, 5]);
    }

    #[test]
    fn unknown_week_keys_require_explicit_opt_in() {
        let filter = OfferingFilter {
            calendar_id: Some(122),
            weekday: Some(2),
            start_slot: Some(3),
            end_slot: Some(4),
            week: Some(5),
            include_unknown_schedule: false,
            ..OfferingFilter::default()
        };
        let strict = selection_filter_expression(&filter).expect("time filter");
        assert!(strict.contains("calendarId = 122"));
        assert!(strict.contains("d2s3w5"));
        assert!(!strict.contains("d2s3wu"));
        assert!(strict.contains("scheduleUnknown = false"));

        let permissive = selection_filter_expression(&OfferingFilter {
            include_unknown_schedule: true,
            ..filter
        })
        .expect("permissive time filter");
        assert!(permissive.contains("d2s3wu"));
        assert!(permissive.contains("scheduleUnknown = true"));
    }

    #[test]
    fn unknown_week_slots_do_not_receive_known_week_agnostic_keys() {
        let keys = slot_keys(&[
            SelectionTimeslotIndexRow {
                course_id: 1,
                weekday: 2,
                start_slot: 3,
                end_slot: 3,
                week_numbers: Vec::new(),
                weeks_unknown: true,
            },
            SelectionTimeslotIndexRow {
                course_id: 1,
                weekday: 2,
                start_slot: 4,
                end_slot: 4,
                week_numbers: vec![5],
                weeks_unknown: false,
            },
        ]);

        assert!(keys.contains(&"d2s3wu".into()));
        assert!(!keys.contains(&"d2s3".into()));
        assert!(keys.contains(&"d2s4".into()));
        assert!(keys.contains(&"d2s4w5".into()));
    }

    #[tokio::test]
    async fn selection_search_maps_configured_backend_failures_to_unavailable() {
        let result = search_selection_offering_ids(
            "not a valid URL",
            "",
            "course",
            &OfferingFilter::default(),
            0,
            20,
        )
        .await;

        assert!(matches!(result, Err(AppError::ServiceUnavailable)));
    }

    #[tokio::test]
    async fn catalogue_search_maps_configured_backend_failures_to_unavailable() {
        let result =
            search_document_ids("not a valid URL", "", "course", SearchDocumentKind::Course, 20)
                .await;

        assert!(matches!(result, Err(AppError::ServiceUnavailable)));
    }
}

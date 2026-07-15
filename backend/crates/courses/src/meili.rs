//! Meilisearch integration for courses search, index setup, and document sync.
//!
//! Background synchronization degrades safely when Meilisearch is unreachable.
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
            }
        }
        let slot_clause = format!("slotKeys IN [{}]", keys.join(", "));
        clauses.push(if filter.include_unknown_schedule {
            format!("({slot_clause} OR scheduleUnknown = true)")
        } else {
            slot_clause
        });
    }
    (!clauses.is_empty()).then(|| clauses.join(" AND "))
}

pub struct SelectionSearchCandidates {
    pub ids: Vec<i64>,
    pub consumed: usize,
    pub has_more: bool,
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
        return Ok(SelectionSearchCandidates { ids: Vec::new(), consumed: 0, has_more: false });
    }
    let client = Client::new(url, meili_api_key(api_key))
        .map_err(|error| meili_app_failure("selection search client creation", error))?;
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
        .map_err(|error| meili_app_failure("selection candidate search", error))?;
    let consumed = results.hits.len();
    let total_hits = results.estimated_total_hits.or(results.total_hits);
    let has_more = total_hits
        .map(|total| offset.saturating_add(consumed) < total)
        .unwrap_or(consumed == limit);
    let mut seen = HashSet::new();
    let ids = results
        .hits
        .into_iter()
        .filter_map(|hit| hit.result.id.parse::<i64>().ok())
        .filter(|id| *id > 0 && seen.insert(*id))
        .collect();
    Ok(SelectionSearchCandidates { ids, consumed, has_more })
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
            keys.insert(format!("d{}s{slot}", row.weekday));
            if row.weeks_unknown {
                keys.insert(format!("d{}s{slot}wu", row.weekday));
            } else {
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

/// Atomically replace the rebuildable selection index and await every task.
pub async fn sync_selection_courses_to_meili(
    url: &str,
    api_key: &str,
    pool: &PgPool,
) -> AppResult<usize> {
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
    let deletion_task = index
        .delete_all_documents()
        .await
        .map_err(|error| meili_app_failure("selection index clear enqueue", error))?;
    wait_for_task(&client, deletion_task, "selection index clear")
        .await
        .map_err(|error| meili_app_failure("selection index clear", error))?;
    for batch in documents.chunks(1_000) {
        let addition_task = index
            .add_documents(batch, Some("id"))
            .await
            .map_err(|error| meili_app_failure("selection index addition enqueue", error))?;
        wait_for_task(&client, addition_task, "selection index addition")
            .await
            .map_err(|error| meili_app_failure("selection index addition", error))?;
    }
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
    use super::{
        ranked_candidate_ids, selection_filter_expression, SearchCandidate, SearchDocumentKind,
    };
    use crate::selection_repo::OfferingFilter;

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
    fn unknown_week_keys_require_explicit_opt_in() {
        let filter = OfferingFilter {
            weekday: Some(2),
            start_slot: Some(3),
            end_slot: Some(4),
            week: Some(5),
            include_unknown_schedule: false,
            ..OfferingFilter::default()
        };
        let strict = selection_filter_expression(&filter).expect("time filter");
        assert!(strict.contains("d2s3w5"));
        assert!(!strict.contains("d2s3wu"));
        assert!(!strict.contains("scheduleUnknown"));

        let permissive = selection_filter_expression(&OfferingFilter {
            include_unknown_schedule: true,
            ..filter
        })
        .expect("permissive time filter");
        assert!(permissive.contains("d2s3wu"));
        assert!(permissive.contains("scheduleUnknown = true"));
    }
}

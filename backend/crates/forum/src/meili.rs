//! Meilisearch integration for the public forum-thread index.
//!
//! Meilisearch is only a candidate source. PostgreSQL remains the visibility
//! authority for both indexing and public search results.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::{Error as MeiliError, ErrorCode};
use meilisearch_sdk::task_info::TaskInfo;
use serde_json::Value;
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgPool};

const FORUM_THREADS_INDEX: &str = "forum_threads";
const TASK_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TASK_TIMEOUT: Duration = Duration::from_secs(120);

fn meili_api_key(api_key: &str) -> Option<&str> {
    let api_key = api_key.trim();
    (!api_key.is_empty()).then_some(api_key)
}

fn meili_failure(context: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::warn!(%error, operation = context, "forum meilisearch operation failed");
    AppError::Internal(anyhow::anyhow!("forum meilisearch {context} failed"))
}

fn meili_client(meili_url: &str, meili_key: &str) -> AppResult<Client> {
    Client::new(meili_url, meili_api_key(meili_key))
        .map_err(|error| meili_failure("client creation", error))
}

async fn wait_for_task(client: &Client, task: TaskInfo, operation: &'static str) -> AppResult<()> {
    let task = task
        .wait_for_completion(client, Some(TASK_POLL_INTERVAL), Some(TASK_TIMEOUT))
        .await
        .map_err(|error| meili_failure(operation, error))?;
    if task.is_failure() {
        return Err(meili_failure(operation, task.unwrap_failure()));
    }
    if !task.is_success() {
        return Err(meili_failure(operation, "task finished without a terminal success state"));
    }
    Ok(())
}

fn is_index_not_found(error: &MeiliError) -> bool {
    matches!(
        error,
        MeiliError::Meilisearch(error) if error.error_code == ErrorCode::IndexNotFound
    )
}

/// Canonical public document stored in the forum thread index.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumThreadDoc {
    /// Stable thread identifier used as the Meilisearch primary key.
    pub id: String,
    /// Current public title.
    pub title: String,
    /// Current public body, bounded for the search document.
    pub body_excerpt: String,
    /// Current board slug.
    pub board: String,
    /// Current tag slugs.
    pub tags: Vec<String>,
    /// Public author handle.
    pub author_handle: String,
    /// Current count of public replies.
    pub reply_count: i32,
    /// Current aggregate thread vote count.
    pub vote_count: i32,
    /// Creation time as Unix seconds.
    pub created_at: i64,
    /// Public thread status, currently always `visible`.
    pub status: String,
}

#[derive(Debug, FromRow)]
struct ForumThreadDocumentRow {
    id: i64,
    title: String,
    body: Option<String>,
    content_format: String,
    board: String,
    tags: Vec<String>,
    author_handle: String,
    reply_count: i32,
    vote_count: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    status: String,
}

impl From<ForumThreadDocumentRow> for ForumThreadDoc {
    fn from(row: ForumThreadDocumentRow) -> Self {
        Self {
            id: row.id.to_string(),
            title: row.title,
            body_excerpt: crate::content_policy::plain_text_projection(
                row.body.as_deref().unwrap_or_default(),
                crate::dto::ContentFormat::from_db(&row.content_format),
                2048,
            ),
            board: row.board,
            tags: row.tags,
            author_handle: row.author_handle,
            reply_count: row.reply_count,
            vote_count: row.vote_count,
            created_at: row.created_at.timestamp(),
            status: row.status,
        }
    }
}

/// Loads canonical documents for the requested IDs when they are currently public.
///
/// A thread is public only while its status is `visible` and it is neither hidden,
/// archived, nor soft-deleted. Missing and non-public IDs are omitted.
pub async fn load_public_thread_documents(
    pool: &PgPool,
    thread_ids: &[i64],
) -> AppResult<Vec<ForumThreadDoc>> {
    if thread_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows = sqlx::query_as::<_, ForumThreadDocumentRow>(
        "SELECT thread.id, thread.title, thread.body, thread.content_format, board.slug AS board, \
                ARRAY(SELECT tag.slug FROM forum.thread_tags thread_tag \
                      JOIN forum.tags tag ON tag.id = thread_tag.tag_id \
                      WHERE thread_tag.thread_id = thread.id ORDER BY tag.name) AS tags, \
                account.handle AS author_handle, thread.reply_count, thread.vote_count, \
                thread.created_at, thread.status \
         FROM forum.threads thread \
         JOIN identity.accounts account ON account.id = thread.author_id \
         JOIN forum.boards board ON board.id = thread.board_id \
         WHERE thread.id = ANY($1) AND thread.status = 'visible' \
           AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
           AND thread.archived_at IS NULL \
         ORDER BY thread.id",
    )
    .bind(thread_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(ForumThreadDoc::from).collect())
}

async fn load_all_public_thread_documents(pool: &PgPool) -> AppResult<Vec<ForumThreadDoc>> {
    let rows = sqlx::query_as::<_, ForumThreadDocumentRow>(
        "SELECT thread.id, thread.title, thread.body, thread.content_format, board.slug AS board, \
                ARRAY(SELECT tag.slug FROM forum.thread_tags thread_tag \
                      JOIN forum.tags tag ON tag.id = thread_tag.tag_id \
                      WHERE thread_tag.thread_id = thread.id ORDER BY tag.name) AS tags, \
                account.handle AS author_handle, thread.reply_count, thread.vote_count, \
                thread.created_at, thread.status \
         FROM forum.threads thread \
         JOIN identity.accounts account ON account.id = thread.author_id \
         JOIN forum.boards board ON board.id = thread.board_id \
         WHERE thread.status = 'visible' AND thread.deleted_at IS NULL \
           AND thread.hidden_at IS NULL AND thread.archived_at IS NULL \
         ORDER BY thread.id",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(ForumThreadDoc::from).collect())
}

async fn add_thread_document(client: &Client, document: &ForumThreadDoc) -> AppResult<()> {
    let task = client
        .index(FORUM_THREADS_INDEX)
        .add_documents(std::slice::from_ref(document), Some("id"))
        .await
        .map_err(|error| meili_failure("thread upsert enqueue", error))?;
    wait_for_task(client, task, "thread upsert").await
}

/// Removes a thread document and waits until Meilisearch applies the deletion.
///
/// A missing index is already equivalent to the requested final state.
pub async fn delete_thread_from_meili(
    meili_url: &str,
    meili_key: &str,
    thread_id: i64,
) -> AppResult<()> {
    let client = meili_client(meili_url, meili_key)?;
    let task = match client.index(FORUM_THREADS_INDEX).delete_document(thread_id).await {
        Ok(task) => task,
        Err(error) if is_index_not_found(&error) => return Ok(()),
        Err(error) => return Err(meili_failure("thread deletion enqueue", error)),
    };
    wait_for_task(&client, task, "thread deletion").await
}

/// Reconciles one index document against the thread's current database visibility.
///
/// Visible threads are rebuilt from canonical database fields; all other states
/// remove the document. Call this after a transaction changes thread content,
/// board, visibility, archive, or deletion state.
pub async fn reconcile_thread_in_meili(
    pool: &PgPool,
    meili_url: &str,
    meili_key: &str,
    thread_id: i64,
) -> AppResult<()> {
    let mut documents = load_public_thread_documents(pool, &[thread_id]).await?;
    match documents.pop() {
        Some(document) => {
            let client = meili_client(meili_url, meili_key)?;
            add_thread_document(&client, &document).await
        }
        None => delete_thread_from_meili(meili_url, meili_key, thread_id).await,
    }
}

/// Reconcile one thread after a committed state transition without delaying the response.
pub fn reconcile_thread_in_background(state: &AppState, thread_id: i64) {
    let pool = state.db.clone();
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_master_key.clone();
    tokio::spawn(async move {
        if let Err(error) =
            reconcile_thread_in_meili(&pool, &meili_url, &meili_key, thread_id).await
        {
            tracing::warn!(%error, thread_id, "failed to reconcile forum search document");
        }
    });
}

fn candidate_thread_ids(hits: &[Value]) -> Vec<i64> {
    let mut seen = HashSet::new();
    hits.iter()
        .filter_map(|hit| {
            let id = hit.get("id")?;
            let parsed = id.as_i64().or_else(|| id.as_str()?.parse().ok())?;
            (parsed > 0 && seen.insert(parsed)).then_some(parsed)
        })
        .collect()
}

fn order_public_documents(
    candidate_ids: &[i64],
    documents: Vec<ForumThreadDoc>,
    limit: usize,
) -> Vec<ForumThreadDoc> {
    let mut documents_by_id: HashMap<i64, ForumThreadDoc> = documents
        .into_iter()
        .filter_map(|document| document.id.parse().ok().map(|id| (id, document)))
        .collect();
    candidate_ids
        .iter()
        .filter_map(|thread_id| documents_by_id.remove(thread_id))
        .take(limit)
        .collect()
}

/// Searches forum threads while treating Meilisearch hits only as ranked candidates.
///
/// Every returned document is reconstructed from a visibility-checked PostgreSQL
/// row. Meilisearch failure degrades to an empty result; database failure is returned.
pub async fn search_threads(
    pool: &PgPool,
    meili_url: &str,
    meili_key: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<ForumThreadDoc>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let client = match Client::new(meili_url, meili_api_key(meili_key)) {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(%error, "forum search meilisearch client creation failed");
            return Ok(Vec::new());
        }
    };
    let candidate_limit = limit.saturating_mul(4).min(1_000);
    let hits = match client
        .index(FORUM_THREADS_INDEX)
        .search()
        .with_query(query)
        .with_limit(candidate_limit)
        .execute::<Value>()
        .await
    {
        Ok(results) => results.hits.into_iter().map(|hit| hit.result).collect::<Vec<_>>(),
        Err(error) => {
            tracing::warn!(%error, "forum thread search failed");
            return Ok(Vec::new());
        }
    };

    let candidate_ids = candidate_thread_ids(&hits);
    let documents = load_public_thread_documents(pool, &candidate_ids).await?;
    Ok(order_public_documents(&candidate_ids, documents, limit))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReindexStep {
    Clear,
    Add,
}

fn reindex_plan(has_documents: bool) -> &'static [ReindexStep] {
    const CLEAR_ONLY: &[ReindexStep] = &[ReindexStep::Clear];
    const CLEAR_AND_ADD: &[ReindexStep] = &[ReindexStep::Clear, ReindexStep::Add];
    if has_documents {
        CLEAR_AND_ADD
    } else {
        CLEAR_ONLY
    }
}

async fn ensure_forum_index(client: &Client) -> AppResult<()> {
    match client.get_index(FORUM_THREADS_INDEX).await {
        Ok(_) => Ok(()),
        Err(error) if is_index_not_found(&error) => {
            let task = client
                .create_index(FORUM_THREADS_INDEX, Some("id"))
                .await
                .map_err(|error| meili_failure("index creation enqueue", error))?;
            wait_for_task(client, task, "index creation").await
        }
        Err(error) => Err(meili_failure("index lookup", error)),
    }
}

/// Rebuilds the forum index from the current set of public database rows.
///
/// The clear task is confirmed successful before any addition is enqueued, and
/// the addition task is also confirmed before this function returns.
pub async fn reindex_forum(pool: &PgPool, meili_url: &str, meili_key: &str) -> AppResult<()> {
    let documents = load_all_public_thread_documents(pool).await?;
    let client = meili_client(meili_url, meili_key)?;
    ensure_forum_index(&client).await?;
    let index = client.index(FORUM_THREADS_INDEX);

    for step in reindex_plan(!documents.is_empty()) {
        let (task, operation) = match step {
            ReindexStep::Clear => (
                index
                    .delete_all_documents()
                    .await
                    .map_err(|error| meili_failure("index clear enqueue", error))?,
                "index clear",
            ),
            ReindexStep::Add => (
                index
                    .add_documents(&documents, Some("id"))
                    .await
                    .map_err(|error| meili_failure("index rebuild enqueue", error))?,
                "index rebuild",
            ),
        };
        wait_for_task(&client, task, operation).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(id: i64, title: &str) -> ForumThreadDoc {
        ForumThreadDoc {
            id: id.to_string(),
            title: title.into(),
            body_excerpt: String::new(),
            board: "general".into(),
            tags: Vec::new(),
            author_handle: "author".into(),
            reply_count: 0,
            vote_count: 0,
            created_at: 0,
            status: "visible".into(),
        }
    }

    #[test]
    fn candidate_ids_reject_malformed_and_duplicate_hits() {
        let hits = vec![
            serde_json::json!({"id": "3"}),
            serde_json::json!({"id": 2}),
            serde_json::json!({"id": "3"}),
            serde_json::json!({"id": 0}),
            serde_json::json!({"id": "not-an-id"}),
            serde_json::json!({"title": "missing id"}),
        ];

        assert_eq!(candidate_thread_ids(&hits), vec![3, 2]);
    }

    #[test]
    fn public_documents_keep_candidate_rank_and_omit_missing_rows() {
        let ordered = order_public_documents(
            &[30, 20, 10],
            vec![document(10, "ten"), document(30, "thirty")],
            10,
        );

        assert_eq!(ordered, vec![document(30, "thirty"), document(10, "ten")]);
    }

    #[test]
    fn reindex_plan_always_clears_before_optional_addition() {
        assert_eq!(reindex_plan(false), &[ReindexStep::Clear]);
        assert_eq!(reindex_plan(true), &[ReindexStep::Clear, ReindexStep::Add]);
    }
}

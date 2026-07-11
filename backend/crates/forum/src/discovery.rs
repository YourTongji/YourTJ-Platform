//! Board and tag discovery for federated search.
//!
//! Meilisearch supplies ranked candidate identifiers only. Every response is
//! reconstructed from current PostgreSQL rows.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::{Error as MeiliError, ErrorCode};
use meilisearch_sdk::task_info::TaskInfo;
use serde::Serialize;
use serde_json::Value;
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgPool};

const FORUM_DISCOVERY_INDEX: &str = "forum_discovery";
const TASK_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TASK_TIMEOUT: Duration = Duration::from_secs(120);

pub use crate::user_discovery::{load_user_hits, UserSearchHit};

/// Forum-owned entity kinds maintained in the discovery index.
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryEntityKind {
    Board,
    Tag,
}

impl DiscoveryEntityKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Board => "board",
            Self::Tag => "tag",
        }
    }
}

/// One public forum board result.
#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSearchHit {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub thread_count: i32,
}

/// One public forum tag result.
#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSearchHit {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub thread_count: i32,
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryDocument {
    id: String,
    entity_id: i64,
    kind: String,
    slug: String,
    name: String,
    description: Option<String>,
}

fn meili_key(api_key: &str) -> Option<&str> {
    let api_key = api_key.trim();
    (!api_key.is_empty()).then_some(api_key)
}

fn failure(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::warn!(%error, operation, "forum discovery index operation failed");
    AppError::Internal(anyhow::anyhow!("forum discovery index {operation} failed"))
}

fn client(meili_url: &str, api_key: &str) -> AppResult<Client> {
    Client::new(meili_url, meili_key(api_key)).map_err(|error| failure("client creation", error))
}

fn is_missing_index(error: &MeiliError) -> bool {
    matches!(
        error,
        MeiliError::Meilisearch(error) if error.error_code == ErrorCode::IndexNotFound
    )
}

async fn wait_for_task(client: &Client, task: TaskInfo, operation: &'static str) -> AppResult<()> {
    let task = task
        .wait_for_completion(client, Some(TASK_POLL_INTERVAL), Some(TASK_TIMEOUT))
        .await
        .map_err(|error| failure(operation, error))?;
    if task.is_failure() {
        return Err(failure(operation, task.unwrap_failure()));
    }
    if !task.is_success() {
        return Err(failure(operation, "task finished without success"));
    }
    Ok(())
}

async fn ensure_index(client: &Client) -> AppResult<()> {
    match client.get_index(FORUM_DISCOVERY_INDEX).await {
        Ok(_) => {}
        Err(error) if is_missing_index(&error) => {
            let create_task = client
                .create_index(FORUM_DISCOVERY_INDEX, Some("id"))
                .await
                .map_err(|error| failure("index creation enqueue", error))?;
            wait_for_task(client, create_task, "index creation").await?;
        }
        Err(error) => return Err(failure("index lookup", error)),
    }
    let index = client.index(FORUM_DISCOVERY_INDEX);
    let searchable_task = index
        .set_searchable_attributes(&["name", "slug", "description"])
        .await
        .map_err(|error| failure("searchable attributes enqueue", error))?;
    wait_for_task(client, searchable_task, "searchable attributes").await?;
    let filterable_task = index
        .set_filterable_attributes(&["kind"])
        .await
        .map_err(|error| failure("filterable attributes enqueue", error))?;
    wait_for_task(client, filterable_task, "filterable attributes").await
}

fn document_id(kind: &str, entity_id: i64) -> String {
    format!("{kind}-{entity_id}")
}

async fn load_documents(
    pool: &PgPool,
    kind: Option<&str>,
    entity_id: Option<i64>,
) -> AppResult<Vec<DiscoveryDocument>> {
    let rows = sqlx::query_as::<_, DiscoveryDocument>(
        "SELECT 'board-' || board.id AS id, board.id AS entity_id, 'board' AS kind, \
                board.slug, board.name, board.description \
         FROM forum.boards board \
         WHERE ($1::text IS NULL OR $1 = 'board') AND ($2::bigint IS NULL OR board.id = $2) \
         UNION ALL \
         SELECT 'tag-' || tag.id AS id, tag.id AS entity_id, 'tag' AS kind, \
                tag.slug, tag.name, tag.description \
         FROM forum.tags tag \
         WHERE ($1::text IS NULL OR $1 = 'tag') AND ($2::bigint IS NULL OR tag.id = $2) \
         ORDER BY kind, entity_id",
    )
    .bind(kind)
    .bind(entity_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

fn ranked_entity_ids(hits: impl IntoIterator<Item = Value>) -> Vec<i64> {
    let mut seen = HashSet::new();
    hits.into_iter()
        .filter_map(|hit| {
            let entity_id = hit
                .get("entityId")
                .and_then(|id| id.as_i64().or_else(|| id.as_str()?.parse().ok()))?;
            (entity_id > 0 && seen.insert(entity_id)).then_some(entity_id)
        })
        .collect()
}

async fn search_entity_ids(
    meili_url: &str,
    api_key: &str,
    query: &str,
    kind: &str,
    limit: usize,
) -> AppResult<Vec<i64>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let client = match client(meili_url, api_key) {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(%error, entity_kind = kind, "forum discovery client unavailable");
            return Ok(Vec::new());
        }
    };
    let filter = format!(r#"kind = "{kind}""#);
    let results = match client
        .index(FORUM_DISCOVERY_INDEX)
        .search()
        .with_query(query)
        .with_filter(&filter)
        .with_limit(limit.saturating_mul(4).min(1_000))
        .execute::<Value>()
        .await
    {
        Ok(results) => results,
        Err(error) => {
            tracing::warn!(%error, entity_kind = kind, "forum discovery search failed");
            return Ok(Vec::new());
        }
    };
    Ok(ranked_entity_ids(results.hits.into_iter().map(|hit| hit.result)))
}

fn order_hits<T>(candidate_ids: &[i64], hits: Vec<T>, id: impl Fn(&T) -> i64) -> Vec<T> {
    let mut hits = hits.into_iter().map(|hit| (id(&hit), hit)).collect::<HashMap<_, _>>();
    candidate_ids.iter().filter_map(|entity_id| hits.remove(entity_id)).collect()
}

/// Search public forum boards and reconstruct current counters from PostgreSQL.
pub async fn search_boards(
    pool: &PgPool,
    meili_url: &str,
    api_key: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<BoardSearchHit>> {
    let candidate_ids = search_entity_ids(meili_url, api_key, query, "board", limit).await?;
    if candidate_ids.is_empty() {
        return Ok(Vec::new());
    }
    let hits = sqlx::query_as::<_, BoardSearchHit>(
        "SELECT id::text AS id, slug, name, description, thread_count \
         FROM forum.boards WHERE id = ANY($1)",
    )
    .bind(&candidate_ids)
    .fetch_all(pool)
    .await?;
    Ok(order_hits(&candidate_ids, hits, |hit| hit.id.parse().unwrap_or_default()))
}

/// Search public forum tags and reconstruct current counters from PostgreSQL.
pub async fn search_tags(
    pool: &PgPool,
    meili_url: &str,
    api_key: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<TagSearchHit>> {
    let candidate_ids = search_entity_ids(meili_url, api_key, query, "tag", limit).await?;
    if candidate_ids.is_empty() {
        return Ok(Vec::new());
    }
    let hits = sqlx::query_as::<_, TagSearchHit>(
        "SELECT id::text AS id, slug, name, description, thread_count \
         FROM forum.tags WHERE id = ANY($1)",
    )
    .bind(&candidate_ids)
    .fetch_all(pool)
    .await?;
    Ok(order_hits(&candidate_ids, hits, |hit| hit.id.parse().unwrap_or_default()))
}

async fn reconcile_entity(
    pool: &PgPool,
    meili_url: &str,
    api_key: &str,
    kind: DiscoveryEntityKind,
    entity_id: i64,
) -> AppResult<()> {
    if meili_url.trim().is_empty() {
        return Ok(());
    }
    let client = client(meili_url, api_key)?;
    ensure_index(&client).await?;
    let kind = kind.as_str();
    let mut documents = load_documents(pool, Some(kind), Some(entity_id)).await?;
    let task = match documents.pop() {
        Some(document) => client
            .index(FORUM_DISCOVERY_INDEX)
            .add_documents(&[document], Some("id"))
            .await
            .map_err(|error| failure("document upsert enqueue", error))?,
        None => match client
            .index(FORUM_DISCOVERY_INDEX)
            .delete_document(document_id(kind, entity_id))
            .await
        {
            Ok(task) => task,
            Err(error) if is_missing_index(&error) => return Ok(()),
            Err(error) => return Err(failure("document deletion enqueue", error)),
        },
    };
    wait_for_task(&client, task, "document reconciliation").await
}

/// Reconcile one board or tag document without delaying an admin response.
pub fn reconcile_entity_in_background(state: &AppState, kind: DiscoveryEntityKind, entity_id: i64) {
    let pool = state.db.clone();
    let meili_url = state.meili_url.clone();
    let api_key = state.meili_master_key.clone();
    tokio::spawn(async move {
        if let Err(error) = reconcile_entity(&pool, &meili_url, &api_key, kind, entity_id).await {
            tracing::warn!(%error, entity_kind = kind.as_str(), entity_id, "failed to reconcile forum discovery document");
        }
    });
}

/// Rebuild the complete forum board/tag candidate index.
pub async fn reindex_discovery(pool: &PgPool, meili_url: &str, api_key: &str) -> AppResult<()> {
    let client = client(meili_url, api_key)?;
    ensure_index(&client).await?;
    let index = client.index(FORUM_DISCOVERY_INDEX);
    let clear_task = index
        .delete_all_documents()
        .await
        .map_err(|error| failure("index clear enqueue", error))?;
    wait_for_task(&client, clear_task, "index clear").await?;
    let documents = load_documents(pool, None, None).await?;
    if documents.is_empty() {
        return Ok(());
    }
    let add_task = index
        .add_documents(&documents, Some("id"))
        .await
        .map_err(|error| failure("index rebuild enqueue", error))?;
    wait_for_task(&client, add_task, "index rebuild").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_ids_are_positive_ranked_and_unique() {
        let hits = vec![
            serde_json::json!({"entityId": "4"}),
            serde_json::json!({"entityId": 2}),
            serde_json::json!({"entityId": "4"}),
            serde_json::json!({"entityId": 0}),
        ];
        assert_eq!(ranked_entity_ids(hits), vec![4, 2]);
    }
}

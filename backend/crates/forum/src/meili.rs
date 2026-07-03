//! Meilisearch integration for forum threads index.
//!
//! The index is named `forum_threads`. Documents are synced on write.
//! The admin reindex endpoint rebuilds the index from scratch.

use shared::AppResult;

/// Document structure stored in Meilisearch.
#[allow(dead_code)]
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumThreadDoc {
    pub id: String,
    pub title: String,
    pub body_excerpt: String, // first 2048 chars
    pub board: String,        // board slug
    pub tags: Vec<String>,
    pub author_handle: String,
    pub reply_count: i32,
    pub vote_count: i32,
    pub created_at: i64,
    pub status: String,
}

/// Sync a single thread to Meilisearch on create/edit.
/// Logs error but does not fail the request.
#[allow(dead_code)]
pub async fn sync_thread_to_meili(meili_url: &str, meili_key: &str, doc: &ForumThreadDoc) {
    let client = match meilisearch_sdk::client::Client::new(meili_url, Some(meili_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to create meilisearch client");
            return;
        }
    };
    let index = client.index("forum_threads");
    if let Err(e) = index.add_documents(&[doc], Some("id")).await {
        tracing::warn!(error = %e, "failed to sync thread to meilisearch");
    }
}

/// Delete a thread document from Meilisearch on soft-delete.
#[allow(dead_code)]
pub async fn delete_thread_from_meili(meili_url: &str, meili_key: &str, thread_id: i64) {
    let client = match meilisearch_sdk::client::Client::new(meili_url, Some(meili_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to create meilisearch client");
            return;
        }
    };
    let index = client.index("forum_threads");
    let id_str = thread_id.to_string();
    if let Err(e) = index.delete_document(&id_str).await {
        tracing::warn!(error = %e, "failed to delete thread from meilisearch");
    }
}

/// Search forum threads via Meilisearch. Returns empty Vec on failure
/// (graceful degradation when Meilisearch is unreachable).
pub async fn search_threads(
    meili_url: &str,
    meili_key: &str,
    query: &str,
    limit: usize,
) -> Vec<serde_json::Value> {
    let client = match meilisearch_sdk::client::Client::new(meili_url, Some(meili_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Meili client failed — forum search returning empty");
            return Vec::new();
        }
    };

    let index = client.index("forum_threads");
    match index.search().with_query(query).with_limit(limit).execute::<serde_json::Value>().await {
        Ok(results) => results.hits.into_iter().map(|h| h.result).collect(),
        Err(e) => {
            tracing::warn!(error = %e, query = %query, "forum thread search failed");
            Vec::new()
        }
    }
}

/// Rebuild the entire forum_threads index from the database.
/// Requires access to the PgPool to query all visible threads.
#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub async fn reindex_forum(pool: &sqlx::PgPool, meili_url: &str, meili_key: &str) -> AppResult<()> {
    let rows: Vec<(
        i64,
        String,
        Option<String>,
        String,
        i32,
        i32,
        String,
        chrono::DateTime<chrono::Utc>,
        String,
    )> = sqlx::query_as(
        "SELECT t.id, t.title, t.body, a.handle, t.reply_count, t.vote_count, \
         t.status, t.created_at, b.slug \
         FROM forum.threads t \
         JOIN identity.accounts a ON a.id = t.author_id \
         JOIN forum.boards b ON b.id = t.board_id \
         WHERE t.deleted_at IS NULL AND t.hidden_at IS NULL \
         ORDER BY t.id",
    )
    .fetch_all(pool)
    .await?;

    let docs: Vec<ForumThreadDoc> = rows
        .into_iter()
        .map(|r| ForumThreadDoc {
            id: r.0.to_string(),
            title: r.1,
            body_excerpt: r.2.unwrap_or_default().chars().take(2048).collect(),
            board: r.8,
            tags: vec![],
            author_handle: r.3,
            reply_count: r.4,
            vote_count: r.5,
            created_at: r.7.timestamp(),
            status: r.6,
        })
        .collect();

    let client = match meilisearch_sdk::client::Client::new(meili_url, Some(meili_key)) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to create meilisearch client for reindex");
            return Err(shared::AppError::Internal(anyhow::anyhow!(e)));
        }
    };
    let index = client.index("forum_threads");

    if docs.is_empty() {
        if let Err(e) = index.delete_all_documents().await {
            tracing::warn!(error = %e, "failed to clear meilisearch forum index");
        }
        return Ok(());
    }

    if let Err(e) = index.add_documents(&docs, Some("id")).await {
        tracing::warn!(error = %e, "failed to rebuild meilisearch forum index");
    }

    Ok(())
}

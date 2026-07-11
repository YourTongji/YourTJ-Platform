//! Meilisearch candidate index for public account discovery.
//!
//! The index stores only public handles and optional display names. PostgreSQL
//! remains authoritative for lifecycle, discoverability, privacy, and sanctions.

use std::collections::HashSet;
use std::time::Duration;

use meilisearch_sdk::client::Client;
use meilisearch_sdk::errors::{Error as MeiliError, ErrorCode};
use meilisearch_sdk::task_info::TaskInfo;
use serde::Serialize;
use serde_json::Value;
use shared::{AppError, AppResult, AppState};
use sqlx::{FromRow, PgPool};

const IDENTITY_USERS_INDEX: &str = "identity_users";
const TASK_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TASK_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSearchDocument {
    id: String,
    handle: String,
    display_name: Option<String>,
}

fn meili_key(api_key: &str) -> Option<&str> {
    let api_key = api_key.trim();
    (!api_key.is_empty()).then_some(api_key)
}

fn meili_failure(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::warn!(%error, operation, "identity search index operation failed");
    AppError::Internal(anyhow::anyhow!("identity search index {operation} failed"))
}

fn client(meili_url: &str, api_key: &str) -> AppResult<Client> {
    Client::new(meili_url, meili_key(api_key))
        .map_err(|error| meili_failure("client creation", error))
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
        .map_err(|error| meili_failure(operation, error))?;
    if task.is_failure() {
        return Err(meili_failure(operation, task.unwrap_failure()));
    }
    if !task.is_success() {
        return Err(meili_failure(operation, "task finished without success"));
    }
    Ok(())
}

async fn ensure_index(client: &Client) -> AppResult<()> {
    match client.get_index(IDENTITY_USERS_INDEX).await {
        Ok(_) => {}
        Err(error) if is_missing_index(&error) => {
            let create_task = client
                .create_index(IDENTITY_USERS_INDEX, Some("id"))
                .await
                .map_err(|error| meili_failure("index creation enqueue", error))?;
            wait_for_task(client, create_task, "index creation").await?;
        }
        Err(error) => return Err(meili_failure("index lookup", error)),
    }
    let searchable_task = client
        .index(IDENTITY_USERS_INDEX)
        .set_searchable_attributes(&["handle", "displayName"])
        .await
        .map_err(|error| meili_failure("searchable attributes enqueue", error))?;
    wait_for_task(client, searchable_task, "searchable attributes").await
}

async fn load_documents(
    pool: &PgPool,
    account_ids: Option<&[i64]>,
) -> AppResult<Vec<UserSearchDocument>> {
    let rows = sqlx::query_as::<_, UserSearchDocument>(
        "SELECT account.id::text AS id, account.handle::text AS handle, profile.display_name \
         FROM identity.accounts account \
         LEFT JOIN identity.profiles profile ON profile.account_id = account.id \
         LEFT JOIN identity.profile_privacy privacy ON privacy.account_id = account.id \
         WHERE ($1::bigint[] IS NULL OR account.id = ANY($1)) \
           AND account.status = 'active'::identity.account_status \
           AND account.email_verified_at IS NOT NULL \
           AND COALESCE(privacy.discoverable, TRUE) \
           AND COALESCE(privacy.profile_visibility, 'campus') <> 'only_me' \
         ORDER BY account.id",
    )
    .bind(account_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Return ranked account identifiers from Meilisearch without trusting visibility.
pub async fn search_user_ids(
    meili_url: &str,
    api_key: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<i64>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let client = client(meili_url, api_key)?;
    let results = client
        .index(IDENTITY_USERS_INDEX)
        .search()
        .with_query(query)
        .with_limit(limit.saturating_mul(4).min(1_000))
        .execute::<Value>()
        .await
        .map_err(|error| meili_failure("candidate search", error))?;
    let mut seen = HashSet::new();
    Ok(results
        .hits
        .into_iter()
        .filter_map(|hit| {
            let id = hit.result.get("id")?;
            let account_id = id.as_i64().or_else(|| id.as_str()?.parse().ok())?;
            (account_id > 0 && seen.insert(account_id)).then_some(account_id)
        })
        .collect())
}

/// Reconcile one account document after an owner or staff mutation.
pub async fn reconcile_user(
    pool: &PgPool,
    meili_url: &str,
    api_key: &str,
    account_id: i64,
) -> AppResult<()> {
    if meili_url.trim().is_empty() {
        return Ok(());
    }
    let client = client(meili_url, api_key)?;
    ensure_index(&client).await?;
    let mut documents = load_documents(pool, Some(&[account_id])).await?;
    let task = match documents.pop() {
        Some(document) => client
            .index(IDENTITY_USERS_INDEX)
            .add_documents(&[document], Some("id"))
            .await
            .map_err(|error| meili_failure("document upsert enqueue", error))?,
        None => match client.index(IDENTITY_USERS_INDEX).delete_document(account_id).await {
            Ok(task) => task,
            Err(error) if is_missing_index(&error) => return Ok(()),
            Err(error) => return Err(meili_failure("document deletion enqueue", error)),
        },
    };
    wait_for_task(&client, task, "document reconciliation").await
}

/// Reconcile an account search document without delaying an HTTP response.
pub fn reconcile_user_in_background(state: &AppState, account_id: i64) {
    let pool = state.db.clone();
    let meili_url = state.meili_url.clone();
    let api_key = state.meili_master_key.clone();
    tokio::spawn(async move {
        if let Err(error) = reconcile_user(&pool, &meili_url, &api_key, account_id).await {
            tracing::warn!(%error, account_id, "failed to reconcile public user search document");
        }
    });
}

/// Rebuild the complete privacy-minimized public account candidate index.
pub async fn reindex_users(pool: &PgPool, meili_url: &str, api_key: &str) -> AppResult<()> {
    let client = client(meili_url, api_key)?;
    ensure_index(&client).await?;
    let index = client.index(IDENTITY_USERS_INDEX);
    let clear_task = index
        .delete_all_documents()
        .await
        .map_err(|error| meili_failure("index clear enqueue", error))?;
    wait_for_task(&client, clear_task, "index clear").await?;
    let documents = load_documents(pool, None).await?;
    if documents.is_empty() {
        return Ok(());
    }
    let add_task = index
        .add_documents(&documents, Some("id"))
        .await
        .map_err(|error| meili_failure("index rebuild enqueue", error))?;
    wait_for_task(&client, add_task, "index rebuild").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meili_key_omits_blank_credentials() {
        assert_eq!(meili_key("  "), None);
        assert_eq!(meili_key("secret"), Some("secret"));
    }
}

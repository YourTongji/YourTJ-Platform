//! Tag CRUD and thread-tag association.

use std::collections::HashMap;

use crate::models::TagRow;
use shared::AppResult;
use sqlx::{PgConnection, PgPool};

/// List all tags.
pub async fn list_tags(pool: &PgPool) -> AppResult<Vec<TagRow>> {
    let rows = sqlx::query_as::<_, TagRow>(
        "SELECT id, slug, name, description, thread_count, created_at FROM forum.tags ORDER BY name"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Find a tag by ID.
pub async fn find_tag(pool: &PgPool, id: i64) -> AppResult<Option<TagRow>> {
    let row = sqlx::query_as::<_, TagRow>(
        "SELECT id, slug, name, description, thread_count, created_at FROM forum.tags WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Find a tag by slug.
pub async fn find_tag_by_slug(pool: &PgPool, slug: &str) -> AppResult<Option<TagRow>> {
    let row = sqlx::query_as::<_, TagRow>(
        "SELECT id, slug, name, description, thread_count, created_at FROM forum.tags WHERE slug = $1"
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Create a new tag.
pub async fn create_tag(
    executor: impl sqlx::PgExecutor<'_>,
    slug: &str,
    name: &str,
    description: Option<&str>,
) -> AppResult<TagRow> {
    let row = sqlx::query_as::<_, TagRow>(
        "INSERT INTO forum.tags (slug, name, description) VALUES ($1, $2, $3) \
         RETURNING id, slug, name, description, thread_count, created_at",
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

/// Update a tag.
pub async fn update_tag(
    executor: impl sqlx::PgExecutor<'_>,
    id: i64,
    slug: Option<&str>,
    name: Option<&str>,
    description: Option<Option<&str>>,
) -> AppResult<TagRow> {
    let row = sqlx::query_as::<_, TagRow>(
        "UPDATE forum.tags SET \
         slug = COALESCE($1, slug), \
         name = COALESCE($2, name), \
         description = COALESCE($3, description) \
         WHERE id = $4 \
         RETURNING id, slug, name, description, thread_count, created_at",
    )
    .bind(slug)
    .bind(name)
    .bind(description.flatten())
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

/// Delete a tag.
pub async fn delete_tag(executor: impl sqlx::PgExecutor<'_>, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM forum.tags WHERE id = $1").bind(id).execute(executor).await?;
    Ok(())
}

/// Set tags on a thread (replaces existing).
pub async fn set_thread_tags(pool: &PgPool, thread_id: i64, tag_ids: &[i64]) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    set_thread_tags_tx(&mut tx, thread_id, tag_ids).await?;
    tx.commit().await?;
    Ok(())
}

/// Resolve every requested slug while locking tag rows in a stable order.
pub(crate) async fn resolve_tag_slugs_tx(
    connection: &mut PgConnection,
    slugs: &[String],
) -> AppResult<Vec<i64>> {
    let mut sorted_slugs = slugs.to_vec();
    sorted_slugs.sort();
    let mut tag_ids = Vec::with_capacity(sorted_slugs.len());
    for slug in sorted_slugs {
        let tag_id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM forum.tags WHERE slug = $1 FOR UPDATE")
                .bind(&slug)
                .fetch_optional(&mut *connection)
                .await?;
        let tag_id =
            tag_id.ok_or_else(|| shared::AppError::BadRequest(format!("unknown tag: {slug}")))?;
        tag_ids.push(tag_id);
    }
    Ok(tag_ids)
}

/// Replace thread tags and refresh public tag counters in the active transaction.
pub(crate) async fn set_thread_tags_tx(
    connection: &mut PgConnection,
    thread_id: i64,
    tag_ids: &[i64],
) -> AppResult<()> {
    let old_ids: Vec<i64> =
        sqlx::query_scalar("SELECT tag_id FROM forum.thread_tags WHERE thread_id = $1")
            .bind(thread_id)
            .fetch_all(&mut *connection)
            .await?;

    sqlx::query("DELETE FROM forum.thread_tags WHERE thread_id = $1")
        .bind(thread_id)
        .execute(&mut *connection)
        .await?;

    for &tag_id in tag_ids {
        sqlx::query(
            "INSERT INTO forum.thread_tags (thread_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        )
        .bind(thread_id)
        .bind(tag_id)
        .execute(&mut *connection)
        .await?;
    }

    let mut all_affected: Vec<i64> = old_ids;
    all_affected.extend_from_slice(tag_ids);
    all_affected.sort_unstable();
    all_affected.dedup();

    for tag_id in all_affected {
        sqlx::query(
            "UPDATE forum.tags SET thread_count = ( \
               SELECT COUNT(*)::int FROM forum.thread_tags thread_tag \
               JOIN forum.threads thread ON thread.id = thread_tag.thread_id \
               WHERE thread_tag.tag_id = $1 AND thread.status = 'visible' \
                 AND thread.deleted_at IS NULL AND thread.hidden_at IS NULL \
                 AND thread.archived_at IS NULL \
             ) WHERE id = $1",
        )
        .bind(tag_id)
        .execute(&mut *connection)
        .await?;
    }

    Ok(())
}

/// Get tag slugs for a thread.
pub async fn get_thread_tag_slugs(pool: &PgPool, thread_id: i64) -> AppResult<Vec<String>> {
    let slugs: Vec<String> = sqlx::query_scalar(
        "SELECT t.slug FROM forum.tags t \
         JOIN forum.thread_tags tt ON tt.tag_id = t.id \
         WHERE tt.thread_id = $1 \
         ORDER BY t.name",
    )
    .bind(thread_id)
    .fetch_all(pool)
    .await?;
    Ok(slugs)
}

/// Batch tag slugs for thread-list responses.
pub async fn get_thread_tag_slugs_batch(
    pool: &PgPool,
    thread_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<String>>> {
    if thread_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT thread_tag.thread_id, tag.slug \
         FROM forum.thread_tags thread_tag \
         JOIN forum.tags tag ON tag.id = thread_tag.tag_id \
         WHERE thread_tag.thread_id = ANY($1) \
         ORDER BY thread_tag.thread_id, tag.name, tag.id",
    )
    .bind(thread_ids)
    .fetch_all(pool)
    .await?;
    let mut tags = HashMap::new();
    for (thread_id, slug) in rows {
        tags.entry(thread_id).or_insert_with(Vec::new).push(slug);
    }
    Ok(tags)
}

/// Resolve a list of tag slugs to their (id, slug) pairs.
pub async fn resolve_tag_slugs(pool: &PgPool, slugs: &[String]) -> AppResult<Vec<(i64, String)>> {
    if slugs.is_empty() {
        return Ok(vec![]);
    }
    let mut results = Vec::new();
    for slug in slugs {
        if let Some(row) = find_tag_by_slug(pool, slug).await? {
            results.push((row.id, row.slug));
        }
    }
    Ok(results)
}

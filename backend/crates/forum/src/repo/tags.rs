//! Tag CRUD and thread-tag association.

use crate::models::TagRow;
use shared::AppResult;
use sqlx::PgPool;

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
    // Get existing tag IDs so we can recalculate thread_count for tags being removed too.
    let old_ids: Vec<i64> =
        sqlx::query_scalar("SELECT tag_id FROM forum.thread_tags WHERE thread_id = $1")
            .bind(thread_id)
            .fetch_all(pool)
            .await?;

    // Remove existing.
    sqlx::query("DELETE FROM forum.thread_tags WHERE thread_id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;

    // Insert new.
    for &tag_id in tag_ids {
        sqlx::query(
            "INSERT INTO forum.thread_tags (thread_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        )
        .bind(thread_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    }

    // Recalculate thread_count for all affected tags (old + new).
    let mut all_affected: Vec<i64> = old_ids;
    all_affected.extend_from_slice(tag_ids);
    all_affected.sort_unstable();
    all_affected.dedup();

    for tag_id in all_affected {
        sqlx::query(
            "UPDATE forum.tags SET thread_count = (SELECT COUNT(*) FROM forum.thread_tags WHERE tag_id = $1) WHERE id = $1"
        )
        .bind(tag_id)
        .execute(pool)
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

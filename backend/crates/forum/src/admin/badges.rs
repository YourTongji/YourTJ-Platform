//! Admin badge management endpoints: list, create badges, and feature threads.
//!
//! Badges live in `platform.badges` and `platform.account_badges` but the
//! award logic (and thus the admin surface) lives in the forum crate because
//! awards are driven by forum actions.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{AppError, AppResult, AppState};

/// A badge row for API responses.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub mint_amount: i64,
    pub created_at: i64,
}

/// Input for creating a new badge.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBadgeInput {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub mint_amount: i64,
}

/// Input for featuring a thread.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureThreadInput {
    pub featured: bool,
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// GET /api/v2/admin/platform/badges — list all badges
pub async fn list_badges(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<BadgeDto>>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    #[derive(sqlx::FromRow)]
    struct BadgeRow {
        id: i64,
        slug: String,
        name: String,
        description: Option<String>,
        icon_url: Option<String>,
        mint_amount: i64,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows: Vec<BadgeRow> = sqlx::query_as(
        "SELECT id, slug, name, description, icon_url, mint_amount, created_at \
         FROM platform.badges ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    let dtos: Vec<BadgeDto> = rows
        .into_iter()
        .map(|r| BadgeDto {
            id: r.id.to_string(),
            slug: r.slug,
            name: r.name,
            description: r.description,
            icon_url: r.icon_url,
            mint_amount: r.mint_amount,
            created_at: r.created_at.timestamp(),
        })
        .collect();

    Ok(Json(dtos))
}

/// POST /api/v2/admin/platform/badges — create a new badge
pub async fn create_badge(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBadgeInput>,
) -> AppResult<Json<BadgeDto>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    // Validate slug format.
    if body.slug.is_empty() {
        return Err(AppError::BadRequest("slug is required".into()));
    }
    if body.name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    #[derive(sqlx::FromRow)]
    struct BadgeRow {
        id: i64,
        slug: String,
        name: String,
        description: Option<String>,
        icon_url: Option<String>,
        mint_amount: i64,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let row: BadgeRow = sqlx::query_as(
        "INSERT INTO platform.badges (slug, name, description, icon_url, mint_amount) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, slug, name, description, icon_url, mint_amount, created_at",
    )
    .bind(&body.slug)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.icon_url)
    .bind(body.mint_amount)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("badges_slug_key") {
                return AppError::Conflict(format!("badge slug '{}' already exists", body.slug));
            }
        }
        AppError::from(e)
    })?;

    Ok(Json(BadgeDto {
        id: row.id.to_string(),
        slug: row.slug,
        name: row.name,
        description: row.description,
        icon_url: row.icon_url,
        mint_amount: row.mint_amount,
        created_at: row.created_at.timestamp(),
    }))
}

/// POST /api/v2/admin/forum/threads/{id}/feature — feature/unfeature a thread
///
/// Featuring a thread marks it as featured and auto-awards the "quality-author"
/// badge to the thread author.
pub async fn feature_thread(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
    Json(body): Json<FeatureThreadInput>,
) -> AppResult<Json<Value>> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_mod().map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;

    // Fetch thread to verify existence and get author.
    let thread = crate::repo::find_thread(&state.db, id).await?.ok_or(AppError::NotFound)?;

    if body.featured {
        // Set featured_at timestamp on the thread.
        sqlx::query(
            "UPDATE forum.threads SET featured_at = now() WHERE id = $1 AND featured_at IS NULL",
        )
        .bind(id)
        .execute(&state.db)
        .await?;

        // Log mod action.
        crate::repo::insert_mod_action(&state.db, auth.id, "feature", "thread", id, None, None)
            .await?;

        // Award quality-author badge (fire-and-forget).
        let pool = state.db.clone();
        let author_id = thread.author_id;
        let awarded_by = auth.id;
        tokio::spawn(async move {
            match crate::badges::award_quality_author_badge(&pool, author_id, awarded_by).await {
                Ok(newly_awarded) => {
                    if newly_awarded {
                        tracing::info!(author_id, "quality-author badge awarded");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, author_id, "failed to award quality-author badge");
                }
            }
        });
    } else {
        // Unfeature: clear featured_at.
        sqlx::query("UPDATE forum.threads SET featured_at = NULL WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;

        crate::repo::insert_mod_action(&state.db, auth.id, "unfeature", "thread", id, None, None)
            .await?;
    }

    // Bump cache.
    shared::cache::bump_version_silent(state.redis.as_ref(), "board", &id.to_string()).await;

    Ok(Json(json!({"ok": true})))
}

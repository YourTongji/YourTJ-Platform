//! Admin badge management endpoints: list, create badges, and feature threads.
//!
//! Badges live in `platform.badges` and `platform.account_badges` but the
//! award logic (and thus the admin surface) lives in the forum crate because
//! awards are driven by forum actions.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
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
    pub reason: String,
}

/// Input for featuring a thread.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureThreadInput {
    pub featured: bool,
    pub reason: String,
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
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
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

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
) -> AppResult<(StatusCode, Json<BadgeDto>)> {
    let auth = identity::auth_middleware::authenticate(
        &headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let slug = body.slug.trim();
    let name = body.name.trim();
    let reason = validate_reason(&body.reason)?;
    if slug.is_empty() || slug.chars().count() > 64 {
        return Err(AppError::BadRequest("slug must be 1–64 characters".into()));
    }
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::BadRequest("name must be 1–100 characters".into()));
    }
    if !(0..=100_000).contains(&body.mint_amount) {
        return Err(AppError::BadRequest("mintAmount must be between 0 and 100000".into()));
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

    let mut tx = state.db.begin().await?;
    let row: BadgeRow = sqlx::query_as(
        "INSERT INTO platform.badges (slug, name, description, icon_url, mint_amount) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, slug, name, description, icon_url, mint_amount, created_at",
    )
    .bind(slug)
    .bind(name)
    .bind(&body.description)
    .bind(&body.icon_url)
    .bind(body.mint_amount)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("badges_slug_key") {
                return AppError::Conflict(format!("badge slug '{slug}' already exists"));
            }
        }
        AppError::from(e)
    })?;

    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        "platform.badge.created",
        "badge",
        &row.id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(BadgeDto {
            id: row.id.to_string(),
            slug: row.slug,
            name: row.name,
            description: row.description,
            icon_url: row.icon_url,
            mint_amount: row.mint_amount,
            created_at: row.created_at.timestamp(),
        }),
    ))
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
    auth.require_capability(shared::auth::Capability::ManageCommunity)
        .map_err(|_| AppError::Forbidden)?;

    let id: i64 = id_str.parse().map_err(|_| AppError::NotFound)?;
    let reason = validate_reason(&body.reason)?;
    let mut tx = state.db.begin().await?;
    let thread: (i64, i64) =
        sqlx::query_as("SELECT author_id, board_id FROM forum.threads WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    super::require_lower_author_role(&mut tx, &auth, Some(thread.0)).await?;

    if body.featured {
        // Set featured_at timestamp on the thread.
        sqlx::query(
            "UPDATE forum.threads SET featured_at = now() WHERE id = $1 AND featured_at IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    } else {
        // Unfeature: clear featured_at.
        sqlx::query("UPDATE forum.threads SET featured_at = NULL WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    let action = if body.featured { "feature" } else { "unfeature" };
    crate::repo::insert_mod_action(&mut *tx, auth.id, action, "thread", id, Some(reason), None)
        .await?;
    governance::record_account_event_tx(
        &mut tx,
        governance::AccountActor { account_id: auth.id, role: &auth.role },
        &format!("forum.thread.{action}"),
        "thread",
        &id.to_string(),
        reason,
        None,
    )
    .await?;
    tx.commit().await?;

    if body.featured {
        let pool = state.db.clone();
        let author_id = thread.0;
        let awarded_by = auth.id;
        tokio::spawn(async move {
            match crate::badges::award_quality_author_badge(&pool, author_id, awarded_by).await {
                Ok(true) => tracing::info!(author_id, "quality-author badge awarded"),
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(?error, author_id, "failed to award quality-author badge");
                }
            }
        });
    }

    crate::cache::invalidate_thread_surfaces(state.redis.as_ref(), id, thread.1).await;

    Ok(Json(json!({"ok": true})))
}

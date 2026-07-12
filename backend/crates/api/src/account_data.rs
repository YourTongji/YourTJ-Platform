//! Gateway composition for owner-data exports and cross-domain lifecycle cleanup.

use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Serialize;
use shared::{AppError, AppResult, AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DataExportJobDto {
    id: String,
    status: String,
    created_at: i64,
    updated_at: i64,
    expires_at: i64,
    error_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadGrantDto {
    token: String,
    expires_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountDataExportBundle {
    schema_version: &'static str,
    generated_at: i64,
    included_sections: [&'static str; 8],
    identity: identity::data_export::IdentityExport,
    forum: forum::data_export::ForumExport,
    reviews: reviews::data_export::ReviewsExport,
    governance: governance::data_export::GovernanceExport,
    credit: credit::data_export::CreditExport,
    activity: Vec<activity::data_export::ExportActivityDay>,
    platform: platform::data_export::PlatformExport,
    media: Vec<media::data_export::ExportUpload>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MediaPurgeDecision {
    Complete,
    Defer(&'static str),
    Block(&'static str),
}

fn decide_media_purge(progress: media::AccountMediaPurgeProgress) -> MediaPurgeDecision {
    if progress.dead_letter_deletions > 0 {
        MediaPurgeDecision::Block("media_deletion_dead_letter")
    } else if progress.missing_deletion_jobs > 0 {
        MediaPurgeDecision::Block("media_deletion_job_missing")
    } else if progress.has_more || progress.pending_deletions > 0 {
        MediaPurgeDecision::Defer("media_deletion_pending")
    } else {
        MediaPurgeDecision::Complete
    }
}

async fn apply_media_purge_decision(
    pool: &sqlx::PgPool,
    job: &identity::lifecycle::LifecycleJob,
    progress: media::AccountMediaPurgeProgress,
) -> AppResult<Option<i64>> {
    let decision = decide_media_purge(progress);
    match decision {
        MediaPurgeDecision::Block(_) => tracing::warn!(
            job_id = job.id,
            account_id = job.account_id,
            scheduled = progress.scheduled,
            has_more = progress.has_more,
            pending_deletions = progress.pending_deletions,
            dead_letter_deletions = progress.dead_letter_deletions,
            retained_assets = progress.retained_assets,
            missing_deletion_jobs = progress.missing_deletion_jobs,
            ?decision,
            "account media purge reached a terminal blocker"
        ),
        MediaPurgeDecision::Complete | MediaPurgeDecision::Defer(_) => tracing::info!(
            job_id = job.id,
            account_id = job.account_id,
            scheduled = progress.scheduled,
            has_more = progress.has_more,
            pending_deletions = progress.pending_deletions,
            dead_letter_deletions = progress.dead_letter_deletions,
            retained_assets = progress.retained_assets,
            missing_deletion_jobs = progress.missing_deletion_jobs,
            ?decision,
            "account media purge progress evaluated"
        ),
    }
    match decision {
        MediaPurgeDecision::Complete => identity::lifecycle::complete_purge(pool, job).await,
        MediaPurgeDecision::Defer(error_code) => {
            identity::lifecycle::defer_running_job(pool, job, Duration::seconds(60), error_code)
                .await?;
            Ok(None)
        }
        MediaPurgeDecision::Block(error_code) => {
            identity::lifecycle::block_running_job(pool, job, error_code).await?;
            Ok(None)
        }
    }
}

fn job_dto(job: identity::data_export::ExportJobRecord) -> DataExportJobDto {
    DataExportJobDto {
        id: job.id.to_string(),
        status: job.status,
        created_at: job.created_at.timestamp(),
        updated_at: job.updated_at.timestamp(),
        expires_at: job.expires_at.timestamp(),
        error_code: job.error_code,
    }
}

async fn auth_context(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<identity::auth_middleware::AuthenticatedContext> {
    identity::auth_middleware::authenticate_context_allow_incomplete_onboarding(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

async fn create_export(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<(StatusCode, Json<DataExportJobDto>)> {
    let context = auth_context(&state, &headers).await?;
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Idempotency-Key is required".into()))?;
    let job = identity::data_export::create_job(&state.db, &context, idempotency_key).await?;
    Ok((StatusCode::ACCEPTED, Json(job_dto(job))))
}

async fn list_exports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<DataExportJobDto>>> {
    let context = auth_context(&state, &headers).await?;
    let jobs = identity::data_export::list_jobs(&state.db, context.account.id)
        .await?
        .into_iter()
        .map(job_dto)
        .collect();
    Ok(Json(jobs))
}

async fn get_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(export_id): Path<String>,
) -> AppResult<Json<DataExportJobDto>> {
    let context = auth_context(&state, &headers).await?;
    let export_id = export_id
        .parse::<uuid::Uuid>()
        .map_err(|_| AppError::BadRequest("invalid export id".into()))?;
    let job = identity::data_export::get_job(&state.db, context.account.id, export_id).await?;
    Ok(Json(job_dto(job)))
}

async fn create_download_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(export_id): Path<String>,
) -> AppResult<Json<DownloadGrantDto>> {
    let context = auth_context(&state, &headers).await?;
    let export_id = export_id
        .parse::<uuid::Uuid>()
        .map_err(|_| AppError::BadRequest("invalid export id".into()))?;
    let grant = identity::data_export::issue_download_grant(&state.db, &context, export_id).await?;
    Ok(Json(DownloadGrantDto { token: grant.token, expires_at: grant.expires_at.timestamp() }))
}

async fn download_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(export_id): Path<String>,
) -> AppResult<(HeaderMap, Json<serde_json::Value>)> {
    let context = auth_context(&state, &headers).await?;
    let export_id = export_id
        .parse::<uuid::Uuid>()
        .map_err(|_| AppError::BadRequest("invalid export id".into()))?;
    let token = headers
        .get("X-Export-Token")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let email = identity::data_export::owner_email(
        &state.db,
        state.email_encryption.as_ref(),
        context.account.id,
    )
    .await?;
    let mut artifact = identity::data_export::consume_download_grant(
        &state.db,
        context.account.id,
        export_id,
        token,
    )
    .await?;
    let account = artifact
        .pointer_mut("/identity/account")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("invalid account export artifact")))?;
    account.insert("email".into(), serde_json::Value::String(email));
    let mut response_headers = HeaderMap::new();
    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response_headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=yourtj-account-export.json"),
    );
    response_headers.insert("Cache-Control", HeaderValue::from_static("no-store"));
    Ok((response_headers, Json(artifact)))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/me/data-exports", get(list_exports).post(create_export))
        .route("/api/v2/me/data-exports/{id}", get(get_export))
        .route("/api/v2/me/data-exports/{id}/download-grant", post(create_download_grant))
        .route("/api/v2/me/data-exports/{id}/download", get(download_export))
        .with_state(state)
}

async fn process_export_job(state: &AppState) -> AppResult<bool> {
    let Some(job) = identity::data_export::claim_job(&state.db).await? else {
        return Ok(false);
    };
    let result = async {
        let bundle = assemble_export(state, job.account_id).await?;
        identity::data_export::complete_job(&state.db, job.id, &bundle).await
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(?error, export_id = %job.id, "account export job failed");
        identity::data_export::fail_job(&state.db, job.id, "domain_projection_failed").await?;
    }
    Ok(true)
}

async fn assemble_export(state: &AppState, account_id: i64) -> AppResult<AccountDataExportBundle> {
    let (identity, forum, reviews, governance, credit, activity, platform, media) = tokio::try_join!(
        identity::data_export::snapshot(&state.db, state.email_encryption.as_ref(), account_id,),
        forum::data_export::snapshot(&state.db, account_id),
        reviews::data_export::snapshot(&state.db, account_id),
        governance::data_export::snapshot(&state.db, account_id),
        credit::data_export::snapshot(&state.db, account_id),
        activity::data_export::snapshot(&state.db, account_id),
        platform::data_export::snapshot(&state.db, account_id),
        media::data_export::snapshot(&state.db, account_id),
    )?;
    Ok(AccountDataExportBundle {
        schema_version: "yourtj.account-export.v1",
        generated_at: Utc::now().timestamp(),
        included_sections: [
            "identity",
            "forum",
            "reviews",
            "governance",
            "credit",
            "activity",
            "platform",
            "mediaMetadata",
        ],
        identity,
        forum,
        reviews,
        governance,
        credit,
        activity,
        platform,
        media,
    })
}

async fn process_lifecycle_job(state: &AppState) -> AppResult<bool> {
    let Some(job) = identity::lifecycle::claim_due_job(&state.db).await? else {
        return Ok(false);
    };
    let result = match job.job_type.as_str() {
        "mark_deleted" => identity::lifecycle::complete_mark_deleted(&state.db, &job).await,
        "purge" => {
            let cleanup = tokio::try_join!(
                forum::data_export::purge_account_private_data(&state.db, job.account_id),
                reviews::data_export::purge_account_private_data(&state.db, job.account_id),
                credit::data_export::purge_account_private_data(&state.db, job.account_id),
                activity::data_export::purge_account_data(&state.db, job.account_id),
                platform::data_export::purge_account_private_data(&state.db, job.account_id),
                media::prepare_account_media_purge(
                    &state.db,
                    job.account_id,
                    state.config.media_retention_gc_enabled,
                ),
            );
            match cleanup {
                Ok((_, _, _, _, _, progress)) => {
                    apply_media_purge_decision(&state.db, &job, progress).await
                }
                Err(error) => Err(error),
            }
        }
        _ => Err(AppError::Internal(anyhow::anyhow!("unknown account lifecycle job type"))),
    };
    match result {
        Ok(Some(account_id)) => {
            if let Err(error) = identity::public_search::reconcile_user(
                &state.db,
                &state.meili_url,
                &state.meili_master_key,
                account_id,
            )
            .await
            {
                tracing::warn!(?error, account_id, "closed account search reconciliation failed");
            }
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(?error, job_id = job.id, "account lifecycle job failed");
            match identity::lifecycle::fail_job(&state.db, &job, "owner_cleanup_failed").await {
                Ok(()) => {}
                Err(AppError::Conflict(_)) => tracing::info!(
                    job_id = job.id,
                    "account lifecycle job lease was superseded before failure persistence"
                ),
                Err(failure_error) => return Err(failure_error),
            }
        }
    }
    Ok(true)
}

pub fn spawn_workers(state: AppState) {
    tokio::spawn(async move {
        loop {
            let mut did_work = false;
            match process_export_job(&state).await {
                Ok(processed) => did_work |= processed,
                Err(error) => tracing::warn!(?error, "account export worker failed"),
            }
            match process_lifecycle_job(&state).await {
                Ok(processed) => did_work |= processed,
                Err(error) => tracing::warn!(?error, "account lifecycle worker failed"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(if did_work { 1 } else { 15 })).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{assemble_export, decide_media_purge, MediaPurgeDecision};

    fn media_progress(
        has_more: bool,
        pending_deletions: i64,
        dead_letter_deletions: i64,
        retained_assets: i64,
        missing_deletion_jobs: i64,
    ) -> media::AccountMediaPurgeProgress {
        media::AccountMediaPurgeProgress {
            scheduled: 0,
            has_more,
            pending_deletions,
            dead_letter_deletions,
            retained_assets,
            missing_deletion_jobs,
        }
    }

    #[test]
    fn media_purge_decision_blocks_terminal_anomalies_before_waiting() {
        assert_eq!(
            decide_media_purge(media_progress(true, 1, 1, 0, 1)),
            MediaPurgeDecision::Block("media_deletion_dead_letter")
        );
        assert_eq!(
            decide_media_purge(media_progress(true, 1, 0, 0, 1)),
            MediaPurgeDecision::Block("media_deletion_job_missing")
        );
    }

    #[test]
    fn media_purge_decision_defers_bounded_or_provider_work() {
        assert_eq!(
            decide_media_purge(media_progress(true, 0, 0, 0, 0)),
            MediaPurgeDecision::Defer("media_deletion_pending")
        );
        assert_eq!(
            decide_media_purge(media_progress(false, 1, 0, 0, 0)),
            MediaPurgeDecision::Defer("media_deletion_pending")
        );
    }

    #[test]
    fn media_purge_decision_allows_policy_retained_assets() {
        assert_eq!(
            decide_media_purge(media_progress(false, 0, 0, 3, 0)),
            MediaPurgeDecision::Complete
        );
        assert_eq!(
            decide_media_purge(media_progress(false, 0, 0, 0, 0)),
            MediaPurgeDecision::Complete
        );
    }

    #[tokio::test]
    async fn export_composition_uses_every_owner_projection_on_a_fresh_schema() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://yourtj:yourtj@localhost:5432/yourtj_test".to_string());
        let pool =
            sqlx::PgPool::connect(&database_url).await.expect("connect export test database");
        sqlx::migrate!("../../migrations").run(&pool).await.expect("apply export test migrations");
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let account_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.accounts (email, email_verified_at, handle) \
             VALUES ($1, now(), $2) RETURNING id",
        )
        .bind(format!("export-{suffix}@tongji.edu.cn"))
        .bind(format!("export-{suffix}"))
        .fetch_one(&pool)
        .await
        .expect("insert export account");
        sqlx::query("INSERT INTO identity.profiles (account_id) VALUES ($1)")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("insert export profile");
        sqlx::query("INSERT INTO identity.profile_privacy (account_id) VALUES ($1)")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("insert export privacy");
        sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
            .bind(account_id)
            .bind(format!("ledger-verification-key-{suffix}"))
            .execute(&pool)
            .await
            .expect("insert ledger verification key");

        let mut config = shared::Config::from_env().expect("test config");
        config.database_url = database_url;
        let state = shared::AppState {
            db: pool,
            config,
            jwt_secret: "export-test-jwt-secret-32-bytes".into(),
            jwt_ttl: 900,
            refresh_ttl: 604800,
            meili_url: String::new(),
            meili_master_key: String::new(),
            redis: None,
            system_private_key: vec![0; 32],
            system_public_key_b64: String::new(),
            email_encryption: None,
            captcha_verifier: None,
            sse_tx: None,
        };
        let bundle = assemble_export(&state, account_id).await.expect("assemble owner export");
        let artifact = serde_json::to_value(bundle).expect("serialize owner export");

        assert_eq!(artifact["schemaVersion"], "yourtj.account-export.v1");
        assert_eq!(artifact["identity"]["account"]["id"], account_id.to_string());
        assert!(artifact["identity"]["account"].get("email").is_none());
        assert_eq!(artifact["includedSections"].as_array().map(Vec::len), Some(8));

        sqlx::query(
            "UPDATE identity.accounts SET status = 'deleted', \
                 deletion_requested_at = now() - interval '31 days', \
                 deletion_recover_until = now() - interval '1 day', deleted_at = now(), \
                 purge_started_at = now(), \
                 lifecycle_version = lifecycle_version + 1 WHERE id = $1",
        )
        .bind(account_id)
        .execute(&state.db)
        .await
        .expect("make account purgeable");
        let lease_token = uuid::Uuid::new_v4();
        let purge_job_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.account_lifecycle_jobs \
             (account_id, job_type, status, attempts, next_attempt_at, locked_at, lease_token) \
             VALUES ($1, 'purge', 'running', 1, now(), now(), $2) RETURNING id",
        )
        .bind(account_id)
        .bind(lease_token)
        .fetch_one(&state.db)
        .await
        .expect("insert purge job");
        tokio::try_join!(
            forum::data_export::purge_account_private_data(&state.db, account_id),
            reviews::data_export::purge_account_private_data(&state.db, account_id),
            credit::data_export::purge_account_private_data(&state.db, account_id),
            activity::data_export::purge_account_data(&state.db, account_id),
            platform::data_export::purge_account_private_data(&state.db, account_id),
            media::prepare_account_media_purge(&state.db, account_id, true),
        )
        .expect("run every owner purge projection");
        identity::lifecycle::complete_purge(
            &state.db,
            &identity::lifecycle::LifecycleJob {
                id: purge_job_id,
                account_id,
                job_type: "purge".into(),
                lease_token,
            },
        )
        .await
        .expect("finish identity tombstone");
        let status: String =
            sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
                .bind(account_id)
                .fetch_one(&state.db)
                .await
                .expect("read purged account");
        assert_eq!(status, "purged");
        let key_revoked: bool = sqlx::query_scalar(
            "SELECT revoked_at IS NOT NULL FROM identity.account_keys WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_one(&state.db)
        .await
        .expect("retain ledger verification key");
        assert!(key_revoked);
    }

    #[tokio::test]
    async fn partial_owner_cleanup_failure_cannot_reopen_an_account_after_purge_start() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://yourtj:yourtj@localhost:5432/yourtj_test".to_string());
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("connect purge barrier test database");
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("apply purge barrier test migrations");
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let account_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.accounts (email, email_verified_at, handle) \
             VALUES ($1, now(), $2) RETURNING id",
        )
        .bind(format!("purge-barrier-{suffix}@tongji.edu.cn"))
        .bind(format!("barrier-{suffix}"))
        .fetch_one(&pool)
        .await
        .expect("insert purge barrier account");
        sqlx::query(
            "UPDATE identity.accounts SET status = 'deleted', \
                 deletion_requested_at = now() - interval '1 day', \
                 deletion_recover_until = now() + interval '1 hour', deleted_at = now(), \
                 lifecycle_version = lifecycle_version + 1 WHERE id = $1",
        )
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("make account recoverable before purge starts");
        let recovery =
            identity::lifecycle::issue_recovery_credential(&pool, account_id, "password")
                .await
                .expect("issue pre-purge recovery credential");
        sqlx::query(
            "INSERT INTO activity.daily_counts \
             (account_id, activity_date, threads_created, comments_created, likes_given) \
             VALUES ($1, current_date, 1, 2, 3)",
        )
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("insert owner-domain projection");
        let purge_job_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.account_lifecycle_jobs \
             (account_id, job_type, next_attempt_at) \
             VALUES ($1, 'purge', now() - interval '100 years') RETURNING id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("insert early purge job");

        let early_claim =
            identity::lifecycle::claim_due_job(&pool).await.expect("evaluate early purge job");
        assert!(early_claim.is_none());
        let early_state: (String, Option<chrono::DateTime<chrono::Utc>>, i64) = sqlx::query_as(
            "SELECT job.status, account.purge_started_at, \
                    (SELECT COUNT(*) FROM activity.daily_counts WHERE account_id = account.id) \
             FROM identity.account_lifecycle_jobs job \
             JOIN identity.accounts account ON account.id = job.account_id \
             WHERE job.id = $1",
        )
        .bind(purge_job_id)
        .fetch_one(&pool)
        .await
        .expect("read early purge state");
        assert_eq!(early_state.0, "queued");
        assert!(early_state.1.is_none());
        assert_eq!(early_state.2, 1);

        sqlx::query(
            "UPDATE identity.accounts SET deletion_recover_until = now() - interval '1 minute' \
             WHERE id = $1",
        )
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("expire recovery deadline");
        sqlx::query(
            "UPDATE identity.account_lifecycle_jobs \
             SET next_attempt_at = now() - interval '100 years' WHERE id = $1",
        )
        .bind(purge_job_id)
        .execute(&pool)
        .await
        .expect("make purge job due");
        let purge_job = identity::lifecycle::claim_due_job(&pool)
            .await
            .expect("claim purge after deadline")
            .expect("purge job should be due");
        assert_eq!(purge_job.id, purge_job_id);
        let purge_started: bool = sqlx::query_scalar(
            "SELECT purge_started_at IS NOT NULL FROM identity.accounts WHERE id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("read irreversible purge marker");
        assert!(purge_started);

        sqlx::query(
            "UPDATE identity.accounts SET deletion_recover_until = now() + interval '1 hour' \
             WHERE id = $1",
        )
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("make the old recovery window appear open");
        activity::data_export::purge_account_data(&pool, account_id)
            .await
            .expect("commit first owner cleanup");
        identity::lifecycle::fail_job(&pool, &purge_job, "simulated_owner_cleanup_failure")
            .await
            .expect("persist later owner cleanup failure");
        let owner_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM activity.daily_counts WHERE account_id = $1")
                .bind(account_id)
                .fetch_one(&pool)
                .await
                .expect("read partially purged owner data");
        assert_eq!(owner_rows, 0);
        assert!(matches!(
            identity::lifecycle::reactivate(&pool, &recovery.token).await,
            Err(shared::AppError::Forbidden)
        ));

        sqlx::query(
            "UPDATE identity.account_lifecycle_jobs SET status = 'queued', locked_at = NULL, \
                 next_attempt_at = now() + interval '1 hour' WHERE id = $1",
        )
        .bind(purge_job_id)
        .execute(&pool)
        .await
        .expect("isolate marker from failed job status");
        assert!(matches!(
            identity::lifecycle::reactivate(&pool, &recovery.token).await,
            Err(shared::AppError::Forbidden)
        ));

        sqlx::query(
            "UPDATE identity.accounts SET deletion_recover_until = now() - interval '1 minute' \
             WHERE id = $1",
        )
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("restore expired recovery deadline");
        sqlx::query(
            "UPDATE identity.account_lifecycle_jobs \
             SET next_attempt_at = now() - interval '100 years' WHERE id = $1",
        )
        .bind(purge_job_id)
        .execute(&pool)
        .await
        .expect("make retry due");
        let retry = identity::lifecycle::claim_due_job(&pool)
            .await
            .expect("claim purge retry")
            .expect("purge retry should be due");
        assert_eq!(retry.id, purge_job_id);
        tokio::try_join!(
            forum::data_export::purge_account_private_data(&pool, account_id),
            reviews::data_export::purge_account_private_data(&pool, account_id),
            credit::data_export::purge_account_private_data(&pool, account_id),
            activity::data_export::purge_account_data(&pool, account_id),
            platform::data_export::purge_account_private_data(&pool, account_id),
            media::prepare_account_media_purge(&pool, account_id, true),
        )
        .expect("retry every idempotent owner cleanup");
        identity::lifecycle::complete_purge(&pool, &retry)
            .await
            .expect("finish retry with identity tombstone");
        let final_state: String =
            sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
                .bind(account_id)
                .fetch_one(&pool)
                .await
                .expect("read final lifecycle state");
        assert_eq!(final_state, "purged");
    }
}

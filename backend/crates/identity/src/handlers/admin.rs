//! Staff account directory, invitation, session, and sanction workflows.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use governance::AccountActor;
use serde::Deserialize;
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, AuthAccount, Page};

use crate::dto::{
    AdminLifecycleJobDto, AdminReasonInput, AdminUserDto, AdminUserInviteInput, AdminUserRoleInput,
    SanctionDto, SanctionInput, UnsanctionInput,
};
use crate::repo;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUsersQuery {
    q: Option<String>,
    role: Option<String>,
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

/// Bounded filters for the account lifecycle operator queue.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLifecycleJobsQuery {
    account_id: Option<String>,
    job_type: Option<String>,
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LifecycleJobRow {
    id: i64,
    account_id: i64,
    account_handle: String,
    account_state: String,
    job_type: String,
    status: String,
    attempts: i16,
    next_attempt_at: DateTime<Utc>,
    locked_at: Option<DateTime<Utc>>,
    last_error_code: Option<String>,
    purge_started_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct SanctionRow {
    id: i64,
    account_id: i64,
    kind: String,
    reason: String,
    issued_by: Option<i64>,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

fn admin_user_dto(row: repo::AdminUserRow) -> AdminUserDto {
    AdminUserDto {
        id: row.id.to_string(),
        handle: row.handle,
        avatar_url: row.avatar_url,
        role: row.role,
        status: row.status,
        trust_level: row.trust_level,
        last_active_at: row.last_active_at.map(|timestamp| timestamp.timestamp()),
        created_at: row.created_at.timestamp(),
    }
}

fn lifecycle_job_dto(row: LifecycleJobRow) -> AdminLifecycleJobDto {
    AdminLifecycleJobDto {
        id: row.id.to_string(),
        account_id: row.account_id.to_string(),
        account_handle: row.account_handle,
        account_state: row.account_state,
        job_type: row.job_type,
        status: row.status,
        attempts: row.attempts,
        next_attempt_at: row.next_attempt_at.timestamp(),
        locked_at: row.locked_at.map(|timestamp| timestamp.timestamp()),
        last_error_code: row.last_error_code,
        purge_started_at: row.purge_started_at.map(|timestamp| timestamp.timestamp()),
        created_at: row.created_at.timestamp(),
        updated_at: row.updated_at.timestamp(),
    }
}

async fn find_lifecycle_job(pool: &sqlx::PgPool, job_id: i64) -> AppResult<LifecycleJobRow> {
    sqlx::query_as::<_, LifecycleJobRow>(
        "SELECT job.id, job.account_id, account.handle::text AS account_handle, \
                account.status::text AS account_state, job.job_type, job.status, job.attempts, \
                job.next_attempt_at, job.locked_at, job.last_error_code, \
                account.purge_started_at, job.created_at, job.updated_at \
         FROM identity.account_lifecycle_jobs job \
         JOIN identity.accounts account ON account.id = job.account_id \
         WHERE job.id = $1",
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

async fn authenticate(headers: &HeaderMap, state: &AppState) -> AppResult<AuthAccount> {
    crate::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

async fn authenticate_context(
    headers: &HeaderMap,
    state: &AppState,
) -> AppResult<crate::auth_middleware::AuthenticatedContext> {
    crate::auth_middleware::authenticate_context(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

fn parse_id(value: &str, field: &str) -> AppResult<i64> {
    value.parse().map_err(|_| AppError::BadRequest(format!("invalid {field}")))
}

fn validate_reason(reason: &str) -> AppResult<&str> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return Err(AppError::BadRequest("reason must be 3–500 characters".into()));
    }
    Ok(reason)
}

fn validate_directory_filter(value: Option<&str>, allowed: &[&str], name: &str) -> AppResult<()> {
    if value.is_some_and(|value| !allowed.contains(&value)) {
        return Err(AppError::BadRequest(format!("invalid {name}")));
    }
    Ok(())
}

fn role_rank(role: &str) -> i8 {
    match role {
        "admin" => 2,
        "mod" => 1,
        _ => 0,
    }
}

fn require_lower_role(actor: &AuthAccount, target_id: i64, target_role: &str) -> AppResult<()> {
    if actor.id == target_id || role_rank(&actor.role) <= role_rank(target_role) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn sanction_end(timestamp: Option<i64>) -> AppResult<Option<DateTime<Utc>>> {
    timestamp
        .map(|timestamp| {
            DateTime::from_timestamp(timestamp, 0)
                .filter(|ends_at| *ends_at > Utc::now())
                .ok_or_else(|| AppError::BadRequest("endsAt must be a future timestamp".into()))
        })
        .transpose()
}

async fn invalidate_sanction_cache(state: &AppState, account_id: i64) {
    let Some(redis) = &state.redis else {
        return;
    };
    let Ok(mut connection) = redis.get().await else {
        return;
    };
    let silence_key = format!("identity:silence:{account_id}");
    let suspend_key = format!("identity:suspend:{account_id}");
    let result: redis::RedisResult<()> =
        redis::cmd("DEL").arg(silence_key).arg(suspend_key).query_async(&mut connection).await;
    if let Err(error) = result {
        tracing::warn!(?error, account_id, "failed to invalidate sanction cache");
    }
}

/// GET /api/v2/admin/account-lifecycle/jobs — inspect durable lifecycle work and dead letters.
pub async fn list_lifecycle_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminLifecycleJobsQuery>,
) -> AppResult<Json<Page<AdminLifecycleJobDto>>> {
    let auth = authenticate(&headers, &state).await?;
    auth.require_capability(Capability::RunOperations).map_err(|_| AppError::Forbidden)?;
    validate_directory_filter(query.job_type.as_deref(), &["mark_deleted", "purge"], "jobType")?;
    validate_directory_filter(
        query.status.as_deref(),
        &["queued", "running", "succeeded", "failed"],
        "status",
    )?;
    let account_id =
        query.account_id.as_deref().map(|value| parse_id(value, "accountId")).transpose()?;
    let cursor = query.cursor.as_deref().map(|value| parse_id(value, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let rows = sqlx::query_as::<_, LifecycleJobRow>(
        "SELECT job.id, job.account_id, account.handle::text AS account_handle, \
                account.status::text AS account_state, job.job_type, job.status, job.attempts, \
                job.next_attempt_at, job.locked_at, job.last_error_code, \
                account.purge_started_at, job.created_at, job.updated_at \
         FROM identity.account_lifecycle_jobs job \
         JOIN identity.accounts account ON account.id = job.account_id \
         WHERE ($1::bigint IS NULL OR job.account_id = $1) \
           AND ($2::text IS NULL OR job.job_type = $2) \
           AND ($3::text IS NULL OR job.status = $3) \
           AND ($4::bigint IS NULL OR job.id < $4) \
         ORDER BY job.id DESC LIMIT $5",
    )
    .bind(account_id)
    .bind(query.job_type.as_deref())
    .bind(query.status.as_deref())
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible_rows = if has_more { &rows[..limit as usize] } else { &rows };
    let next_cursor = has_more.then(|| visible_rows.last().map(|row| row.id.to_string())).flatten();
    let items = visible_rows.iter().cloned().map(lifecycle_job_dto).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/admin/account-lifecycle/jobs/{id}/requeue — repair one failed job.
pub async fn requeue_lifecycle_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AdminReasonInput>,
) -> AppResult<Json<AdminLifecycleJobDto>> {
    let auth_context = authenticate_context(&headers, &state).await?;
    let auth = &auth_context.account;
    auth.require_capability(Capability::RunOperations).map_err(|_| AppError::Forbidden)?;
    let job_id = parse_id(&job_id, "job id")?;
    let reason = validate_reason(&body.reason)?;
    let mut tx = state.db.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(&auth_context, &mut tx).await?;
    let job: Option<(i64, String, String, i16, Option<String>, bool)> = sqlx::query_as(
        "SELECT job.account_id, job.job_type, job.status, job.attempts, job.last_error_code, \
                account.purge_started_at IS NOT NULL AS purge_started \
         FROM identity.account_lifecycle_jobs job \
         JOIN identity.accounts account ON account.id = job.account_id \
         WHERE job.id = $1 FOR UPDATE OF job",
    )
    .bind(job_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (account_id, job_type, previous_status, previous_attempts, last_error_code, purge_started) =
        job.ok_or(AppError::NotFound)?;
    if previous_status != "failed" {
        return Err(AppError::Conflict("only failed lifecycle jobs can be requeued".into()));
    }
    sqlx::query(
        "UPDATE identity.account_lifecycle_jobs \
         SET status = 'queued', attempts = 0, next_attempt_at = now(), locked_at = NULL, \
             last_error_code = NULL, updated_at = now() WHERE id = $1",
    )
    .bind(job_id)
    .execute(&mut *tx)
    .await?;
    let metadata = serde_json::json!({
        "accountId": account_id.to_string(),
        "jobType": job_type,
        "previousAttempts": previous_attempts,
        "lastErrorCode": last_error_code,
        "purgeStarted": purge_started,
    });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "identity.lifecycle_job.requeued",
        "account_lifecycle_job",
        &job_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(lifecycle_job_dto(find_lifecycle_job(&state.db, job_id).await?)))
}

/// GET /api/v2/admin/users — privacy-safe user search for staff.
pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminUsersQuery>,
) -> AppResult<Json<Page<AdminUserDto>>> {
    let auth = authenticate(&headers, &state).await?;
    auth.require_capability(Capability::SearchUsers).map_err(|_| AppError::Forbidden)?;
    validate_directory_filter(query.role.as_deref(), &["user", "mod", "admin"], "role")?;
    validate_directory_filter(
        query.status.as_deref(),
        &["active", "suspended", "deactivated", "deletion_requested", "deleted", "purged"],
        "status",
    )?;
    let cursor = query.cursor.as_deref().map(|cursor| parse_id(cursor, "cursor")).transpose()?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let query_text = query.q.as_deref().map(str::trim).filter(|query| !query.is_empty());
    if query_text.is_some_and(|query| query.chars().count() > 100) {
        return Err(AppError::BadRequest("q must be at most 100 characters".into()));
    }
    let rows = repo::list_admin_users(
        &state.db,
        cursor,
        limit,
        query_text,
        query.role.as_deref(),
        query.status.as_deref(),
    )
    .await?;
    let has_more = rows.len() > limit as usize;
    let visible_rows = if has_more { &rows[..limit as usize] } else { &rows };
    let next_cursor = has_more.then(|| visible_rows.last().map(|row| row.id.to_string())).flatten();
    let items = visible_rows.iter().map(|row| admin_user_dto(row.to_owned())).collect();
    Ok(Json(Page::new(items, next_cursor)))
}

/// POST /api/v2/admin/users — provision an unverified campus invitation.
pub async fn invite_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AdminUserInviteInput>,
) -> AppResult<(StatusCode, Json<AdminUserDto>)> {
    let auth = authenticate(&headers, &state).await?;
    auth.require_capability(Capability::InviteUsers).map_err(|_| AppError::Forbidden)?;
    let reason = validate_reason(&body.reason)?;
    let email = super::normalize_campus_email(&body.email)?;
    let handle = body.handle.trim().to_lowercase();
    super::validate_handle(&handle)?;
    if repo::find_account_by_email(&state.db, state.email_encryption.as_ref(), &email)
        .await?
        .is_some()
    {
        return Err(crate::error::IdentityError::EmailAlreadyUsed.into());
    }
    let handle_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE handle = $1)")
            .bind(&handle)
            .fetch_one(&state.db)
            .await?;
    if handle_exists {
        return Err(crate::error::IdentityError::HandleTaken.into());
    }

    let mut tx = state.db.begin().await?;
    let account = repo::insert_invited_account(
        &mut tx,
        state.email_encryption.as_ref(),
        &email,
        &handle,
        "user",
        auth.id,
    )
    .await?;
    let metadata = serde_json::json!({ "role": "user" });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "identity.user.invited",
        "account",
        &account.id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;

    let invitation = crate::email_templates::community_invitation();
    if let Err(error) = shared::email::send_email(
        &state.config,
        &email,
        invitation.subject,
        &invitation.text,
        Some(&invitation.html),
    )
    .await
    {
        tracing::warn!(?error, account_id = account.id, "community invitation email failed");
    }
    let row = repo::find_admin_user(&state.db, account.id).await?.ok_or(AppError::NotFound)?;
    Ok((StatusCode::CREATED, Json(admin_user_dto(row))))
}

/// PATCH /api/v2/admin/users/{id}/role — change role with hierarchy protection.
pub async fn change_role(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AdminUserRoleInput>,
) -> AppResult<Json<AdminUserDto>> {
    let auth_context = authenticate_context(&headers, &state).await?;
    let auth = &auth_context.account;
    auth.require_capability(Capability::ChangeRoles).map_err(|_| AppError::Forbidden)?;
    let reason = validate_reason(&body.reason)?;
    if !matches!(body.role.as_str(), "user" | "mod") {
        return Err(AppError::BadRequest(
            "role changes support only user/mod; administrator provisioning is out of band".into(),
        ));
    }
    let account_id = parse_id(&account_id, "account id")?;
    let mut tx = state.db.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(&auth_context, &mut tx).await?;
    let old_role: String =
        sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    require_lower_role(auth, account_id, &old_role)?;
    if old_role == body.role {
        return Err(AppError::Conflict("account already has that role".into()));
    }
    sqlx::query(
        "UPDATE identity.accounts SET role = $1::identity.account_role, updated_at = now(), \
                auth_version = auth_version + 1, legacy_access_revoked_before = now() \
         WHERE id = $2",
    )
    .bind(&body.role)
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE identity.sessions SET revoked_at = now() \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    let metadata = serde_json::json!({ "oldRole": old_role, "newRole": body.role });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "identity.user.role_changed",
        "account",
        &account_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    crate::public_search::reconcile_user_in_background(&state, account_id);
    let updated = repo::find_admin_user(&state.db, account_id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(admin_user_dto(updated)))
}

/// POST /api/v2/admin/users/{id}/sessions/revoke — force sign-out on every device.
pub async fn revoke_sessions(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AdminReasonInput>,
) -> AppResult<StatusCode> {
    let auth_context = authenticate_context(&headers, &state).await?;
    let auth = &auth_context.account;
    auth.require_capability(Capability::SuspendUsers).map_err(|_| AppError::Forbidden)?;
    let reason = validate_reason(&body.reason)?;
    let account_id = parse_id(&account_id, "account id")?;
    let mut tx = state.db.begin().await?;
    crate::auth_middleware::require_recent_auth_tx(&auth_context, &mut tx).await?;
    let target_role: String =
        sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    require_lower_role(auth, account_id, &target_role)?;
    let revoked = sqlx::query(
        "UPDATE identity.sessions SET revoked_at = now() \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query(
        "UPDATE identity.accounts SET auth_version = auth_version + 1, \
                legacy_access_revoked_before = now() WHERE id = $1",
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await?;
    let metadata = serde_json::json!({ "revokedSessionCount": revoked });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "identity.user.sessions_revoked",
        "account",
        &account_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_sanction(
    state: &AppState,
    auth_context: &crate::auth_middleware::AuthenticatedContext,
    account_id: i64,
    kind: &str,
    body: SanctionInput,
) -> AppResult<()> {
    let auth = &auth_context.account;
    let capability =
        if kind == "suspend" { Capability::SuspendUsers } else { Capability::SilenceUsers };
    auth.require_capability(capability).map_err(|_| AppError::Forbidden)?;
    let reason = validate_reason(&body.reason)?;
    let ends_at = sanction_end(body.ends_at)?;
    if kind == "silence" && auth.role == "mod" {
        let maximum_end = Utc::now() + chrono::Duration::days(30);
        if !matches!(ends_at.as_ref(), Some(ends_at) if *ends_at <= maximum_end) {
            return Err(AppError::BadRequest(
                "moderator silence requires a future endsAt no more than 30 days away".into(),
            ));
        }
    }
    let mut tx = state.db.begin().await?;
    if kind == "suspend" {
        crate::auth_middleware::require_recent_auth_tx(auth_context, &mut tx).await?;
    }
    let target_role: String =
        sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(AppError::NotFound)?;
    require_lower_role(auth, account_id, &target_role)?;
    let already_active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity.sanctions \
         WHERE account_id = $1 AND kind = $2 AND revoked_at IS NULL \
           AND (ends_at IS NULL OR ends_at > now()))",
    )
    .bind(account_id)
    .bind(kind)
    .fetch_one(&mut *tx)
    .await?;
    if already_active {
        return Err(AppError::Conflict(format!("account already has an active {kind}")));
    }
    let sanction_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by, ends_at) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(account_id)
    .bind(kind)
    .bind(reason)
    .bind(auth.id)
    .bind(ends_at)
    .fetch_one(&mut *tx)
    .await?;
    if kind == "suspend" {
        sqlx::query(
            "UPDATE identity.sessions SET revoked_at = now() \
             WHERE account_id = $1 AND revoked_at IS NULL",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE identity.accounts SET auth_version = auth_version + 1, \
                    legacy_access_revoked_before = now() WHERE id = $1",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    }
    let metadata = serde_json::json!({
        "kind": kind,
        "endsAt": ends_at.map(|timestamp| timestamp.timestamp()),
    });
    let event_id = governance::record_account_event_with_id_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "identity.user.sanctioned",
        "sanction",
        &sanction_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    governance::notices::create_notice_tx(
        &mut tx,
        account_id,
        "sanction_applied",
        &format!("audit:{event_id}:sanction"),
        Some(event_id),
        None,
        "sanction",
        &sanction_id.to_string(),
        &format!(
            "账号已被{}，可在申诉中心查看并申请复核。",
            if kind == "suspend" { "封禁" } else { "禁言" }
        ),
    )
    .await?;
    tx.commit().await?;
    invalidate_sanction_cache(state, account_id).await;
    crate::public_search::reconcile_user_in_background(state, account_id);
    Ok(())
}

/// POST /api/v2/admin/users/{id}/silence — prevent community writes.
pub async fn silence_user(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<SanctionInput>,
) -> AppResult<StatusCode> {
    let auth = authenticate_context(&headers, &state).await?;
    let account_id = parse_id(&account_id, "account id")?;
    create_sanction(&state, &auth, account_id, "silence", body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v2/admin/users/{id}/suspend — prevent authentication and revoke sessions.
pub async fn suspend_user(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<SanctionInput>,
) -> AppResult<StatusCode> {
    let auth_context = authenticate_context(&headers, &state).await?;
    auth_context
        .account
        .require_capability(Capability::SuspendUsers)
        .map_err(|_| AppError::Forbidden)?;
    let account_id = parse_id(&account_id, "account id")?;
    create_sanction(&state, &auth_context, account_id, "suspend", body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v2/admin/users/{id}/unsanction — revoke a sanction without erasing history.
pub async fn unsanction_user(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UnsanctionInput>,
) -> AppResult<StatusCode> {
    let auth_context = authenticate_context(&headers, &state).await?;
    let auth = &auth_context.account;
    let account_id = parse_id(&account_id, "account id")?;
    let sanction_id = parse_id(&body.sanction_id, "sanctionId")?;
    let reason = validate_reason(&body.reason)?;
    let kind: String = sqlx::query_scalar(
        "SELECT kind FROM identity.sanctions \
         WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL",
    )
    .bind(sanction_id)
    .bind(account_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let capability =
        if kind == "suspend" { Capability::SuspendUsers } else { Capability::SilenceUsers };
    auth.require_capability(capability).map_err(|_| AppError::Forbidden)?;
    let mut tx = state.db.begin().await?;
    if kind == "suspend" {
        crate::auth_middleware::require_recent_auth_tx(&auth_context, &mut tx).await?;
    }
    let sanction: Option<(String, String, String)> = sqlx::query_as(
        "SELECT sanctions.kind, sanctions.reason, accounts.role::text \
         FROM identity.sanctions sanctions \
         JOIN identity.accounts accounts ON accounts.id = sanctions.account_id \
         WHERE sanctions.id = $1 AND sanctions.account_id = $2 \
           AND sanctions.revoked_at IS NULL FOR UPDATE",
    )
    .bind(sanction_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (locked_kind, original_reason, target_role) = sanction.ok_or(AppError::NotFound)?;
    if locked_kind != kind {
        return Err(AppError::Conflict("sanction type changed during update".into()));
    }
    require_lower_role(auth, account_id, &target_role)?;
    sqlx::query("UPDATE identity.sanctions SET revoked_at = now(), revoked_by = $1 WHERE id = $2")
        .bind(auth.id)
        .bind(sanction_id)
        .execute(&mut *tx)
        .await?;
    let metadata = serde_json::json!({ "kind": locked_kind, "originalReason": original_reason });
    governance::record_account_event_tx(
        &mut tx,
        AccountActor { account_id: auth.id, role: &auth.role },
        "identity.user.sanction_revoked",
        "sanction",
        &sanction_id.to_string(),
        reason,
        Some(&metadata),
    )
    .await?;
    tx.commit().await?;
    invalidate_sanction_cache(&state, account_id).await;
    crate::public_search::reconcile_user_in_background(&state, account_id);
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v2/admin/users/{id}/sanctions — return sanction history for a user.
pub async fn list_user_sanctions(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<SanctionDto>>> {
    let auth = authenticate(&headers, &state).await?;
    auth.require_capability(Capability::SearchUsers).map_err(|_| AppError::Forbidden)?;
    let account_id = parse_id(&account_id, "account id")?;
    let account_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity.accounts WHERE id = $1)")
            .bind(account_id)
            .fetch_one(&state.db)
            .await?;
    if !account_exists {
        return Err(AppError::NotFound);
    }
    let rows: Vec<SanctionRow> = sqlx::query_as(
        "SELECT id, account_id, kind, reason, issued_by, starts_at, ends_at, revoked_at, created_at \
         FROM identity.sanctions WHERE account_id = $1 ORDER BY id DESC",
    )
    .bind(account_id)
    .fetch_all(&state.db)
    .await?;
    let items = rows
        .into_iter()
        .map(|row| SanctionDto {
            id: row.id.to_string(),
            account_id: row.account_id.to_string(),
            kind: row.kind,
            reason: row.reason,
            issued_by: row.issued_by.map(|id| id.to_string()),
            starts_at: row.starts_at.timestamp(),
            ends_at: row.ends_at.map(|timestamp| timestamp.timestamp()),
            revoked_at: row.revoked_at.map(|timestamp| timestamp.timestamp()),
            created_at: row.created_at.timestamp(),
        })
        .collect();
    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use shared::AuthAccount;

    use super::{require_lower_role, role_rank, validate_reason};

    #[test]
    fn role_hierarchy_orders_user_mod_admin() {
        assert!(role_rank("user") < role_rank("mod"));
        assert!(role_rank("mod") < role_rank("admin"));
    }

    #[test]
    fn moderator_can_act_only_on_lower_role() {
        let moderator = AuthAccount { id: 10, role: "mod".into(), status: "active".into() };
        assert!(require_lower_role(&moderator, 11, "user").is_ok());
        assert!(require_lower_role(&moderator, 12, "mod").is_err());
        assert!(require_lower_role(&moderator, 13, "admin").is_err());
    }

    #[test]
    fn staff_cannot_act_on_themselves() {
        let administrator = AuthAccount { id: 10, role: "admin".into(), status: "active".into() };
        assert!(require_lower_role(&administrator, 10, "user").is_err());
    }

    #[test]
    fn staff_reason_is_trimmed_and_bounded() {
        assert_eq!(
            validate_reason("  policy violation  ").expect("valid reason"),
            "policy violation"
        );
        assert!(validate_reason("no").is_err());
        assert!(validate_reason(&"x".repeat(501)).is_err());
    }
}

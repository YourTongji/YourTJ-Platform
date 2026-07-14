//! Thin HTTP composition for governance appeals and owner-domain reversals.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use governance::appeals::{AppealRecord, VerifiedAppealTarget};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use shared::auth::Capability;
use shared::{AppError, AppResult, AppState, AuthAccount, Page};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppealListQuery {
    cursor: Option<String>,
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    unread: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitAppealInput {
    governance_event_id: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppealTransitionInput {
    expected_version: i64,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppealDecisionInput {
    expected_version: i64,
    outcome: String,
    reason: String,
    amended_ends_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkGovernanceNoticesReadInput {
    ids: Option<Vec<String>>,
    all: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UnreadCountDto {
    count: i64,
}

fn default_limit() -> i64 {
    30
}

fn parse_id(value: &str, field: &str) -> AppResult<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::BadRequest(format!("invalid {field}")))
}

fn parse_cursor(value: Option<&str>) -> AppResult<Option<i64>> {
    value.map(|value| parse_id(value, "cursor")).transpose()
}

fn role_rank(role: &str) -> Option<u8> {
    match role {
        "user" => Some(0),
        "mod" => Some(1),
        "admin" => Some(2),
        _ => None,
    }
}

fn require_lower_role(actor: &AuthAccount, target_id: i64, target_role: &str) -> AppResult<()> {
    let actor_rank = role_rank(&actor.role).ok_or(AppError::Forbidden)?;
    let target_rank = role_rank(target_role).ok_or(AppError::Forbidden)?;
    if actor.id == target_id || actor_rank <= target_rank {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn authenticate_subject(headers: &HeaderMap, state: &AppState) -> AppResult<AuthAccount> {
    identity::auth_middleware::authenticate_appeal_subject(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)
}

async fn authenticate_reviewer(headers: &HeaderMap, state: &AppState) -> AppResult<AuthAccount> {
    let actor = identity::auth_middleware::authenticate(
        headers,
        &state.db,
        &state.jwt_secret,
        state.redis.as_ref(),
    )
    .await
    .map_err(|_| AppError::Unauthorized)?;
    actor.require_capability(Capability::ReviewAppeals).map_err(|_| AppError::Forbidden)?;
    Ok(actor)
}

async fn verify_owner_target(
    connection: &mut sqlx::PgConnection,
    event: &governance::appeals::AppealableAuditEvent,
    appellant_account_id: i64,
) -> AppResult<(VerifiedAppealTarget, String)> {
    let sanction_id = match (event.action.as_str(), event.target_type.as_str()) {
        ("identity.user.sanctioned", "sanction") => {
            Some(parse_id(&event.target_id, "sanction target")?)
        }
        ("identity.sanction.auto_silence", "account") => event
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("sanctionId"))
            .and_then(serde_json::Value::as_str)
            .map(|sanction_id| parse_id(sanction_id, "sanction target"))
            .transpose()?,
        _ => None,
    };
    if let Some(sanction_id) = sanction_id {
        let target = identity::sanctions::inspect_appealable_sanction_tx(
            connection,
            sanction_id,
            appellant_account_id,
        )
        .await?;
        return Ok((
            VerifiedAppealTarget {
                target_kind: "sanction".into(),
                target_id: sanction_id.to_string(),
                disposition_kind: target.kind,
            },
            target.account_role,
        ));
    }
    if event.action.starts_with("forum.") {
        let target = forum::appeals::inspect_appealable_content_tx(
            connection,
            &event.action,
            &event.target_type,
            &event.target_id,
            event.metadata.as_ref(),
            appellant_account_id,
        )
        .await?;
        return Ok((
            VerifiedAppealTarget {
                target_kind: target.target_kind,
                target_id: target.target_id.to_string(),
                disposition_kind: target.disposition_kind,
            },
            target.author_role,
        ));
    }
    if event.action.starts_with("reviews.") {
        let target = reviews::appeals::inspect_appealable_review_tx(
            connection,
            &event.action,
            &event.target_type,
            &event.target_id,
            event.metadata.as_ref(),
            appellant_account_id,
        )
        .await?;
        return Ok((
            VerifiedAppealTarget {
                target_kind: "review".into(),
                target_id: target.review_id.to_string(),
                disposition_kind: "hide".into(),
            },
            target.author_role,
        ));
    }
    Err(AppError::NotFound)
}

/// GET /api/v2/me/appeals
async fn list_my_appeals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AppealListQuery>,
) -> AppResult<Json<Page<governance::appeals::AppealDto>>> {
    let actor = authenticate_subject(&headers, &state).await?;
    let page = governance::appeals::list_my_appeals(
        &state.db,
        actor.id,
        parse_cursor(query.cursor.as_deref())?,
        query.limit,
    )
    .await?;
    Ok(Json(page))
}

/// POST /api/v2/me/appeals
async fn submit_appeal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SubmitAppealInput>,
) -> AppResult<(StatusCode, Json<governance::appeals::AppealDto>)> {
    let actor = authenticate_subject(&headers, &state).await?;
    shared::ratelimit::check_token_bucket(
        state.redis.as_ref(),
        "governance_appeal_submit",
        &actor.id.to_string(),
        10,
        3600,
    )
    .await?;
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Idempotency-Key is required".into()))?;
    let governance_event_id = parse_id(&body.governance_event_id, "governanceEventId")?;
    let canonical_request = format!("{governance_event_id}\n{}", body.reason.trim());
    let request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let mut transaction = state.db.begin().await?;
    let event =
        governance::appeals::find_appealable_event_tx(&mut transaction, governance_event_id)
            .await?;
    let (target, _target_role) = verify_owner_target(&mut transaction, &event, actor.id).await?;
    let (appeal, replayed) = governance::appeals::submit_appeal_tx(
        &mut transaction,
        governance::appeals::SubmitAppeal {
            actor: governance::AccountActor { account_id: actor.id, role: &actor.role },
            governance_event: event,
            target,
            reason: &body.reason,
            idempotency_key,
            request_hash: &request_hash,
        },
    )
    .await?;
    transaction.commit().await?;
    let dto = governance::appeals::get_my_appeal(&state.db, actor.id, appeal.id).await?;
    Ok((if replayed { StatusCode::OK } else { StatusCode::CREATED }, Json(dto)))
}

/// POST /api/v2/me/appeals/{id}/withdraw
async fn withdraw_appeal(
    State(state): State<AppState>,
    Path(appeal_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AppealTransitionInput>,
) -> AppResult<Json<governance::appeals::AppealDto>> {
    let actor = authenticate_subject(&headers, &state).await?;
    let appeal_id = parse_id(&appeal_id, "appeal id")?;
    let mut transaction = state.db.begin().await?;
    let appeal =
        governance::appeals::find_appeal_for_update_tx(&mut transaction, appeal_id).await?;
    governance::appeals::withdraw_appeal_tx(
        &mut transaction,
        &appeal,
        governance::AccountActor { account_id: actor.id, role: &actor.role },
        body.expected_version,
        &body.reason,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(governance::appeals::get_my_appeal(&state.db, actor.id, appeal_id).await?))
}

/// GET /api/v2/me/governance-notices
async fn list_governance_notices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AppealListQuery>,
) -> AppResult<Json<Page<governance::notices::GovernanceNoticeDto>>> {
    let actor = authenticate_subject(&headers, &state).await?;
    Ok(Json(
        governance::notices::list_notices(
            &state.db,
            actor.id,
            parse_cursor(query.cursor.as_deref())?,
            query.unread,
            query.limit,
        )
        .await?,
    ))
}

async fn governance_notice_unread_count(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<UnreadCountDto>> {
    let actor = authenticate_subject(&headers, &state).await?;
    Ok(Json(UnreadCountDto {
        count: governance::notices::unread_count(&state.db, actor.id).await?,
    }))
}

async fn mark_governance_notices_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MarkGovernanceNoticesReadInput>,
) -> AppResult<StatusCode> {
    let actor = authenticate_subject(&headers, &state).await?;
    let ids = body
        .ids
        .as_ref()
        .map(|ids| ids.iter().map(|id| parse_id(id, "notice id")).collect::<AppResult<Vec<_>>>())
        .transpose()?;
    match (ids.as_deref(), body.all) {
        (Some(ids), None) => governance::notices::mark_read(&state.db, actor.id, Some(ids)).await?,
        (None, Some(true)) => governance::notices::mark_read(&state.db, actor.id, None).await?,
        _ => return Err(AppError::BadRequest("send ids or all=true".into())),
    }
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v2/admin/appeals
async fn list_admin_appeals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AppealListQuery>,
) -> AppResult<Json<Page<governance::appeals::AdminAppealDto>>> {
    let actor = authenticate_reviewer(&headers, &state).await?;
    let page = governance::appeals::list_admin_appeals(
        &state.db,
        actor.id,
        &actor.role,
        query.status.as_deref(),
        parse_cursor(query.cursor.as_deref())?,
        query.limit,
    )
    .await?;
    Ok(Json(page))
}

/// POST /api/v2/admin/appeals/{id}/review
async fn start_review(
    State(state): State<AppState>,
    Path(appeal_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AppealTransitionInput>,
) -> AppResult<Json<governance::appeals::AdminAppealDto>> {
    let actor = authenticate_reviewer(&headers, &state).await?;
    let appeal_id = parse_id(&appeal_id, "appeal id")?;
    let mut transaction = state.db.begin().await?;
    let appeal =
        governance::appeals::find_appeal_for_update_tx(&mut transaction, appeal_id).await?;
    let target = identity::public_accounts::find_account_authorization_state_by_id(
        &mut transaction,
        appeal.appellant_account_id,
    )
    .await?
    .ok_or(AppError::NotFound)?;
    require_lower_role(&actor, appeal.appellant_account_id, &target.role)?;
    governance::appeals::start_review_tx(
        &mut transaction,
        &appeal,
        governance::AccountActor { account_id: actor.id, role: &actor.role },
        body.expected_version,
        &body.reason,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(governance::appeals::get_admin_appeal(&state.db, appeal_id).await?))
}

enum AppealMutation {
    None,
    Sanction { account_id: i64 },
    Forum(forum::appeals::ForumAppealMutation),
    Review(reviews::appeals::ReviewAppealMutation),
}

async fn apply_owner_decision(
    connection: &mut sqlx::PgConnection,
    appeal: &AppealRecord,
    reviewer_id: i64,
    body: &AppealDecisionInput,
) -> AppResult<(AppealMutation, Option<serde_json::Value>)> {
    if body.outcome == "upheld" {
        if body.amended_ends_at.is_some() {
            return Err(AppError::BadRequest("upheld decisions cannot contain amendment".into()));
        }
        return Ok((AppealMutation::None, None));
    }
    let event =
        governance::appeals::find_appealable_event_tx(connection, appeal.original_event_id).await?;
    match (body.outcome.as_str(), appeal.target_kind.as_str()) {
        ("overturned", "sanction") => {
            let sanction_id = parse_id(&appeal.target_id, "sanction target")?;
            identity::sanctions::overturn_sanction_for_appeal_tx(
                connection,
                sanction_id,
                appeal.appellant_account_id,
                reviewer_id,
            )
            .await?;
            Ok((AppealMutation::Sanction { account_id: appeal.appellant_account_id }, None))
        }
        ("amended", "sanction") => {
            let timestamp = body.amended_ends_at.ok_or_else(|| {
                AppError::BadRequest("amendedEndsAt is required for sanction amendment".into())
            })?;
            let amended_ends_at = DateTime::<Utc>::from_timestamp(timestamp, 0)
                .ok_or_else(|| AppError::BadRequest("invalid amendedEndsAt".into()))?;
            let sanction_id = parse_id(&appeal.target_id, "sanction target")?;
            identity::sanctions::amend_sanction_for_appeal_tx(
                connection,
                sanction_id,
                appeal.appellant_account_id,
                amended_ends_at,
            )
            .await?;
            Ok((
                AppealMutation::Sanction { account_id: appeal.appellant_account_id },
                Some(serde_json::json!({ "endsAt": timestamp })),
            ))
        }
        ("overturned", "forum_thread" | "forum_comment") => {
            let target_id = parse_id(&appeal.target_id, "forum target")?;
            let mutation = forum::appeals::overturn_content_for_appeal_tx(
                connection,
                appeal.original_event_id,
                event.created_at,
                &event.action,
                &event.target_type,
                &event.target_id,
                event.metadata.as_ref(),
                &appeal.target_kind,
                target_id,
                &appeal.disposition_kind,
                appeal.appellant_account_id,
            )
            .await?;
            Ok((AppealMutation::Forum(mutation), None))
        }
        ("overturned", "review") => {
            let review_id = parse_id(&appeal.target_id, "review target")?;
            let mutation = reviews::appeals::overturn_review_for_appeal_tx(
                connection,
                appeal.original_event_id,
                &event.action,
                &event.target_type,
                &event.target_id,
                event.metadata.as_ref(),
                review_id,
                appeal.appellant_account_id,
            )
            .await?;
            Ok((AppealMutation::Review(mutation), None))
        }
        ("amended", _) => Err(AppError::Conflict(
            "this owner domain does not support a safe amendment; choose upheld or overturned"
                .into(),
        )),
        _ => Err(AppError::BadRequest("invalid appeal outcome".into())),
    }
}

/// POST /api/v2/admin/appeals/{id}/decision
async fn decide_appeal(
    State(state): State<AppState>,
    Path(appeal_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AppealDecisionInput>,
) -> AppResult<Json<governance::appeals::AdminAppealDto>> {
    let actor = authenticate_reviewer(&headers, &state).await?;
    let appeal_id = parse_id(&appeal_id, "appeal id")?;
    let mut transaction = state.db.begin().await?;
    let appeal =
        governance::appeals::find_appeal_for_update_tx(&mut transaction, appeal_id).await?;
    let target = identity::public_accounts::find_account_authorization_state_by_id(
        &mut transaction,
        appeal.appellant_account_id,
    )
    .await?
    .ok_or(AppError::NotFound)?;
    require_lower_role(&actor, appeal.appellant_account_id, &target.role)?;
    let (mutation, amendment) =
        apply_owner_decision(&mut transaction, &appeal, actor.id, &body).await?;
    governance::appeals::decide_appeal_tx(
        &mut transaction,
        &appeal,
        governance::AccountActor { account_id: actor.id, role: &actor.role },
        body.expected_version,
        &body.outcome,
        &body.reason,
        amendment.as_ref(),
    )
    .await?;
    transaction.commit().await?;

    match mutation {
        AppealMutation::None => {}
        AppealMutation::Sanction { account_id } => {
            identity::sanctions::invalidate_sanction_caches(state.redis.as_ref(), account_id).await;
            identity::public_search::reconcile_user_in_background(&state, account_id);
        }
        AppealMutation::Forum(mutation) => {
            forum::cache::invalidate_thread_surfaces(
                state.redis.as_ref(),
                mutation.thread_id,
                mutation.board_id,
            )
            .await;
            forum::meili::reconcile_thread_in_background(&state, mutation.thread_id);
        }
        AppealMutation::Review(mutation) => {
            let course_id = mutation.course_id.to_string();
            shared::cache::bump_version_opt(state.redis.as_ref(), "course", &course_id).await.ok();
            shared::cache::bump_version_opt(state.redis.as_ref(), "reviews", &course_id).await.ok();
            reviews::search::sync_search_document(
                &state.meili_url,
                &state.meili_master_key,
                mutation.review_id,
                &state.db,
            )
            .await;
        }
    }
    Ok(Json(governance::appeals::get_admin_appeal(&state.db, appeal_id).await?))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v2/me/appeals", get(list_my_appeals).post(submit_appeal))
        .route("/api/v2/me/appeals/{id}/withdraw", post(withdraw_appeal))
        .route("/api/v2/me/governance-notices", get(list_governance_notices))
        .route("/api/v2/me/governance-notices/unread-count", get(governance_notice_unread_count))
        .route("/api/v2/me/governance-notices/read", post(mark_governance_notices_read))
        .route("/api/v2/admin/appeals", get(list_admin_appeals))
        .route("/api/v2/admin/appeals/{id}/review", post(start_review))
        .route("/api/v2/admin/appeals/{id}/decision", post(decide_appeal))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use shared::{AppError, AuthAccount};
    use tower::ServiceExt as _;

    use super::{parse_id, require_lower_role, routes};

    #[test]
    fn reviewer_must_outrank_the_appellant() {
        let moderator = AuthAccount { id: 2, role: "mod".into(), status: "active".into() };
        assert!(require_lower_role(&moderator, 3, "user").is_ok());
        assert!(matches!(require_lower_role(&moderator, 4, "mod"), Err(AppError::Forbidden)));
    }

    #[test]
    fn ids_are_strictly_positive() {
        assert_eq!(parse_id("7", "id").expect("positive"), 7);
        assert!(parse_id("0", "id").is_err());
        assert!(parse_id("not-an-id", "id").is_err());
    }

    async fn test_state() -> shared::AppState {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://yourtj:yourtj@localhost:5432/yourtj_test".to_string());
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("connect dedicated appeal test database");
        sqlx::migrate!("../../migrations").run(&pool).await.expect("apply appeal test migrations");
        let mut config = shared::Config::from_env().expect("test config");
        config.database_url = database_url;
        shared::AppState {
            db: pool,
            config,
            jwt_secret: "appeal-test-jwt-secret-32-bytes".into(),
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
        }
    }

    async fn insert_account(pool: &sqlx::PgPool, role: &str, suffix: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO identity.accounts (email, email_verified_at, handle, role) \
             VALUES ($1, now(), $2, $3::identity.account_role) RETURNING id",
        )
        .bind(format!("appeal-{suffix}@tongji.edu.cn"))
        .bind(format!("appeal-{suffix}"))
        .bind(role)
        .fetch_one(pool)
        .await
        .expect("insert appeal test account")
    }

    async fn insert_withdrawn_appeal(
        pool: &sqlx::PgPool,
        appellant_id: i64,
        original_actor_id: i64,
        suffix: &str,
    ) -> i64 {
        let mut transaction = pool.begin().await.expect("begin queue fixture transaction");
        let event_id = governance::record_account_event_with_id_tx(
            &mut transaction,
            governance::AccountActor { account_id: original_actor_id, role: "admin" },
            "identity.user.sanctioned",
            "sanction",
            suffix,
            "queue fixture disposition",
            Some(&json!({ "kind": "silence" })),
        )
        .await
        .expect("insert queue fixture audit event");
        let appeal_id: i64 = sqlx::query_scalar(
            "INSERT INTO governance.appeals \
             (original_event_id, appellant_account_id, original_actor_id, original_action, \
              target_kind, target_id, disposition_kind, status, submission_reason, \
              idempotency_key, request_hash, appealable_until, decision_reason, decided_at) \
             VALUES ($1, $2, $3, 'identity.user.sanctioned', 'sanction', $4, 'silence', \
                     'withdrawn', 'queue fixture appeal', $5, $6, now() + interval '30 days', \
                     'queue fixture withdrawn', now()) RETURNING id",
        )
        .bind(event_id)
        .bind(appellant_id)
        .bind(original_actor_id)
        .bind(suffix)
        .bind(format!("queue-{suffix}"))
        .bind("a".repeat(64))
        .fetch_one(&mut *transaction)
        .await
        .expect("insert withdrawn queue appeal");
        sqlx::query(
            "INSERT INTO governance.appeal_events \
             (appeal_id, actor_kind, actor_account_id, from_status, to_status, reason) \
             VALUES ($1, 'account', $2, NULL, 'submitted', 'queue fixture submitted'), \
                    ($1, 'account', $2, 'submitted', 'withdrawn', 'queue fixture withdrawn')",
        )
        .bind(appeal_id)
        .bind(appellant_id)
        .execute(&mut *transaction)
        .await
        .expect("insert queue appeal history");
        transaction.commit().await.expect("commit queue appeal fixture");
        appeal_id
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), 1_000_000).await.expect("read response body");
        serde_json::from_slice(&bytes).expect("response json")
    }

    fn assert_append_only_rejection(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) {
        let error = result.expect_err("governance append-only statement must be rejected");
        let message = error
            .as_database_error()
            .map(|database| database.message().to_owned())
            .unwrap_or_default();
        assert!(message.contains("append-only"), "unexpected database error: {message}");
    }

    fn json_request(method: &str, uri: &str, token: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("build appeal request")
    }

    async fn submit_case(
        app: &axum::Router,
        token: &str,
        event_id: i64,
        idempotency_key: &str,
        reason: &str,
    ) -> Value {
        let mut request = json_request(
            "POST",
            "/api/v2/me/appeals",
            token,
            json!({ "governanceEventId": event_id.to_string(), "reason": reason }),
        );
        request
            .headers_mut()
            .insert("idempotency-key", idempotency_key.parse().expect("idempotency header"));
        let response = app.clone().oneshot(request).await.expect("submit appeal case");
        assert_eq!(response.status(), StatusCode::CREATED);
        response_json(response).await
    }

    async fn claim_and_overturn(app: &axum::Router, token: &str, appeal_id: &str) -> Value {
        let claim = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/appeals/{appeal_id}/review"),
                token,
                json!({ "expectedVersion": 1, "reason": "independent review started" }),
            ))
            .await
            .expect("claim appeal case");
        assert_eq!(claim.status(), StatusCode::OK);
        let decision = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/appeals/{appeal_id}/decision"),
                token,
                json!({
                    "expectedVersion": 2,
                    "outcome": "overturned",
                    "reason": "restriction was not proportionate"
                }),
            ))
            .await
            .expect("decide appeal case");
        assert_eq!(decision.status(), StatusCode::OK);
        response_json(decision).await
    }

    #[tokio::test]
    async fn suspended_subject_appeals_and_independent_reviewer_overturns_atomically() {
        let state = test_state().await;
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let appellant_id = insert_account(&state.db, "user", &format!("user-{suffix}")).await;
        let other_user_id =
            insert_account(&state.db, "user", &format!("other-user-{suffix}")).await;
        let original_actor_id =
            insert_account(&state.db, "admin", &format!("admin-{suffix}")).await;
        let reviewer_id = insert_account(&state.db, "mod", &format!("mod-{suffix}")).await;
        let sanction_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by) \
             VALUES ($1, 'suspend', 'account safety review', $2) RETURNING id",
        )
        .bind(appellant_id)
        .bind(original_actor_id)
        .fetch_one(&state.db)
        .await
        .expect("insert sanction");
        sqlx::query("UPDATE identity.accounts SET status = 'suspended' WHERE id = $1")
            .bind(appellant_id)
            .execute(&state.db)
            .await
            .expect("suspend appellant account");
        let mut transaction = state.db.begin().await.expect("begin audit transaction");
        let event_id = governance::record_account_event_with_id_tx(
            &mut transaction,
            governance::AccountActor { account_id: original_actor_id, role: "admin" },
            "identity.user.sanctioned",
            "sanction",
            &sanction_id.to_string(),
            "account safety review",
            Some(&json!({ "kind": "suspend" })),
        )
        .await
        .expect("record original disposition");
        transaction.commit().await.expect("commit disposition");

        let appeal_token =
            identity::auth::create_appeal_access_token(appellant_id, &state.jwt_secret, 3600)
                .expect("appeal access token");
        let app = routes(state.clone());
        let other_user_token =
            identity::auth::create_appeal_access_token(other_user_id, &state.jwt_secret, 3600)
                .expect("other user's appeal access token");
        let regular_identity_response =
            identity::routes(state.clone(), std::sync::Arc::new(reviews::LegacyReviewClaimer))
                .oneshot(json_request("GET", "/api/v2/me", &other_user_token, Value::Null))
                .await
                .expect("regular identity response");
        assert_eq!(regular_identity_response.status(), StatusCode::UNAUTHORIZED);

        let mut cross_owner_request = json_request(
            "POST",
            "/api/v2/me/appeals",
            &other_user_token,
            json!({ "governanceEventId": event_id.to_string(), "reason": "not my disposition" }),
        );
        cross_owner_request
            .headers_mut()
            .insert("idempotency-key", "appeal-handler-cross-owner".parse().expect("header"));
        let cross_owner_response =
            app.clone().oneshot(cross_owner_request).await.expect("cross-owner response");
        assert_eq!(cross_owner_response.status(), StatusCode::NOT_FOUND);

        let mut submit_request = json_request(
            "POST",
            "/api/v2/me/appeals",
            &appeal_token,
            json!({ "governanceEventId": event_id.to_string(), "reason": "context was misunderstood" }),
        );
        submit_request
            .headers_mut()
            .insert("idempotency-key", "appeal-handler-test-1".parse().expect("header"));
        let response = app.clone().oneshot(submit_request).await.expect("submit response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let submitted = response_json(response).await;
        let appeal_id = submitted["id"].as_str().expect("appeal id");
        assert_eq!(submitted["status"], "submitted");
        assert_eq!(submitted["originalReason"], "账号封禁处置");
        assert!(submitted.get("reviewerAccountId").is_none());

        let mut replay_request = json_request(
            "POST",
            "/api/v2/me/appeals",
            &appeal_token,
            json!({ "governanceEventId": event_id.to_string(), "reason": "context was misunderstood" }),
        );
        replay_request
            .headers_mut()
            .insert("idempotency-key", "appeal-handler-test-1".parse().expect("header"));
        let replay = app.clone().oneshot(replay_request).await.expect("replay response");
        assert_eq!(replay.status(), StatusCode::OK);

        let mut conflicting_replay = json_request(
            "POST",
            "/api/v2/me/appeals",
            &appeal_token,
            json!({ "governanceEventId": event_id.to_string(), "reason": "different request" }),
        );
        conflicting_replay
            .headers_mut()
            .insert("idempotency-key", "appeal-handler-test-1".parse().expect("header"));
        let conflict =
            app.clone().oneshot(conflicting_replay).await.expect("conflicting replay response");
        assert_eq!(conflict.status(), StatusCode::CONFLICT);

        let original_actor_token =
            identity::auth::create_access_token(original_actor_id, &state.jwt_secret, 3600)
                .expect("original actor token");
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/appeals/{appeal_id}/review"),
                &original_actor_token,
                json!({ "expectedVersion": 1, "reason": "review assignment" }),
            ))
            .await
            .expect("conflict response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let reviewer_token =
            identity::auth::create_access_token(reviewer_id, &state.jwt_secret, 3600)
                .expect("reviewer token");
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/appeals/{appeal_id}/review"),
                &reviewer_token,
                json!({ "expectedVersion": 1, "reason": "independent review started" }),
            ))
            .await
            .expect("claim response");
        assert_eq!(response.status(), StatusCode::OK);
        let claimed = response_json(response).await;
        assert_eq!(claimed["status"], "in_review");

        let stale_claim = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/appeals/{appeal_id}/review"),
                &reviewer_token,
                json!({ "expectedVersion": 1, "reason": "duplicate review claim" }),
            ))
            .await
            .expect("stale claim response");
        assert_eq!(stale_claim.status(), StatusCode::CONFLICT);

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/appeals/{appeal_id}/decision"),
                &reviewer_token,
                json!({
                    "expectedVersion": 2,
                    "outcome": "overturned",
                    "reason": "restriction was not proportionate"
                }),
            ))
            .await
            .expect("decision response");
        assert_eq!(response.status(), StatusCode::OK);
        let decided = response_json(response).await;
        assert_eq!(decided["status"], "overturned");
        let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT revoked_at FROM identity.sanctions WHERE id = $1")
                .bind(sanction_id)
                .fetch_one(&state.db)
                .await
                .expect("read sanction");
        assert!(revoked_at.is_some());
        let transition_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM governance.appeal_events WHERE appeal_id = $1",
        )
        .bind(appeal_id.parse::<i64>().expect("numeric appeal id"))
        .fetch_one(&state.db)
        .await
        .expect("count transitions");
        assert_eq!(transition_count, 3);

        let board_id: i64 = sqlx::query_scalar(
            "INSERT INTO forum.boards (slug, name) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("appeal-board-{suffix}"))
        .bind("Appeal test board")
        .fetch_one(&state.db)
        .await
        .expect("insert forum board");
        let thread_id: i64 = sqlx::query_scalar(
            "INSERT INTO forum.threads (board_id, author_id, title, body) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(board_id)
        .bind(appellant_id)
        .bind("Appealable forum thread")
        .bind("body")
        .fetch_one(&state.db)
        .await
        .expect("insert forum thread");
        let forum_disposition = forum::routes(state.clone())
            .oneshot(json_request(
                "POST",
                &format!("/api/v2/admin/forum/threads/{thread_id}/hide"),
                &original_actor_token,
                json!({ "reason": "forum safety review" }),
            ))
            .await
            .expect("forum disposition response");
        assert_eq!(forum_disposition.status(), StatusCode::OK);
        let forum_event_id: i64 = sqlx::query_scalar(
            "SELECT id FROM governance.audit_events \
             WHERE action = 'forum.thread.hide' AND target_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(thread_id.to_string())
        .fetch_one(&state.db)
        .await
        .expect("forum governance event");
        let forum_appeal = submit_case(
            &app,
            &appeal_token,
            forum_event_id,
            "appeal-handler-forum",
            "the forum context was misunderstood",
        )
        .await;
        let forum_decided = claim_and_overturn(
            &app,
            &reviewer_token,
            forum_appeal["id"].as_str().expect("forum appeal id"),
        )
        .await;
        assert_eq!(forum_decided["status"], "overturned");
        let thread_hidden_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT hidden_at FROM forum.threads WHERE id = $1")
                .bind(thread_id)
                .fetch_one(&state.db)
                .await
                .expect("read forum thread state");
        assert!(thread_hidden_at.is_none());

        let course_id: i64 = sqlx::query_scalar(
            "INSERT INTO courses.courses (code, name, review_count, review_avg) \
             VALUES ($1, $2, 1, 5) RETURNING id",
        )
        .bind(format!("APPEAL-{suffix}"))
        .bind("Appeal test course")
        .fetch_one(&state.db)
        .await
        .expect("insert course");
        let review_id: i64 = sqlx::query_scalar(
            "INSERT INTO reviews.reviews (course_id, account_id, rating, comment, status) \
             VALUES ($1, $2, 5, $3, 'visible') RETURNING id",
        )
        .bind(course_id)
        .bind(appellant_id)
        .bind("Appealable review")
        .fetch_one(&state.db)
        .await
        .expect("insert review");
        let review_disposition = reviews::routes(state.clone())
            .oneshot(json_request(
                "DELETE",
                &format!("/api/v2/admin/reviews/{review_id}"),
                &original_actor_token,
                json!({ "reason": "review safety check" }),
            ))
            .await
            .expect("review disposition response");
        assert_eq!(review_disposition.status(), StatusCode::NO_CONTENT);
        let review_event_id: i64 = sqlx::query_scalar(
            "SELECT id FROM governance.audit_events \
             WHERE action = 'reviews.review.hidden' AND target_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(review_id.to_string())
        .fetch_one(&state.db)
        .await
        .expect("review governance event");
        let review_appeal = submit_case(
            &app,
            &appeal_token,
            review_event_id,
            "appeal-handler-review",
            "the review context was misunderstood",
        )
        .await;
        let review_decided = claim_and_overturn(
            &app,
            &reviewer_token,
            review_appeal["id"].as_str().expect("review appeal id"),
        )
        .await;
        assert_eq!(review_decided["status"], "overturned");
        let review_state: (String, i32) = sqlx::query_as(
            "SELECT review.status::text, course.review_count \
             FROM reviews.reviews review JOIN courses.courses course ON course.id = review.course_id \
             WHERE review.id = $1",
        )
        .bind(review_id)
        .fetch_one(&state.db)
        .await
        .expect("read review state and projection");
        assert_eq!(review_state, ("visible".into(), 1));

        let appeal_id = appeal_id.parse::<i64>().expect("numeric appeal id");
        assert_append_only_rejection(
            sqlx::query("UPDATE governance.audit_events SET reason = 'rewritten' WHERE id = $1")
                .bind(event_id)
                .execute(&state.db)
                .await,
        );
        assert_append_only_rejection(
            sqlx::query("DELETE FROM governance.audit_events WHERE id = $1")
                .bind(event_id)
                .execute(&state.db)
                .await,
        );
        assert_append_only_rejection(
            sqlx::query(
                "UPDATE governance.appeal_events SET reason = 'rewritten' WHERE appeal_id = $1",
            )
            .bind(appeal_id)
            .execute(&state.db)
            .await,
        );
        assert_append_only_rejection(
            sqlx::query("DELETE FROM governance.appeal_events WHERE appeal_id = $1")
                .bind(appeal_id)
                .execute(&state.db)
                .await,
        );
        assert_append_only_rejection(
            sqlx::query("TRUNCATE governance.audit_events CASCADE").execute(&state.db).await,
        );
        assert_append_only_rejection(
            sqlx::query("TRUNCATE governance.appeal_events").execute(&state.db).await,
        );

        let mut append_transaction = state.db.begin().await.expect("begin post-rejection append");
        governance::record_system_event_tx(
            &mut append_transaction,
            "governance.append_only.verified",
            "appeal",
            &appeal_id.to_string(),
            "normal append remains available",
            None,
        )
        .await
        .expect("append after rejected mutations");
        append_transaction.commit().await.expect("commit post-rejection append");
    }

    #[tokio::test]
    async fn appeal_queue_applies_hierarchy_and_recusal_before_sql_pagination() {
        let state = test_state().await;
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let moderator_id = insert_account(&state.db, "mod", &format!("queue-mod-{suffix}")).await;
        let administrator_id =
            insert_account(&state.db, "admin", &format!("queue-admin-{suffix}")).await;
        let other_administrator_id =
            insert_account(&state.db, "admin", &format!("queue-origin-{suffix}")).await;
        let user_id = insert_account(&state.db, "user", &format!("queue-user-{suffix}")).await;
        let other_user_id =
            insert_account(&state.db, "user", &format!("queue-recused-{suffix}")).await;

        let eligible_id = insert_withdrawn_appeal(
            &state.db,
            user_id,
            other_administrator_id,
            &format!("eligible-{suffix}"),
        )
        .await;
        insert_withdrawn_appeal(
            &state.db,
            administrator_id,
            other_administrator_id,
            &format!("higher-role-{suffix}"),
        )
        .await;
        let recused_id = insert_withdrawn_appeal(
            &state.db,
            other_user_id,
            moderator_id,
            &format!("recused-{suffix}"),
        )
        .await;

        let moderator_page = governance::appeals::list_admin_appeals(
            &state.db,
            moderator_id,
            "mod",
            Some("withdrawn"),
            Some(recused_id + 1),
            1,
        )
        .await
        .expect("list moderator appeal queue");
        assert_eq!(moderator_page.items.len(), 1);
        assert_eq!(moderator_page.items[0].appeal.id, eligible_id.to_string());

        let moderator_appellant_id =
            insert_account(&state.db, "mod", &format!("queue-appellant-mod-{suffix}")).await;
        let moderator_appeal_id = insert_withdrawn_appeal(
            &state.db,
            moderator_appellant_id,
            other_administrator_id,
            &format!("admin-visible-{suffix}"),
        )
        .await;
        let administrator_page = governance::appeals::list_admin_appeals(
            &state.db,
            administrator_id,
            "admin",
            Some("withdrawn"),
            Some(moderator_appeal_id + 1),
            1,
        )
        .await
        .expect("list administrator appeal queue");
        assert_eq!(administrator_page.items.len(), 1);
        assert_eq!(administrator_page.items[0].appeal.id, moderator_appeal_id.to_string());
    }
}

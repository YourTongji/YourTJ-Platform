//! Integration coverage for staff invitation and audit invariants.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn admin_invitation_requires_mailbox_proof_and_records_audit() {
    let (pool, app) = helpers::create_test_app().await;
    let invitation_key = uuid::Uuid::new_v4().simple().to_string();
    let suffix = &invitation_key[..8];
    let admin_email = format!("admin-{suffix}@tongji.edu.cn");
    let invited_email = format!("invite-{suffix}@tongji.edu.cn");
    let invited_handle = format!("invite-{suffix}");
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, email_verified_at, handle, role) \
         VALUES ($1, now(), $2, 'admin') RETURNING id",
    )
    .bind(&admin_email)
    .bind(format!("admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed admin");
    let token =
        identity::auth::create_access_token(admin_id, "integration-test-secret-32bytes!", 3600)
            .expect("admin token");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/admin/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({
                        "email": invited_email,
                        "handle": invited_handle,
                        "reason": "campus community onboarding"
                    })
                    .to_string(),
                ))
                .expect("invitation request"),
        )
        .await
        .expect("invitation response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = helpers::read_json(response).await;
    assert_eq!(body["handle"], invited_handle);
    assert_eq!(body["role"], "user");

    let invitation: (i64, Option<chrono::DateTime<chrono::Utc>>, bool) = sqlx::query_as(
        "SELECT id, email_verified_at, invitation_expires_at > now() \
         FROM identity.accounts WHERE email = $1",
    )
    .bind(&invited_email)
    .fetch_one(&pool)
    .await
    .expect("invited account");
    assert!(invitation.1.is_none());
    assert!(invitation.2);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE action = 'identity.user.invited' AND target_id = $1",
    )
    .bind(invitation.0.to_string())
    .fetch_one(&pool)
    .await
    .expect("invitation audit");
    assert_eq!(audit_count, 1);
    let delivery_job: (String, String) = sqlx::query_as(
        "SELECT kind, status FROM identity.email_delivery_jobs \
         WHERE account_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(invitation.0)
    .fetch_one(&pool)
    .await
    .expect("durable invitation delivery job");
    assert_eq!(delivery_job, ("admin_invitation".into(), "queued".into()));

    helpers::insert_valid_code(&pool, &invited_email, "123456").await;
    let verification = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/email/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "email": invited_email, "code": "123456" }).to_string()))
                .expect("verification request"),
        )
        .await
        .expect("verification response");
    assert_eq!(verification.status(), StatusCode::OK);
    let accepted: bool = sqlx::query_scalar(
        "SELECT email_verified_at IS NOT NULL AND invitation_accepted_at IS NOT NULL \
         FROM identity.accounts WHERE id = $1",
    )
    .bind(invitation.0)
    .fetch_one(&pool)
    .await
    .expect("accepted invitation");
    assert!(accepted);
}

#[tokio::test]
async fn moderator_silence_requires_expiry_but_admin_may_issue_indefinite() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("sanction-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("sanction-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed sanction admin");
    let moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("sanction-mod-{suffix}@tongji.edu.cn"))
    .bind(format!("sanction-mod-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed sanction moderator");
    let temporary_target_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("sanction-temp-{suffix}@tongji.edu.cn"))
    .bind(format!("sanction-temp-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed temporary target");
    let indefinite_target_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("sanction-indefinite-{suffix}@tongji.edu.cn"))
    .bind(format!("sanction-indefinite-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed indefinite target");
    let moderator_token =
        identity::auth::create_access_token(moderator_id, "integration-test-secret-32bytes!", 3600)
            .expect("moderator token");
    let admin_token =
        identity::auth::create_access_token(admin_id, "integration-test-secret-32bytes!", 3600)
            .expect("admin token");

    let no_expiry = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/users/{temporary_target_id}/silence"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::from(json!({ "reason": "temporary moderation action" }).to_string()))
                .expect("moderator indefinite request"),
        )
        .await
        .expect("moderator indefinite response");
    assert_eq!(no_expiry.status(), StatusCode::BAD_REQUEST);

    let overlong_end = chrono::Utc::now().timestamp() + 31 * 24 * 3600;
    let overlong = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/users/{temporary_target_id}/silence"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::from(
                    json!({ "reason": "overlong moderation action", "endsAt": overlong_end })
                        .to_string(),
                ))
                .expect("moderator overlong request"),
        )
        .await
        .expect("moderator overlong response");
    assert_eq!(overlong.status(), StatusCode::BAD_REQUEST);

    let future_end = chrono::Utc::now().timestamp() + 3600;
    let temporary = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/users/{temporary_target_id}/silence"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::from(
                    json!({ "reason": "temporary moderation action", "endsAt": future_end })
                        .to_string(),
                ))
                .expect("moderator temporary request"),
        )
        .await
        .expect("moderator temporary response");
    assert_eq!(temporary.status(), StatusCode::NO_CONTENT);

    let indefinite = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/users/{indefinite_target_id}/silence"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::from(json!({ "reason": "indefinite safety action" }).to_string()))
                .expect("admin indefinite request"),
        )
        .await
        .expect("admin indefinite response");
    assert_eq!(indefinite.status(), StatusCode::NO_CONTENT);

    let ends_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT ends_at FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'silence' AND revoked_at IS NULL",
    )
    .bind(indefinite_target_id)
    .fetch_one(&pool)
    .await
    .expect("indefinite sanction");
    assert!(ends_at.is_none());
}

#[tokio::test]
async fn role_endpoint_rejects_administrator_provisioning() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("role-admin-{suffix}@tongji.edu.cn"))
    .bind(format!("role-admin-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed role admin");
    let target_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'user') RETURNING id",
    )
    .bind(format!("role-target-{suffix}@tongji.edu.cn"))
    .bind(format!("role-target-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed role target");
    let token =
        identity::auth::create_access_token(admin_id, "integration-test-secret-32bytes!", 3600)
            .expect("admin token");

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v2/admin/users/{target_id}/role"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(
                    json!({ "role": "admin", "reason": "attempted administrator promotion" })
                        .to_string(),
                ))
                .expect("role request"),
        )
        .await
        .expect("role response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let persisted_role: String =
        sqlx::query_scalar("SELECT role::text FROM identity.accounts WHERE id = $1")
            .bind(target_id)
            .fetch_one(&pool)
            .await
            .expect("persisted target role");
    assert_eq!(persisted_role, "user");
}

#[tokio::test]
async fn lifecycle_dead_letter_is_visible_and_recent_auth_requeue_is_audited() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let admin_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'admin') RETURNING id",
    )
    .bind(format!("lifecycle-operator-{suffix}@tongji.edu.cn"))
    .bind(format!("operator-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed lifecycle operator");
    let moderator_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle, role) \
         VALUES ($1, $2, 'mod') RETURNING id",
    )
    .bind(format!("lifecycle-moderator-{suffix}@tongji.edu.cn"))
    .bind(format!("lifecycle-mod-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed lifecycle moderator");
    let target_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("lifecycle-dead-letter-{suffix}@tongji.edu.cn"))
    .bind(format!("dead-letter-{suffix}"))
    .fetch_one(&pool)
    .await
    .expect("seed lifecycle target");
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deleted', \
             deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 day', deleted_at = now(), \
             purge_started_at = now(), lifecycle_version = lifecycle_version + 1 \
         WHERE id = $1",
    )
    .bind(target_id)
    .execute(&pool)
    .await
    .expect("make target an irreversible purge");
    let job_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.account_lifecycle_jobs \
         (account_id, job_type, status, attempts, next_attempt_at, last_error_code) \
         VALUES ($1, 'purge', 'failed', 20, now(), 'owner_cleanup_failed') RETURNING id",
    )
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("seed exhausted lifecycle job");

    let legacy_admin_token =
        identity::auth::create_access_token(admin_id, "integration-test-secret-32bytes!", 3600)
            .expect("legacy admin token");
    let moderator_token =
        identity::auth::create_access_token(moderator_id, "integration-test-secret-32bytes!", 3600)
            .expect("moderator token");
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v2/admin/account-lifecycle/jobs?accountId={target_id}&status=failed"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {moderator_token}"))
                .body(Body::empty())
                .expect("moderator lifecycle jobs request"),
        )
        .await
        .expect("moderator lifecycle jobs response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v2/admin/account-lifecycle/jobs?accountId={target_id}&status=failed"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {legacy_admin_token}"))
                .body(Body::empty())
                .expect("admin lifecycle jobs request"),
        )
        .await
        .expect("admin lifecycle jobs response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = helpers::read_json(listed).await;
    assert_eq!(listed_body["items"][0]["id"], job_id.to_string());
    assert_eq!(listed_body["items"][0]["attempts"], 20);
    assert!(listed_body["items"][0]["purgeStartedAt"].is_number());

    let missing_step_up = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/account-lifecycle/jobs/{job_id}/requeue"))
                .header(header::AUTHORIZATION, format!("Bearer {legacy_admin_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "reason": "retry exhausted account erasure" }).to_string(),
                ))
                .expect("non-recent requeue request"),
        )
        .await
        .expect("non-recent requeue response");
    assert_eq!(missing_step_up.status(), StatusCode::PRECONDITION_REQUIRED);

    let session_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.sessions \
         (account_id, refresh_hash, family_id, expires_at, recent_authenticated_at, \
          recent_auth_method) \
         VALUES ($1, $2, $3, now() + interval '1 day', now(), 'email_code') RETURNING id",
    )
    .bind(admin_id)
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(uuid::Uuid::new_v4())
    .fetch_one(&pool)
    .await
    .expect("seed recent operator session");
    let auth_version: i64 =
        sqlx::query_scalar("SELECT auth_version FROM identity.accounts WHERE id = $1")
            .bind(admin_id)
            .fetch_one(&pool)
            .await
            .expect("operator auth version");
    let recent_admin_token = identity::auth::create_session_access_token(
        admin_id,
        session_id,
        auth_version,
        "integration-test-secret-32bytes!",
        3600,
    )
    .expect("recent admin token");
    let requeued = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v2/admin/account-lifecycle/jobs/{job_id}/requeue"))
                .header(header::AUTHORIZATION, format!("Bearer {recent_admin_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "reason": "retry exhausted account erasure" }).to_string(),
                ))
                .expect("recent requeue request"),
        )
        .await
        .expect("recent requeue response");
    assert_eq!(requeued.status(), StatusCode::OK);
    let requeued_body = helpers::read_json(requeued).await;
    assert_eq!(requeued_body["status"], "queued");
    assert_eq!(requeued_body["attempts"], 0);
    assert!(requeued_body["purgeStartedAt"].is_number());
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE action = 'identity.lifecycle_job.requeued' \
           AND target_type = 'account_lifecycle_job' AND target_id = $1 \
           AND actor_account_id = $2",
    )
    .bind(job_id.to_string())
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .expect("read lifecycle requeue audit");
    assert_eq!(audit_count, 1);

    let retry = identity::lifecycle::claim_due_job(&pool)
        .await
        .expect("claim repaired lifecycle job")
        .expect("repaired lifecycle job is due");
    assert_eq!(retry.id, job_id);
    identity::lifecycle::complete_purge(&pool, &retry)
        .await
        .expect("finish repaired lifecycle job");
    let final_state: String =
        sqlx::query_scalar("SELECT status::text FROM identity.accounts WHERE id = $1")
            .bind(target_id)
            .fetch_one(&pool)
            .await
            .expect("read repaired account tombstone");
    assert_eq!(final_state, "purged");
}

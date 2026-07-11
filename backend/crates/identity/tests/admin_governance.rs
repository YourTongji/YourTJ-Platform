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

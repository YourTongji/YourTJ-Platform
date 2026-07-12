//! Handler-to-PostgreSQL coverage for appeal-only password and email credentials.

#[path = "helpers/mod.rs"]
mod helpers;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

fn password_hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash test password")
        .to_string()
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Value,
    bearer: Option<&str>,
) -> axum::response::Response {
    let mut builder =
        Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(
            builder.body(Body::from(body.to_string())).expect("build appeal credential request"),
        )
        .await
        .expect("appeal credential response")
}

#[tokio::test]
async fn suspended_account_receives_only_a_scoped_non_refreshable_appeal_credential() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("appeal-auth-{suffix}@tongji.edu.cn");
    let password = "correct-horse-battery-staple!";
    let account_id: i64 = sqlx::query_scalar(
        "INSERT INTO identity.accounts \
         (email, email_verified_at, handle, password_hash) \
         VALUES ($1, now(), $2, $3) RETURNING id",
    )
    .bind(&email)
    .bind(format!("appeal-auth-{suffix}"))
    .bind(password_hash(password))
    .fetch_one(&pool)
    .await
    .expect("insert appeal test account");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, ends_at) \
         VALUES ($1, 'suspend', 'appeal credential integration test', now() + interval '1 day')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("suspend appeal test account");

    let ordinary_login = request(
        &app,
        Method::POST,
        "/api/v2/auth/password/login",
        json!({ "email": email, "password": password }),
        None,
    )
    .await;
    assert_eq!(ordinary_login.status(), StatusCode::FORBIDDEN);

    let appeal_login = request(
        &app,
        Method::POST,
        "/api/v2/auth/appeal/password",
        json!({ "email": email, "password": password }),
        None,
    )
    .await;
    assert_eq!(appeal_login.status(), StatusCode::OK);
    let appeal_body = helpers::read_json(appeal_login).await;
    let appeal_token = appeal_body["accessToken"].as_str().expect("appeal access token");
    assert!(appeal_body.get("refreshToken").is_none());

    let ordinary_surface =
        request(&app, Method::GET, "/api/v2/me", Value::Null, Some(appeal_token)).await;
    assert_eq!(ordinary_surface.status(), StatusCode::UNAUTHORIZED);
    let session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity.sessions WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("appeal credential session count");
    assert_eq!(session_count, 0);

    helpers::insert_valid_code_for_purpose(&pool, &email, "112233", "appeal").await;
    let full_login_with_appeal_code = request(
        &app,
        Method::POST,
        "/api/v2/auth/email/verify",
        json!({ "email": email, "code": "112233", "purpose": "appeal" }),
        None,
    )
    .await;
    assert_eq!(full_login_with_appeal_code.status(), StatusCode::BAD_REQUEST);

    helpers::insert_valid_code_for_purpose(&pool, &email, "223344", "appeal").await;
    let appeal_email = request(
        &app,
        Method::POST,
        "/api/v2/auth/appeal/email/verify",
        json!({ "email": email, "code": "223344" }),
        None,
    )
    .await;
    assert_eq!(appeal_email.status(), StatusCode::OK);
    let email_appeal_body = helpers::read_json(appeal_email).await;
    assert!(email_appeal_body["accessToken"].as_str().is_some());
    assert!(email_appeal_body.get("refreshToken").is_none());
    let final_session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity.sessions WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("email appeal credential session count");
    assert_eq!(final_session_count, 0);
}

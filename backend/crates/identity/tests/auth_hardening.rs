//! Security regressions for purpose-bound codes and refresh-token rotation.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build JSON request")
}

async fn insert_account(pool: &sqlx::PgPool, email: &str, handle: &str) -> i64 {
    sqlx::query_scalar("INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id")
        .bind(email)
        .bind(handle)
        .fetch_one(pool)
        .await
        .expect("insert account")
}

async fn password_login(app: axum::Router, email: &str, password: &str, user_agent: &str) -> Value {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v2/auth/password/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::USER_AGENT, user_agent)
        .body(Body::from(json!({ "email": email, "password": password }).to_string()))
        .expect("password login request");
    let response = app.oneshot(request).await.expect("password login response");
    assert_eq!(response.status(), StatusCode::OK);
    helpers::read_json(response).await
}

async fn authenticated_status(
    app: axum::Router,
    method: Method,
    uri: &str,
    access_token: &str,
    body: Option<Value>,
) -> StatusCode {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {access_token}"));
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    app.oneshot(
        request
            .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
            .expect("authenticated request"),
    )
    .await
    .expect("authenticated response")
    .status()
}

#[tokio::test]
async fn codes_cannot_cross_authentication_purposes() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("purpose-{suffix}@tongji.edu.cn");
    insert_account(&pool, &email, &format!("purpose-{suffix}")).await;

    helpers::insert_valid_code_for_purpose(&pool, &email, "111111", "login").await;
    let reset_response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/password/reset",
            json!({
                "email": email,
                "code": "111111",
                "newPassword": "correct-horse-battery-staple!"
            }),
        ))
        .await
        .expect("reset response");
    assert_eq!(reset_response.status(), StatusCode::BAD_REQUEST);

    let login_response = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/email/verify",
            json!({ "email": email, "code": "111111", "purpose": "login" }),
        ))
        .await
        .expect("login response");
    assert_eq!(login_response.status(), StatusCode::OK);

    helpers::insert_valid_code_for_purpose(&pool, &email, "222222", "password_reset").await;
    let verify_response = app
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/email/verify",
            json!({ "email": email, "code": "222222" }),
        ))
        .await
        .expect("verification response");
    assert_eq!(verify_response.status(), StatusCode::BAD_REQUEST);
    let reset_code_unused: bool = sqlx::query_scalar(
        "SELECT used_at IS NULL FROM identity.email_codes \
         WHERE email = $1 AND purpose = 'password_reset'",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("password reset code state");
    assert!(reset_code_unused);
}

#[tokio::test]
async fn persisted_registration_purpose_survives_account_state_change() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("state-change-{suffix}@tongji.edu.cn");
    helpers::insert_valid_code_for_purpose(&pool, &email, "333333", "registration").await;
    insert_account(&pool, &email, &format!("state-{suffix}")).await;

    let response = app
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/email/verify",
            json!({
                "email": email,
                "code": "333333",
                "purpose": "registration",
                "password": "correct-horse-battery-staple!"
            }),
        ))
        .await
        .expect("verification response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let was_consumed: bool =
        sqlx::query_scalar("SELECT used_at IS NOT NULL FROM identity.email_codes WHERE email = $1")
            .bind(&email)
            .fetch_one(&pool)
            .await
            .expect("code state");
    assert!(was_consumed);
    let password_was_set: bool = sqlx::query_scalar(
        "SELECT password_hash IS NOT NULL FROM identity.accounts WHERE email = $1",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .expect("password state");
    assert!(!password_was_set);
}

#[tokio::test]
async fn fifty_concurrent_verifications_consume_code_once() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("concurrent-code-{suffix}@tongji.edu.cn");
    let handle = format!("concurrent-{}", &suffix[..16]);
    helpers::insert_valid_code_for_purpose(&pool, &email, "444444", "registration").await;

    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..50 {
        let app = app.clone();
        let email = email.clone();
        let handle = handle.clone();
        requests.spawn(async move {
            app.oneshot(json_request(
                Method::POST,
                "/api/v2/auth/email/verify",
                json!({
                    "email": email,
                    "code": "444444",
                    "purpose": "registration",
                    "handle": handle
                }),
            ))
            .await
            .expect("verification response")
            .status()
        });
    }
    let mut successes = 0;
    while let Some(result) = requests.join_next().await {
        if result.expect("verification task") == StatusCode::OK {
            successes += 1;
        }
    }
    assert_eq!(successes, 1);
    let accounts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity.accounts WHERE email = $1")
            .bind(&email)
            .fetch_one(&pool)
            .await
            .expect("account count");
    assert_eq!(accounts, 1);
}

#[tokio::test]
async fn password_login_is_neutral_for_missing_password_account_and_unavailable_state() {
    const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    const DUMMY_PASSWORD: &str = "yourtj-constant-dummy-password";
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let wrong_email = format!("wrong-{suffix}@tongji.edu.cn");
    let no_password_email = format!("no-password-{suffix}@tongji.edu.cn");
    let missing_email = format!("missing-{suffix}@tongji.edu.cn");
    let suspended_email = format!("suspended-{suffix}@tongji.edu.cn");
    let wrong_id = insert_account(&pool, &wrong_email, &format!("wrong-{suffix}")).await;
    insert_account(&pool, &no_password_email, &format!("none-{suffix}")).await;
    let suspended_id =
        insert_account(&pool, &suspended_email, &format!("suspended-{suffix}")).await;
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(DUMMY_HASH)
        .bind(wrong_id)
        .execute(&pool)
        .await
        .expect("set password hash");
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(DUMMY_HASH)
        .bind(suspended_id)
        .execute(&pool)
        .await
        .expect("set suspended password account");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, ends_at) \
         VALUES ($1, 'suspend', 'password response neutrality test', now() + interval '1 day')",
    )
    .bind(suspended_id)
    .execute(&pool)
    .await
    .expect("suspend password account");

    let mut bodies = Vec::new();
    for email in [wrong_email, no_password_email, missing_email] {
        let response = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v2/auth/password/login",
                json!({ "email": email, "password": "definitely-wrong-password" }),
            ))
            .await
            .expect("password login response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        bodies.push(helpers::read_json(response).await);
    }
    assert_eq!(bodies[0], bodies[1]);
    assert_eq!(bodies[1], bodies[2]);

    let suspended_response = app
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/password/login",
            json!({ "email": suspended_email, "password": DUMMY_PASSWORD }),
        ))
        .await
        .expect("suspended password login response");
    assert_eq!(suspended_response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(helpers::read_json(suspended_response).await, bodies[0]);
}

#[tokio::test]
async fn password_forgot_is_neutral_when_account_or_delivery_is_missing() {
    const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    let mut config = shared::Config::from_env().expect("test config");
    config.email_provider = shared::config::EmailProvider::Cloudflare;
    config.email_from = "welcome@yourtj.de".into();
    config.cloudflare_email_account_id = "0".repeat(32);
    config.cloudflare_email_api_token = "test-token-long-enough".into();
    config.cloudflare_email_api_base_url = "http://127.0.0.1:1".into();
    let (pool, app) = helpers::create_test_app_with_config(config).await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let existing_email = format!("forgot-existing-{suffix}@tongji.edu.cn");
    let missing_email = format!("forgot-missing-{suffix}@tongji.edu.cn");
    let account_id = insert_account(&pool, &existing_email, &format!("forgot-{suffix}")).await;
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(PASSWORD_HASH)
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("set password");

    for email in [existing_email, missing_email] {
        let response = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v2/auth/password/forgot",
                json!({ "email": email, "captchaToken": uuid::Uuid::new_v4().to_string() }),
            ))
            .await
            .expect("forgot response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}

#[tokio::test]
async fn password_reset_is_neutral_for_missing_ineligible_and_code_missing_accounts() {
    const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let existing_email = format!("reset-neutral-existing-{suffix}@tongji.edu.cn");
    let code_only_email = format!("reset-neutral-code-only-{suffix}@tongji.edu.cn");
    let missing_email = format!("reset-neutral-missing-{suffix}@tongji.edu.cn");
    let account_id =
        insert_account(&pool, &existing_email, &format!("reset-neutral-{suffix}")).await;
    insert_account(&pool, &code_only_email, &format!("code-only-{suffix}")).await;
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(PASSWORD_HASH)
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("set password");

    let mut bodies = Vec::new();
    for email in [existing_email, code_only_email, missing_email] {
        let response = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/v2/auth/password/reset",
                json!({
                    "email": email,
                    "code": "123456",
                    "newPassword": "neutral-reset-correct-horse-battery-staple!"
                }),
            ))
            .await
            .expect("neutral reset response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        bodies.push(helpers::read_json(response).await);
    }
    assert_eq!(bodies[0], bodies[1]);
    assert_eq!(bodies[1], bodies[2]);
}

#[tokio::test]
async fn refresh_rotation_creates_one_successor_and_replay_revokes_family() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("refresh-{suffix}@tongji.edu.cn");
    let handle = format!("refresh-{}", &suffix[..16]);
    helpers::insert_valid_code_for_purpose(&pool, &email, "555555", "registration").await;
    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/email/verify",
            json!({
                "email": email,
                "code": "555555",
                "purpose": "registration",
                "handle": handle
            }),
        ))
        .await
        .expect("login response");
    assert_eq!(login.status(), StatusCode::OK);
    let login_body = helpers::read_json(login).await;
    let refresh_token = login_body["refreshToken"].as_str().expect("refresh token").to_owned();
    let original_session_id =
        i64::from_str_radix(refresh_token.split_once(':').expect("combined refresh token").0, 16)
            .expect("session id");

    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..50 {
        let app = app.clone();
        let refresh_token = refresh_token.clone();
        requests.spawn(async move {
            app.oneshot(json_request(
                Method::POST,
                "/api/v2/auth/refresh",
                json!({ "refreshToken": refresh_token }),
            ))
            .await
            .expect("refresh response")
            .status()
        });
    }
    let mut successes = 0;
    while let Some(result) = requests.join_next().await {
        if result.expect("refresh task") == StatusCode::OK {
            successes += 1;
        }
    }
    assert_eq!(successes, 1);

    let family_state: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COUNT(*) FILTER (WHERE revoked_at IS NULL) \
         FROM identity.sessions WHERE family_id = ( \
           SELECT family_id FROM identity.sessions WHERE id = $1 \
         )",
    )
    .bind(original_session_id)
    .fetch_one(&pool)
    .await
    .expect("session family state");
    assert_eq!(family_state, (2, 0));
}

#[tokio::test]
async fn device_session_controls_preserve_only_the_current_session() {
    const PASSWORD: &str = "yourtj-constant-dummy-password";
    const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("devices-{suffix}@tongji.edu.cn");
    let account_id = insert_account(&pool, &email, &format!("devices-{suffix}")).await;
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(PASSWORD_HASH)
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("set password");

    let first = password_login(app.clone(), &email, PASSWORD, "First device").await;
    let second = password_login(app.clone(), &email, PASSWORD, "Second device").await;
    let first_access = first["accessToken"].as_str().expect("first access token");
    let second_access = second["accessToken"].as_str().expect("second access token");

    let sessions_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v2/me/sessions")
        .header(header::AUTHORIZATION, format!("Bearer {first_access}"))
        .body(Body::empty())
        .expect("session list request");
    let sessions_response =
        app.clone().oneshot(sessions_request).await.expect("session list response");
    assert_eq!(sessions_response.status(), StatusCode::OK);
    let sessions = helpers::read_json(sessions_response).await;
    let session_items = sessions["items"].as_array().expect("sessions array");
    assert_eq!(session_items.len(), 2);
    assert_eq!(session_items.iter().filter(|row| row["isCurrent"] == true).count(), 1);

    assert_eq!(
        authenticated_status(
            app.clone(),
            Method::POST,
            "/api/v2/me/sessions/revoke-others",
            first_access,
            None,
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", first_access, None).await,
        StatusCode::OK
    );
    assert_eq!(
        authenticated_status(app, Method::GET, "/api/v2/me", second_access, None).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn password_reset_replaces_every_access_and_refresh_session() {
    const PASSWORD: &str = "yourtj-constant-dummy-password";
    const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("reset-revoke-{suffix}@tongji.edu.cn");
    let account_id = insert_account(&pool, &email, &format!("reset-{suffix}")).await;
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(PASSWORD_HASH)
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("set password");
    let first = password_login(app.clone(), &email, PASSWORD, "Reset device one").await;
    let second = password_login(app.clone(), &email, PASSWORD, "Reset device two").await;
    let first_access = first["accessToken"].as_str().expect("first access token").to_owned();
    let second_access = second["accessToken"].as_str().expect("second access token").to_owned();
    helpers::insert_valid_code_for_purpose(&pool, &email, "666666", "password_reset").await;

    let reset = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v2/auth/password/reset",
            json!({
                "email": email,
                "code": "666666",
                "newPassword": "a-different-correct-horse-battery-staple!"
            }),
        ))
        .await
        .expect("password reset response");
    assert_eq!(reset.status(), StatusCode::OK);
    let reset_body = helpers::read_json(reset).await;
    let replacement_access =
        reset_body["accessToken"].as_str().expect("replacement access token").to_owned();
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", &first_access, None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", &second_access, None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", &replacement_access, None,)
            .await,
        StatusCode::OK
    );
    let active_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.sessions WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("active session count");
    assert_eq!(active_sessions, 1);
}

#[tokio::test]
async fn password_change_replaces_current_session_and_revokes_others() {
    const PASSWORD: &str = "yourtj-constant-dummy-password";
    const PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$lMsuCNrM/Jk4lpdAY/Gk9w$NkmJDYSq0o5US61ZPai1ajtpZWKmn7Rvn4wqQn3DR7Y";
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("change-password-{suffix}@tongji.edu.cn");
    let account_id = insert_account(&pool, &email, &format!("change-{suffix}")).await;
    sqlx::query("UPDATE identity.accounts SET password_hash = $1 WHERE id = $2")
        .bind(PASSWORD_HASH)
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("set password");
    let current = password_login(app.clone(), &email, PASSWORD, "Current device").await;
    let other = password_login(app.clone(), &email, PASSWORD, "Other device").await;
    let current_access = current["accessToken"].as_str().expect("current access token").to_owned();
    let other_access = other["accessToken"].as_str().expect("other access token").to_owned();

    let change = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/auth/password/change")
                .header(header::AUTHORIZATION, format!("Bearer {current_access}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "currentPassword": PASSWORD,
                        "newPassword": "changed-correct-horse-battery-staple!"
                    })
                    .to_string(),
                ))
                .expect("password change request"),
        )
        .await
        .expect("password change response");
    assert_eq!(change.status(), StatusCode::OK);
    let change_body = helpers::read_json(change).await;
    let replacement_access =
        change_body["accessToken"].as_str().expect("replacement access token").to_owned();
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", &current_access, None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", &other_access, None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        authenticated_status(app, Method::GET, "/api/v2/me", &replacement_access, None).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn logout_all_invalidates_rolling_legacy_access_tokens() {
    let (pool, app) = helpers::create_test_app().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let email = format!("legacy-access-{suffix}@tongji.edu.cn");
    insert_account(&pool, &email, &format!("legacy-{suffix}")).await;
    let (legacy_access, _) = helpers::create_access_token_for(&email, &pool).await;
    assert_eq!(
        authenticated_status(app.clone(), Method::GET, "/api/v2/me", &legacy_access, None).await,
        StatusCode::OK
    );
    assert_eq!(
        authenticated_status(
            app.clone(),
            Method::POST,
            "/api/v2/auth/logout-all",
            &legacy_access,
            None,
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        authenticated_status(app, Method::GET, "/api/v2/me", &legacy_access, None).await,
        StatusCode::UNAUTHORIZED
    );
}

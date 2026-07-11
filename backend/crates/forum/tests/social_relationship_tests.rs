//! Handler-to-database coverage for follow, mute, block, privacy, and counts.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

fn request(
    method: Method,
    uri: impl AsRef<str>,
    token: Option<&str>,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri.as_ref());
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .expect("social request")
}

async fn status(
    app: &Router,
    method: Method,
    uri: impl AsRef<str>,
    token: Option<&str>,
    body: Option<Value>,
) -> StatusCode {
    app.clone().oneshot(request(method, uri, token, body)).await.expect("social response").status()
}

async fn json_response(
    app: &Router,
    method: Method,
    uri: impl AsRef<str>,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let response =
        app.clone().oneshot(request(method, uri, token, body)).await.expect("social JSON response");
    let response_status = response.status();
    let response_body = helpers::read_json(response).await;
    (response_status, response_body)
}

#[tokio::test]
async fn social_graph_enforces_counts_privacy_and_block_boundaries() {
    let (pool, app) = helpers::create_test_app().await;
    let (alice_id, alice_token) =
        helpers::create_test_account(&pool, "social-alice@tongji.edu.cn", "social-alice").await;
    let (bob_id, bob_token) =
        helpers::create_test_account(&pool, "social-bob@tongji.edu.cn", "social-bob").await;
    let (charlie_id, charlie_token) =
        helpers::create_test_account(&pool, "social-charlie@tongji.edu.cn", "social-charlie").await;
    sqlx::query("UPDATE identity.accounts SET trust_level = 1 WHERE id = ANY($1)")
        .bind(vec![alice_id, bob_id, charlie_id])
        .execute(&pool)
        .await
        .expect("raise social test trust");

    assert_eq!(
        status(&app, Method::GET, "/api/v2/users/social-bob", None, None).await,
        StatusCode::NOT_FOUND
    );
    let (campus_profile_status, campus_profile) =
        json_response(&app, Method::GET, "/api/v2/users/social-bob", Some(&alice_token), None)
            .await;
    assert_eq!(campus_profile_status, StatusCode::OK);
    assert_eq!(campus_profile["followerCount"], 0);
    assert!(campus_profile.get("email").is_none());

    let mut follow_tasks = Vec::new();
    for _ in 0..12 {
        let app = app.clone();
        let token = alice_token.clone();
        follow_tasks.push(tokio::spawn(async move {
            status(&app, Method::PUT, "/api/v2/users/social-bob/follow", Some(&token), None).await
        }));
    }
    for task in follow_tasks {
        assert_eq!(task.await.expect("follow task"), StatusCode::NO_CONTENT);
    }
    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-alice/follow", Some(&bob_token), None,)
            .await,
        StatusCode::NO_CONTENT
    );

    let (relationship_status, relationship) = json_response(
        &app,
        Method::GET,
        "/api/v2/users/social-bob/relationship",
        Some(&alice_token),
        None,
    )
    .await;
    assert_eq!(relationship_status, StatusCode::OK);
    assert_eq!(relationship["following"], true);
    assert_eq!(relationship["followedBy"], true);
    assert_eq!(relationship["canStartConversation"], true);

    let counts: Vec<(i64, i32, i32)> = sqlx::query_as(
        "SELECT account_id, follower_count, following_count \
         FROM forum.user_social_stats WHERE account_id = ANY($1) ORDER BY account_id",
    )
    .bind(vec![alice_id, bob_id])
    .fetch_all(&pool)
    .await
    .expect("read social counts");
    assert_eq!(counts.len(), 2);
    assert!(counts.iter().all(|(_, followers, following)| *followers == 1 && *following == 1));

    let (followers_status, followers) = json_response(
        &app,
        Method::GET,
        "/api/v2/users/social-bob/followers",
        Some(&alice_token),
        None,
    )
    .await;
    assert_eq!(followers_status, StatusCode::OK);
    assert_eq!(followers["items"][0]["id"], alice_id.to_string());
    assert_eq!(
        status(
            &app,
            Method::GET,
            "/api/v2/users/social-bob/followers",
            Some(&charlie_token),
            None,
        )
        .await,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-bob/mute", Some(&alice_token), None,).await,
        StatusCode::NO_CONTENT
    );
    let bob_thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, 'Muted author thread', 'body') RETURNING id",
    )
    .bind(bob_id)
    .fetch_one(&pool)
    .await
    .expect("seed muted author thread");
    let poll_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.polls (thread_id, question) \
         VALUES ($1, 'Blocked poll?') RETURNING id",
    )
    .bind(bob_thread_id)
    .fetch_one(&pool)
    .await
    .expect("seed blocked author poll");
    let poll_option_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.poll_options (poll_id, position, label) \
         VALUES ($1, 0, 'Yes') RETURNING id",
    )
    .bind(poll_id)
    .fetch_one(&pool)
    .await
    .expect("seed blocked author poll option");
    let (_, muted_feed) =
        json_response(&app, Method::GET, "/api/v2/forum/threads", Some(&alice_token), None).await;
    let bob_thread_id_text = bob_thread_id.to_string();
    assert!(muted_feed["items"]
        .as_array()
        .expect("muted feed items")
        .iter()
        .all(|item| item["id"].as_str() != Some(bob_thread_id_text.as_str())));

    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-bob/block", Some(&alice_token), None,)
            .await,
        StatusCode::NO_CONTENT
    );
    let follow_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.user_follows \
         WHERE follower_id = ANY($1) AND followed_id = ANY($1)",
    )
    .bind(vec![alice_id, bob_id])
    .fetch_one(&pool)
    .await
    .expect("follow rows after block");
    assert_eq!(follow_rows, 0);
    let count_sum: i64 = sqlx::query_scalar(
        "SELECT SUM(follower_count + following_count) \
         FROM forum.user_social_stats WHERE account_id = ANY($1)",
    )
    .bind(vec![alice_id, bob_id])
    .fetch_one(&pool)
    .await
    .expect("counts after block");
    assert_eq!(count_sum, 0);
    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-bob/follow", Some(&alice_token), None,)
            .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(
            &app,
            Method::POST,
            format!("/api/v2/forum/threads/{bob_thread_id}/comments"),
            Some(&alice_token),
            Some(json!({ "body": "blocked reply" })),
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(
            &app,
            Method::POST,
            format!("/api/v2/forum/posts/{bob_thread_id}/vote"),
            Some(&alice_token),
            Some(json!({ "postType": "thread", "value": "up" })),
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(
            &app,
            Method::POST,
            format!("/api/v2/forum/polls/{poll_id}/vote"),
            Some(&alice_token),
            Some(json!({ "optionId": poll_option_id.to_string() })),
        )
        .await,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        status(&app, Method::DELETE, "/api/v2/users/social-bob/block", Some(&alice_token), None,)
            .await,
        StatusCode::NO_CONTENT
    );
    let (_, unblocked_relationship) = json_response(
        &app,
        Method::GET,
        "/api/v2/users/social-bob/relationship",
        Some(&alice_token),
        None,
    )
    .await;
    assert_eq!(unblocked_relationship["following"], false);
    assert_eq!(unblocked_relationship["followedBy"], false);
    assert_eq!(unblocked_relationship["blockedByMe"], false);
    assert_eq!(unblocked_relationship["canStartConversation"], false);

    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-alice/follow", Some(&bob_token), None,)
            .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        status(
            &app,
            Method::POST,
            "/api/v2/forum/dm/conversations",
            Some(&alice_token),
            Some(json!({ "recipientHandle": "social-bob" })),
        )
        .await,
        StatusCode::OK
    );
    sqlx::query(
        "INSERT INTO identity.profile_privacy \
         (account_id, profile_visibility, followers_visibility, following_visibility, \
          discoverable, dm_policy) \
         VALUES ($1, 'only_me', 'only_me', 'only_me', FALSE, 'nobody') \
         ON CONFLICT (account_id) DO UPDATE \
         SET profile_visibility = EXCLUDED.profile_visibility, \
             followers_visibility = EXCLUDED.followers_visibility, \
             following_visibility = EXCLUDED.following_visibility, \
             discoverable = EXCLUDED.discoverable, dm_policy = EXCLUDED.dm_policy",
    )
    .bind(bob_id)
    .execute(&pool)
    .await
    .expect("tighten Bob privacy");
    assert_eq!(
        status(&app, Method::GET, "/api/v2/users/social-bob", Some(&alice_token), None,).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status(&app, Method::GET, "/api/v2/users/social-bob", Some(&bob_token), None,).await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-bob/follow", Some(&alice_token), None,)
            .await,
        StatusCode::NOT_FOUND
    );

    sqlx::query(
        "INSERT INTO identity.sanctions \
         (account_id, kind, reason, starts_at) VALUES ($1, 'suspend', 'test suspension', now())",
    )
    .bind(charlie_id)
    .execute(&pool)
    .await
    .expect("suspend Charlie");
    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-charlie/follow", Some(&alice_token), None,)
            .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status(&app, Method::PUT, "/api/v2/users/social-charlie/block", Some(&alice_token), None,)
            .await,
        StatusCode::NO_CONTENT
    );
    let suspended_block_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM forum.user_ignores \
         WHERE account_id = $1 AND ignored_account_id = $2)",
    )
    .bind(alice_id)
    .bind(charlie_id)
    .fetch_one(&pool)
    .await
    .expect("suspended target block");
    assert!(suspended_block_exists);

    let (dave_id, dave_token) =
        helpers::create_test_account(&pool, "social-dave@tongji.edu.cn", "social-dave").await;
    let (eve_id, eve_token) =
        helpers::create_test_account(&pool, "social-eve@tongji.edu.cn", "social-eve").await;
    let (frank_id, frank_token) =
        helpers::create_test_account(&pool, "social-frank@tongji.edu.cn", "social-frank").await;
    let cycle_requests = [
        (dave_token, "/api/v2/users/social-eve/follow"),
        (eve_token, "/api/v2/users/social-frank/follow"),
        (frank_token, "/api/v2/users/social-dave/follow"),
    ];
    let mut cycle_tasks = Vec::new();
    for (token, uri) in cycle_requests {
        let app = app.clone();
        cycle_tasks.push(tokio::spawn(async move {
            status(&app, Method::PUT, uri, Some(&token), None).await
        }));
    }
    for task in cycle_tasks {
        assert_eq!(task.await.expect("cyclic follow task"), StatusCode::NO_CONTENT);
    }
    let cycle_counts: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT follower_count, following_count FROM forum.user_social_stats \
         WHERE account_id = ANY($1)",
    )
    .bind(vec![dave_id, eve_id, frank_id])
    .fetch_all(&pool)
    .await
    .expect("cyclic follow counts");
    assert_eq!(cycle_counts.len(), 3);
    assert!(cycle_counts.iter().all(|counts| *counts == (1, 1)));
}

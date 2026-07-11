//! Integration coverage for viewer state and reversible forum interactions.

mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn json_request(
    app: &axum::Router,
    method: Method,
    uri: String,
    token: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request_body = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(request_body).expect("build forum parity request"))
        .await
        .expect("forum parity response")
}

#[tokio::test]
async fn viewer_state_subscriptions_tags_polls_bookmarks_and_reads_are_consistent() {
    let (pool, app) = create_test_app().await;
    let (_, author_token) =
        create_test_account(&pool, "parity-author@tongji.edu.cn", "parity-author").await;
    let (viewer_id, viewer_token) =
        create_test_account(&pool, "parity-viewer@tongji.edu.cn", "parity-viewer").await;
    forum::repo::create_tag(&pool, "campus", "Campus", None).await.expect("create tag");
    sqlx::query("ALTER TABLE forum.threads ALTER COLUMN id RESTART WITH 1")
        .execute(&pool)
        .await
        .expect("restart thread identity");
    sqlx::query("ALTER TABLE forum.comments ALTER COLUMN id RESTART WITH 1")
        .execute(&pool)
        .await
        .expect("restart comment identity");

    let create_response = json_request(
        &app,
        Method::POST,
        "/api/v2/forum/threads".into(),
        Some(&author_token),
        Some(json!({
            "boardId": "1",
            "title": "Tagged poll thread",
            "tags": ["campus"],
            "poll": {"question": "Choose", "options": ["A", "B"]}
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created = read_json(create_response).await;
    let thread_id = created["id"].as_str().expect("thread id").to_owned();
    let poll_id = created["poll"]["id"].as_str().expect("poll id").to_owned();
    let first_option =
        created["poll"]["options"][0]["id"].as_str().expect("first poll option").to_owned();
    let second_option =
        created["poll"]["options"][1]["id"].as_str().expect("second poll option").to_owned();

    let tag_feed = json_request(
        &app,
        Method::GET,
        "/api/v2/forum/threads?sort=new&tag=campus".into(),
        None,
        None,
    )
    .await;
    assert_eq!(tag_feed.status(), StatusCode::OK);
    let tag_feed = read_json(tag_feed).await;
    assert_eq!(tag_feed["items"].as_array().expect("tag items").len(), 1);
    assert_eq!(tag_feed["items"][0]["tags"], json!(["campus"]));

    let subscribe_board = json_request(
        &app,
        Method::PUT,
        "/api/v2/forum/subscriptions".into(),
        Some(&viewer_token),
        Some(json!({"targetType": "board", "targetId": "1", "level": "tracking"})),
    )
    .await;
    assert_eq!(subscribe_board.status(), StatusCode::NO_CONTENT);
    let subscription_feed = json_request(
        &app,
        Method::GET,
        "/api/v2/forum/threads?sort=subscriptions".into(),
        Some(&viewer_token),
        None,
    )
    .await;
    assert_eq!(subscription_feed.status(), StatusCode::OK);
    assert_eq!(read_json(subscription_feed).await["items"][0]["id"], thread_id);

    let mute_thread = json_request(
        &app,
        Method::PUT,
        "/api/v2/forum/subscriptions".into(),
        Some(&viewer_token),
        Some(json!({"targetType": "thread", "targetId": thread_id, "level": "muted"})),
    )
    .await;
    assert_eq!(mute_thread.status(), StatusCode::NO_CONTENT);
    let muted_feed = json_request(
        &app,
        Method::GET,
        "/api/v2/forum/threads?sort=subscriptions".into(),
        Some(&viewer_token),
        None,
    )
    .await;
    assert!(read_json(muted_feed).await["items"].as_array().expect("muted feed items").is_empty());
    let remove_override = json_request(
        &app,
        Method::DELETE,
        "/api/v2/forum/subscriptions".into(),
        Some(&viewer_token),
        Some(json!({"targetType": "thread", "targetId": thread_id})),
    )
    .await;
    assert_eq!(remove_override.status(), StatusCode::NO_CONTENT);
    let subscriptions = json_request(
        &app,
        Method::GET,
        "/api/v2/forum/subscriptions?limit=1&type=board".into(),
        Some(&viewer_token),
        None,
    )
    .await;
    let subscriptions = read_json(subscriptions).await;
    assert_eq!(subscriptions["items"][0]["targetType"], "board");
    let invalid_subscription = json_request(
        &app,
        Method::PUT,
        "/api/v2/forum/subscriptions".into(),
        Some(&viewer_token),
        Some(json!({"targetType": "thread", "targetId": "999999", "level": "tracking"})),
    )
    .await;
    assert_eq!(invalid_subscription.status(), StatusCode::NOT_FOUND);

    let thread_id_number = thread_id.parse::<i64>().expect("numeric thread id");
    let comment_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         SELECT $1, id, 'first comment', '0001' FROM identity.accounts \
         WHERE handle = 'parity-author' RETURNING id",
    )
    .bind(thread_id_number)
    .fetch_one(&pool)
    .await
    .expect("insert first comment");
    assert_eq!(comment_id, thread_id_number);
    for post_type in ["thread", "comment"] {
        let bookmark = json_request(
            &app,
            Method::PUT,
            format!("/api/v2/forum/posts/{thread_id}/bookmark"),
            Some(&viewer_token),
            Some(json!({"postType": post_type})),
        )
        .await;
        assert_eq!(bookmark.status(), StatusCode::NO_CONTENT);
    }
    let bookmarks = json_request(
        &app,
        Method::GET,
        "/api/v2/forum/bookmarks".into(),
        Some(&viewer_token),
        None,
    )
    .await;
    assert_eq!(read_json(bookmarks).await["items"].as_array().expect("bookmarks").len(), 2);
    let remove_thread_bookmark = json_request(
        &app,
        Method::DELETE,
        format!("/api/v2/forum/posts/{thread_id}/bookmark?postType=thread"),
        Some(&viewer_token),
        None,
    )
    .await;
    assert_eq!(remove_thread_bookmark.status(), StatusCode::NO_CONTENT);
    let remaining_type: String =
        sqlx::query_scalar("SELECT target_type FROM forum.bookmarks WHERE account_id = $1")
            .bind(viewer_id)
            .fetch_one(&pool)
            .await
            .expect("remaining bookmark");
    assert_eq!(remaining_type, "comment");
    let comments = json_request(
        &app,
        Method::GET,
        format!("/api/v2/forum/threads/{thread_id}/comments"),
        Some(&viewer_token),
        None,
    )
    .await;
    assert_eq!(read_json(comments).await["items"][0]["isBookmarked"], true);
    let vote_thread = json_request(
        &app,
        Method::POST,
        format!("/api/v2/forum/posts/{thread_id}/vote"),
        Some(&viewer_token),
        Some(json!({"value": "up", "postType": "thread"})),
    )
    .await;
    assert_eq!(read_json(vote_thread).await["viewerVote"], "up");

    for option_id in [&first_option, &second_option] {
        let vote = json_request(
            &app,
            Method::POST,
            format!("/api/v2/forum/polls/{poll_id}/vote"),
            Some(&viewer_token),
            Some(json!({"optionId": option_id})),
        )
        .await;
        assert_eq!(vote.status(), StatusCode::OK);
    }
    let poll_results = json_request(
        &app,
        Method::GET,
        format!("/api/v2/forum/polls/{poll_id}/results"),
        Some(&viewer_token),
        None,
    )
    .await;
    let poll_results = read_json(poll_results).await;
    assert_eq!(poll_results["options"][0]["voteCount"], 0);
    assert_eq!(poll_results["options"][1]["voteCount"], 1);
    assert_eq!(poll_results["myVotes"], json!([second_option]));
    let remove_poll_vote = json_request(
        &app,
        Method::DELETE,
        format!("/api/v2/forum/polls/{poll_id}/vote?optionId={second_option}"),
        Some(&viewer_token),
        None,
    )
    .await;
    assert_eq!(read_json(remove_poll_vote).await["myVotes"], json!([]));
    let option_counts: Vec<i32> = sqlx::query_scalar(
        "SELECT vote_count FROM forum.poll_options WHERE poll_id = $1 ORDER BY position",
    )
    .bind(poll_id.parse::<i64>().expect("numeric poll id"))
    .fetch_all(&pool)
    .await
    .expect("poll counts");
    assert_eq!(option_counts, vec![0, 0]);
    sqlx::query("UPDATE forum.polls SET closes_at = now() - interval '1 second' WHERE id = $1")
        .bind(poll_id.parse::<i64>().expect("numeric poll id"))
        .execute(&pool)
        .await
        .expect("close poll");
    let closed_poll_vote = json_request(
        &app,
        Method::POST,
        format!("/api/v2/forum/polls/{poll_id}/vote"),
        Some(&viewer_token),
        Some(json!({"optionId": first_option})),
    )
    .await;
    assert_eq!(closed_poll_vote.status(), StatusCode::CONFLICT);

    let report_read = json_request(
        &app,
        Method::POST,
        format!("/api/v2/forum/threads/{thread_id}/read"),
        Some(&viewer_token),
        Some(json!({"lastReadCommentId": null})),
    )
    .await;
    assert_eq!(report_read.status(), StatusCode::NO_CONTENT);
    sqlx::query(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         SELECT $1, id, 'second comment', '0002' FROM identity.accounts \
         WHERE handle = 'parity-author'",
    )
    .bind(thread_id_number)
    .execute(&pool)
    .await
    .expect("insert second comment");
    let unread = json_request(
        &app,
        Method::GET,
        "/api/v2/forum/threads?sort=unread".into(),
        Some(&viewer_token),
        None,
    )
    .await;
    assert_eq!(read_json(unread).await["items"][0]["unreadCount"], 1);
    let backward_read = json_request(
        &app,
        Method::POST,
        format!("/api/v2/forum/threads/{thread_id}/read"),
        Some(&viewer_token),
        Some(json!({"lastReadCommentId": comment_id.to_string()})),
    )
    .await;
    assert_eq!(backward_read.status(), StatusCode::NO_CONTENT);

    let detail = json_request(
        &app,
        Method::GET,
        format!("/api/v2/forum/threads/{thread_id}"),
        Some(&viewer_token),
        None,
    )
    .await;
    let detail = read_json(detail).await;
    assert_eq!(detail["myLastReadCommentId"], comment_id.to_string());
    assert_eq!(detail["mySubscriptionLevel"], "tracking");
    assert_eq!(detail["isBookmarked"], false);
    assert_eq!(detail["viewerVote"], "up");
    assert_eq!(detail["poll"]["myVotes"], json!([]));

    let other_thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title) \
         SELECT 1, id, 'other thread' FROM identity.accounts \
         WHERE handle = 'parity-author' RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert other thread");
    let foreign_comment_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         SELECT $1, id, 'foreign comment', '0001' FROM identity.accounts \
         WHERE handle = 'parity-author' RETURNING id",
    )
    .bind(other_thread_id)
    .fetch_one(&pool)
    .await
    .expect("insert foreign comment");
    let invalid_read = json_request(
        &app,
        Method::POST,
        format!("/api/v2/forum/threads/{thread_id}/read"),
        Some(&viewer_token),
        Some(json!({"lastReadCommentId": foreign_comment_id.to_string()})),
    )
    .await;
    assert_eq!(invalid_read.status(), StatusCode::BAD_REQUEST);
}

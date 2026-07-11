//! Integration coverage for canonical 1:1 DMs, unread state, blocking, and reports.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, Response, StatusCode};
use axum::Router;
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> Response<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let request_body = if let Some(value) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(value.to_string())
    } else {
        Body::empty()
    };
    app.clone().oneshot(builder.body(request_body).expect("build request")).await.unwrap()
}

#[tokio::test]
async fn dm_lifecycle_is_canonical_private_readable_and_moderated() {
    let (pool, app) = create_test_app().await;
    let (alice_id, alice_token) =
        create_test_account(&pool, "dm-alice@tongji.edu.cn", "dm-alice").await;
    let (bob_id, bob_token) = create_test_account(&pool, "dm-bob@tongji.edu.cn", "dm-bob").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "dm-mod@tongji.edu.cn", "dm-mod").await;

    sqlx::query(
        "UPDATE identity.accounts \
         SET trust_level = 1, \
             avatar_url = CASE WHEN id = $2 THEN 'https://example.test/bob.png' ELSE avatar_url END, \
             role = CASE WHEN id = $3 THEN 'mod'::identity.account_role ELSE role END \
         WHERE id = ANY($1)",
    )
    .bind(vec![alice_id, bob_id, moderator_id])
    .bind(bob_id)
    .bind(moderator_id)
    .execute(&pool)
    .await
    .expect("prepare DM accounts");

    let (first_id, second_id) = tokio::join!(
        forum::repo::dms::find_or_create_conversation(&pool, alice_id, bob_id),
        forum::repo::dms::find_or_create_conversation(&pool, bob_id, alice_id),
    );
    assert_eq!(first_id.unwrap(), second_id.unwrap());
    let conversation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.dm_conversations \
         WHERE account_low_id = LEAST($1, $2) AND account_high_id = GREATEST($1, $2)",
    )
    .bind(alice_id)
    .bind(bob_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(conversation_count, 1);

    let create_response = request(
        &app,
        Method::POST,
        "/api/v2/forum/dm/conversations",
        &alice_token,
        Some(json!({ "recipientHandle": "dm-bob" })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let conversation = read_json(create_response).await;
    let conversation_id = conversation["id"].as_str().unwrap();
    assert_eq!(conversation["participantId"], bob_id.to_string());
    assert_eq!(conversation["participantHandle"], "dm-bob");
    assert_eq!(conversation["participantAvatarUrl"], "https://example.test/bob.png");
    assert_eq!(conversation["unreadCount"], 0);

    let send_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/messages"),
        &alice_token,
        Some(json!({ "body": "hello bob" })),
    )
    .await;
    assert_eq!(send_response.status(), StatusCode::CREATED);
    let sent_message = read_json(send_response).await;
    let message_id = sent_message["id"].as_str().unwrap();

    let inbox_response =
        request(&app, Method::GET, "/api/v2/forum/dm/conversations?limit=20", &bob_token, None)
            .await;
    assert_eq!(inbox_response.status(), StatusCode::OK);
    let inbox = read_json(inbox_response).await;
    assert_eq!(inbox["items"].as_array().unwrap().len(), 1);
    assert_eq!(inbox["items"][0]["participantId"], alice_id.to_string());
    assert_eq!(inbox["items"][0]["lastMessageExcerpt"], "hello bob");
    assert_eq!(inbox["items"][0]["unreadCount"], 1);

    let read_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/read"),
        &bob_token,
        Some(json!({ "lastReadMessageId": message_id })),
    )
    .await;
    assert_eq!(read_response.status(), StatusCode::NO_CONTENT);

    let inbox_after_read =
        request(&app, Method::GET, "/api/v2/forum/dm/conversations?limit=20", &bob_token, None)
            .await;
    let inbox_after_read = read_json(inbox_after_read).await;
    assert_eq!(inbox_after_read["items"][0]["unreadCount"], 0);

    let report_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/messages/{message_id}/report"),
        &bob_token,
        Some(json!({ "reason": "abuse", "note": "targeted harassment" })),
    )
    .await;
    assert_eq!(report_response.status(), StatusCode::ACCEPTED);
    let report = read_json(report_response).await;
    let report_id = report["id"].as_str().unwrap();

    let staff_cannot_browse = request(
        &app,
        Method::GET,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/messages"),
        &moderator_token,
        None,
    )
    .await;
    assert_eq!(staff_cannot_browse.status(), StatusCode::FORBIDDEN);

    let report_queue = request(
        &app,
        Method::GET,
        "/api/v2/admin/dm/reports?status=open&limit=20",
        &moderator_token,
        None,
    )
    .await;
    assert_eq!(report_queue.status(), StatusCode::OK);
    let report_queue = read_json(report_queue).await;
    assert_eq!(report_queue["items"].as_array().unwrap().len(), 1);
    assert_eq!(report_queue["items"][0]["messageExcerpt"], "hello bob");
    let evidence_audit: (String, String, serde_json::Value) = sqlx::query_as(
        "SELECT target_id, reason, metadata \
         FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'forum.dm_report.evidence_listed' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(moderator_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(evidence_audit.0, "open");
    assert_eq!(evidence_audit.1, "DM report evidence listed");
    assert_eq!(evidence_audit.2, json!({ "count": 1, "status": "open" }));

    let resolve_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/dm/reports/{report_id}/resolve"),
        &moderator_token,
        Some(json!({ "action": "uphold", "note": "confirmed abuse" })),
    )
    .await;
    assert_eq!(resolve_response.status(), StatusCode::OK);
    let resolved_report = read_json(resolve_response).await;
    assert_eq!(resolved_report["status"], "upheld");
    let logged_action: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
           SELECT 1 FROM forum.mod_actions \
           WHERE actor_id = $1 AND target_type = 'dm_report' AND target_id = $2 \
         )",
    )
    .bind(moderator_id)
    .bind(report_id.parse::<i64>().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(logged_action);
    let governance_action: (String, serde_json::Value) = sqlx::query_as(
        "SELECT reason, metadata \
         FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'forum.dm_report.resolved' \
           AND target_type = 'dm_report' AND target_id = $2 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(moderator_id)
    .bind(report_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(governance_action.0, "confirmed abuse");
    assert_eq!(governance_action.1, json!({ "decision": "uphold" }));

    sqlx::query("INSERT INTO forum.user_ignores (account_id, ignored_account_id) VALUES ($1, $2)")
        .bind(bob_id)
        .bind(alice_id)
        .execute(&pool)
        .await
        .unwrap();
    let blocked_send = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/messages"),
        &alice_token,
        Some(json!({ "body": "this must be blocked" })),
    )
    .await;
    assert_eq!(blocked_send.status(), StatusCode::FORBIDDEN);

    let oversized_send = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/messages"),
        &alice_token,
        Some(json!({ "body": "x".repeat(16001) })),
    )
    .await;
    assert_eq!(oversized_send.status(), StatusCode::BAD_REQUEST);
}

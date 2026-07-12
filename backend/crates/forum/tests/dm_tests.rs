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

async fn request_with_idempotency(
    app: &Router,
    uri: &str,
    token: &str,
    idempotency_key: &str,
    body: Value,
) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("Idempotency-Key", idempotency_key)
                .body(Body::from(body.to_string()))
                .expect("build idempotent request"),
        )
        .await
        .unwrap()
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
        "INSERT INTO identity.profiles (account_id, display_name) \
         VALUES ($1, 'Alice Chen'), ($2, 'Bob Li') \
         ON CONFLICT (account_id) DO UPDATE SET display_name = EXCLUDED.display_name",
    )
    .bind(alice_id)
    .bind(bob_id)
    .execute(&pool)
    .await
    .expect("set DM participant display names");

    sqlx::query(
        "UPDATE identity.accounts \
         SET trust_level = 1, \
             role = CASE WHEN id = $2 THEN 'mod'::identity.account_role ELSE role END \
         WHERE id = ANY($1)",
    )
    .bind(vec![alice_id, bob_id, moderator_id])
    .bind(moderator_id)
    .execute(&pool)
    .await
    .expect("prepare DM accounts");
    forum::repo::relationships::follow(&pool, bob_id, alice_id)
        .await
        .expect("recipient permits followed sender to start DM");

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
    assert_eq!(conversation["participantDisplayName"], "Bob Li");
    assert!(conversation["participantAvatarUrl"].is_null());
    assert_eq!(conversation["unreadCount"], 0);
    assert_eq!(conversation["isArchived"], false);
    assert_eq!(conversation["isMuted"], false);
    assert_eq!(conversation["isDeleted"], false);

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
    assert_eq!(sent_message["senderHandle"], "dm-alice");
    assert_eq!(sent_message["senderDisplayName"], "Alice Chen");

    let inbox_response =
        request(&app, Method::GET, "/api/v2/forum/dm/conversations?limit=20", &bob_token, None)
            .await;
    assert_eq!(inbox_response.status(), StatusCode::OK);
    let inbox = read_json(inbox_response).await;
    assert_eq!(inbox["items"].as_array().unwrap().len(), 1);
    assert_eq!(inbox["items"][0]["participantId"], alice_id.to_string());
    assert_eq!(inbox["items"][0]["participantHandle"], "dm-alice");
    assert_eq!(inbox["items"][0]["participantDisplayName"], "Alice Chen");
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
    assert_eq!(report_queue["items"][0]["reporterDisplayName"], "Bob Li");
    assert_eq!(report_queue["items"][0]["senderDisplayName"], "Alice Chen");
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

    let mute_response = request(
        &app,
        Method::PUT,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/mute"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(mute_response.status(), StatusCode::NO_CONTENT);

    let archive_response = request(
        &app,
        Method::PUT,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/archive"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(archive_response.status(), StatusCode::NO_CONTENT);
    let archived_inbox = request(
        &app,
        Method::GET,
        "/api/v2/forum/dm/conversations?view=archived&limit=20",
        &bob_token,
        None,
    )
    .await;
    let archived_inbox = read_json(archived_inbox).await;
    assert_eq!(archived_inbox["items"].as_array().unwrap().len(), 1);
    assert_eq!(archived_inbox["items"][0]["isArchived"], true);
    assert_eq!(archived_inbox["items"][0]["isMuted"], true);

    let wake_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/messages"),
        &alice_token,
        Some(json!({ "body": "wake archived conversation" })),
    )
    .await;
    assert_eq!(wake_response.status(), StatusCode::CREATED);
    let wake_message = read_json(wake_response).await;
    let wake_message_id = wake_message["id"].as_str().unwrap();

    let search_inbox = request(
        &app,
        Method::GET,
        "/api/v2/forum/dm/conversations?q=wake&limit=20",
        &bob_token,
        None,
    )
    .await;
    let search_inbox = read_json(search_inbox).await;
    assert_eq!(search_inbox["items"].as_array().unwrap().len(), 1);
    assert_eq!(search_inbox["items"][0]["isArchived"], false);
    assert_eq!(search_inbox["items"][0]["isMuted"], true);

    let unread_response =
        request(&app, Method::GET, "/api/v2/forum/dm/unread-count", &bob_token, None).await;
    assert_eq!(unread_response.status(), StatusCode::OK);
    assert_eq!(read_json(unread_response).await["count"], 1);

    let delete_response = request(
        &app,
        Method::DELETE,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
    let deleted_inbox = request(
        &app,
        Method::GET,
        "/api/v2/forum/dm/conversations?view=deleted&limit=20",
        &bob_token,
        None,
    )
    .await;
    let deleted_inbox = read_json(deleted_inbox).await;
    assert_eq!(deleted_inbox["items"].as_array().unwrap().len(), 1);
    assert_eq!(deleted_inbox["items"][0]["isDeleted"], true);
    let hidden_messages = request(
        &app,
        Method::GET,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/messages"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(hidden_messages.status(), StatusCode::FORBIDDEN);

    let recover_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/recover"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(recover_response.status(), StatusCode::NO_CONTENT);
    let read_wake_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{conversation_id}/read"),
        &bob_token,
        Some(json!({ "lastReadMessageId": wake_message_id })),
    )
    .await;
    assert_eq!(read_wake_response.status(), StatusCode::NO_CONTENT);

    let invalid_view =
        request(&app, Method::GET, "/api/v2/forum/dm/conversations?view=unknown", &bob_token, None)
            .await;
    assert_eq!(invalid_view.status(), StatusCode::BAD_REQUEST);

    forum::repo::relationships::block(&pool, bob_id, alice_id).await.expect("block DM sender");
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

    let (request_sender_id, request_sender_token) =
        create_test_account(&pool, "dm-request-sender@tongji.edu.cn", "request-sender").await;
    let (request_recipient_id, request_recipient_token) =
        create_test_account(&pool, "dm-request-recipient@tongji.edu.cn", "request-recipient").await;
    let (other_recipient_id, _) =
        create_test_account(&pool, "dm-request-other@tongji.edu.cn", "request-other").await;
    sqlx::query("UPDATE identity.accounts SET trust_level = 1 WHERE id = $1")
        .bind(request_sender_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, dm_policy) \
         SELECT account_id, 'everyone' FROM unnest($1::bigint[]) AS account_id \
         ON CONFLICT (account_id) DO UPDATE SET dm_policy = EXCLUDED.dm_policy",
    )
    .bind(vec![request_recipient_id, other_recipient_id])
    .execute(&pool)
    .await
    .unwrap();

    let request_body = json!({
        "recipientHandle": "request-recipient",
        "requestMessage": "你好，想请教一下课程资料"
    });
    let create_request = request_with_idempotency(
        &app,
        "/api/v2/forum/dm/conversations",
        &request_sender_token,
        "dm-request-create-1",
        request_body.clone(),
    )
    .await;
    assert_eq!(create_request.status(), StatusCode::OK);
    let pending = read_json(create_request).await;
    let request_conversation_id = pending["id"].as_str().unwrap();
    assert_eq!(pending["requestStatus"], "pending");
    assert_eq!(pending["requestDirection"], "outgoing");
    assert_eq!(pending["canSend"], false);
    let request_notification_payload: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM platform.outbox_events \
         WHERE event_type = 'dm_request' AND payload ->> 'conversationId' = $1",
    )
    .bind(request_conversation_id)
    .fetch_one(&pool)
    .await
    .expect("load request notification lifecycle payload");
    assert!(request_notification_payload["requestedAtMicros"].as_str().is_some());

    let replay = request_with_idempotency(
        &app,
        "/api/v2/forum/dm/conversations",
        &request_sender_token,
        "dm-request-create-1",
        request_body,
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(read_json(replay).await["id"], request_conversation_id);
    let request_message_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM forum.dm_messages WHERE conversation_id = $1")
            .bind(request_conversation_id.parse::<i64>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(request_message_count, 1);
    let database_rejection = sqlx::query(
        "INSERT INTO forum.dm_messages (conversation_id, sender_id, body) \
         VALUES ($1, $2, 'database must reject a second request message')",
    )
    .bind(request_conversation_id.parse::<i64>().unwrap())
    .bind(request_sender_id)
    .execute(&pool)
    .await
    .expect_err("pending request delivery must be database-enforced");
    assert_eq!(
        database_rejection.as_database_error().and_then(|error| error.code()).as_deref(),
        Some("23514")
    );

    let conflicting_replay = request_with_idempotency(
        &app,
        "/api/v2/forum/dm/conversations",
        &request_sender_token,
        "dm-request-create-1",
        json!({
            "recipientHandle": "request-other",
            "requestMessage": "same key, different target"
        }),
    )
    .await;
    assert_eq!(conflicting_replay.status(), StatusCode::CONFLICT);

    let (nobody_recipient_id, _) =
        create_test_account(&pool, "dm-nobody@tongji.edu.cn", "nobody-recipient").await;
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, dm_policy) VALUES ($1, 'nobody') \
         ON CONFLICT (account_id) DO UPDATE SET dm_policy = EXCLUDED.dm_policy",
    )
    .bind(nobody_recipient_id)
    .execute(&pool)
    .await
    .unwrap();
    forum::repo::relationships::follow(&pool, nobody_recipient_id, request_sender_id)
        .await
        .unwrap();
    let nobody_rejects_followed_sender = request(
        &app,
        Method::POST,
        "/api/v2/forum/dm/conversations",
        &request_sender_token,
        Some(json!({
            "recipientHandle": "nobody-recipient",
            "requestMessage": "nobody must remain closed"
        })),
    )
    .await;
    assert_eq!(nobody_rejects_followed_sender.status(), StatusCode::FORBIDDEN);

    let pending_second_message = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{request_conversation_id}/messages"),
        &request_sender_token,
        Some(json!({ "body": "second message must wait" })),
    )
    .await;
    assert_eq!(pending_second_message.status(), StatusCode::FORBIDDEN);

    let recipient_counts =
        request(&app, Method::GET, "/api/v2/forum/dm/unread-count", &request_recipient_token, None)
            .await;
    let recipient_counts = read_json(recipient_counts).await;
    assert_eq!(recipient_counts, json!({ "count": 1, "unreadCount": 0, "requestCount": 1 }));
    let incoming_requests = request(
        &app,
        Method::GET,
        "/api/v2/forum/dm/conversations?view=requests&limit=20",
        &request_recipient_token,
        None,
    )
    .await;
    let incoming_requests = read_json(incoming_requests).await;
    assert_eq!(incoming_requests["items"][0]["requestDirection"], "incoming");
    assert_eq!(incoming_requests["items"][0]["lastMessageExcerpt"], "你好，想请教一下课程资料");
    let sent_requests = request(
        &app,
        Method::GET,
        "/api/v2/forum/dm/conversations?view=sent&limit=20",
        &request_sender_token,
        None,
    )
    .await;
    assert_eq!(read_json(sent_requests).await["items"][0]["requestDirection"], "outgoing");

    let sender_cannot_accept = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/requests/{request_conversation_id}/accept"),
        &request_sender_token,
        None,
    )
    .await;
    assert_eq!(sender_cannot_accept.status(), StatusCode::FORBIDDEN);
    sqlx::query("UPDATE identity.profile_privacy SET dm_policy = 'nobody' WHERE account_id = $1")
        .bind(request_recipient_id)
        .execute(&pool)
        .await
        .unwrap();
    let nobody_cannot_accept = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/requests/{request_conversation_id}/accept"),
        &request_recipient_token,
        None,
    )
    .await;
    assert_eq!(nobody_cannot_accept.status(), StatusCode::FORBIDDEN);
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, dm_policy) VALUES ($1, 'everyone') \
         ON CONFLICT (account_id) DO UPDATE SET dm_policy = EXCLUDED.dm_policy",
    )
    .bind(request_recipient_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason) \
         VALUES ($1, 'suspend', 'request sender suspended')",
    )
    .bind(request_sender_id)
    .execute(&pool)
    .await
    .unwrap();
    let suspended_sender_cannot_be_accepted = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/requests/{request_conversation_id}/accept"),
        &request_recipient_token,
        None,
    )
    .await;
    assert_eq!(suspended_sender_cannot_be_accepted.status(), StatusCode::FORBIDDEN);
    sqlx::query("DELETE FROM identity.sanctions WHERE account_id = $1")
        .bind(request_sender_id)
        .execute(&pool)
        .await
        .unwrap();

    let accept_request = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/requests/{request_conversation_id}/accept"),
        &request_recipient_token,
        None,
    )
    .await;
    assert_eq!(accept_request.status(), StatusCode::OK);
    let accepted = read_json(accept_request).await;
    assert_eq!(accepted["requestStatus"], "accepted");
    assert!(accepted["requestDirection"].is_null());
    assert_eq!(accepted["canSend"], true);
    let acceptance_notification_payload: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM platform.outbox_events \
         WHERE event_type = 'dm_request_accepted' AND payload ->> 'conversationId' = $1",
    )
    .bind(request_conversation_id)
    .fetch_one(&pool)
    .await
    .expect("load acceptance notification lifecycle payload");
    assert_eq!(
        acceptance_notification_payload["requestedAtMicros"],
        request_notification_payload["requestedAtMicros"]
    );
    let accept_replay = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/requests/{request_conversation_id}/accept"),
        &request_recipient_token,
        None,
    )
    .await;
    assert_eq!(accept_replay.status(), StatusCode::OK);
    let counts_after_accept =
        request(&app, Method::GET, "/api/v2/forum/dm/unread-count", &request_recipient_token, None)
            .await;
    assert_eq!(
        read_json(counts_after_accept).await,
        json!({ "count": 0, "unreadCount": 0, "requestCount": 0 })
    );
    let accepted_send = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/conversations/{request_conversation_id}/messages"),
        &request_sender_token,
        Some(json!({ "body": "thanks for accepting" })),
    )
    .await;
    assert_eq!(accepted_send.status(), StatusCode::CREATED);
    let accepted_message = read_json(accepted_send).await;
    let accepted_message_id = accepted_message["id"].as_str().expect("accepted message id");
    let message_notification_payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM platform.outbox_events WHERE source_key = $1")
            .bind(format!("dm-message:{accepted_message_id}"))
            .fetch_one(&pool)
            .await
            .expect("load message notification lifecycle payload");
    assert_eq!(message_notification_payload["messageId"], accepted_message_id);

    let (decline_sender_id, decline_sender_token) =
        create_test_account(&pool, "dm-decline-sender@tongji.edu.cn", "decline-sender").await;
    let (decline_recipient_id, decline_recipient_token) =
        create_test_account(&pool, "dm-decline-recipient@tongji.edu.cn", "decline-recipient").await;
    sqlx::query("UPDATE identity.accounts SET trust_level = 1 WHERE id = $1")
        .bind(decline_sender_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, dm_policy) VALUES ($1, 'everyone') \
         ON CONFLICT (account_id) DO UPDATE SET dm_policy = EXCLUDED.dm_policy",
    )
    .bind(decline_recipient_id)
    .execute(&pool)
    .await
    .unwrap();
    let decline_request = request(
        &app,
        Method::POST,
        "/api/v2/forum/dm/conversations",
        &decline_sender_token,
        Some(json!({
            "recipientHandle": "decline-recipient",
            "requestMessage": "unsolicited request"
        })),
    )
    .await;
    let decline_request = read_json(decline_request).await;
    let decline_conversation_id = decline_request["id"].as_str().unwrap();
    let decline_response = request(
        &app,
        Method::DELETE,
        &format!("/api/v2/forum/dm/requests/{decline_conversation_id}"),
        &decline_recipient_token,
        None,
    )
    .await;
    assert_eq!(decline_response.status(), StatusCode::NO_CONTENT);
    let decline_state: (String, i64) = sqlx::query_as(
        "SELECT request_status, \
                (SELECT COUNT(*) FROM forum.dm_messages WHERE conversation_id = conversation.id) \
         FROM forum.dm_conversations AS conversation WHERE id = $1",
    )
    .bind(decline_conversation_id.parse::<i64>().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(decline_state, ("declined".into(), 0));
    let decline_did_not_block: bool =
        sqlx::query_scalar("SELECT NOT forum.user_pair_blocked($1, $2)")
            .bind(decline_sender_id)
            .bind(decline_recipient_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(decline_did_not_block);
    let cooldown_retry = request(
        &app,
        Method::POST,
        "/api/v2/forum/dm/conversations",
        &decline_sender_token,
        Some(json!({
            "recipientHandle": "decline-recipient",
            "requestMessage": "retry too soon"
        })),
    )
    .await;
    assert_eq!(cooldown_retry.status(), StatusCode::CONFLICT);

    let (report_sender_id, report_sender_token) =
        create_test_account(&pool, "dm-report-sender@tongji.edu.cn", "report-sender").await;
    let (report_recipient_id, report_recipient_token) =
        create_test_account(&pool, "dm-report-recipient@tongji.edu.cn", "report-recipient").await;
    sqlx::query("UPDATE identity.accounts SET trust_level = 1 WHERE id = $1")
        .bind(report_sender_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, dm_policy) VALUES ($1, 'everyone') \
         ON CONFLICT (account_id) DO UPDATE SET dm_policy = EXCLUDED.dm_policy",
    )
    .bind(report_recipient_id)
    .execute(&pool)
    .await
    .unwrap();
    let report_request = request(
        &app,
        Method::POST,
        "/api/v2/forum/dm/conversations",
        &report_sender_token,
        Some(json!({
            "recipientHandle": "report-recipient",
            "requestMessage": "reportable spam evidence"
        })),
    )
    .await;
    let report_request = read_json(report_request).await;
    let report_conversation_id = report_request["id"].as_str().unwrap();
    let report_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/dm/requests/{report_conversation_id}/report"),
        &report_recipient_token,
        Some(json!({ "reason": "spam", "note": "unsolicited advertising" })),
    )
    .await;
    assert_eq!(report_response.status(), StatusCode::ACCEPTED);
    let preserved_evidence: (String, String) = sqlx::query_as(
        "SELECT conversation.request_status, message.body \
         FROM forum.dm_conversations AS conversation \
         JOIN forum.dm_messages AS message ON message.conversation_id = conversation.id \
         JOIN forum.dm_message_reports AS report ON report.message_id = message.id \
         WHERE conversation.id = $1 AND report.reported_by = $2",
    )
    .bind(report_conversation_id.parse::<i64>().unwrap())
    .bind(report_recipient_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(preserved_evidence, ("declined".into(), "reportable spam evidence".into()));

    let (limited_sender_id, limited_sender_token) =
        create_test_account(&pool, "dm-limited@tongji.edu.cn", "limited-sender").await;
    sqlx::query("UPDATE identity.accounts SET trust_level = 1 WHERE id = $1")
        .bind(limited_sender_id)
        .execute(&pool)
        .await
        .unwrap();
    for request_index in 0..11 {
        let recipient_handle = format!("limited-recipient-{request_index}");
        let recipient_email = format!("dm-limited-{request_index}@tongji.edu.cn");
        let (recipient_id, _) =
            create_test_account(&pool, &recipient_email, &recipient_handle).await;
        sqlx::query(
            "INSERT INTO identity.profile_privacy (account_id, dm_policy) \
             VALUES ($1, 'everyone') \
             ON CONFLICT (account_id) DO UPDATE SET dm_policy = EXCLUDED.dm_policy",
        )
        .bind(recipient_id)
        .execute(&pool)
        .await
        .unwrap();
        let response = request(
            &app,
            Method::POST,
            "/api/v2/forum/dm/conversations",
            &limited_sender_token,
            Some(json!({
                "recipientHandle": recipient_handle,
                "requestMessage": "bounded request attempt"
            })),
        )
        .await;
        assert_eq!(
            response.status(),
            if request_index < 10 { StatusCode::OK } else { StatusCode::TOO_MANY_REQUESTS }
        );
    }
}

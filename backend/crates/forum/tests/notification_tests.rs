//! Integration tests for notification filtering, pagination, and read state.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn seed_notification(pool: &PgPool, account_id: i64, is_read: bool) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO forum.notifications (account_id, type, payload, read_at) \
         VALUES ($1, 'reply', '{\"threadId\":\"1\"}'::jsonb, \
                 CASE WHEN $2 THEN now() ELSE NULL END) \
         RETURNING id",
    )
    .bind(account_id)
    .bind(is_read)
    .fetch_one(pool)
    .await
    .expect("seed notification")
}

async fn enqueue_and_deliver(
    pool: &PgPool,
    account_id: i64,
    event_type: &str,
    payload: serde_json::Value,
    actor_id: Option<i64>,
) {
    let event = enqueue_and_claim(pool, account_id, event_type, payload, actor_id, None).await;
    forum::notification_delivery::deliver_event(pool, &event)
        .await
        .expect("deliver notification event");
}

async fn enqueue_and_claim(
    pool: &PgPool,
    account_id: i64,
    event_type: &str,
    payload: serde_json::Value,
    actor_id: Option<i64>,
    aggregation_key: Option<&str>,
) -> platform::outbox::OutboxEvent {
    let mut transaction = pool.begin().await.expect("begin outbox transaction");
    let event_id = platform::outbox::enqueue_notification_tx(
        &mut transaction,
        &format!("notification-test:{}", uuid::Uuid::new_v4()),
        account_id,
        actor_id,
        event_type,
        &payload,
        aggregation_key,
        None,
    )
    .await
    .expect("enqueue notification event");
    transaction.commit().await.expect("commit notification event");
    let worker_id = uuid::Uuid::new_v4();
    sqlx::query_as::<_, platform::outbox::OutboxEvent>(
        "UPDATE platform.outbox_events \
         SET state = 'running', attempts = attempts + 1, claimed_by = $2, \
             lease_expires_at = now() + interval '30 seconds', updated_at = now() \
         WHERE id = $1 AND state = 'queued' \
         RETURNING id, topic, source_key, recipient_account_id, actor_account_id, event_type, \
                   payload, aggregation_key, attempts, max_attempts, available_at, claimed_by, \
                   lease_expires_at",
    )
    .bind(event_id)
    .bind(worker_id)
    .fetch_one(pool)
    .await
    .expect("claim matching notification event")
}

async fn receipt_outcome(pool: &PgPool, event_id: i64) -> String {
    sqlx::query_scalar(
        "SELECT outcome FROM forum.notification_delivery_receipts WHERE outbox_event_id = $1",
    )
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("load notification delivery receipt")
}

async fn list_notifications(
    app: &axum::Router,
    token: &str,
    query: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v2/notifications?{query}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build notification list request"),
        )
        .await
        .expect("notification list response")
}

async fn mark_notifications_read(
    app: &axum::Router,
    token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/notifications/read")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("build mark-read request"),
        )
        .await
        .expect("mark-read response")
}

async fn notification_preferences(
    app: &axum::Router,
    token: &str,
    method: Method,
    body: Option<serde_json::Value>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri("/api/v2/me/notification-prefs")
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let body = match body {
        Some(value) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(request.body(body).expect("build notification preference request"))
        .await
        .expect("notification preference response")
}

#[tokio::test]
async fn unread_filter_paginates_without_skipping_the_lookahead_row() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "notify-page@tongji.edu.cn", "notify-page").await;

    let oldest_unread_id = seed_notification(&pool, account_id, false).await;
    seed_notification(&pool, account_id, true).await;
    let middle_unread_id = seed_notification(&pool, account_id, false).await;
    let newest_unread_id = seed_notification(&pool, account_id, false).await;

    let first_response = list_notifications(&app, &token, "unread=true&limit=2").await;
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_page = read_json(first_response).await;
    assert_eq!(first_page["items"][0]["id"], newest_unread_id.to_string());
    assert_eq!(first_page["items"][1]["id"], middle_unread_id.to_string());
    assert_eq!(first_page["nextCursor"], middle_unread_id.to_string());
    assert_eq!(first_page["hasMore"], true);
    assert_eq!(first_page["items"][0]["targetUrl"], "/forum/threads/1");

    let second_response =
        list_notifications(&app, &token, &format!("unread=true&limit=2&cursor={middle_unread_id}"))
            .await;
    assert_eq!(second_response.status(), StatusCode::OK);
    let second_page = read_json(second_response).await;
    assert_eq!(second_page["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(second_page["items"][0]["id"], oldest_unread_id.to_string());
    assert_eq!(second_page["hasMore"], false);
}

#[tokio::test]
async fn mark_read_is_scoped_to_the_authenticated_account() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "notify-owner@tongji.edu.cn", "notify-owner").await;
    let (other_account_id, _) =
        create_test_account(&pool, "notify-other@tongji.edu.cn", "notify-other").await;
    let selected_id = seed_notification(&pool, account_id, false).await;
    let remaining_id = seed_notification(&pool, account_id, false).await;
    let foreign_id = seed_notification(&pool, other_account_id, false).await;

    let selected_response = mark_notifications_read(
        &app,
        &token,
        json!({ "ids": [selected_id.to_string(), foreign_id.to_string()] }),
    )
    .await;
    assert_eq!(selected_response.status(), StatusCode::NO_CONTENT);

    let selected_is_read: bool =
        sqlx::query_scalar("SELECT read_at IS NOT NULL FROM forum.notifications WHERE id = $1")
            .bind(selected_id)
            .fetch_one(&pool)
            .await
            .expect("selected read state");
    let remaining_is_read: bool =
        sqlx::query_scalar("SELECT read_at IS NOT NULL FROM forum.notifications WHERE id = $1")
            .bind(remaining_id)
            .fetch_one(&pool)
            .await
            .expect("remaining read state");
    let foreign_is_read: bool =
        sqlx::query_scalar("SELECT read_at IS NOT NULL FROM forum.notifications WHERE id = $1")
            .bind(foreign_id)
            .fetch_one(&pool)
            .await
            .expect("foreign read state");
    assert!(selected_is_read);
    assert!(!remaining_is_read);
    assert!(!foreign_is_read);

    let all_response = mark_notifications_read(&app, &token, json!({ "all": true })).await;
    assert_eq!(all_response.status(), StatusCode::NO_CONTENT);
    let account_unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("owner unread count");
    let foreign_unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(other_account_id)
    .fetch_one(&pool)
    .await
    .expect("foreign unread count");
    assert_eq!(account_unread, 0);
    assert_eq!(foreign_unread, 1);
}

#[tokio::test]
async fn rejects_invalid_list_limits_and_ambiguous_mark_read_input() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "notify-input@tongji.edu.cn", "notify-input").await;

    assert_eq!(list_notifications(&app, &token, "limit=0").await.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        list_notifications(&app, &token, "limit=101").await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        mark_notifications_read(&app, &token, json!({ "ids": [] })).await.status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        mark_notifications_read(&app, &token, json!({ "ids": ["1"], "all": true })).await.status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn notification_preferences_are_typed_persisted_and_enforced() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "notify-prefs@tongji.edu.cn", "notify-prefs").await;

    let default_response = notification_preferences(&app, &token, Method::GET, None).await;
    assert_eq!(default_response.status(), StatusCode::OK);
    let default_body = read_json(default_response).await;
    assert_eq!(default_body["prefs"]["inApp"]["replies"], true);
    assert_eq!(default_body["prefs"]["inApp"]["follows"], true);
    assert_eq!(default_body["prefs"]["inApp"]["directMessages"], true);
    assert_eq!(default_body["prefs"]["email"]["weeklyDigest"], false);

    let updated_preferences = json!({
        "prefs": {
            "inApp": {
                "replies": false,
                "mentions": true,
                "quotes": true,
                "votes": true,
                "badges": true,
                "subscriptions": true,
                "follows": false,
                "directMessages": true
            },
            "email": { "weeklyDigest": true }
        }
    });
    let update_response =
        notification_preferences(&app, &token, Method::PUT, Some(updated_preferences.clone()))
            .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    assert_eq!(read_json(update_response).await, updated_preferences);

    enqueue_and_deliver(&pool, account_id, "reply", json!({ "threadId": "1" }), None).await;
    enqueue_and_deliver(&pool, account_id, "content_moderated", json!({}), None).await;
    let event_types: Vec<String> = sqlx::query_scalar(
        "SELECT type FROM forum.notifications WHERE account_id = $1 ORDER BY id",
    )
    .bind(account_id)
    .fetch_all(&pool)
    .await
    .expect("load notification event types");
    assert_eq!(event_types, vec!["content_moderated"]);

    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT prefs FROM forum.notification_prefs WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("load stored notification preferences");
    assert_eq!(stored, updated_preferences["prefs"]);
}

#[tokio::test]
async fn durable_delivery_is_idempotent_aggregated_and_adds_profile_targets() {
    let (pool, _) = create_test_app().await;
    let (recipient_id, _) =
        create_test_account(&pool, "notify-durable@tongji.edu.cn", "notify-durable").await;
    let (first_actor_id, _) =
        create_test_account(&pool, "notify-actor-a@tongji.edu.cn", "notify-actor-a").await;
    let (second_actor_id, _) =
        create_test_account(&pool, "notify-actor-b@tongji.edu.cn", "notify-actor-b").await;
    let vote_thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, 'Aggregate vote source', 'Visible body') RETURNING id",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("seed aggregate vote thread");

    let mut verification = enqueue_and_claim(
        &pool,
        recipient_id,
        "verification_granted",
        json!({ "title": "学院认证已通过" }),
        None,
        None,
    )
    .await;
    forum::notification_delivery::deliver_event(&pool, &verification)
        .await
        .expect("deliver verification notification");
    let profile_target: String = sqlx::query_scalar(
        "SELECT payload ->> 'targetUrl' FROM forum.notifications WHERE account_id = $1",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("load verification target");
    assert_eq!(profile_target, "/profile/notify-durable");

    let replay_worker = uuid::Uuid::new_v4();
    sqlx::query(
        "UPDATE platform.outbox_events \
         SET state = 'running', completed_at = NULL, claimed_by = $2, \
             lease_expires_at = now() + interval '30 seconds' WHERE id = $1",
    )
    .bind(verification.id)
    .bind(replay_worker)
    .execute(&pool)
    .await
    .expect("simulate redelivery lease");
    verification.claimed_by = replay_worker;
    forum::notification_delivery::deliver_event(&pool, &verification)
        .await
        .expect("idempotent redelivery");
    let verification_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications \
         WHERE account_id = $1 AND type = 'verification_granted'",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("count verification notifications");
    assert_eq!(verification_count, 1);

    let first_vote_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO forum.votes (post_type, post_id, account_id, value) \
         VALUES ('thread', $1, $2, 1) RETURNING updated_at",
    )
    .bind(vote_thread_id)
    .bind(first_actor_id)
    .fetch_one(&pool)
    .await
    .expect("seed first aggregate vote");
    let second_vote_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO forum.votes (post_type, post_id, account_id, value) \
         VALUES ('thread', $1, $2, 1) RETURNING updated_at",
    )
    .bind(vote_thread_id)
    .bind(second_actor_id)
    .fetch_one(&pool)
    .await
    .expect("seed second aggregate vote");
    let first = enqueue_and_claim(
        &pool,
        recipient_id,
        "vote",
        json!({
            "postType": "thread",
            "postId": vote_thread_id.to_string(),
            "threadId": vote_thread_id.to_string(),
            "voterId": first_actor_id.to_string(),
            "voteUpdatedAtMicros": first_vote_at.timestamp_micros().to_string(),
            "title": "帖子获得赞同"
        }),
        Some(first_actor_id),
        Some("vote:thread:1"),
    )
    .await;
    let second = enqueue_and_claim(
        &pool,
        recipient_id,
        "vote",
        json!({
            "postType": "thread",
            "postId": vote_thread_id.to_string(),
            "threadId": vote_thread_id.to_string(),
            "voterId": second_actor_id.to_string(),
            "voteUpdatedAtMicros": second_vote_at.timestamp_micros().to_string(),
            "title": "帖子获得赞同"
        }),
        Some(second_actor_id),
        Some("vote:thread:1"),
    )
    .await;
    let first_delivery = forum::notification_delivery::deliver_event(&pool, &first);
    let second_delivery = forum::notification_delivery::deliver_event(&pool, &second);
    let (first_result, second_result) = tokio::join!(first_delivery, second_delivery);
    first_result.expect("deliver first aggregate");
    second_result.expect("deliver second aggregate");
    let aggregate: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX((payload ->> 'count')::bigint), 1) \
         FROM forum.notifications WHERE account_id = $1 AND type = 'vote'",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("load aggregated notification");
    assert_eq!(aggregate, (1, 2));
}

#[tokio::test]
async fn stale_worker_is_fenced_before_notification_side_effects() {
    let (pool, _) = create_test_app().await;
    let (recipient_id, _) =
        create_test_account(&pool, "notify-fence@tongji.edu.cn", "notify-fence").await;
    let (actor_id, _) =
        create_test_account(&pool, "notify-fence-actor@tongji.edu.cn", "notify-fence-actor").await;
    let followed_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO forum.user_follows (follower_id, followed_id) VALUES ($1, $2) \
         RETURNING created_at",
    )
    .bind(actor_id)
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("seed fenced follow source");
    let stale_event = enqueue_and_claim(
        &pool,
        recipient_id,
        "follow",
        json!({
            "followedAtMicros": followed_at.timestamp_micros().to_string(),
            "title": "新的关注"
        }),
        Some(actor_id),
        None,
    )
    .await;
    let replacement_worker = uuid::Uuid::new_v4();
    sqlx::query(
        "UPDATE platform.outbox_events \
         SET claimed_by = $2, lease_expires_at = now() + interval '30 seconds' WHERE id = $1",
    )
    .bind(stale_event.id)
    .bind(replacement_worker)
    .execute(&pool)
    .await
    .expect("replace expired worker claim");

    assert!(forum::notification_delivery::deliver_event(&pool, &stale_event)
        .await
        .expect("fence stale worker")
        .is_none());
    let side_effect_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND type = 'follow'",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("count stale-worker side effects");
    assert_eq!(side_effect_count, 0);

    let mut replacement_event = stale_event;
    replacement_event.claimed_by = replacement_worker;
    forum::notification_delivery::deliver_event(&pool, &replacement_event)
        .await
        .expect("deliver replacement worker event");
    let side_effect_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications WHERE account_id = $1 AND type = 'follow'",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("count replacement-worker side effects");
    assert_eq!(side_effect_count, 1);
}

#[tokio::test]
async fn delivery_suppresses_follow_and_vote_after_the_source_is_reversed() {
    let (pool, _) = create_test_app().await;
    let (recipient_id, _) =
        create_test_account(&pool, "notify-reverse@tongji.edu.cn", "notify-reverse").await;
    let (actor_id, _) =
        create_test_account(&pool, "notify-reverse-actor@tongji.edu.cn", "notify-reverse-actor")
            .await;

    let followed_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO forum.user_follows (follower_id, followed_id) VALUES ($1, $2) \
         RETURNING created_at",
    )
    .bind(actor_id)
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("seed reversible follow");
    let follow = enqueue_and_claim(
        &pool,
        recipient_id,
        "follow",
        json!({ "followedAtMicros": followed_at.timestamp_micros().to_string() }),
        Some(actor_id),
        None,
    )
    .await;
    let follow_id = follow.id;
    let mut unfollow = pool.begin().await.expect("begin concurrent unfollow");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("forum-social:{}:{}", recipient_id.min(actor_id), recipient_id.max(actor_id)))
        .execute(&mut *unfollow)
        .await
        .expect("lock follow pair");
    sqlx::query("DELETE FROM forum.user_follows WHERE follower_id = $1 AND followed_id = $2")
        .bind(actor_id)
        .bind(recipient_id)
        .execute(&mut *unfollow)
        .await
        .expect("reverse follow");
    let delivery_pool = pool.clone();
    let follow_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &follow).await
    });
    tokio::task::yield_now().await;
    unfollow.commit().await.expect("commit concurrent unfollow");
    follow_delivery.await.expect("join stale follow delivery").expect("suppress stale follow");
    assert_eq!(receipt_outcome(&pool, follow_id).await, "content_unavailable");

    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, 'Reversible vote source', 'Visible body') RETURNING id",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("seed reversible vote thread");
    let vote_updated_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "INSERT INTO forum.votes (post_type, post_id, account_id, value) \
         VALUES ('thread', $1, $2, 1) RETURNING updated_at",
    )
    .bind(thread_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("seed reversible vote");
    let vote = enqueue_and_claim(
        &pool,
        recipient_id,
        "vote",
        json!({
            "postType": "thread",
            "postId": thread_id.to_string(),
            "threadId": thread_id.to_string(),
            "voterId": actor_id.to_string(),
            "voteUpdatedAtMicros": vote_updated_at.timestamp_micros().to_string()
        }),
        Some(actor_id),
        None,
    )
    .await;
    let vote_id = vote.id;
    let mut unvote = pool.begin().await.expect("begin concurrent unvote");
    sqlx::query(
        "SELECT value FROM forum.votes \
         WHERE post_type = 'thread' AND post_id = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(thread_id)
    .bind(actor_id)
    .execute(&mut *unvote)
    .await
    .expect("lock vote source");
    sqlx::query(
        "DELETE FROM forum.votes \
         WHERE post_type = 'thread' AND post_id = $1 AND account_id = $2",
    )
    .bind(thread_id)
    .bind(actor_id)
    .execute(&mut *unvote)
    .await
    .expect("reverse vote");
    let delivery_pool = pool.clone();
    let vote_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &vote).await
    });
    tokio::task::yield_now().await;
    unvote.commit().await.expect("commit concurrent unvote");
    vote_delivery.await.expect("join stale vote delivery").expect("suppress stale vote");
    assert_eq!(receipt_outcome(&pool, vote_id).await, "content_unavailable");

    sqlx::query(
        "INSERT INTO forum.subscriptions (account_id, target_type, target_id, level) \
         VALUES ($1, 'thread', $2, 'watching')",
    )
    .bind(recipient_id)
    .bind(thread_id)
    .execute(&pool)
    .await
    .expect("seed reversible watching subscription");
    let watching = enqueue_and_claim(
        &pool,
        recipient_id,
        "watching",
        json!({ "threadId": thread_id.to_string() }),
        Some(actor_id),
        None,
    )
    .await;
    let watching_id = watching.id;
    let mut unwatch = pool.begin().await.expect("begin concurrent unwatch");
    forum::repo::subscriptions::lock_account_subscriptions(&mut unwatch, recipient_id)
        .await
        .expect("lock effective subscription");
    sqlx::query(
        "UPDATE forum.subscriptions SET level = 'muted' \
         WHERE account_id = $1 AND target_type = 'thread' AND target_id = $2",
    )
    .bind(recipient_id)
    .bind(thread_id)
    .execute(&mut *unwatch)
    .await
    .expect("reverse watching subscription");
    let delivery_pool = pool.clone();
    let watching_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &watching).await
    });
    tokio::task::yield_now().await;
    unwatch.commit().await.expect("commit concurrent unwatch");
    watching_delivery
        .await
        .expect("join stale watching delivery")
        .expect("suppress stale watching event");
    assert_eq!(receipt_outcome(&pool, watching_id).await, "content_unavailable");

    let delivered: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications \
         WHERE account_id = $1 AND type IN ('follow', 'vote', 'watching')",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("count reversed interaction notifications");
    assert_eq!(delivered, 0);
}

#[tokio::test]
async fn delivery_suppresses_stale_dm_request_acceptance_and_message() {
    let (pool, _) = create_test_app().await;
    let (sender_id, _) =
        create_test_account(&pool, "notify-dm-sender@tongji.edu.cn", "notify-dm-sender").await;
    let (recipient_id, _) =
        create_test_account(&pool, "notify-dm-recipient@tongji.edu.cn", "notify-dm-recipient")
            .await;
    let (accepted_sender_id, _) = create_test_account(
        &pool,
        "notify-dm-accepted-sender@tongji.edu.cn",
        "notify-dm-accepted-sender",
    )
    .await;
    let (accepted_recipient_id, _) = create_test_account(
        &pool,
        "notify-dm-accepted-recipient@tongji.edu.cn",
        "notify-dm-accepted-recipient",
    )
    .await;
    let (message_sender_id, _) = create_test_account(
        &pool,
        "notify-dm-message-sender@tongji.edu.cn",
        "notify-dm-message-sender",
    )
    .await;
    let (message_recipient_id, _) = create_test_account(
        &pool,
        "notify-dm-message-recipient@tongji.edu.cn",
        "notify-dm-message-recipient",
    )
    .await;
    let account_low_id = sender_id.min(recipient_id);
    let account_high_id = sender_id.max(recipient_id);

    let (request_conversation_id, requested_at): (i64, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "INSERT INTO forum.dm_conversations \
             (account_low_id, account_high_id, request_status, request_sender_id, \
              request_recipient_id, requested_at) \
             VALUES ($1, $2, 'pending', $3, $4, now()) RETURNING id, requested_at",
        )
        .bind(account_low_id)
        .bind(account_high_id)
        .bind(sender_id)
        .bind(recipient_id)
        .fetch_one(&pool)
        .await
        .expect("seed pending request conversation");
    sqlx::query(
        "INSERT INTO forum.dm_participants (conversation_id, account_id) \
         VALUES ($1, $2), ($1, $3)",
    )
    .bind(request_conversation_id)
    .bind(sender_id)
    .bind(recipient_id)
    .execute(&pool)
    .await
    .expect("seed request participants");
    let request_event = enqueue_and_claim(
        &pool,
        recipient_id,
        "dm_request",
        json!({
            "conversationId": request_conversation_id.to_string(),
            "requestedAtMicros": requested_at.timestamp_micros().to_string()
        }),
        Some(sender_id),
        None,
    )
    .await;
    let request_event_id = request_event.id;
    let mut decline = pool.begin().await.expect("begin concurrent request decline");
    sqlx::query("SELECT id FROM forum.dm_conversations WHERE id = $1 FOR UPDATE")
        .bind(request_conversation_id)
        .execute(&mut *decline)
        .await
        .expect("lock pending request");
    sqlx::query("UPDATE forum.dm_conversations SET request_status = 'declined' WHERE id = $1")
        .bind(request_conversation_id)
        .execute(&mut *decline)
        .await
        .expect("decline pending request");
    let delivery_pool = pool.clone();
    let request_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &request_event).await
    });
    tokio::task::yield_now().await;
    decline.commit().await.expect("commit request decline");
    request_delivery.await.expect("join stale request delivery").expect("suppress stale request");
    assert_eq!(receipt_outcome(&pool, request_event_id).await, "content_unavailable");

    let accepted_low_id = accepted_sender_id.min(accepted_recipient_id);
    let accepted_high_id = accepted_sender_id.max(accepted_recipient_id);
    let (accepted_conversation_id, accepted_requested_at): (i64, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "INSERT INTO forum.dm_conversations \
         (account_low_id, account_high_id, request_status, request_sender_id, \
          request_recipient_id, requested_at, responded_at) \
         VALUES ($1, $2, 'accepted', $3, $4, now(), now()) RETURNING id, requested_at",
        )
        .bind(accepted_low_id)
        .bind(accepted_high_id)
        .bind(accepted_sender_id)
        .bind(accepted_recipient_id)
        .fetch_one(&pool)
        .await
        .expect("seed accepted request conversation");
    sqlx::query(
        "INSERT INTO forum.dm_participants (conversation_id, account_id) \
         VALUES ($1, $2), ($1, $3)",
    )
    .bind(accepted_conversation_id)
    .bind(accepted_sender_id)
    .bind(accepted_recipient_id)
    .execute(&pool)
    .await
    .expect("seed accepted participants");
    let accepted_event = enqueue_and_claim(
        &pool,
        accepted_sender_id,
        "dm_request_accepted",
        json!({
            "conversationId": accepted_conversation_id.to_string(),
            "requestedAtMicros": accepted_requested_at.timestamp_micros().to_string()
        }),
        Some(accepted_recipient_id),
        None,
    )
    .await;
    let accepted_event_id = accepted_event.id;
    let mut acceptance_reversal = pool.begin().await.expect("begin acceptance state change");
    sqlx::query("SELECT id FROM forum.dm_conversations WHERE id = $1 FOR UPDATE")
        .bind(accepted_conversation_id)
        .execute(&mut *acceptance_reversal)
        .await
        .expect("lock accepted request");
    sqlx::query("UPDATE forum.dm_conversations SET request_status = 'declined' WHERE id = $1")
        .bind(accepted_conversation_id)
        .execute(&mut *acceptance_reversal)
        .await
        .expect("change accepted request state");
    let delivery_pool = pool.clone();
    let accepted_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &accepted_event).await
    });
    tokio::task::yield_now().await;
    acceptance_reversal.commit().await.expect("commit acceptance state change");
    accepted_delivery
        .await
        .expect("join stale acceptance delivery")
        .expect("suppress stale acceptance");
    assert_eq!(receipt_outcome(&pool, accepted_event_id).await, "content_unavailable");

    let message_low_id = message_sender_id.min(message_recipient_id);
    let message_high_id = message_sender_id.max(message_recipient_id);
    let message_conversation_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.dm_conversations (account_low_id, account_high_id, request_status) \
         VALUES ($1, $2, 'accepted') RETURNING id",
    )
    .bind(message_low_id)
    .bind(message_high_id)
    .fetch_one(&pool)
    .await
    .expect("seed accepted message conversation");
    sqlx::query(
        "INSERT INTO forum.dm_participants (conversation_id, account_id) \
         VALUES ($1, $2), ($1, $3)",
    )
    .bind(message_conversation_id)
    .bind(message_sender_id)
    .bind(message_recipient_id)
    .execute(&pool)
    .await
    .expect("seed message participants");
    let message_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.dm_messages (conversation_id, sender_id, body) \
         VALUES ($1, $2, 'message removed before notification') RETURNING id",
    )
    .bind(message_conversation_id)
    .bind(message_sender_id)
    .fetch_one(&pool)
    .await
    .expect("seed removable message");
    let message_event = enqueue_and_claim(
        &pool,
        message_recipient_id,
        "dm",
        json!({
            "conversationId": message_conversation_id.to_string(),
            "messageId": message_id.to_string()
        }),
        Some(message_sender_id),
        None,
    )
    .await;
    let message_event_id = message_event.id;
    let mut delete_message = pool.begin().await.expect("begin concurrent message delete");
    sqlx::query("SELECT id FROM forum.dm_messages WHERE id = $1 FOR UPDATE")
        .bind(message_id)
        .execute(&mut *delete_message)
        .await
        .expect("lock message source");
    sqlx::query("DELETE FROM forum.dm_messages WHERE id = $1")
        .bind(message_id)
        .execute(&mut *delete_message)
        .await
        .expect("delete message source");
    let delivery_pool = pool.clone();
    let message_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &message_event).await
    });
    tokio::task::yield_now().await;
    delete_message.commit().await.expect("commit message delete");
    message_delivery.await.expect("join stale message delivery").expect("suppress stale message");
    assert_eq!(receipt_outcome(&pool, message_event_id).await, "content_unavailable");

    let delivered: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.notifications \
         WHERE account_id = ANY($1) \
           AND type IN ('dm_request', 'dm_request_accepted', 'dm')",
    )
    .bind(vec![recipient_id, accepted_sender_id, message_recipient_id])
    .fetch_one(&pool)
    .await
    .expect("count stale DM notifications");
    assert_eq!(delivered, 0);
}

#[tokio::test]
async fn delivery_rechecks_privacy_relationship_lifecycle_and_content_at_commit_time() {
    let (pool, _) = create_test_app().await;
    let (recipient_id, _) =
        create_test_account(&pool, "notify-policy@tongji.edu.cn", "notify-policy").await;
    let (actor_id, _) =
        create_test_account(&pool, "notify-policy-actor@tongji.edu.cn", "notify-policy-actor")
            .await;
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body) \
         VALUES (1, $1, 'Notification policy source', 'Visible body') RETURNING id",
    )
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("seed notification thread");
    let comment_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.comments (thread_id, author_id, body, path) \
         VALUES ($1, $2, 'Visible comment', '0001') RETURNING id",
    )
    .bind(thread_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("seed notification comment");

    let mention = enqueue_and_claim(
        &pool,
        recipient_id,
        "mention",
        json!({ "threadId": thread_id.to_string(), "commentId": comment_id.to_string() }),
        Some(actor_id),
        None,
    )
    .await;
    let mut privacy_change = pool.begin().await.expect("begin privacy change");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("identity-profile-privacy:{recipient_id}"))
        .execute(&mut *privacy_change)
        .await
        .expect("lock mention privacy");
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, mention_policy) VALUES ($1, 'nobody') \
         ON CONFLICT (account_id) DO UPDATE SET mention_policy = EXCLUDED.mention_policy",
    )
    .bind(recipient_id)
    .execute(&mut *privacy_change)
    .await
    .expect("tighten mention privacy");
    let delivery_pool = pool.clone();
    let mention_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &mention).await
    });
    tokio::task::yield_now().await;
    privacy_change.commit().await.expect("commit mention privacy");
    mention_delivery.await.expect("join mention delivery").expect("suppress mention delivery");
    let mention_outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM forum.notification_delivery_receipts \
         WHERE outbox_event_id = (SELECT id FROM platform.outbox_events \
           WHERE recipient_account_id = $1 AND event_type = 'mention' ORDER BY id DESC LIMIT 1)",
    )
    .bind(recipient_id)
    .fetch_one(&pool)
    .await
    .expect("load mention outcome");
    assert_eq!(mention_outcome, "mention_disallowed");

    let reply = enqueue_and_claim(
        &pool,
        recipient_id,
        "reply",
        json!({ "threadId": thread_id.to_string(), "commentId": comment_id.to_string() }),
        Some(actor_id),
        None,
    )
    .await;
    let reply_id = reply.id;
    let mut moderation = pool.begin().await.expect("begin content moderation");
    sqlx::query("SELECT id FROM forum.comments WHERE id = $1 FOR UPDATE")
        .bind(comment_id)
        .execute(&mut *moderation)
        .await
        .expect("lock source comment");
    sqlx::query("UPDATE forum.comments SET hidden_at = now() WHERE id = $1")
        .bind(comment_id)
        .execute(&mut *moderation)
        .await
        .expect("hide source comment");
    let delivery_pool = pool.clone();
    let reply_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &reply).await
    });
    tokio::task::yield_now().await;
    moderation.commit().await.expect("commit content moderation");
    reply_delivery.await.expect("join reply delivery").expect("suppress hidden content delivery");
    assert_eq!(receipt_outcome(&pool, reply_id).await, "content_unavailable");

    let muted_follow =
        enqueue_and_claim(&pool, recipient_id, "follow", json!({}), Some(actor_id), None).await;
    let muted_follow_id = muted_follow.id;
    let mut mute_change = pool.begin().await.expect("begin mute change");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("forum-social:{}:{}", recipient_id.min(actor_id), recipient_id.max(actor_id)))
        .execute(&mut *mute_change)
        .await
        .expect("lock notification pair for mute");
    sqlx::query(
        "INSERT INTO forum.user_mutes (account_id, muted_account_id) VALUES ($1, $2) \
         ON CONFLICT (account_id, muted_account_id) DO NOTHING",
    )
    .bind(recipient_id)
    .bind(actor_id)
    .execute(&mut *mute_change)
    .await
    .expect("mute queued notification actor");
    let delivery_pool = pool.clone();
    let muted_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &muted_follow).await
    });
    tokio::task::yield_now().await;
    mute_change.commit().await.expect("commit mute change");
    muted_delivery.await.expect("join muted delivery").expect("suppress muted actor delivery");
    assert_eq!(receipt_outcome(&pool, muted_follow_id).await, "relationship_hidden");

    forum::repo::relationships::unmute(&pool, recipient_id, actor_id)
        .await
        .expect("clear mute before block race");
    let blocked_vote =
        enqueue_and_claim(&pool, recipient_id, "vote", json!({}), Some(actor_id), None).await;
    let blocked_vote_id = blocked_vote.id;
    let mut block_change = pool.begin().await.expect("begin block change");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("forum-social:{}:{}", recipient_id.min(actor_id), recipient_id.max(actor_id)))
        .execute(&mut *block_change)
        .await
        .expect("lock notification pair for block");
    sqlx::query(
        "INSERT INTO forum.user_ignores (account_id, ignored_account_id) VALUES ($1, $2) \
         ON CONFLICT (account_id, ignored_account_id) DO NOTHING",
    )
    .bind(recipient_id)
    .bind(actor_id)
    .execute(&mut *block_change)
    .await
    .expect("block queued notification actor");
    let delivery_pool = pool.clone();
    let blocked_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &blocked_vote).await
    });
    tokio::task::yield_now().await;
    block_change.commit().await.expect("commit block change");
    blocked_delivery
        .await
        .expect("join blocked delivery")
        .expect("suppress blocked actor delivery");
    assert_eq!(receipt_outcome(&pool, blocked_vote_id).await, "relationship_hidden");

    let suspended_actor = enqueue_and_claim(
        &pool,
        recipient_id,
        "reply",
        json!({ "title": "reply from a newly suspended actor" }),
        Some(actor_id),
        None,
    )
    .await;
    let suspended_actor_id = suspended_actor.id;
    let mut actor_suspension = pool.begin().await.expect("begin actor suspension");
    sqlx::query("SELECT id FROM identity.accounts WHERE id = $1 FOR UPDATE")
        .bind(actor_id)
        .execute(&mut *actor_suspension)
        .await
        .expect("lock notification actor");
    sqlx::query("UPDATE identity.accounts SET status = 'suspended' WHERE id = $1")
        .bind(actor_id)
        .execute(&mut *actor_suspension)
        .await
        .expect("suspend notification actor");
    let delivery_pool = pool.clone();
    let actor_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &suspended_actor).await
    });
    tokio::task::yield_now().await;
    actor_suspension.commit().await.expect("commit actor suspension");
    actor_delivery
        .await
        .expect("join suspended actor delivery")
        .expect("suppress suspended actor delivery");
    assert_eq!(receipt_outcome(&pool, suspended_actor_id).await, "actor_unavailable");

    let suspended = enqueue_and_claim(
        &pool,
        recipient_id,
        "system",
        json!({ "title": "non-governance system hint" }),
        None,
        None,
    )
    .await;
    let suspended_id = suspended.id;
    let mut recipient_suspension = pool.begin().await.expect("begin recipient suspension");
    sqlx::query("SELECT id FROM identity.accounts WHERE id = $1 FOR UPDATE")
        .bind(recipient_id)
        .execute(&mut *recipient_suspension)
        .await
        .expect("lock notification recipient");
    sqlx::query("UPDATE identity.accounts SET status = 'suspended' WHERE id = $1")
        .bind(recipient_id)
        .execute(&mut *recipient_suspension)
        .await
        .expect("suspend notification recipient");
    let delivery_pool = pool.clone();
    let recipient_delivery = tokio::spawn(async move {
        forum::notification_delivery::deliver_event(&delivery_pool, &suspended).await
    });
    tokio::task::yield_now().await;
    recipient_suspension.commit().await.expect("commit recipient suspension");
    recipient_delivery
        .await
        .expect("join suspended recipient delivery")
        .expect("suppress suspended recipient delivery");
    assert_eq!(receipt_outcome(&pool, suspended_id).await, "recipient_unavailable");
}

#[tokio::test]
async fn notification_preferences_reject_unknown_or_incomplete_shapes() {
    let (pool, app) = create_test_app().await;
    let (_, token) = create_test_account(&pool, "notify-shape@tongji.edu.cn", "notify-shape").await;

    let legacy_response = notification_preferences(
        &app,
        &token,
        Method::PUT,
        Some(json!({ "prefs": { "emailPush": true } })),
    )
    .await;
    assert_eq!(legacy_response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let incomplete_response = notification_preferences(
        &app,
        &token,
        Method::PUT,
        Some(json!({
            "prefs": {
                "inApp": { "replies": false },
                "email": { "weeklyDigest": false }
            }
        })),
    )
    .await;
    assert_eq!(incomplete_response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rolling_preference_writes_serialize_without_resetting_the_follow_choice() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "notify-pref-race@tongji.edu.cn", "notify-pref-race").await;
    let legacy = notification_preferences(
        &app,
        &token,
        Method::PUT,
        Some(json!({
            "prefs": {
                "inApp": {
                    "replies": false,
                    "mentions": true,
                    "quotes": true,
                    "votes": true,
                    "badges": true,
                    "subscriptions": true,
                    "directMessages": true
                },
                "email": { "weeklyDigest": false }
            }
        })),
    );
    let current = notification_preferences(
        &app,
        &token,
        Method::PUT,
        Some(json!({
            "prefs": {
                "inApp": {
                    "replies": true,
                    "mentions": true,
                    "quotes": true,
                    "votes": true,
                    "badges": true,
                    "subscriptions": true,
                    "follows": false,
                    "directMessages": true
                },
                "email": { "weeklyDigest": false }
            }
        })),
    );
    let (legacy, current) = tokio::join!(legacy, current);
    assert_eq!(legacy.status(), StatusCode::OK);
    assert_eq!(current.status(), StatusCode::OK);
    let follows: bool = sqlx::query_scalar(
        "SELECT (prefs -> 'inApp' ->> 'follows')::boolean \
         FROM forum.notification_prefs WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("load serialized follow preference");
    assert!(!follows);
}

//! Integration tests for the tip handler.

mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine as _;
use helpers::{
    create_test_account, create_test_app, create_tip_comment, create_tip_review, create_tip_thread,
    create_token, mint_to_account, read_json, signed_post_request,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn send_tip(
    app: &axum::Router,
    pool: &PgPool,
    token: &str,
    sender_id: i64,
    recipient_id: i64,
    target_type: &str,
    target_id: i64,
) -> StatusCode {
    let body = json!({
        "toAccountId": recipient_id.to_string(),
        "amount": 10,
        "targetType": target_type,
        "targetId": target_id.to_string(),
    });
    let request = signed_post_request(
        app,
        pool,
        token,
        sender_id,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    app.clone().oneshot(request).await.expect("tip response").status()
}

#[tokio::test]
async fn tip_requires_wallet_bound_key() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "tipper@tongji.edu.cn", "tipper").await;
    let b = create_test_account(&pool, "tiptarget@tongji.edu.cn", "tiptarget").await;
    mint_to_account(&pool, a, 500).await;

    let token = create_token(&pool, "tipper@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": b.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": "1"
    });

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/credit/signing-intents")
                .method("POST")
                .header("Authorization", format!("Bearer {token}"))
                .header("Idempotency-Key", uuid::Uuid::new_v4().to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "action": "credit.tip", "request": body }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tip_requires_auth() {
    let (_pool, app) = create_test_app().await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/wallet/tip")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"toAccountId":"1","amount":10,"targetType":"thread","targetId":"1"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tip_invalid_signature_rejected() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "sigsender@tongji.edu.cn", "sigsender").await;
    let b = create_test_account(&pool, "sigtarget@tongji.edu.cn", "sigtarget").await;
    mint_to_account(&pool, a, 500).await;

    let token = create_token(&pool, "sigsender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": b.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": "1"
    });

    let mut request = signed_post_request(
        &app,
        &pool,
        &token,
        a,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    request.headers_mut().insert("X-Wallet-Sig", "AAAA".parse().unwrap());
    let resp = app.oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tip_rejects_intent_after_the_active_key_is_revoked_or_replaced() {
    let (pool, app) = create_test_app().await;
    let recipient =
        create_test_account(&pool, "key-change-target@tongji.edu.cn", "key-change-target").await;
    let target_id = create_tip_thread(&pool, recipient).await;

    let revoked_sender =
        create_test_account(&pool, "revoked-sender@tongji.edu.cn", "revoked-sender").await;
    mint_to_account(&pool, revoked_sender, 50).await;
    let revoked_token = create_token(&pool, "revoked-sender@tongji.edu.cn").await;
    let revoked_body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": target_id.to_string(),
    });
    let revoked_request = signed_post_request(
        &app,
        &pool,
        &revoked_token,
        revoked_sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        revoked_body.clone(),
        Some(revoked_body),
    )
    .await;
    sqlx::query(
        "UPDATE identity.account_keys SET revoked_at = now() \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(revoked_sender)
    .execute(&pool)
    .await
    .expect("revoke intent signing key");

    let revoked_response =
        app.clone().oneshot(revoked_request).await.expect("revoked-key tip response");
    assert_eq!(revoked_response.status(), StatusCode::FORBIDDEN);

    let replaced_sender =
        create_test_account(&pool, "replaced-sender@tongji.edu.cn", "replaced-sender").await;
    mint_to_account(&pool, replaced_sender, 50).await;
    let replaced_token = create_token(&pool, "replaced-sender@tongji.edu.cn").await;
    let replaced_body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": target_id.to_string(),
    });
    let replaced_request = signed_post_request(
        &app,
        &pool,
        &replaced_token,
        replaced_sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        replaced_body.clone(),
        Some(replaced_body),
    )
    .await;
    sqlx::query(
        "UPDATE identity.account_keys SET revoked_at = now() \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(replaced_sender)
    .execute(&pool)
    .await
    .expect("revoke replaced intent key");
    let replacement_public_key = base64::engine::general_purpose::STANDARD
        .encode(credit::ledger::derive_public_key(&[91u8; 32]));
    sqlx::query("INSERT INTO identity.account_keys (account_id, public_key) VALUES ($1, $2)")
        .bind(replaced_sender)
        .bind(replacement_public_key)
        .execute(&pool)
        .await
        .expect("insert replacement active key");

    let replaced_response =
        app.clone().oneshot(replaced_request).await.expect("replaced-key tip response");
    assert_eq!(replaced_response.status(), StatusCode::FORBIDDEN);

    let consumed_intents: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.signing_intents \
         WHERE account_id = ANY($1) AND consumed_at IS NOT NULL",
    )
    .bind(&[revoked_sender, replaced_sender][..])
    .fetch_one(&pool)
    .await
    .expect("count consumed key-change intents");
    assert_eq!(consumed_intents, 0);
    let tip_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM credit.ledger WHERE type = 'tip' AND from_account = ANY($1)",
    )
    .bind(&[revoked_sender, replaced_sender][..])
    .fetch_one(&pool)
    .await
    .expect("count rejected key-change tips");
    assert_eq!(tip_entries, 0);
}

#[tokio::test]
async fn tip_insufficient_balance() {
    let (pool, app) = create_test_app().await;
    let a = create_test_account(&pool, "poortipper@tongji.edu.cn", "poortipper").await;
    let b = create_test_account(&pool, "richtarget@tongji.edu.cn", "richtarget").await;
    let target_id = create_tip_thread(&pool, b).await;
    // Give very little points.
    mint_to_account(&pool, a, 5).await;

    let token = create_token(&pool, "poortipper@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": b.to_string(),
        "amount": 100,
        "targetType": "thread",
        "targetId": target_id.to_string()
    });

    let request = signed_post_request(
        &app,
        &pool,
        &token,
        a,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    let resp = app.oneshot(request).await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tip_signature_is_bound_to_exact_ledger_entry() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(&pool, "exactsender@tongji.edu.cn", "exactsender").await;
    let recipient =
        create_test_account(&pool, "exactrecipient@tongji.edu.cn", "exactrecipient").await;
    let target_id = create_tip_thread(&pool, recipient).await;
    mint_to_account(&pool, sender, 50).await;
    let token = create_token(&pool, "exactsender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": target_id.to_string()
    });
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let verify = app
        .oneshot(
            Request::builder().uri("/api/v2/wallet/ledger/verify").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert!(read_json(verify).await["ok"].as_bool().unwrap());
}

#[tokio::test]
async fn tip_rejects_ledger_columns_changed_without_resigning_exact_bytes() {
    let (pool, app) = create_test_app().await;
    let sender =
        create_test_account(&pool, "column-tamper-sender@tongji.edu.cn", "column-tamper-sender")
            .await;
    let recipient = create_test_account(
        &pool,
        "column-tamper-recipient@tongji.edu.cn",
        "column-tamper-recipient",
    )
    .await;
    let target_id = create_tip_thread(&pool, recipient).await;
    mint_to_account(&pool, sender, 50).await;
    let token = create_token(&pool, "column-tamper-sender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": target_id.to_string(),
    });
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body),
    )
    .await;
    let intent_id = request
        .headers()
        .get("x-wallet-intent")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<uuid::Uuid>().ok())
        .expect("signed request intent id");
    let mut ledger_entry: Value =
        sqlx::query_scalar("SELECT ledger_entry FROM credit.signing_intents WHERE id = $1")
            .bind(intent_id)
            .fetch_one(&pool)
            .await
            .expect("read prepared ledger entry");
    ledger_entry["amount"] = json!(40);
    let tampered_canonical = credit::ledger::canonicalize(&ledger_entry);
    sqlx::query(
        "UPDATE credit.signing_intents \
         SET ledger_entry = $2, ledger_canonical = $3 WHERE id = $1",
    )
    .bind(intent_id)
    .bind(&ledger_entry)
    .bind(&tampered_canonical)
    .execute(&pool)
    .await
    .expect("tamper prepared ledger columns without changing signing bytes");

    let response = app.oneshot(request).await.expect("tampered tip response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let state: (i64, i64, i64, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT \
           COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $1), 0), \
           COALESCE((SELECT balance FROM credit.wallets WHERE account_id = $2), 0), \
           (SELECT COUNT(*)::bigint FROM credit.ledger WHERE type = 'tip'), \
           (SELECT consumed_at FROM credit.signing_intents WHERE id = $3)",
    )
    .bind(sender)
    .bind(recipient)
    .bind(intent_id)
    .fetch_one(&pool)
    .await
    .expect("read rejected tip side effects");
    assert_eq!(state.0, 50);
    assert_eq!(state.1, 0);
    assert_eq!(state.2, 0);
    assert!(state.3.is_none());
}

#[tokio::test]
async fn consumed_tip_intent_cannot_be_replayed() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(&pool, "replaysender@tongji.edu.cn", "replaysender").await;
    let recipient =
        create_test_account(&pool, "replayrecipient@tongji.edu.cn", "replayrecipient").await;
    mint_to_account(&pool, sender, 50).await;
    let target_id = create_tip_thread(&pool, recipient).await;
    let token = create_token(&pool, "replaysender@tongji.edu.cn").await;
    let body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": target_id.to_string(),
    });
    let first_request = signed_post_request(
        &app,
        &pool,
        &token,
        sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        body.clone(),
        Some(body.clone()),
    )
    .await;
    let replay_headers = first_request.headers().clone();
    let mut replay_request = Request::builder()
        .uri("/api/v2/wallet/tip")
        .method("POST")
        .body(Body::from(body.to_string()))
        .unwrap();
    *replay_request.headers_mut() = replay_headers;

    let first = app.clone().oneshot(first_request).await.unwrap();
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    let replay = app.oneshot(replay_request).await.unwrap();
    assert_eq!(replay.status(), StatusCode::FORBIDDEN);
    let balances: (i64, i64) = sqlx::query_as(
        "SELECT \
           (SELECT balance FROM credit.wallets WHERE account_id = $1), \
           (SELECT balance FROM credit.wallets WHERE account_id = $2)",
    )
    .bind(sender)
    .bind(recipient)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(balances, (40, 10));
}

#[tokio::test]
async fn tip_intent_rejects_request_body_tampering() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(&pool, "tampersender@tongji.edu.cn", "tampersender").await;
    let recipient =
        create_test_account(&pool, "tamperrecipient@tongji.edu.cn", "tamperrecipient").await;
    mint_to_account(&pool, sender, 50).await;
    let target_id = create_tip_thread(&pool, recipient).await;
    let token = create_token(&pool, "tampersender@tongji.edu.cn").await;
    let signed_body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 10,
        "targetType": "thread",
        "targetId": target_id.to_string(),
    });
    let tampered_body = json!({
        "toAccountId": recipient.to_string(),
        "amount": 11,
        "targetType": "thread",
        "targetId": target_id.to_string(),
    });
    let request = signed_post_request(
        &app,
        &pool,
        &token,
        sender,
        "/api/v2/wallet/tip",
        "credit.tip",
        signed_body,
        Some(tampered_body),
    )
    .await;

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let sender_balance: i64 =
        sqlx::query_scalar("SELECT balance FROM credit.wallets WHERE account_id = $1")
            .bind(sender)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sender_balance, 50);
}

#[tokio::test]
async fn tip_accepts_visible_review_and_comment_targets() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(&pool, "multitipper@tongji.edu.cn", "multitipper").await;
    let recipient = create_test_account(&pool, "multiauthor@tongji.edu.cn", "multiauthor").await;
    mint_to_account(&pool, sender, 40).await;
    let review_id = create_tip_review(&pool, recipient).await;
    let comment_id = create_tip_comment(&pool, recipient).await;
    let token = create_token(&pool, "multitipper@tongji.edu.cn").await;

    assert_eq!(
        send_tip(&app, &pool, &token, sender, recipient, "review", review_id).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send_tip(&app, &pool, &token, sender, recipient, "comment", comment_id).await,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn tip_rejects_hidden_deleted_and_nonexistent_targets() {
    let (pool, app) = create_test_app().await;
    let sender =
        create_test_account(&pool, "visibilitytipper@tongji.edu.cn", "visibilitytipper").await;
    let recipient =
        create_test_account(&pool, "visibilityauthor@tongji.edu.cn", "visibilityauthor").await;
    mint_to_account(&pool, sender, 40).await;
    let hidden_thread_id = create_tip_thread(&pool, recipient).await;
    sqlx::query("UPDATE forum.threads SET hidden_at = now() WHERE id = $1")
        .bind(hidden_thread_id)
        .execute(&pool)
        .await
        .unwrap();
    let deleted_comment_id = create_tip_comment(&pool, recipient).await;
    sqlx::query("UPDATE forum.comments SET deleted_at = now() WHERE id = $1")
        .bind(deleted_comment_id)
        .execute(&pool)
        .await
        .unwrap();
    let token = create_token(&pool, "visibilitytipper@tongji.edu.cn").await;

    assert_eq!(
        send_tip(&app, &pool, &token, sender, recipient, "thread", hidden_thread_id,).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_tip(&app, &pool, &token, sender, recipient, "comment", deleted_comment_id,).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_tip(&app, &pool, &token, sender, recipient, "thread", i64::MAX).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn tip_rejects_self_tip_and_recipient_mismatch() {
    let (pool, app) = create_test_app().await;
    let sender = create_test_account(&pool, "selftipper@tongji.edu.cn", "selftipper").await;
    let author = create_test_account(&pool, "mismatchauthor@tongji.edu.cn", "mismatchauthor").await;
    let wrong_recipient =
        create_test_account(&pool, "wrongrecipient@tongji.edu.cn", "wrongrecipient").await;
    mint_to_account(&pool, sender, 40).await;
    let own_thread_id = create_tip_thread(&pool, sender).await;
    let other_thread_id = create_tip_thread(&pool, author).await;
    let token = create_token(&pool, "selftipper@tongji.edu.cn").await;

    assert_eq!(
        send_tip(&app, &pool, &token, sender, sender, "thread", own_thread_id).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send_tip(&app, &pool, &token, sender, wrong_recipient, "thread", other_thread_id,).await,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn tip_rejects_suspended_or_deleted_recipient_account() {
    let (pool, app) = create_test_app().await;
    let sender =
        create_test_account(&pool, "eligibilitytipper@tongji.edu.cn", "eligibilitytipper").await;
    let suspended =
        create_test_account(&pool, "suspendedauthor@tongji.edu.cn", "suspendedauthor").await;
    let deleted = create_test_account(&pool, "deletedauthor@tongji.edu.cn", "deletedauthor").await;
    mint_to_account(&pool, sender, 40).await;
    let suspended_target = create_tip_thread(&pool, suspended).await;
    let deleted_target = create_tip_thread(&pool, deleted).await;
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by) \
         VALUES ($1, 'suspend', 'credit eligibility test', $2)",
    )
    .bind(suspended)
    .bind(sender)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE identity.accounts SET status = 'deleted', \
             deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 day', deleted_at = now() \
         WHERE id = $1",
    )
    .bind(deleted)
    .execute(&pool)
    .await
    .unwrap();
    let token = create_token(&pool, "eligibilitytipper@tongji.edu.cn").await;

    assert_eq!(
        send_tip(&app, &pool, &token, sender, suspended, "thread", suspended_target,).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_tip(&app, &pool, &token, sender, deleted, "thread", deleted_target,).await,
        StatusCode::NOT_FOUND
    );
}

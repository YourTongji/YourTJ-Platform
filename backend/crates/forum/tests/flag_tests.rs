//! Integration coverage for report thresholds and reversible auto-hide state.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, Response, StatusCode};
use axum::Router;
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(sqlx::FromRow)]
struct ReportAttempt {
    id: i64,
    status: String,
    reason: String,
    note: Option<String>,
    resolution_note: Option<String>,
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Value,
) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("build request"),
        )
        .await
        .expect("request response")
}

async fn create_thread(app: &Router, token: &str, title: &str) -> i64 {
    let response = request(
        app,
        Method::POST,
        "/api/v2/forum/threads",
        token,
        json!({ "boardId": "1", "title": title, "body": "report state test" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json(response).await["id"].as_str().expect("thread id").parse().expect("numeric thread id")
}

async fn thread_activity(pool: &sqlx::PgPool, account_id: i64) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(threads_created), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await
    .expect("thread activity count")
}

async fn board_thread_count(pool: &sqlx::PgPool) -> i32 {
    sqlx::query_scalar("SELECT thread_count FROM forum.boards WHERE id = 1")
        .fetch_one(pool)
        .await
        .expect("board thread count")
}

async fn set_trust_level(pool: &sqlx::PgPool, account_id: i64, trust_level: i16) {
    let mut transaction = pool.begin().await.expect("begin reporter trust fixture");
    sqlx::query(
        "INSERT INTO activity.account_trust_progress \
         (account_id, trust_level, qualifying_score, policy_version) \
         SELECT $1, $2, \
           CASE $2 \
             WHEN 1 THEN 0 \
             WHEN 2 THEN policy.threshold_level_2 \
             WHEN 3 THEN policy.threshold_level_3 \
             WHEN 4 THEN policy.threshold_level_4 \
             WHEN 5 THEN policy.threshold_level_5 \
             ELSE policy.threshold_level_6 \
           END, \
           policy.version \
         FROM activity.trust_level_policies policy \
         ORDER BY policy.version DESC LIMIT 1 \
         ON CONFLICT (account_id) DO UPDATE \
         SET trust_level = EXCLUDED.trust_level, \
             qualifying_score = EXCLUDED.qualifying_score, \
             policy_version = EXCLUDED.policy_version, updated_at = now()",
    )
    .bind(account_id)
    .bind(trust_level)
    .execute(&mut *transaction)
    .await
    .expect("set authoritative reporter trust level");
    sqlx::query("UPDATE identity.accounts SET trust_level = $1 WHERE id = $2")
        .bind(trust_level)
        .bind(account_id)
        .execute(&mut *transaction)
        .await
        .expect("set reporter trust projection");
    transaction.commit().await.expect("commit reporter trust fixture");
}

#[tokio::test]
async fn staff_cannot_use_user_reports_as_privileged_moderation_actions() {
    let (pool, app) = create_test_app().await;
    let (author_id, author_token) =
        create_test_account(&pool, "staff-flag-author@tongji.edu.cn", "staff-flag-author").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "staff-flag-mod@tongji.edu.cn", "staff-flag-mod").await;
    let (administrator_id, administrator_token) =
        create_test_account(&pool, "staff-flag-admin@tongji.edu.cn", "staff-flag-admin").await;
    sqlx::query(
        "UPDATE identity.accounts SET role = CASE \
           WHEN id = $1 THEN 'mod'::identity.account_role \
           WHEN id = $2 THEN 'admin'::identity.account_role ELSE role END",
    )
    .bind(moderator_id)
    .bind(administrator_id)
    .execute(&pool)
    .await
    .expect("promote staff reporters");
    let thread_id = create_thread(&app, &author_token, "Staff report target").await;

    for token in [&moderator_token, &administrator_token] {
        let response = request(
            &app,
            Method::POST,
            &format!("/api/v2/forum/posts/{thread_id}/flag"),
            token,
            json!({ "postType": "thread", "reason": "abuse", "note": "staff report" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let flag_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM forum.flags WHERE target_type = 'thread' AND target_id = $1",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("staff flag count");
    let is_hidden: bool =
        sqlx::query_scalar("SELECT hidden_at IS NOT NULL FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("thread hidden state");
    let sanction_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.sanctions WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("author sanctions");
    assert_eq!(flag_count, 0);
    assert!(!is_hidden);
    assert_eq!(sanction_count, 0);
    assert_eq!(board_thread_count(&pool).await, 1);
}

#[tokio::test]
async fn report_resolution_respects_target_author_role_hierarchy() {
    let (pool, app) = create_test_app().await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "resolve-hierarchy-mod@tongji.edu.cn", "resolve-hierarchy-mod")
            .await;
    let (administrator_id, administrator_token) = create_test_account(
        &pool,
        "resolve-hierarchy-admin@tongji.edu.cn",
        "resolve-hierarchy-admin",
    )
    .await;
    let (moderator_author_id, moderator_author_token) = create_test_account(
        &pool,
        "resolve-hierarchy-mod-author@tongji.edu.cn",
        "resolve-hierarchy-mod-author",
    )
    .await;
    let (administrator_author_id, administrator_author_token) = create_test_account(
        &pool,
        "resolve-hierarchy-admin-author@tongji.edu.cn",
        "resolve-hierarchy-admin-author",
    )
    .await;
    let (reporter_id, reporter_token) = create_test_account(
        &pool,
        "resolve-hierarchy-reporter@tongji.edu.cn",
        "resolve-hierarchy-reporter",
    )
    .await;
    sqlx::query(
        "UPDATE identity.accounts SET role = CASE \
           WHEN id IN ($1, $2) THEN 'mod'::identity.account_role \
           WHEN id IN ($3, $4) THEN 'admin'::identity.account_role ELSE role END",
    )
    .bind(moderator_id)
    .bind(moderator_author_id)
    .bind(administrator_id)
    .bind(administrator_author_id)
    .execute(&pool)
    .await
    .expect("assign hierarchy roles");

    let moderator_thread_id =
        create_thread(&app, &moderator_author_token, "Moderator authored target").await;
    let administrator_thread_id =
        create_thread(&app, &administrator_author_token, "Administrator authored target").await;
    for thread_id in [moderator_thread_id, administrator_thread_id] {
        let report = request(
            &app,
            Method::POST,
            &format!("/api/v2/forum/posts/{thread_id}/flag"),
            &reporter_token,
            json!({ "postType": "thread", "reason": "other", "note": "hierarchy review" }),
        )
        .await;
        assert_eq!(report.status(), StatusCode::OK);
    }
    let moderator_flag_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.flags WHERE target_type = 'thread' AND target_id = $1 \
         AND reporter_id = $2 AND status = 'open'",
    )
    .bind(moderator_thread_id)
    .bind(reporter_id)
    .fetch_one(&pool)
    .await
    .expect("moderator target flag");
    let administrator_flag_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.flags WHERE target_type = 'thread' AND target_id = $1 \
         AND reporter_id = $2 AND status = 'open'",
    )
    .bind(administrator_thread_id)
    .bind(reporter_id)
    .fetch_one(&pool)
    .await
    .expect("administrator target flag");

    for (flag_id, token) in [
        (moderator_flag_id, moderator_token.as_str()),
        (administrator_flag_id, moderator_token.as_str()),
        (administrator_flag_id, administrator_token.as_str()),
    ] {
        let response = request(
            &app,
            Method::POST,
            &format!("/api/v2/admin/forum/flags/{flag_id}/resolve"),
            token,
            json!({ "action": "uphold", "note": "role hierarchy enforcement" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let allowed_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{moderator_flag_id}/resolve"),
        &administrator_token,
        json!({ "action": "uphold", "note": "administrator reviewed moderator content" }),
    )
    .await;
    assert_eq!(allowed_response.status(), StatusCode::OK);
    let moderator_target_deleted: bool =
        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM forum.threads WHERE id = $1")
            .bind(moderator_thread_id)
            .fetch_one(&pool)
            .await
            .expect("moderator target state");
    let administrator_target_deleted: bool =
        sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM forum.threads WHERE id = $1")
            .bind(administrator_thread_id)
            .fetch_one(&pool)
            .await
            .expect("administrator target state");
    assert!(moderator_target_deleted);
    assert!(!administrator_target_deleted);
    assert_eq!(board_thread_count(&pool).await, 1);
}

#[tokio::test]
async fn report_auto_hide_is_reversible_without_overriding_manual_moderation() {
    let (pool, app) = create_test_app().await;
    let (author_id, author_token) =
        create_test_account(&pool, "flag-author@tongji.edu.cn", "flag-author").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "flag-mod@tongji.edu.cn", "flag-mod").await;
    let (first_reporter_id, first_reporter_token) =
        create_test_account(&pool, "flag-reporter-one@tongji.edu.cn", "flag-reporter-one").await;
    let (second_reporter_id, second_reporter_token) =
        create_test_account(&pool, "flag-reporter-two@tongji.edu.cn", "flag-reporter-two").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    set_trust_level(&pool, first_reporter_id, 3).await;
    set_trust_level(&pool, second_reporter_id, 3).await;

    let thread_id = create_thread(&app, &author_token, "Auto-hide report target").await;
    assert_eq!(thread_activity(&pool, author_id).await, 1);
    assert_eq!(board_thread_count(&pool).await, 1);

    let first_flag_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &first_reporter_token,
        json!({
            "postType": "thread",
            "reason": "abuse",
            "note": "first credible abuse report"
        }),
    )
    .await;
    assert_eq!(first_flag_response.status(), StatusCode::OK);
    assert_eq!(read_json(first_flag_response).await["autoHidden"], false);
    assert_eq!(board_thread_count(&pool).await, 1);
    let flag_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &second_reporter_token,
        json!({
            "postType": "thread",
            "reason": "abuse",
            "note": "second independent abuse report"
        }),
    )
    .await;
    let flag_status = flag_response.status();
    let flag_body = read_json(flag_response).await;
    assert_eq!(flag_status, StatusCode::OK, "unexpected flag response: {flag_body}");
    assert_eq!(flag_body["autoHidden"], true);
    assert_eq!(thread_activity(&pool, author_id).await, 0);
    assert_eq!(board_thread_count(&pool).await, 0);
    let auto_hidden_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT hidden_at FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("auto-hidden thread state");
    assert!(auto_hidden_at.is_some());

    let flag_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.flags \
         WHERE target_type = 'thread' AND target_id = $1 AND reporter_id = $2",
    )
    .bind(thread_id)
    .bind(first_reporter_id)
    .fetch_one(&pool)
    .await
    .expect("flag id");
    let short_note_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{flag_id}/resolve"),
        &moderator_token,
        json!({ "action": "reject", "note": "no" }),
    )
    .await;
    assert_eq!(short_note_response.status(), StatusCode::BAD_REQUEST);

    let resolution_note = "report evidence did not support removal";
    let reject_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{flag_id}/resolve"),
        &moderator_token,
        json!({ "action": "reject", "note": resolution_note }),
    )
    .await;
    assert_eq!(reject_response.status(), StatusCode::OK);
    let rejected_flag = read_json(reject_response).await;
    assert_eq!(rejected_flag["status"], "rejected");
    assert_eq!(rejected_flag["resolutionNote"], resolution_note);

    let restored_hidden_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT hidden_at FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(&pool)
            .await
            .expect("restored thread state");
    assert!(restored_hidden_at.is_none());
    assert_eq!(thread_activity(&pool, author_id).await, 1);
    assert_eq!(board_thread_count(&pool).await, 1);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND action = 'forum.flag.reject' \
           AND target_type = 'forum_content' AND target_id = $2 AND reason = $3",
    )
    .bind(moderator_id)
    .bind(format!("thread:{thread_id}"))
    .bind(resolution_note)
    .fetch_one(&pool)
    .await
    .expect("flag resolution audit");
    assert_eq!(audit_count, 1);

    let self_flag_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &author_token,
        json!({ "postType": "thread", "reason": "other", "note": "self report" }),
    )
    .await;
    assert_eq!(self_flag_response.status(), StatusCode::BAD_REQUEST);

    let manual_thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads (board_id, author_id, title, body, hidden_at) \
         VALUES (1, $1, 'Manual moderation target', 'already hidden', now()) RETURNING id",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("seed manually hidden thread");
    let manual_hidden_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT hidden_at FROM forum.threads WHERE id = $1")
            .bind(manual_thread_id)
            .fetch_one(&pool)
            .await
            .expect("manual hidden timestamp");

    let manual_flag_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{manual_thread_id}/flag"),
        &first_reporter_token,
        json!({ "postType": "thread", "reason": "spam", "note": "review manually hidden" }),
    )
    .await;
    assert_eq!(manual_flag_response.status(), StatusCode::OK);
    let manual_flag_body = read_json(manual_flag_response).await;
    assert_eq!(manual_flag_body["autoHidden"], false);

    let manual_flag_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.flags \
         WHERE target_type = 'thread' AND target_id = $1 AND reporter_id = $2",
    )
    .bind(manual_thread_id)
    .bind(first_reporter_id)
    .fetch_one(&pool)
    .await
    .expect("manual target flag id");
    let manual_reject_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{manual_flag_id}/resolve"),
        &moderator_token,
        json!({ "action": "reject", "note": "report rejected; manual action remains" }),
    )
    .await;
    assert_eq!(manual_reject_response.status(), StatusCode::OK);

    let hidden_after_reject: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT hidden_at FROM forum.threads WHERE id = $1")
            .bind(manual_thread_id)
            .fetch_one(&pool)
            .await
            .expect("manual hidden state after rejection");
    assert_eq!(hidden_after_reject, manual_hidden_at);
}

#[tokio::test]
async fn repeated_report_preserves_terminal_attempt_and_queue_includes_hidden_evidence() {
    let (pool, app) = create_test_app().await;
    let (author_id, author_token) =
        create_test_account(&pool, "flag-history-author@tongji.edu.cn", "flag-history-author")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "flag-history-mod@tongji.edu.cn", "flag-history-mod").await;
    let (first_reporter_id, first_reporter_token) = create_test_account(
        &pool,
        "flag-history-reporter-one@tongji.edu.cn",
        "flag-history-reporter-one",
    )
    .await;
    let (second_reporter_id, second_reporter_token) = create_test_account(
        &pool,
        "flag-history-reporter-two@tongji.edu.cn",
        "flag-history-reporter-two",
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    set_trust_level(&pool, first_reporter_id, 3).await;
    set_trust_level(&pool, second_reporter_id, 3).await;
    let thread_id = create_thread(&app, &author_token, "Flag evidence title").await;

    let first_report = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &first_reporter_token,
        json!({
            "postType": "thread",
            "reason": "abuse",
            "note": "first evidence note"
        }),
    )
    .await;
    assert_eq!(first_report.status(), StatusCode::OK);
    let first_report_body = read_json(first_report).await;
    assert_eq!(first_report_body["autoHidden"], false);
    let threshold_report = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &second_reporter_token,
        json!({
            "postType": "thread",
            "reason": "abuse",
            "note": "independent corroborating evidence"
        }),
    )
    .await;
    assert_eq!(threshold_report.status(), StatusCode::OK);
    assert_eq!(read_json(threshold_report).await["autoHidden"], true);

    let queue_response = request(
        &app,
        Method::GET,
        "/api/v2/admin/forum/flags?status=open",
        &moderator_token,
        json!({}),
    )
    .await;
    assert_eq!(queue_response.status(), StatusCode::OK);
    let queue = read_json(queue_response).await;
    let thread_id_string = thread_id.to_string();
    let queue_item = queue["items"]
        .as_array()
        .expect("flag queue items")
        .iter()
        .find(|item| item["targetId"].as_str() == Some(thread_id_string.as_str()))
        .expect("target flag in queue");
    assert_eq!(queue_item["authorHandle"], "flag-history-author");
    assert_eq!(queue_item["targetTitle"], "Flag evidence title");
    assert_eq!(queue_item["contentExcerpt"], "report state test");

    let first_flag_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.flags \
         WHERE target_type = 'thread' AND target_id = $1 AND reporter_id = $2 \
           AND status = 'open'",
    )
    .bind(thread_id)
    .bind(first_reporter_id)
    .fetch_one(&pool)
    .await
    .expect("first open flag id");
    let reject_response = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{first_flag_id}/resolve"),
        &moderator_token,
        json!({ "action": "reject", "note": "first report was not upheld" }),
    )
    .await;
    assert_eq!(reject_response.status(), StatusCode::OK);

    let second_report = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &first_reporter_token,
        json!({
            "postType": "thread",
            "reason": "spam",
            "note": "later independent evidence"
        }),
    )
    .await;
    assert_eq!(second_report.status(), StatusCode::OK);
    assert_eq!(read_json(second_report).await["autoHidden"], false);
    let second_threshold_report = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        &second_reporter_token,
        json!({
            "postType": "thread",
            "reason": "spam",
            "note": "later corroborating evidence"
        }),
    )
    .await;
    assert_eq!(second_threshold_report.status(), StatusCode::OK);
    assert_eq!(read_json(second_threshold_report).await["autoHidden"], true);

    let attempts: Vec<ReportAttempt> = sqlx::query_as(
        "SELECT id, status, reason, note, resolution_note \
         FROM forum.flags \
         WHERE target_type = 'thread' AND target_id = $1 AND reporter_id = $2 \
         ORDER BY id",
    )
    .bind(thread_id)
    .bind(first_reporter_id)
    .fetch_all(&pool)
    .await
    .expect("report attempts");
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].id, first_flag_id);
    assert_eq!(attempts[0].status, "rejected");
    assert_eq!(attempts[0].reason, "abuse");
    assert_eq!(attempts[0].note.as_deref(), Some("first evidence note"));
    assert_eq!(attempts[0].resolution_note.as_deref(), Some("first report was not upheld"));
    assert_ne!(attempts[1].id, first_flag_id);
    assert_eq!(attempts[1].status, "open");
    assert_eq!(attempts[1].reason, "spam");

    let author_sanction_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM identity.sanctions \
         WHERE account_id = $1 AND kind = 'silence' AND revoked_at IS NULL",
    )
    .bind(author_id)
    .fetch_one(&pool)
    .await
    .expect("auto-silence state");
    assert_eq!(author_sanction_count, 1);
}

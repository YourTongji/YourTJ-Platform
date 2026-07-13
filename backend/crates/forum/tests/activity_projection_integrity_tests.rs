//! Handler-to-PostgreSQL coverage for forum activity subtree integrity.

#[path = "helpers/mod.rs"]
mod helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Method, Request, Response, StatusCode};
use axum::Router;
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::Barrier;
use tower::ServiceExt;

#[derive(Debug, PartialEq, Eq)]
struct ActivityCounts {
    threads: i64,
    comments: i64,
    likes: i64,
}

struct ActivityFixture {
    thread_id: i64,
    valid_comment_id: i64,
    second_comment_id: i64,
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let body = if let Some(body) = body {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(request.body(body).expect("build activity integrity request"))
        .await
        .expect("activity integrity response")
}

async fn create_activity_fixture(
    app: &Router,
    thread_author_token: &str,
    comment_author_token: &str,
    voter_token: &str,
    title: &str,
) -> ActivityFixture {
    let thread_response = request(
        app,
        Method::POST,
        "/api/v2/forum/threads",
        thread_author_token,
        Some(json!({ "boardId": "1", "title": title, "body": "activity subtree" })),
    )
    .await;
    assert_eq!(thread_response.status(), StatusCode::CREATED);
    let thread_id = parse_id(&read_json(thread_response).await, "id");

    let valid_comment_id =
        create_comment(app, thread_id, comment_author_token, "valid descendant comment").await;
    let second_comment_id =
        create_comment(app, thread_id, comment_author_token, "second descendant comment").await;

    for (post_type, post_id) in
        [("thread", thread_id), ("comment", valid_comment_id), ("comment", second_comment_id)]
    {
        let response = request(
            app,
            Method::POST,
            &format!("/api/v2/forum/posts/{post_id}/vote"),
            voter_token,
            Some(json!({ "postType": post_type, "value": "up" })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "vote on {post_type}:{post_id}");
    }

    ActivityFixture { thread_id, valid_comment_id, second_comment_id }
}

async fn create_comment(app: &Router, thread_id: i64, token: &str, body: &str) -> i64 {
    let response = request(
        app,
        Method::POST,
        &format!("/api/v2/forum/threads/{thread_id}/comments"),
        token,
        Some(json!({ "body": body })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    parse_id(&read_json(response).await, "id")
}

fn parse_id(body: &Value, field: &str) -> i64 {
    body[field].as_str().expect("string id").parse().expect("numeric id")
}

async fn admin_thread_action(
    app: &Router,
    thread_id: i64,
    action: &str,
    moderator_token: &str,
) -> StatusCode {
    request(
        app,
        Method::POST,
        &format!("/api/v2/admin/forum/threads/{thread_id}/{action}"),
        moderator_token,
        Some(json!({ "reason": format!("activity subtree {action} verification") })),
    )
    .await
    .status()
}

async fn admin_comment_action(
    app: &Router,
    comment_id: i64,
    action: &str,
    moderator_token: &str,
) -> StatusCode {
    request(
        app,
        Method::POST,
        &format!("/api/v2/admin/forum/comments/{comment_id}/{action}"),
        moderator_token,
        Some(json!({ "reason": format!("activity comment {action} verification") })),
    )
    .await
    .status()
}

async fn activity_counts(pool: &PgPool, account_id: i64) -> ActivityCounts {
    let (threads, comments, likes): (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(threads_created), 0)::bigint, \
                COALESCE(SUM(comments_created), 0)::bigint, \
                COALESCE(SUM(likes_given), 0)::bigint \
         FROM activity.daily_counts WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(pool)
    .await
    .expect("activity counts");
    ActivityCounts { threads, comments, likes }
}

async fn assert_fixture_activity(
    pool: &PgPool,
    thread_author_id: i64,
    comment_author_id: i64,
    voter_id: i64,
    expected_thread_count: i64,
    expected_comment_count: i64,
    expected_like_count: i64,
) {
    assert_eq!(
        activity_counts(pool, thread_author_id).await,
        ActivityCounts { threads: expected_thread_count, comments: 0, likes: 0 }
    );
    assert_eq!(
        activity_counts(pool, comment_author_id).await,
        ActivityCounts { threads: 0, comments: expected_comment_count, likes: 0 }
    );
    assert_eq!(
        activity_counts(pool, voter_id).await,
        ActivityCounts { threads: 0, comments: 0, likes: expected_like_count }
    );
}

async fn promote_moderator(pool: &PgPool, account_id: i64) {
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(account_id)
        .execute(pool)
        .await
        .expect("promote activity moderator");
}

async fn shanghai_date(
    pool: &PgPool,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> chrono::NaiveDate {
    sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE 'Asia/Shanghai')::date")
        .bind(timestamp)
        .fetch_one(pool)
        .await
        .expect("Shanghai activity date")
}

async fn active_activity_date(pool: &PgPool, source_key: &str) -> chrono::NaiveDate {
    sqlx::query_scalar(
        "SELECT event.activity_date FROM activity.events event \
         WHERE event.source_key = $1 AND event.delta = 1 \
           AND NOT EXISTS ( \
             SELECT 1 FROM activity.events reversal \
             WHERE reversal.reverses_event_id = event.id \
           ) \
         ORDER BY event.generation DESC LIMIT 1",
    )
    .bind(source_key)
    .fetch_one(pool)
    .await
    .expect("active contribution date")
}

#[tokio::test]
async fn restored_subtree_keeps_each_historical_source_date() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, _) =
        create_test_account(&pool, "activity-date-author@tongji.edu.cn", "activity-date-author")
            .await;
    let (comment_author_id, _) = create_test_account(
        &pool,
        "activity-date-commenter@tongji.edu.cn",
        "activity-date-commenter",
    )
    .await;
    let (voter_id, _) =
        create_test_account(&pool, "activity-date-voter@tongji.edu.cn", "activity-date-voter")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "activity-date-mod@tongji.edu.cn", "activity-date-mod").await;
    promote_moderator(&pool, moderator_id).await;

    let thread_created_at = chrono::Utc::now() - chrono::Duration::days(12);
    let comment_created_at = chrono::Utc::now() - chrono::Duration::days(9);
    let thread_vote_at = chrono::Utc::now() - chrono::Duration::days(6);
    let comment_vote_at = chrono::Utc::now() - chrono::Duration::days(3);
    let thread_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.threads \
           (board_id, author_id, title, body, reply_count, vote_count, created_at, last_activity_at) \
         VALUES (1, $1, 'Historical activity subtree', 'historical body', 1, 1, $2, $3) \
         RETURNING id",
    )
    .bind(thread_author_id)
    .bind(thread_created_at)
    .bind(comment_created_at)
    .fetch_one(&pool)
    .await
    .expect("historical thread");
    let comment_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.comments \
           (thread_id, path, author_id, body, vote_count, created_at) \
         VALUES ($1, '0001', $2, 'historical comment', 1, $3) RETURNING id",
    )
    .bind(thread_id)
    .bind(comment_author_id)
    .bind(comment_created_at)
    .fetch_one(&pool)
    .await
    .expect("historical comment");
    sqlx::query(
        "INSERT INTO forum.votes \
           (post_type, post_id, account_id, value, created_at, updated_at) \
         VALUES \
           ('thread', $1, $3, 1, $4, $4), \
           ('comment', $2, $3, 1, $5, $5)",
    )
    .bind(thread_id)
    .bind(comment_id)
    .bind(voter_id)
    .bind(thread_vote_at)
    .bind(comment_vote_at)
    .execute(&pool)
    .await
    .expect("historical votes");
    let mut transaction = pool.begin().await.expect("begin historical activity projection");
    for (account_id, kind, source_key, occurred_at) in [
        (
            thread_author_id,
            activity::contributions::ActivityKind::Thread,
            format!("forum_thread:{thread_id}"),
            thread_created_at,
        ),
        (
            comment_author_id,
            activity::contributions::ActivityKind::Comment,
            format!("forum_comment:{comment_id}"),
            comment_created_at,
        ),
        (
            voter_id,
            activity::contributions::ActivityKind::Like,
            format!("forum_vote:thread:{thread_id}:{voter_id}"),
            thread_vote_at,
        ),
        (
            voter_id,
            activity::contributions::ActivityKind::Like,
            format!("forum_vote:comment:{comment_id}:{voter_id}"),
            comment_vote_at,
        ),
    ] {
        activity::contributions::activate_contribution(
            &mut transaction,
            account_id,
            kind,
            &source_key,
            occurred_at,
        )
        .await
        .expect("seed historical contribution");
    }
    transaction.commit().await.expect("commit historical activity projection");

    assert_eq!(
        admin_thread_action(&app, thread_id, "hide", &moderator_token).await,
        StatusCode::OK
    );
    assert_eq!(
        admin_thread_action(&app, thread_id, "unhide", &moderator_token).await,
        StatusCode::OK
    );

    for (source_key, timestamp) in [
        (format!("forum_thread:{thread_id}"), thread_created_at),
        (format!("forum_comment:{comment_id}"), comment_created_at),
        (format!("forum_vote:thread:{thread_id}:{voter_id}"), thread_vote_at),
        (format!("forum_vote:comment:{comment_id}:{voter_id}"), comment_vote_at),
    ] {
        assert_eq!(
            active_activity_date(&pool, &source_key).await,
            shanghai_date(&pool, timestamp).await,
            "restored source {source_key}"
        );
    }
}

#[tokio::test]
async fn thread_lifecycle_synchronizes_descendants_and_restores_only_valid_sources() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, thread_author_token) =
        create_test_account(&pool, "activity-tree-author@tongji.edu.cn", "activity-tree-author")
            .await;
    let (comment_author_id, comment_author_token) = create_test_account(
        &pool,
        "activity-tree-commenter@tongji.edu.cn",
        "activity-tree-commenter",
    )
    .await;
    let (voter_id, voter_token) =
        create_test_account(&pool, "activity-tree-voter@tongji.edu.cn", "activity-tree-voter")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "activity-tree-mod@tongji.edu.cn", "activity-tree-mod").await;
    promote_moderator(&pool, moderator_id).await;
    let fixture = create_activity_fixture(
        &app,
        &thread_author_token,
        &comment_author_token,
        &voter_token,
        "Activity subtree lifecycle",
    )
    .await;

    assert_eq!(
        admin_comment_action(&app, fixture.second_comment_id, "hide", &moderator_token).await,
        StatusCode::OK
    );
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 1, 2).await;

    for (restrict, restore) in [("hide", "unhide"), ("archive", "unarchive"), ("delete", "restore")]
    {
        assert_eq!(
            admin_thread_action(&app, fixture.thread_id, restrict, &moderator_token).await,
            StatusCode::OK,
            "restrict with {restrict}"
        );
        assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 0, 0, 0)
            .await;
        assert_eq!(
            admin_thread_action(&app, fixture.thread_id, restore, &moderator_token).await,
            StatusCode::OK,
            "restore with {restore}"
        );
        assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 1, 2)
            .await;
    }

    assert_eq!(
        admin_comment_action(&app, fixture.valid_comment_id, "delete", &moderator_token).await,
        StatusCode::OK
    );
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 0, 1).await;
    assert_eq!(
        admin_comment_action(&app, fixture.valid_comment_id, "restore", &moderator_token).await,
        StatusCode::OK
    );
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 1, 2).await;

    assert_eq!(
        admin_comment_action(&app, fixture.second_comment_id, "unhide", &moderator_token).await,
        StatusCode::OK
    );
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 2, 3).await;
    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v2/forum/comments/{}", fixture.second_comment_id),
            &comment_author_token,
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 1, 2).await;

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            &format!("/api/v2/forum/threads/{}", fixture.thread_id),
            &thread_author_token,
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 0, 0, 0).await;
}

#[tokio::test]
async fn queued_edit_and_auto_archive_deactivate_complete_subtrees() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, thread_author_token) =
        create_test_account(&pool, "activity-queue-author@tongji.edu.cn", "activity-queue-author")
            .await;
    let (comment_author_id, comment_author_token) = create_test_account(
        &pool,
        "activity-queue-commenter@tongji.edu.cn",
        "activity-queue-commenter",
    )
    .await;
    let (voter_id, voter_token) =
        create_test_account(&pool, "activity-queue-voter@tongji.edu.cn", "activity-queue-voter")
            .await;
    let queued_fixture = create_activity_fixture(
        &app,
        &thread_author_token,
        &comment_author_token,
        &voter_token,
        "Queued activity subtree",
    )
    .await;
    let marker = "activity-subtree-queue-marker";
    sqlx::query("INSERT INTO forum.watched_words (word, action) VALUES ($1, 'queue')")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("insert activity queue policy");
    forum::watched_words::reload_watched_words(&pool).await.expect("reload activity queue policy");
    let queued = request(
        &app,
        Method::PATCH,
        &format!("/api/v2/forum/threads/{}", queued_fixture.thread_id),
        &thread_author_token,
        Some(json!({ "body": marker })),
    )
    .await;
    assert_eq!(queued.status(), StatusCode::OK);
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 0, 0, 0).await;
    sqlx::query("DELETE FROM forum.watched_words WHERE word = $1")
        .bind(marker)
        .execute(&pool)
        .await
        .expect("remove activity queue policy");
    forum::watched_words::reload_watched_words(&pool)
        .await
        .expect("reload cleared activity queue policy");

    let (archive_thread_author_id, archive_thread_author_token) = create_test_account(
        &pool,
        "activity-archive-author@tongji.edu.cn",
        "activity-archive-author",
    )
    .await;
    let (archive_comment_author_id, archive_comment_author_token) = create_test_account(
        &pool,
        "activity-archive-commenter@tongji.edu.cn",
        "activity-archive-commenter",
    )
    .await;
    let (archive_voter_id, archive_voter_token) = create_test_account(
        &pool,
        "activity-archive-voter@tongji.edu.cn",
        "activity-archive-voter",
    )
    .await;
    let archived_fixture = create_activity_fixture(
        &app,
        &archive_thread_author_token,
        &archive_comment_author_token,
        &archive_voter_token,
        "Auto archive activity subtree",
    )
    .await;
    sqlx::query(
        "UPDATE forum.threads SET last_activity_at = now() - interval '91 days' WHERE id = $1",
    )
    .bind(archived_fixture.thread_id)
    .execute(&pool)
    .await
    .expect("age auto archive fixture");
    assert_eq!(forum::repo::auto_archive_stale(&pool).await, 1);
    assert_fixture_activity(
        &pool,
        archive_thread_author_id,
        archive_comment_author_id,
        archive_voter_id,
        0,
        0,
        0,
    )
    .await;
}

#[tokio::test]
async fn report_reject_uphold_and_appeal_overturn_synchronize_complete_subtree() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, thread_author_token) =
        create_test_account(&pool, "activity-flag-author@tongji.edu.cn", "activity-flag-author")
            .await;
    let (comment_author_id, comment_author_token) = create_test_account(
        &pool,
        "activity-flag-commenter@tongji.edu.cn",
        "activity-flag-commenter",
    )
    .await;
    let (voter_id, voter_token) =
        create_test_account(&pool, "activity-flag-voter@tongji.edu.cn", "activity-flag-voter")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "activity-flag-mod@tongji.edu.cn", "activity-flag-mod").await;
    let (first_reporter_id, first_reporter_token) =
        create_test_account(&pool, "activity-flag-one@tongji.edu.cn", "activity-flag-one").await;
    let (second_reporter_id, second_reporter_token) =
        create_test_account(&pool, "activity-flag-two@tongji.edu.cn", "activity-flag-two").await;
    promote_moderator(&pool, moderator_id).await;
    sqlx::query(
        "INSERT INTO activity.account_trust_progress \
         (account_id, trust_level, qualifying_score, policy_version) \
         SELECT ids.account_id, 3, 120, policy.version \
         FROM unnest($1::bigint[]) AS ids(account_id) \
         CROSS JOIN LATERAL ( \
           SELECT version FROM activity.trust_level_policies ORDER BY version DESC LIMIT 1 \
         ) policy \
         ON CONFLICT (account_id) DO UPDATE \
         SET trust_level = 3, qualifying_score = 120, policy_version = EXCLUDED.policy_version, \
             updated_at = now()",
    )
    .bind(vec![first_reporter_id, second_reporter_id])
    .execute(&pool)
    .await
    .expect("raise activity reporters trust");
    sqlx::query("UPDATE identity.accounts SET trust_level = 3 WHERE id = ANY($1)")
        .bind(vec![first_reporter_id, second_reporter_id])
        .execute(&pool)
        .await
        .expect("project activity reporters trust");
    let fixture = create_activity_fixture(
        &app,
        &thread_author_token,
        &comment_author_token,
        &voter_token,
        "Flag activity subtree",
    )
    .await;

    auto_hide_thread(&pool, &app, fixture.thread_id, &first_reporter_token, &second_reporter_token)
        .await;
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 0, 0, 0).await;
    let first_flag_id = latest_open_flag(&pool, fixture.thread_id).await;
    let rejected = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{first_flag_id}/resolve"),
        &moderator_token,
        Some(json!({ "action": "reject", "note": "evidence does not support restriction" })),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::OK);
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 2, 3).await;

    let follow_up_flag = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{}/flag", fixture.thread_id),
        &first_reporter_token,
        Some(json!({
            "postType": "thread",
            "reason": "abuse",
            "note": "follow-up report requiring manual uphold"
        })),
    )
    .await;
    assert_eq!(follow_up_flag.status(), StatusCode::OK);
    let upheld_flag_id = latest_open_flag(&pool, fixture.thread_id).await;
    let upheld = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{upheld_flag_id}/resolve"),
        &moderator_token,
        Some(json!({ "action": "uphold", "note": "evidence confirms content restriction" })),
    )
    .await;
    assert_eq!(upheld.status(), StatusCode::OK);
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 0, 0, 0).await;

    let (event_id, created_at, target_type, target_id, metadata): (
        i64,
        chrono::DateTime<chrono::Utc>,
        String,
        String,
        Option<Value>,
    ) = sqlx::query_as(
        "SELECT id, created_at, target_type, target_id, metadata \
         FROM governance.audit_events \
         WHERE action = 'forum.flag.uphold' AND target_type = 'forum_content' \
           AND target_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(format!("thread:{}", fixture.thread_id))
    .fetch_one(&pool)
    .await
    .expect("upheld activity governance event");
    let mut transaction = pool.begin().await.expect("begin activity appeal reversal");
    forum::appeals::overturn_content_for_appeal_tx(
        &mut transaction,
        event_id,
        created_at,
        "forum.flag.uphold",
        &target_type,
        &target_id,
        metadata.as_ref(),
        "forum_thread",
        fixture.thread_id,
        "delete",
        thread_author_id,
    )
    .await
    .expect("overturn activity subtree restriction");
    transaction.commit().await.expect("commit activity appeal reversal");
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 1, 2, 3).await;
}

async fn auto_hide_thread(
    pool: &PgPool,
    app: &Router,
    thread_id: i64,
    first_reporter_token: &str,
    second_reporter_token: &str,
) {
    for (index, token) in [first_reporter_token, second_reporter_token].into_iter().enumerate() {
        let response = request(
            app,
            Method::POST,
            &format!("/api/v2/forum/posts/{thread_id}/flag"),
            token,
            Some(json!({
                "postType": "thread",
                "reason": "abuse",
                "note": format!("independent activity report {index}")
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let hidden: bool =
        sqlx::query_scalar("SELECT hidden_at IS NOT NULL FROM forum.threads WHERE id = $1")
            .bind(thread_id)
            .fetch_one(pool)
            .await
            .expect("thread auto-hidden");
    assert!(hidden);
}

async fn latest_open_flag(pool: &PgPool, thread_id: i64) -> i64 {
    sqlx::query_scalar(
        "SELECT id FROM forum.flags \
         WHERE target_type = 'thread' AND target_id = $1 AND status = 'open' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(thread_id)
    .fetch_one(pool)
    .await
    .expect("latest open activity flag")
}

#[tokio::test]
async fn concurrent_parent_hide_comment_unhide_and_vote_cannot_reactivate_child_projection() {
    let (pool, app) = create_test_app().await;
    let (thread_author_id, thread_author_token) =
        create_test_account(&pool, "activity-race-author@tongji.edu.cn", "activity-race-author")
            .await;
    let (comment_author_id, comment_author_token) = create_test_account(
        &pool,
        "activity-race-commenter@tongji.edu.cn",
        "activity-race-commenter",
    )
    .await;
    let (voter_id, voter_token) =
        create_test_account(&pool, "activity-race-voter@tongji.edu.cn", "activity-race-voter")
            .await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "activity-race-mod@tongji.edu.cn", "activity-race-mod").await;
    promote_moderator(&pool, moderator_id).await;
    let fixture = create_activity_fixture(
        &app,
        &thread_author_token,
        &comment_author_token,
        &voter_token,
        "Concurrent activity subtree",
    )
    .await;
    assert_eq!(
        admin_comment_action(&app, fixture.valid_comment_id, "hide", &moderator_token).await,
        StatusCode::OK
    );

    let barrier = Arc::new(Barrier::new(4));
    let parent_app = app.clone();
    let parent_token = moderator_token.clone();
    let parent_barrier = Arc::clone(&barrier);
    let thread_id = fixture.thread_id;
    let parent_task = tokio::spawn(async move {
        parent_barrier.wait().await;
        admin_thread_action(&parent_app, thread_id, "hide", &parent_token).await
    });
    let comment_app = app.clone();
    let comment_token = moderator_token.clone();
    let comment_barrier = Arc::clone(&barrier);
    let comment_id = fixture.valid_comment_id;
    let comment_task = tokio::spawn(async move {
        comment_barrier.wait().await;
        admin_comment_action(&comment_app, comment_id, "unhide", &comment_token).await
    });
    let vote_app = app.clone();
    let vote_barrier = Arc::clone(&barrier);
    let vote_comment_id = fixture.second_comment_id;
    let vote_task = tokio::spawn(async move {
        vote_barrier.wait().await;
        request(
            &vote_app,
            Method::POST,
            &format!("/api/v2/forum/posts/{vote_comment_id}/vote"),
            &voter_token,
            Some(json!({ "postType": "comment", "value": "down" })),
        )
        .await
        .status()
    });
    barrier.wait().await;

    let (parent_status, comment_status, vote_status) =
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            (
                parent_task.await.expect("join parent activity transition"),
                comment_task.await.expect("join comment activity transition"),
                vote_task.await.expect("join concurrent activity vote"),
            )
        })
        .await
        .expect("activity transitions must not deadlock");
    assert_eq!(parent_status, StatusCode::OK);
    assert_eq!(comment_status, StatusCode::OK);
    assert!(matches!(vote_status, StatusCode::OK | StatusCode::NOT_FOUND));
    assert_fixture_activity(&pool, thread_author_id, comment_author_id, voter_id, 0, 0, 0).await;

    let (parent_hidden, comment_visible): (bool, bool) = sqlx::query_as(
        "SELECT thread.hidden_at IS NOT NULL, \
                comment.hidden_at IS NULL AND comment.deleted_at IS NULL \
         FROM forum.threads thread \
         JOIN forum.comments comment ON comment.thread_id = thread.id \
         WHERE thread.id = $1 AND comment.id = $2",
    )
    .bind(fixture.thread_id)
    .bind(fixture.valid_comment_id)
    .fetch_one(&pool)
    .await
    .expect("concurrent activity terminal state");
    assert!(parent_hidden);
    assert!(comment_visible);
}

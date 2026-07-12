//! Handler-to-database coverage for Forum Markdown image ownership and binding lifecycle.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    app.clone()
        .oneshot(
            builder
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .expect("forum media request"),
        )
        .await
        .expect("forum media response")
}

async fn seed_upload(pool: &PgPool, account_id: i64, usage: &str, status: &str) -> i64 {
    configure_test_delivery();
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let asset_id = sqlx::query_scalar(
        "INSERT INTO media.uploads \
         (account_id, kind, oss_key, url, bytes, mime, sha256, status, usage, \
          image_width, image_height) \
         VALUES ($1, 'image', $2, $3, 1024, 'image/png', $4, $5, $6, 1200, 800) \
         RETURNING id",
    )
    .bind(account_id)
    .bind(format!("uploads/{account_id}/image/{suffix}.png"))
    .bind(format!("https://cdn.example.test/{suffix}.png"))
    .bind("a".repeat(64))
    .bind(status)
    .bind(usage)
    .fetch_one(pool)
    .await
    .expect("seed forum image");
    if status == "clean" {
        let content_hash = "b".repeat(64);
        for (variant_kind, width, height) in
            [("thumb_256", 256, 171), ("display_1280", 1200, 800), ("full_2048", 1200, 800)]
        {
            sqlx::query(
                "INSERT INTO media.asset_variants \
                 (asset_id, variant_kind, policy_version, object_key, content_sha256, mime, \
                  bytes, width, height, status, published_at) \
                 VALUES ($1, $2, 1, $3, $4, 'image/webp', 512, $5, $6, 'published', now())",
            )
            .bind(asset_id)
            .bind(variant_kind)
            .bind(format!("assets/{asset_id}/1/{variant_kind}-{content_hash}.webp"))
            .bind(&content_hash)
            .bind(width)
            .bind(height)
            .execute(pool)
            .await
            .expect("seed published forum variant");
        }
        sqlx::query(
            "UPDATE media.asset_publications \
             SET policy_version = 1, status = 'published', published_at = now(), \
                 updated_at = now() WHERE asset_id = $1",
        )
        .bind(asset_id)
        .execute(pool)
        .await
        .expect("seed complete forum publication");
    }
    asset_id
}

fn configure_test_delivery() {
    for (key, value) in [
        ("OSS_REGION", "cn-shanghai"),
        ("MEDIA_DELIVERY_OSS_BUCKET", "yourtj-test-delivery"),
        ("MEDIA_DELIVERY_OSS_ACCESS_KEY_ID", "test-delivery-writer"),
        ("MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET", "test-delivery-secret"),
        ("MEDIA_CDN_BASE_URL", "https://media.example.test"),
        ("MEDIA_CDN_PRIMARY_KEY", "testprimarysigningkey"),
        ("MEDIA_CDN_SECONDARY_KEY", "testsecondarysigningkey"),
        ("MEDIA_CDN_SIGNING_KEY_SLOT", "primary"),
        ("MEDIA_CDN_URL_TTL_SECONDS", "300"),
        ("CDN_ACCESS_KEY_ID", "test-purge-operator"),
        ("CDN_ACCESS_KEY_SECRET", "test-purge-secret"),
    ] {
        std::env::set_var(key, value);
    }
}

async fn create_thread(app: &axum::Router, token: &str, body: Value) -> axum::response::Response {
    request(app, Method::POST, "/api/v2/forum/threads", Some(token), Some(body)).await
}

#[tokio::test]
async fn thread_images_require_exact_owned_clean_bindings_and_disclose_only_safe_projection() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) =
        create_test_account(&pool, "forum-image-owner@tongji.edu.cn", "forum-image-owner").await;
    let (other_id, _) =
        create_test_account(&pool, "forum-image-other@tongji.edu.cn", "forum-image-other").await;
    let clean_id = seed_upload(&pool, owner_id, "forum_thread", "clean").await;
    let second_clean_id = seed_upload(&pool, owner_id, "forum_thread", "clean").await;
    let pending_id = seed_upload(&pool, owner_id, "forum_thread", "pending").await;
    let blocked_id = seed_upload(&pool, owner_id, "forum_thread", "blocked").await;
    let other_id = seed_upload(&pool, other_id, "forum_thread", "clean").await;
    let wrong_usage_id = seed_upload(&pool, owner_id, "forum_comment", "clean").await;

    let created = create_thread(
        &app,
        &owner_token,
        json!({
            "boardId": "1",
            "title": "带图主题",
            "body": format!("正文\n\n![校园风景](yourtj-asset:{clean_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [clean_id.to_string()]
        }),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_json = read_json(created).await;
    assert_eq!(created_json["attachments"][0]["assetId"], clean_id.to_string());
    assert_eq!(created_json["attachments"][0]["reference"], format!("yourtj-asset:{clean_id}"));
    assert_eq!(created_json["attachments"][0]["alt"], "校园风景");
    assert_eq!(created_json["attachments"][0]["width"], 1200);
    assert!(created_json["attachments"][0].get("ossKey").is_none());
    assert!(created_json["attachments"][0].get("sha256").is_none());
    let thread_id = created_json["id"].as_str().expect("thread id");

    let usage_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM media.asset_usages \
         WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(thread_id.parse::<i64>().expect("numeric thread id"))
    .fetch_one(&pool)
    .await
    .expect("active binding count");
    assert_eq!(usage_count, 1);

    for rejected_id in [pending_id, blocked_id, other_id, wrong_usage_id] {
        let rejected = create_thread(
            &app,
            &owner_token,
            json!({
                "boardId": "1",
                "title": format!("拒绝 {rejected_id}"),
                "body": format!("![图片](yourtj-asset:{rejected_id})"),
                "contentFormat": "markdown_v1",
                "attachmentAssetIds": [rejected_id.to_string()]
            }),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::NOT_FOUND);
    }

    let invalid_inputs = [
        json!({
            "boardId": "1", "title": "遗漏集合",
            "body": format!("![图片](yourtj-asset:{clean_id})"),
            "contentFormat": "markdown_v1", "attachmentAssetIds": []
        }),
        json!({
            "boardId": "1", "title": "额外集合", "body": "正文",
            "contentFormat": "markdown_v1", "attachmentAssetIds": [clean_id.to_string()]
        }),
        json!({
            "boardId": "1", "title": "顺序不一致",
            "body": format!(
                "![第一张](yourtj-asset:{clean_id}) ![第二张](yourtj-asset:{second_clean_id})"
            ),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [second_clean_id.to_string(), clean_id.to_string()]
        }),
        json!({
            "boardId": "1", "title": "重复引用",
            "body": format!("![一](yourtj-asset:{clean_id}) ![二](yourtj-asset:{clean_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [clean_id.to_string(), clean_id.to_string()]
        }),
        json!({
            "boardId": "1", "title": "缺少替代文本",
            "body": format!("![](yourtj-asset:{clean_id})"),
            "contentFormat": "markdown_v1", "attachmentAssetIds": [clean_id.to_string()]
        }),
        json!({
            "boardId": "1", "title": "远程图片",
            "body": "![跟踪](https://tracker.example/pixel.png)", "contentFormat": "markdown_v1"
        }),
        json!({
            "boardId": "1", "title": "数据图片",
            "body": "![内联](data:image/png;base64,AAAA)", "contentFormat": "markdown_v1"
        }),
        json!({
            "boardId": "1", "title": "纯文本不解析引用",
            "body": format!("![文本](yourtj-asset:{clean_id})"),
            "contentFormat": "plain_v1", "attachmentAssetIds": [clean_id.to_string()]
        }),
    ];
    for invalid in invalid_inputs {
        let rejected = create_thread(&app, &owner_token, invalid).await;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        "INSERT INTO media.asset_usages \
         (asset_id, owner_account_id, target_type, target_id, position, alt_text, bound_content_version) \
         VALUES ($1, $2, 'forum_thread', $3, 1, '不应披露', 1)",
    )
    .bind(wrong_usage_id)
    .bind(owner_id)
    .bind(thread_id.parse::<i64>().expect("numeric thread id"))
    .execute(&pool)
    .await
    .expect("seed corrupt extra usage");
    let fail_closed =
        request(&app, Method::GET, &format!("/api/v2/forum/threads/{thread_id}"), None, None).await;
    assert_eq!(fail_closed.status(), StatusCode::OK);
    assert_eq!(read_json(fail_closed).await["attachments"], json!([]));
}

#[tokio::test]
async fn stale_edit_cannot_switch_binding_and_revision_keeps_the_old_clean_snapshot() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) =
        create_test_account(&pool, "forum-image-edit@tongji.edu.cn", "forum-image-edit").await;
    let first_id = seed_upload(&pool, owner_id, "forum_thread", "clean").await;
    let second_id = seed_upload(&pool, owner_id, "forum_thread", "clean").await;

    let created = create_thread(
        &app,
        &owner_token,
        json!({
            "boardId": "1", "title": "版本图片",
            "body": format!("![第一张](yourtj-asset:{first_id})"),
            "contentFormat": "markdown_v1", "attachmentAssetIds": [first_id.to_string()]
        }),
    )
    .await;
    let created_json = read_json(created).await;
    let thread_id = created_json["id"].as_str().expect("thread id").to_owned();
    sqlx::query(
        "UPDATE forum.threads SET created_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(thread_id.parse::<i64>().expect("numeric thread id"))
    .execute(&pool)
    .await
    .expect("age thread for revision");

    let updated = request(
        &app,
        Method::PATCH,
        &format!("/api/v2/forum/threads/{thread_id}"),
        Some(&owner_token),
        Some(json!({
            "expectedVersion": 1,
            "body": format!("![第二张](yourtj-asset:{second_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [second_id.to_string()]
        })),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);

    let stale = request(
        &app,
        Method::PATCH,
        &format!("/api/v2/forum/threads/{thread_id}"),
        Some(&owner_token),
        Some(json!({
            "expectedVersion": 1,
            "body": format!("![第一张](yourtj-asset:{first_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [first_id.to_string()]
        })),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);

    let active_asset_id: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.asset_usages \
         WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(thread_id.parse::<i64>().expect("numeric thread id"))
    .fetch_one(&pool)
    .await
    .expect("active asset after stale write");
    assert_eq!(active_asset_id, second_id);

    let second_update = request(
        &app,
        Method::PATCH,
        &format!("/api/v2/forum/threads/{thread_id}"),
        Some(&owner_token),
        Some(json!({
            "expectedVersion": 2,
            "body": format!("![第一张恢复](yourtj-asset:{first_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [first_id.to_string()]
        })),
    )
    .await;
    assert_eq!(second_update.status(), StatusCode::OK);

    let revisions = request(
        &app,
        Method::GET,
        &format!("/api/v2/forum/threads/{thread_id}/revisions?limit=2"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(revisions.status(), StatusCode::OK);
    let revision_json = read_json(revisions).await;
    assert_eq!(revision_json["items"][0]["oldContentVersion"], 2);
    assert_eq!(revision_json["items"][0]["attachments"][0]["assetId"], second_id.to_string());
    assert_eq!(revision_json["items"][1]["oldContentVersion"], 1);
    assert_eq!(revision_json["items"][1]["attachments"][0]["assetId"], first_id.to_string());
}

#[tokio::test]
async fn delete_detaches_with_gc_grace_and_only_one_concurrent_restore_rebinds() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) =
        create_test_account(&pool, "forum-image-delete@tongji.edu.cn", "forum-image-delete").await;
    let (moderator_id, moderator_token) =
        create_test_account(&pool, "forum-image-mod@tongji.edu.cn", "forum-image-mod").await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote moderator");
    let image_id = seed_upload(&pool, owner_id, "forum_thread", "clean").await;
    let created = create_thread(
        &app,
        &owner_token,
        json!({
            "boardId": "1", "title": "可恢复图片",
            "body": format!("![图片](yourtj-asset:{image_id})"),
            "contentFormat": "markdown_v1", "attachmentAssetIds": [image_id.to_string()]
        }),
    )
    .await;
    let thread_id = read_json(created).await["id"].as_str().expect("thread id").to_owned();

    let deleted = request(
        &app,
        Method::DELETE,
        &format!("/api/v2/forum/threads/{thread_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);
    let (detached_reason, has_grace): (String, bool) = sqlx::query_as(
        "SELECT detached_reason, gc_eligible_at > detached_at \
         FROM media.asset_usages WHERE target_type = 'forum_thread' AND target_id = $1",
    )
    .bind(thread_id.parse::<i64>().expect("numeric thread id"))
    .fetch_one(&pool)
    .await
    .expect("detached usage");
    assert_eq!(detached_reason, "target_deleted");
    assert!(has_grace);

    let restore_uri = format!("/api/v2/admin/forum/threads/{thread_id}/restore");
    let first = request(
        &app,
        Method::POST,
        &restore_uri,
        Some(&moderator_token),
        Some(json!({ "reason": "恢复误删主题" })),
    );
    let second = request(
        &app,
        Method::POST,
        &restore_uri,
        Some(&moderator_token),
        Some(json!({ "reason": "并发恢复检查" })),
    );
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.status(), second.status()];
    assert!(statuses.contains(&StatusCode::OK));
    assert!(statuses.contains(&StatusCode::CONFLICT));

    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM media.asset_usages \
         WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(thread_id.parse::<i64>().expect("numeric thread id"))
    .fetch_one(&pool)
    .await
    .expect("one restored binding");
    assert_eq!(active_count, 1);
}

#[tokio::test]
async fn moderation_delete_and_appeal_restore_keep_media_bindings_reversible() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) = create_test_account(
        &pool,
        "forum-image-governance-owner@tongji.edu.cn",
        "forum-image-governance-owner",
    )
    .await;
    let (moderator_id, moderator_token) = create_test_account(
        &pool,
        "forum-image-governance-mod@tongji.edu.cn",
        "forum-image-governance-mod",
    )
    .await;
    let (reporter_id, reporter_token) = create_test_account(
        &pool,
        "forum-image-governance-reporter@tongji.edu.cn",
        "forum-image-governance-reporter",
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote governance moderator");

    let image_id = seed_upload(&pool, owner_id, "forum_thread", "clean").await;
    let created = create_thread(
        &app,
        &owner_token,
        json!({
            "boardId": "1",
            "title": "治理恢复图片",
            "body": format!("![治理证据](yourtj-asset:{image_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [image_id.to_string()]
        }),
    )
    .await;
    let thread_id = read_json(created).await["id"]
        .as_str()
        .expect("thread id")
        .parse::<i64>()
        .expect("numeric thread id");

    for action in ["archive", "unarchive"] {
        let response = request(
            &app,
            Method::POST,
            &format!("/api/v2/admin/forum/threads/{thread_id}/{action}"),
            Some(&moderator_token),
            Some(json!({ "reason": "验证归档不释放仍被内容引用的图片" })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let active_after_archive_cycle: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM media.asset_usages \
         WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("archive cycle binding");
    assert_eq!(active_after_archive_cycle, 1);

    for action in ["delete", "restore"] {
        let response = request(
            &app,
            Method::POST,
            &format!("/api/v2/admin/forum/threads/{thread_id}/{action}"),
            Some(&moderator_token),
            Some(json!({ "reason": "验证管理软删除图片绑定生命周期" })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM media.asset_usages \
             WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
        )
        .bind(thread_id)
        .fetch_one(&pool)
        .await
        .expect("admin action binding count");
        assert_eq!(active_count, if action == "restore" { 1 } else { 0 });
    }

    let report = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/posts/{thread_id}/flag"),
        Some(&reporter_token),
        Some(json!({
            "postType": "thread",
            "reason": "other",
            "note": "需要工作人员复核"
        })),
    )
    .await;
    assert_eq!(report.status(), StatusCode::OK);
    let flag_id: i64 = sqlx::query_scalar(
        "SELECT id FROM forum.flags WHERE target_type = 'thread' AND target_id = $1 \
         AND reporter_id = $2 AND status = 'open'",
    )
    .bind(thread_id)
    .bind(reporter_id)
    .fetch_one(&pool)
    .await
    .expect("open media flag");
    let upheld = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/flags/{flag_id}/resolve"),
        Some(&moderator_token),
        Some(json!({ "action": "uphold", "note": "复核后执行软删除" })),
    )
    .await;
    assert_eq!(upheld.status(), StatusCode::OK);
    let active_after_uphold: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM media.asset_usages \
         WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("upheld flag detached binding");
    assert_eq!(active_after_uphold, 0);

    let (event_id, created_at, target_type, target_id, metadata): (
        i64,
        chrono::DateTime<chrono::Utc>,
        String,
        String,
        Option<Value>,
    ) = sqlx::query_as(
        "SELECT id, created_at, target_type, target_id, metadata \
         FROM governance.audit_events WHERE action = 'forum.flag.uphold' \
         AND target_type = 'forum_content' AND target_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(format!("thread:{thread_id}"))
    .fetch_one(&pool)
    .await
    .expect("flag governance event");
    let mut tx = pool.begin().await.expect("appeal reversal transaction");
    forum::appeals::overturn_content_for_appeal_tx(
        &mut tx,
        event_id,
        created_at,
        "forum.flag.uphold",
        &target_type,
        &target_id,
        metadata.as_ref(),
        "forum_thread",
        thread_id,
        "delete",
        owner_id,
    )
    .await
    .expect("overturn flag restriction");
    tx.commit().await.expect("commit appeal reversal");

    let restored_asset_id: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.asset_usages \
         WHERE target_type = 'forum_thread' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("appeal restored binding");
    assert_eq!(restored_asset_id, image_id);
}

async fn assert_comment_appeal_serializes_with_parent_restriction(
    restriction_column: &str,
    restriction_action: &str,
) {
    let (pool, app) = create_test_app().await;
    let suffix = format!("comment-appeal-{restriction_column}");
    let (owner_id, owner_token) = create_test_account(
        &pool,
        &format!("forum-{suffix}-owner@tongji.edu.cn"),
        &format!("forum-{suffix}-owner"),
    )
    .await;
    let (moderator_id, moderator_token) = create_test_account(
        &pool,
        &format!("forum-{suffix}-mod@tongji.edu.cn"),
        &format!("forum-{suffix}-mod"),
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote appeal moderator");
    let thread = create_thread(
        &app,
        &owner_token,
        json!({ "boardId": "1", "title": "申诉锁序父主题", "body": "正文" }),
    )
    .await;
    let thread_id = read_json(thread).await["id"]
        .as_str()
        .expect("thread id")
        .parse::<i64>()
        .expect("numeric thread id");
    let image_id = seed_upload(&pool, owner_id, "forum_comment", "clean").await;
    let comment = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/threads/{thread_id}/comments"),
        Some(&owner_token),
        Some(json!({
            "body": format!("![评论证据](yourtj-asset:{image_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [image_id.to_string()]
        })),
    )
    .await;
    assert_eq!(comment.status(), StatusCode::CREATED);
    let comment_id = read_json(comment).await["id"]
        .as_str()
        .expect("comment id")
        .parse::<i64>()
        .expect("numeric comment id");
    let deleted = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/comments/{comment_id}/delete"),
        Some(&moderator_token),
        Some(json!({ "reason": "需要独立复核的评论" })),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);
    let (event_id, created_at, target_type, target_id): (
        i64,
        chrono::DateTime<chrono::Utc>,
        String,
        String,
    ) = sqlx::query_as(
        "SELECT id, created_at, target_type, target_id \
         FROM governance.audit_events WHERE action = 'forum.comment.delete' \
           AND target_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(comment_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("comment delete governance event");

    let mut parent_transaction = pool.begin().await.expect("begin parent restriction");
    sqlx::query("SELECT id FROM forum.threads WHERE id = $1 FOR UPDATE")
        .bind(thread_id)
        .execute(&mut *parent_transaction)
        .await
        .expect("lock parent thread");
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *parent_transaction)
        .await
        .expect("bound parent lock wait");

    let application_name = format!("appeal-lock-{}", uuid::Uuid::new_v4().simple());
    let appeal_pool = pool.clone();
    let appeal_application_name = application_name.clone();
    let appeal_task = tokio::spawn(async move {
        let mut transaction = appeal_pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query("SELECT set_config('application_name', $1, true)")
            .bind(appeal_application_name)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        forum::appeals::overturn_content_for_appeal_tx(
            &mut transaction,
            event_id,
            created_at,
            "forum.comment.delete",
            &target_type,
            &target_id,
            None,
            "forum_comment",
            comment_id,
            "delete",
            owner_id,
        )
        .await
        .map_err(|error| error.to_string())?;
        transaction.commit().await.map_err(|error| error.to_string())
    });

    let mut appeal_is_waiting = false;
    for _ in 0..100 {
        appeal_is_waiting = sqlx::query_scalar(
            "SELECT EXISTS( \
               SELECT 1 FROM pg_stat_activity \
               WHERE application_name = $1 AND wait_event_type = 'Lock' \
             )",
        )
        .bind(&application_name)
        .fetch_one(&pool)
        .await
        .expect("observe appeal lock wait");
        if appeal_is_waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(appeal_is_waiting, "appeal did not reach the parent lock");

    sqlx::query("SELECT id FROM forum.comments WHERE id = $1 FOR UPDATE")
        .bind(comment_id)
        .execute(&mut *parent_transaction)
        .await
        .expect("parent-first transaction must not deadlock while locking the comment");
    sqlx::query(&format!("UPDATE forum.threads SET {restriction_column} = now() WHERE id = $1"))
        .bind(thread_id)
        .execute(&mut *parent_transaction)
        .await
        .expect("restrict parent thread");
    governance::record_account_event_with_id_tx(
        &mut parent_transaction,
        governance::AccountActor { account_id: moderator_id, role: "mod" },
        restriction_action,
        "thread",
        &thread_id.to_string(),
        "parent restriction wins before comment appeal",
        None,
    )
    .await
    .expect("record parent restriction");
    parent_transaction.commit().await.expect("commit parent restriction");

    tokio::time::timeout(std::time::Duration::from_secs(3), appeal_task)
        .await
        .expect("comment appeal must complete without deadlock")
        .expect("join comment appeal task")
        .expect("comment appeal reversal");

    let (comment_restored, thread_deleted, thread_hidden): (bool, bool, bool) = sqlx::query_as(
        "SELECT comment.deleted_at IS NULL, thread.deleted_at IS NOT NULL, \
                thread.hidden_at IS NOT NULL \
         FROM forum.comments comment \
         JOIN forum.threads thread ON thread.id = comment.thread_id \
         WHERE comment.id = $1",
    )
    .bind(comment_id)
    .fetch_one(&pool)
    .await
    .expect("read serialized appeal state");
    assert!(comment_restored);
    match restriction_column {
        "deleted_at" => assert!(thread_deleted),
        "hidden_at" => assert!(thread_hidden),
        _ => panic!("unsupported parent restriction fixture"),
    }
    let activity_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(delta), 0)::bigint FROM activity.events WHERE source_key = $1",
    )
    .bind(format!("forum_comment:{comment_id}"))
    .fetch_one(&pool)
    .await
    .expect("read comment activity balance");
    assert_eq!(activity_balance, 0, "an unavailable parent must suppress comment reactivation");
    let active_asset_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM media.asset_usages \
         WHERE target_type = 'forum_comment' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(comment_id)
    .fetch_one(&pool)
    .await
    .expect("read restored comment media binding");
    assert_eq!(active_asset_count, 1, "appeal restoration must preserve media rebind");
}

#[tokio::test]
async fn comment_appeal_locks_parent_first_and_does_not_reactivate_under_parent_restrictions() {
    assert_comment_appeal_serializes_with_parent_restriction("hidden_at", "forum.thread.hide")
        .await;
    assert_comment_appeal_serializes_with_parent_restriction("deleted_at", "forum.thread.delete")
        .await;
}

#[tokio::test]
async fn comment_and_cloud_draft_assets_keep_pending_private_until_clean_publish() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) =
        create_test_account(&pool, "forum-comment-image@tongji.edu.cn", "forum-comment-image")
            .await;
    let (other_id, other_token) =
        create_test_account(&pool, "forum-comment-other@tongji.edu.cn", "forum-comment-other")
            .await;
    let pending_id = seed_upload(&pool, owner_id, "forum_comment", "pending").await;
    let quarantined_id = seed_upload(&pool, owner_id, "forum_comment", "quarantined").await;
    let clean_id = seed_upload(&pool, owner_id, "forum_comment", "clean").await;
    let replacement_id = seed_upload(&pool, owner_id, "forum_comment", "clean").await;
    let other_id = seed_upload(&pool, other_id, "forum_comment", "clean").await;
    let thread = create_thread(
        &app,
        &owner_token,
        json!({ "boardId": "1", "title": "评论图片父主题", "body": "正文" }),
    )
    .await;
    let thread_id = read_json(thread).await["id"].as_str().expect("thread id").to_owned();

    let draft_body = json!({
        "draftKey": format!("comment:{thread_id}"),
        "expectedVersion": 0,
        "payload": {
            "kind": "comment",
            "threadId": thread_id,
            "body": format!("![待审](yourtj-asset:{pending_id})"),
            "contentFormat": "markdown_v1",
            "parentId": null,
            "attachmentAssetIds": [pending_id.to_string()]
        }
    });
    let draft = request(
        &app,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(&owner_token),
        Some(draft_body.clone()),
    )
    .await;
    assert_eq!(draft.status(), StatusCode::OK);
    let draft_reference: (i64, String, i16) = sqlx::query_as(
        "SELECT asset_id, target_type, position FROM media.draft_asset_references \
         WHERE account_id = $1 AND draft_key = $2",
    )
    .bind(owner_id)
    .bind(format!("comment:{thread_id}"))
    .fetch_one(&pool)
    .await
    .expect("pending draft asset reference");
    assert_eq!(draft_reference, (pending_id, "forum_comment".into(), 0));
    let other_draft =
        request(&app, Method::PUT, "/api/v2/me/drafts", Some(&other_token), Some(draft_body)).await;
    assert_eq!(other_draft.status(), StatusCode::NOT_FOUND);
    let quarantined_draft = request(
        &app,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(&owner_token),
        Some(json!({
            "draftKey": format!("comment:{thread_id}"),
            "expectedVersion": 1,
            "payload": {
                "kind": "comment",
                "threadId": thread_id,
                "body": format!("![已隔离](yourtj-asset:{quarantined_id})"),
                "contentFormat": "markdown_v1",
                "parentId": null,
                "attachmentAssetIds": [quarantined_id.to_string()]
            }
        })),
    )
    .await;
    assert_eq!(quarantined_draft.status(), StatusCode::NOT_FOUND);

    let updated_draft = request(
        &app,
        Method::PUT,
        "/api/v2/me/drafts",
        Some(&owner_token),
        Some(json!({
            "draftKey": format!("comment:{thread_id}"),
            "expectedVersion": 1,
            "payload": {
                "kind": "comment",
                "threadId": thread_id,
                "body": format!("![替换](yourtj-asset:{replacement_id})"),
                "contentFormat": "markdown_v1",
                "parentId": null,
                "attachmentAssetIds": [replacement_id.to_string()]
            }
        })),
    )
    .await;
    assert_eq!(updated_draft.status(), StatusCode::OK);
    let replaced_draft_asset: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.draft_asset_references \
         WHERE account_id = $1 AND draft_key = $2",
    )
    .bind(owner_id)
    .bind(format!("comment:{thread_id}"))
    .fetch_one(&pool)
    .await
    .expect("replaced draft asset reference");
    assert_eq!(replaced_draft_asset, replacement_id);

    let pending_publish = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/threads/{thread_id}/comments"),
        Some(&owner_token),
        Some(json!({
            "body": format!("![待审](yourtj-asset:{pending_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [pending_id.to_string()]
        })),
    )
    .await;
    assert_eq!(pending_publish.status(), StatusCode::NOT_FOUND);

    let clean_publish = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/threads/{thread_id}/comments"),
        Some(&owner_token),
        Some(json!({
            "body": format!("![评论图](yourtj-asset:{clean_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [clean_id.to_string()]
        })),
    )
    .await;
    assert_eq!(clean_publish.status(), StatusCode::CREATED);
    let clean_json = read_json(clean_publish).await;
    assert_eq!(clean_json["attachments"][0]["assetId"], clean_id.to_string());
    let comment_id = clean_json["id"].as_str().expect("comment id").to_owned();

    let edited = request(
        &app,
        Method::PATCH,
        &format!("/api/v2/forum/comments/{comment_id}"),
        Some(&owner_token),
        Some(json!({
            "expectedVersion": 1,
            "body": format!("![替换图](yourtj-asset:{replacement_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [replacement_id.to_string()]
        })),
    )
    .await;
    assert_eq!(edited.status(), StatusCode::OK);
    let stale = request(
        &app,
        Method::PATCH,
        &format!("/api/v2/forum/comments/{comment_id}"),
        Some(&owner_token),
        Some(json!({
            "expectedVersion": 1,
            "body": format!("![旧图](yourtj-asset:{clean_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [clean_id.to_string()]
        })),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);

    let deleted = request(
        &app,
        Method::DELETE,
        &format!("/api/v2/forum/comments/{comment_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::OK);
    let active_after_delete: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM media.asset_usages \
         WHERE target_type = 'forum_comment' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(comment_id.parse::<i64>().expect("numeric comment id"))
    .fetch_one(&pool)
    .await
    .expect("comment binding detached");
    assert_eq!(active_after_delete, 0);

    let (moderator_id, moderator_token) = create_test_account(
        &pool,
        "forum-comment-image-mod@tongji.edu.cn",
        "forum-comment-image-mod",
    )
    .await;
    sqlx::query("UPDATE identity.accounts SET role = 'mod' WHERE id = $1")
        .bind(moderator_id)
        .execute(&pool)
        .await
        .expect("promote comment moderator");
    let restored = request(
        &app,
        Method::POST,
        &format!("/api/v2/admin/forum/comments/{comment_id}/restore"),
        Some(&moderator_token),
        Some(json!({ "reason": "恢复误删评论" })),
    )
    .await;
    assert_eq!(restored.status(), StatusCode::OK);
    let restored_asset_id: i64 = sqlx::query_scalar(
        "SELECT asset_id FROM media.asset_usages \
         WHERE target_type = 'forum_comment' AND target_id = $1 AND detached_at IS NULL",
    )
    .bind(comment_id.parse::<i64>().expect("numeric comment id"))
    .fetch_one(&pool)
    .await
    .expect("comment binding restored");
    assert_eq!(restored_asset_id, replacement_id);

    let cross_account = request(
        &app,
        Method::POST,
        &format!("/api/v2/forum/threads/{thread_id}/comments"),
        Some(&owner_token),
        Some(json!({
            "body": format!("![越权](yourtj-asset:{other_id})"),
            "contentFormat": "markdown_v1",
            "attachmentAssetIds": [other_id.to_string()]
        })),
    )
    .await;
    assert_eq!(cross_account.status(), StatusCode::NOT_FOUND);

    let deleted_draft = request(
        &app,
        Method::DELETE,
        &format!("/api/v2/me/drafts/comment:{thread_id}"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(deleted_draft.status(), StatusCode::NO_CONTENT);
    let draft_reference_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM media.draft_asset_references \
         WHERE account_id = $1 AND draft_key = $2",
    )
    .bind(owner_id)
    .bind(format!("comment:{thread_id}"))
    .fetch_one(&pool)
    .await
    .expect("deleted draft reference count");
    assert_eq!(draft_reference_count, 0);
}

#[tokio::test]
async fn draft_save_waiting_behind_account_deletion_rechecks_owner_state() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) = create_test_account(
        &pool,
        "forum-draft-delete-race@tongji.edu.cn",
        "forum-draft-delete-race",
    )
    .await;
    let asset_id = seed_upload(&pool, owner_id, "forum_thread", "pending").await;
    let mut deletion_transaction = pool.begin().await.expect("begin account deletion fixture");
    sqlx::query(
        "UPDATE identity.accounts \
         SET status = 'deleted', deletion_requested_at = now() - interval '31 days', \
             deletion_recover_until = now() - interval '1 day', deleted_at = now(), \
             lifecycle_version = lifecycle_version + 1 \
         WHERE id = $1",
    )
    .bind(owner_id)
    .execute(&mut *deletion_transaction)
    .await
    .expect("stage account deletion");

    let save_app = app.clone();
    let save_task = tokio::spawn(async move {
        request(
            &save_app,
            Method::PUT,
            "/api/v2/me/drafts",
            Some(&owner_token),
            Some(json!({
                "draftKey": "thread:new",
                "expectedVersion": 0,
                "payload": {
                    "kind": "thread",
                    "boardId": null,
                    "title": "must not survive account purge",
                    "body": format!("![draft](yourtj-asset:{asset_id})"),
                    "contentFormat": "markdown_v1",
                    "tags": [],
                    "pollQuestion": "",
                    "pollOptions": [],
                    "attachmentAssetIds": [asset_id.to_string()]
                }
            })),
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(!save_task.is_finished(), "draft save should wait on the account lifecycle lock");
    deletion_transaction.commit().await.expect("commit account deletion fixture");
    let response = tokio::time::timeout(std::time::Duration::from_secs(3), save_task)
        .await
        .expect("draft save completes after account deletion")
        .expect("join blocked draft save");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let draft_count: i64 =
        sqlx::query_scalar("SELECT count(*)::bigint FROM forum.drafts WHERE account_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("draft count after account deletion race");
    assert_eq!(draft_count, 0);
    let reference_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM media.draft_asset_references WHERE account_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("draft reference count after account deletion race");
    assert_eq!(reference_count, 0);
}

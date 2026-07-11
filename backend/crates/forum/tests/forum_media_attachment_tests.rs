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
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    sqlx::query_scalar(
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
    .expect("seed forum image")
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

    let revisions = request(
        &app,
        Method::GET,
        &format!("/api/v2/forum/threads/{thread_id}/revisions"),
        Some(&owner_token),
        None,
    )
    .await;
    assert_eq!(revisions.status(), StatusCode::OK);
    let revision_json = read_json(revisions).await;
    assert_eq!(revision_json[0]["oldContentVersion"], 1);
    assert_eq!(revision_json[0]["attachments"][0]["assetId"], first_id.to_string());
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
async fn comment_and_cloud_draft_assets_keep_pending_private_until_clean_publish() {
    let (pool, app) = create_test_app().await;
    let (owner_id, owner_token) =
        create_test_account(&pool, "forum-comment-image@tongji.edu.cn", "forum-comment-image")
            .await;
    let (other_id, other_token) =
        create_test_account(&pool, "forum-comment-other@tongji.edu.cn", "forum-comment-other")
            .await;
    let pending_id = seed_upload(&pool, owner_id, "forum_comment", "pending").await;
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
    let other_draft =
        request(&app, Method::PUT, "/api/v2/me/drafts", Some(&other_token), Some(draft_body)).await;
    assert_eq!(other_draft.status(), StatusCode::NOT_FOUND);

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
}

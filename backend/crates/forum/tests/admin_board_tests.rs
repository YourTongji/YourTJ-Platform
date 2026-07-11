//! Integration coverage for administrator board lifecycle operations.

mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use helpers::{create_test_account, create_test_app, read_json};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn admin_can_create_and_update_board_with_complete_row_mapping() {
    let (pool, app) = create_test_app().await;
    let (account_id, token) =
        create_test_account(&pool, "board-admin@tongji.edu.cn", "board-admin").await;
    sqlx::query("UPDATE identity.accounts SET role = 'admin' WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("promote board administrator");

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v2/admin/forum/boards")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "slug": "campus-life",
                        "name": "Campus Life",
                        "reason": "create the campus discussion board"
                    })
                    .to_string(),
                ))
                .expect("create board request"),
        )
        .await
        .expect("create board response");
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created = read_json(create_response).await;
    assert_eq!(created["slug"], "campus-life");
    assert_eq!(created["isQa"], false);
    let board_id = created["id"].as_str().expect("created board id");

    let update_response = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v2/admin/forum/boards/{board_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Campus Community",
                        "reason": "clarify the board scope"
                    })
                    .to_string(),
                ))
                .expect("update board request"),
        )
        .await
        .expect("update board response");
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated = read_json(update_response).await;
    assert_eq!(updated["name"], "Campus Community");
    assert_eq!(updated["isQa"], false);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM governance.audit_events \
         WHERE actor_account_id = $1 AND target_id = $2 \
           AND action IN ('forum.board.created', 'forum.board.updated')",
    )
    .bind(account_id)
    .bind(board_id)
    .fetch_one(&pool)
    .await
    .expect("board audit count");
    assert_eq!(audit_count, 2);
}

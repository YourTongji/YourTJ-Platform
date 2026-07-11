//! Handler-to-database coverage for privacy-safe federated account search.

#[path = "../../forum/tests/helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use helpers::{create_test_account, create_test_app, read_json};
use shared::AppState;
use tower::ServiceExt;

#[tokio::test]
async fn user_search_rehydrates_ranked_candidates_without_leaking_private_or_hidden_accounts() {
    let (pool, _) = create_test_app().await;
    let (viewer_id, viewer_token) =
        create_test_account(&pool, "search-viewer@tongji.edu.cn", "search-viewer").await;
    let (campus_id, _) =
        create_test_account(&pool, "campus-result@tongji.edu.cn", "campus-result").await;
    let (public_id, _) =
        create_test_account(&pool, "public-result@tongji.edu.cn", "public-result").await;
    let (private_id, _) =
        create_test_account(&pool, "private-result@tongji.edu.cn", "private-result").await;
    let (hidden_id, _) =
        create_test_account(&pool, "hidden-result@tongji.edu.cn", "hidden-result").await;
    let (muted_id, _) =
        create_test_account(&pool, "muted-result@tongji.edu.cn", "muted-result").await;
    let (blocked_id, _) =
        create_test_account(&pool, "blocked-result@tongji.edu.cn", "blocked-result").await;
    let (suspended_id, _) =
        create_test_account(&pool, "suspended-result@tongji.edu.cn", "suspended-result").await;
    let candidate_ids =
        vec![campus_id, public_id, private_id, hidden_id, muted_id, blocked_id, suspended_id];
    sqlx::query("UPDATE identity.accounts SET email_verified_at = now() WHERE id = ANY($1)")
        .bind(&candidate_ids)
        .execute(&pool)
        .await
        .expect("verify search fixtures");
    sqlx::query(
        "INSERT INTO identity.profiles (account_id, display_name) \
         SELECT id, 'Result ' || handle::text FROM identity.accounts WHERE id = ANY($1) \
         ON CONFLICT (account_id) DO UPDATE SET display_name = EXCLUDED.display_name",
    )
    .bind(&candidate_ids)
    .execute(&pool)
    .await
    .expect("seed profile names");
    sqlx::query(
        "INSERT INTO identity.profile_privacy (account_id, profile_visibility, discoverable) \
         VALUES ($1, 'campus', true), ($2, 'public', true), ($3, 'only_me', true), \
                ($4, 'public', false), ($5, 'public', true), ($6, 'public', true), \
                ($7, 'public', true) \
         ON CONFLICT (account_id) DO UPDATE \
         SET profile_visibility = EXCLUDED.profile_visibility, \
             discoverable = EXCLUDED.discoverable",
    )
    .bind(campus_id)
    .bind(public_id)
    .bind(private_id)
    .bind(hidden_id)
    .bind(muted_id)
    .bind(blocked_id)
    .bind(suspended_id)
    .execute(&pool)
    .await
    .expect("seed privacy policies");
    sqlx::query("INSERT INTO forum.user_follows (follower_id, followed_id) VALUES ($1, $2)")
        .bind(viewer_id)
        .bind(public_id)
        .execute(&pool)
        .await
        .expect("seed current follow");
    sqlx::query("INSERT INTO forum.user_mutes (account_id, muted_account_id) VALUES ($1, $2)")
        .bind(viewer_id)
        .bind(muted_id)
        .execute(&pool)
        .await
        .expect("seed search mute");
    sqlx::query("INSERT INTO forum.user_ignores (account_id, ignored_account_id) VALUES ($1, $2)")
        .bind(blocked_id)
        .bind(viewer_id)
        .execute(&pool)
        .await
        .expect("seed reverse search block");
    sqlx::query(
        "INSERT INTO identity.sanctions (account_id, kind, reason, issued_by) \
         VALUES ($1, 'suspend', 'search test', $2)",
    )
    .bind(suspended_id)
    .bind(viewer_id)
    .execute(&pool)
    .await
    .expect("seed suspended search account");
    let board_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.boards (slug, name, description) \
         VALUES ('study-search', 'Study Search', 'Visible board') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed searchable board");
    sqlx::query("ALTER TABLE forum.tags ALTER COLUMN id RESTART WITH 100")
        .execute(&pool)
        .await
        .expect("separate tag identifiers");
    let tag_id: i64 = sqlx::query_scalar(
        "INSERT INTO forum.tags (slug, name, description) \
         VALUES ('study-tag', 'Study Tag', 'Visible tag') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed searchable tag");

    let meili_response = serde_json::json!({
        "hits": candidate_ids.iter().map(|id| serde_json::json!({ "id": id.to_string() })).collect::<Vec<_>>(),
        "offset": 0,
        "limit": 120,
        "estimatedTotalHits": candidate_ids.len(),
        "processingTimeMs": 1,
        "query": "result"
    });
    let discovery_response = serde_json::json!({
        "hits": [
            { "entityId": 999999, "name": "stale" },
            { "entityId": board_id, "name": "stale board" },
            { "entityId": tag_id, "name": "stale tag" }
        ],
        "offset": 0,
        "limit": 120,
        "estimatedTotalHits": 3,
        "processingTimeMs": 1,
        "query": "study"
    });
    let meili = Router::new()
        .route(
            "/indexes/identity_users/search",
            post(move || {
                let response = meili_response.clone();
                async move { Json(response) }
            }),
        )
        .route(
            "/indexes/forum_discovery/search",
            post(move || {
                let response = discovery_response.clone();
                async move { Json(response) }
            }),
        );
    let listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind fake meilisearch");
    let address = listener.local_addr().expect("fake meilisearch address");
    let server = tokio::spawn(async move {
        axum::serve(listener, meili).await.expect("serve fake meilisearch");
    });

    let app = search::routes(AppState {
        db: pool.clone(),
        config: shared::Config::from_env().expect("test config"),
        jwt_secret: "integration-test-secret-32bytes!".into(),
        jwt_ttl: 900,
        refresh_ttl: 604_800,
        meili_url: format!("http://{address}"),
        meili_master_key: String::new(),
        redis: None,
        system_private_key: vec![0; 32],
        system_public_key_b64: String::new(),
        email_encryption: None,
        captcha_verifier: None,
        sse_tx: None,
    });
    let authenticated = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/search?q=result&type=user&limit=30")
                .header(header::AUTHORIZATION, format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .expect("build authenticated search"),
        )
        .await
        .expect("authenticated search response");
    assert_eq!(authenticated.status(), StatusCode::OK);
    let authenticated = read_json(authenticated).await;
    let users = authenticated["users"].as_array().expect("user results");
    assert_eq!(users.len(), 2);
    assert_eq!(users[0]["id"], campus_id.to_string());
    assert_eq!(users[1]["id"], public_id.to_string());
    assert_eq!(users[1]["following"], true);
    assert_eq!(users[1]["followerCount"], 1);
    assert!(authenticated["courses"].as_array().expect("course results").is_empty());
    assert!(authenticated["boards"].as_array().expect("board results").is_empty());

    let anonymous = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/search?q=result&type=user&limit=30")
                .body(Body::empty())
                .expect("build anonymous search"),
        )
        .await
        .expect("anonymous search response");
    assert_eq!(anonymous.status(), StatusCode::OK);
    let anonymous = read_json(anonymous).await;
    let anonymous_users = anonymous["users"].as_array().expect("anonymous users");
    assert_eq!(anonymous_users.len(), 3);
    assert_eq!(anonymous_users[0]["id"], public_id.to_string());
    assert_eq!(anonymous_users[1]["id"], muted_id.to_string());
    assert_eq!(anonymous_users[2]["id"], blocked_id.to_string());

    let board_search = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/search?q=study&type=board")
                .body(Body::empty())
                .expect("build board search"),
        )
        .await
        .expect("board search response");
    assert_eq!(board_search.status(), StatusCode::OK);
    let board_search = read_json(board_search).await;
    assert_eq!(board_search["boards"].as_array().expect("boards").len(), 1);
    assert_eq!(board_search["boards"][0]["id"], board_id.to_string());
    assert_eq!(board_search["boards"][0]["name"], "Study Search");

    let tag_search = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/search?q=study&type=tag")
                .body(Body::empty())
                .expect("build tag search"),
        )
        .await
        .expect("tag search response");
    assert_eq!(tag_search.status(), StatusCode::OK);
    let tag_search = read_json(tag_search).await;
    assert_eq!(tag_search["tags"].as_array().expect("tags").len(), 1);
    assert_eq!(tag_search["tags"][0]["id"], tag_id.to_string());
    assert_eq!(tag_search["tags"][0]["name"], "Study Tag");
    server.abort();
}

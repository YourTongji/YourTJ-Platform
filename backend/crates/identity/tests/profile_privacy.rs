//! Handler-to-database coverage for owner profile and privacy controls.

#[path = "helpers/mod.rs"]
mod helpers;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn request(method: Method, uri: &str, token: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .expect("profile request")
}

#[tokio::test]
async fn profile_and_privacy_are_validated_and_persisted() {
    let (pool, _) = helpers::create_test_app().await;
    let email = "profile-privacy@tongji.edu.cn";
    sqlx::query("INSERT INTO identity.accounts (email, handle) VALUES ($1, 'profile-privacy')")
        .bind(email)
        .execute(&pool)
        .await
        .expect("seed profile account");
    let (token, account_id) = helpers::create_access_token_for(email, &pool).await;
    let app = helpers::create_test_app_with_pool(pool.clone()).await;

    let default_profile_response = app
        .clone()
        .oneshot(request(Method::GET, "/api/v2/me/profile", &token, None))
        .await
        .expect("default profile response");
    assert_eq!(default_profile_response.status(), StatusCode::OK);
    let default_profile = helpers::read_json(default_profile_response).await;
    assert_eq!(default_profile["school"], "同济大学");

    let default_response = app
        .clone()
        .oneshot(request(Method::GET, "/api/v2/me/privacy", &token, None))
        .await
        .expect("default privacy response");
    assert_eq!(default_response.status(), StatusCode::OK);
    let defaults = helpers::read_json(default_response).await;
    assert_eq!(defaults["profileVisibility"], "campus");
    assert_eq!(defaults["activityVisibility"], "only_me");
    assert_eq!(defaults["followersVisibility"], "followers");
    assert_eq!(defaults["followingVisibility"], "followers");
    assert_eq!(defaults["dmPolicy"], "following");
    assert_eq!(defaults["mentionPolicy"], "everyone");
    assert_eq!(defaults["discoverable"], true);

    let profile_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/profile",
            &token,
            Some(json!({
                "displayName": "  Campus Builder  ",
                "school": "  同济大学嘉定校区  ",
                "bio": "Shipping a safer community.",
                "website": "https://example.test/about"
            })),
        ))
        .await
        .expect("profile update response");
    assert_eq!(profile_response.status(), StatusCode::OK);
    let profile = helpers::read_json(profile_response).await;
    assert_eq!(profile["accountId"], account_id.to_string());
    assert_eq!(profile["displayName"], "Campus Builder");
    assert_eq!(profile["school"], "同济大学嘉定校区");
    assert_eq!(profile["website"], "https://example.test/about");
    assert!(profile.get("email").is_none());

    let invalid_website = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/profile",
            &token,
            Some(json!({
                "displayName": null,
                "bio": null,
                "website": "http://tracking.example.test/avatar"
            })),
        ))
        .await
        .expect("invalid website response");
    assert_eq!(invalid_website.status(), StatusCode::BAD_REQUEST);
    let stored_website: Option<String> =
        sqlx::query_scalar("SELECT website FROM identity.profiles WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .expect("stored profile website");
    assert_eq!(stored_website.as_deref(), Some("https://example.test/about"));

    let rolling_profile_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/profile",
            &token,
            Some(json!({
                "displayName": "Campus Builder",
                "bio": "Updated without the additive school field.",
                "website": "https://example.test/about"
            })),
        ))
        .await
        .expect("rolling profile update response");
    assert_eq!(rolling_profile_response.status(), StatusCode::OK);
    let rolling_profile = helpers::read_json(rolling_profile_response).await;
    assert_eq!(rolling_profile["school"], "同济大学嘉定校区");

    let empty_school_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/profile",
            &token,
            Some(json!({
                "displayName": "Campus Builder",
                "school": "   ",
                "bio": null,
                "website": null
            })),
        ))
        .await
        .expect("empty school response");
    assert_eq!(empty_school_response.status(), StatusCode::BAD_REQUEST);

    let privacy_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/privacy",
            &token,
            Some(json!({
                "profileVisibility": "only_me",
                "activityVisibility": "public",
                "followersVisibility": "only_me",
                "followingVisibility": "campus",
                "discoverable": false,
                "dmPolicy": "nobody",
                "mentionPolicy": "following"
            })),
        ))
        .await
        .expect("privacy update response");
    assert_eq!(privacy_response.status(), StatusCode::OK);
    let privacy = helpers::read_json(privacy_response).await;
    assert_eq!(privacy["activityVisibility"], "public");
    assert_eq!(privacy["mentionPolicy"], "following");

    let rolling_client_response = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/privacy",
            &token,
            Some(json!({
                "profileVisibility": "campus",
                "followersVisibility": "campus",
                "followingVisibility": "campus",
                "discoverable": true,
                "dmPolicy": "following"
            })),
        ))
        .await
        .expect("rolling client privacy response");
    assert_eq!(rolling_client_response.status(), StatusCode::OK);
    let rolling_policy = helpers::read_json(rolling_client_response).await;
    assert_eq!(rolling_policy["activityVisibility"], "public");
    assert_eq!(rolling_policy["mentionPolicy"], "following");

    let invalid_privacy = app
        .clone()
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/privacy",
            &token,
            Some(json!({
                "profileVisibility": "public",
                "activityVisibility": "friends",
                "followersVisibility": "campus",
                "followingVisibility": "campus",
                "discoverable": true,
                "dmPolicy": "everyone",
                "mentionPolicy": "everyone"
            })),
        ))
        .await
        .expect("invalid privacy response");
    assert_eq!(invalid_privacy.status(), StatusCode::BAD_REQUEST);
    let invalid_mention = app
        .oneshot(request(
            Method::PUT,
            "/api/v2/me/privacy",
            &token,
            Some(json!({
                "profileVisibility": "public",
                "activityVisibility": "only_me",
                "followersVisibility": "campus",
                "followingVisibility": "campus",
                "discoverable": true,
                "dmPolicy": "everyone",
                "mentionPolicy": "mutuals"
            })),
        ))
        .await
        .expect("invalid mention policy response");
    assert_eq!(invalid_mention.status(), StatusCode::BAD_REQUEST);
    let stored_policy: (String, String, bool, String, String) = sqlx::query_as(
        "SELECT profile_visibility, activity_visibility, discoverable, dm_policy, mention_policy \
         FROM identity.profile_privacy WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_one(&pool)
    .await
    .expect("stored privacy policy");
    assert_eq!(
        stored_policy,
        ("campus".into(), "public".into(), true, "following".into(), "following".into(),)
    );
}

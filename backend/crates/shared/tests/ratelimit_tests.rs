mod helpers;

use helpers::try_connect;

#[tokio::test]
async fn test_token_bucket_allows_under_limit() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    // Clean up.
    let mut conn = pool.get().await.unwrap();
    let _: () =
        redis::cmd("DEL").arg("rl:test_bucket:test_user").query_async(&mut conn).await.unwrap();

    // First 3 requests should pass (max_tokens = 5).
    for _ in 0..3 {
        shared::ratelimit::check_token_bucket(Some(&pool), "test_bucket", "test_user", 5, 60)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn test_token_bucket_blocks_over_limit() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    // Clean up.
    let mut conn = pool.get().await.unwrap();
    let _: () =
        redis::cmd("DEL").arg("rl:test_block:test_block").query_async(&mut conn).await.unwrap();

    // Exhaust the bucket (max 2).
    for _ in 0..2 {
        shared::ratelimit::check_token_bucket(Some(&pool), "test_block", "test_block", 2, 60)
            .await
            .unwrap();
    }

    // Third should fail.
    let err = shared::ratelimit::check_token_bucket(Some(&pool), "test_block", "test_block", 2, 60)
        .await
        .unwrap_err();
    assert!(matches!(err, shared::AppError::RateLimited));
}

#[tokio::test]
async fn test_token_bucket_window_resets() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    let bucket = "test_window";
    let id = "test_window_user";

    // Clean up.
    let mut conn = pool.get().await.unwrap();
    let _: () =
        redis::cmd("DEL").arg(format!("rl:{bucket}:{id}")).query_async(&mut conn).await.unwrap();

    // Set a key with a short window manually.
    let _: () = redis::cmd("SETEX")
        .arg(format!("rl:{bucket}:{id}"))
        .arg(1) // 1-second TTL
        .arg(3) // already at 3
        .query_async(&mut conn)
        .await
        .unwrap();

    // Increment should go to 4, which exceeds max_tokens=3.
    let err =
        shared::ratelimit::check_token_bucket(Some(&pool), bucket, id, 3, 60).await.unwrap_err();
    assert!(matches!(err, shared::AppError::RateLimited));

    // Wait for the key to expire.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Now the key should be gone, so INCR starts at 2 (INCR on expired = 1 on non-existent,
    // then EXPIRE sets window). Actually after expiry, INCR returns 1 and EXPIRE sets it.
    // Let's just verify it doesn't return RateLimited.
    shared::ratelimit::check_token_bucket(Some(&pool), bucket, id, 3, 60).await.unwrap();
}

#[tokio::test]
async fn test_no_redis_returns_ok() {
    // Passing None should always succeed.
    shared::ratelimit::check_token_bucket(None, "any", "any", 1, 1).await.unwrap();
}

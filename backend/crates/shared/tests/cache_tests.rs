mod helpers;

use helpers::try_connect;

#[tokio::test]
async fn test_bump_version_increments() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    let prefix = "test_cache_bump_version";
    let id = "test_bump_v";

    // Version should be 0 initially.
    let mut conn = pool.get().await.unwrap();
    let _v0: Option<i64> =
        redis::cmd("GET").arg(format!("ver:{prefix}:{id}")).query_async(&mut conn).await.unwrap();
    // Reset — delete existing key.
    let _: () =
        redis::cmd("DEL").arg(format!("ver:{prefix}:{id}")).query_async(&mut conn).await.unwrap();

    // Bump once.
    let v1 = shared::cache::bump_version(&pool, prefix, id).await.unwrap();
    assert_eq!(v1, 1);

    // Bump again.
    let v2 = shared::cache::bump_version(&pool, prefix, id).await.unwrap();
    assert_eq!(v2, 2);
}

#[tokio::test]
async fn test_set_and_get_cached() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    let prefix = "test_cache_set_get";
    let id = "test_item";
    // Clean up.
    let mut conn = pool.get().await.unwrap();
    let _: () = redis::cmd("DEL")
        .arg(format!("ver:{prefix}:{id}"))
        .arg(format!("{prefix}:{id}:v1"))
        .arg(format!("{prefix}:{id}:v2"))
        .query_async(&mut conn)
        .await
        .unwrap();

    // Set a value.
    shared::cache::set_cached(&pool, prefix, id, r#"{"hello":"world"}"#, 300).await.unwrap();

    // Read it back.
    let val = shared::cache::get_cached(&pool, prefix, id).await.unwrap();
    assert_eq!(val.as_deref(), Some(r#"{"hello":"world"}"#));
}

#[tokio::test]
async fn test_get_cached_miss_returns_none() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    let val = shared::cache::get_cached(&pool, "test_miss", "nonexistent").await.unwrap();
    assert!(val.is_none());
}

#[tokio::test]
async fn test_set_cached_ttl_expires() {
    let pool = match try_connect().await {
        Some(p) => p,
        None => return,
    };

    let prefix = "test_cache_ttl";
    let id = "ttl_item";
    let mut conn = pool.get().await.unwrap();
    let _: () =
        redis::cmd("DEL").arg(format!("ver:{prefix}:{id}")).query_async(&mut conn).await.unwrap();

    // Set with 1-second TTL.
    shared::cache::set_cached(&pool, prefix, id, "will-expire", 1).await.unwrap();

    // Read immediately — should be present.
    let val = shared::cache::get_cached(&pool, prefix, id).await.unwrap();
    assert!(val.is_some());

    // Wait 2 seconds.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Should now be expired.
    let val2 = shared::cache::get_cached(&pool, prefix, id).await.unwrap();
    assert!(val2.is_none());
}

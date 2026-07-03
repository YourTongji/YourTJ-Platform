/// Compute hot rank scores and batch-store in Redis ZSET (single round-trip).
pub async fn refresh_hot_rank(
    pool: &deadpool_redis::Pool,
    db: &sqlx::PgPool,
) -> anyhow::Result<()> {
    let threads = sqlx::query_as::<_, (i64, i32, i32)>(
        "SELECT id, vote_count, reply_count FROM forum.threads WHERE status = 'normal'",
    )
    .fetch_all(db)
    .await?;

    if threads.is_empty() {
        return Ok(());
    }

    let mut conn = pool.get().await?;
    let mut cmd = redis::cmd("ZADD");
    cmd.arg("hot:threads");
    for (id, vote_count, reply_count) in &threads {
        let score = (*vote_count as f64) * 0.7 + (*reply_count as f64) * 0.3;
        cmd.arg(score).arg(*id);
    }
    cmd.query_async::<()>(&mut conn).await?;
    Ok(())
}

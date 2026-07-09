//! Watched word filter: AhoCorasick matcher loaded from DB, hot-reloaded via Redis.
//!
//! The matcher is stored in an `ArcSwap<AhoCorasick>` so reads are wait-free.
//! Admin mutations bump a Redis version key; readers detect the change and reload.

use std::sync::Arc;

use aho_corasick::AhoCorasick;
use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use sqlx::PgPool;

use shared::AppResult;

#[allow(dead_code)]
static WATCHED_WORDS_MATCHER: Lazy<ArcSwap<AhoCorasick>> = Lazy::new(|| {
    ArcSwap::from_pointee(
        AhoCorasick::new(&[] as &[&str]).expect("empty pattern set must be valid"),
    )
});

#[allow(dead_code)]
static WATCHED_WORDS_ACTIONS: Lazy<ArcSwap<std::collections::HashMap<String, String>>> =
    Lazy::new(|| ArcSwap::from_pointee(std::collections::HashMap::new()));

/// Load all watched words from DB and rebuild the matcher.
#[allow(dead_code)]
pub async fn reload_watched_words(pool: &PgPool) -> AppResult<()> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT word, action FROM forum.watched_words ORDER BY id")
            .fetch_all(pool)
            .await?;

    let patterns: Vec<String> = rows.iter().map(|(w, _)| w.clone()).collect();
    let actions: std::collections::HashMap<String, String> = rows.into_iter().collect();

    let matcher = AhoCorasick::new(&patterns).map_err(|e| {
        shared::AppError::Internal(anyhow::anyhow!("failed to build AhoCorasick: {e}"))
    })?;

    WATCHED_WORDS_MATCHER.store(Arc::new(matcher));
    WATCHED_WORDS_ACTIONS.store(Arc::new(actions));

    Ok(())
}

/// Initialize the matcher at startup. Should be called in bootstrap.
#[allow(dead_code)]
pub async fn init_watched_words(pool: &PgPool) {
    if let Err(e) = reload_watched_words(pool).await {
        tracing::warn!(error = %e, "failed to load watched words at startup");
    }
}

/// Check text against watched words. Returns the action if a match is found.
/// Used before writing posts.
#[allow(dead_code)]
pub fn check_watched_words(text: &str) -> Option<(String, String)> {
    let matcher = WATCHED_WORDS_MATCHER.load();
    let actions = WATCHED_WORDS_ACTIONS.load();

    for m in matcher.find_iter(text) {
        let matched = &text[m.start()..m.end()];
        if let Some(action) = actions.get(matched) {
            return Some((matched.to_lowercase(), action.clone()));
        }
    }
    None
}

/// Replace censored words with ▇ in text for censor action.
#[allow(dead_code)]
pub fn censor_text(text: &str) -> String {
    let matcher = WATCHED_WORDS_MATCHER.load();
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;
    for m in matcher.find_iter(text) {
        result.push_str(&text[last_end..m.start()]);
        for _ in 0..(m.end() - m.start()) {
            result.push('\u{2587}');
        }
        last_end = m.end();
    }
    result.push_str(&text[last_end..]);
    result
}

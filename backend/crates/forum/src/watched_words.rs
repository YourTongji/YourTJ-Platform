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

/// Canonical text and the resulting moderation visibility decision.
pub(crate) struct ModeratedText {
    pub canonical: String,
    pub is_queued: bool,
}

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

fn canonicalize_censor_ranges(text: &str, mut ranges: Vec<(usize, usize)>) -> String {
    ranges.sort_unstable_by_key(|(start, end)| (*start, std::cmp::Reverse(*end)));
    let mut merged_ranges: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        if let Some((_, merged_end)) = merged_ranges.last_mut() {
            if start <= *merged_end {
                *merged_end = (*merged_end).max(end);
                continue;
            }
        }
        merged_ranges.push((start, end));
    }

    let mut canonical = String::with_capacity(text.len());
    let mut last_end = 0;
    for (start, end) in merged_ranges {
        canonical.push_str(&text[last_end..start]);
        let character_count = text[start..end].chars().count();
        for _ in 0..character_count {
            canonical.push('\u{2587}');
        }
        last_end = end;
    }
    canonical.push_str(&text[last_end..]);
    canonical
}

/// Apply all watched-word actions with `block > queue > censor` precedence.
///
/// Only `censor` matches are replaced. A queued value keeps its canonical text
/// for staff review but must not enter public projections.
pub(crate) fn moderate_text(text: &str) -> AppResult<ModeratedText> {
    let matcher = WATCHED_WORDS_MATCHER.load();
    let actions = WATCHED_WORDS_ACTIONS.load();
    let matches = matcher
        .find_overlapping_iter(text)
        .filter_map(|matched| {
            actions
                .get(&text[matched.start()..matched.end()])
                .map(|action| (matched.start(), matched.end(), action.as_str()))
        })
        .collect::<Vec<_>>();

    if matches.iter().any(|(_, _, action)| *action == "block") {
        return Err(shared::AppError::BadRequest("content contains blocked words".into()));
    }

    let is_queued = matches.iter().any(|(_, _, action)| *action == "queue");
    let censor_ranges = matches
        .iter()
        .filter_map(|(start, end, action)| (*action == "censor").then_some((*start, *end)))
        .collect::<Vec<_>>();
    if censor_ranges.is_empty() {
        return Ok(ModeratedText { canonical: text.to_owned(), is_queued });
    }

    Ok(ModeratedText { canonical: canonicalize_censor_ranges(text, censor_ranges), is_queued })
}

#[cfg(test)]
mod tests {
    use super::canonicalize_censor_ranges;

    #[test]
    fn overlapping_censor_matches_hide_the_full_union() {
        let canonical = canonicalize_censor_ranges("abcdef", vec![(0, 3), (1, 5)]);

        assert_eq!(canonical, "▇▇▇▇▇f");
    }
}

//! Canonical validation and watched-word policy for public forum content.

use std::collections::HashSet;

use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use shared::{AppError, AppResult};

use crate::dto::{CommentInput, CommentUpdateInput, ContentFormat, ThreadInput, ThreadUpdateInput};

static MENTION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@([\p{L}\p{N}_.-]+)").expect("mention regex is static"));

/// A validated mutation together with the visibility decision produced by policy.
pub(crate) struct PreparedContent<T> {
    pub input: T,
    pub is_queued: bool,
}

fn validate_title(title: &str) -> AppResult<()> {
    let title_length = title.chars().count();
    if title.trim().is_empty() || title_length > 200 {
        return Err(AppError::BadRequest("title must be 1–200 characters".into()));
    }
    Ok(())
}

fn validate_thread_body(body: Option<&str>, format: ContentFormat) -> AppResult<()> {
    if body.is_some_and(|body| body.chars().count() > 50_000) {
        return Err(AppError::BadRequest("body must not exceed 50000 characters".into()));
    }
    if let Some(body) = body {
        validate_format_profile(body, format, 4_000, 20)?;
    }
    Ok(())
}

fn validate_comment_body(body: &str, format: ContentFormat) -> AppResult<()> {
    let body_length = body.chars().count();
    if body.trim().is_empty() || body_length > 16_000 {
        return Err(AppError::BadRequest("body must be 1–16000 characters".into()));
    }
    validate_format_profile(body, format, 1_600, 8)?;
    Ok(())
}

fn markdown_options() -> Options {
    Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS
}

fn validate_format_profile(
    body: &str,
    format: ContentFormat,
    max_events: usize,
    max_links: usize,
) -> AppResult<()> {
    if format == ContentFormat::PlainV1 {
        return Ok(());
    }

    let mut event_count = 0usize;
    let mut depth = 0usize;
    let mut link_count = 0usize;
    for event in Parser::new_ext(body, markdown_options()) {
        event_count += 1;
        if event_count > max_events {
            return Err(AppError::BadRequest("Markdown document is too complex".into()));
        }
        match event {
            Event::Start(tag) => {
                depth += 1;
                if depth > 32 {
                    return Err(AppError::BadRequest("Markdown nesting is too deep".into()));
                }
                match tag {
                    Tag::Link { dest_url, .. } => {
                        link_count += 1;
                        if link_count > max_links {
                            return Err(AppError::BadRequest(
                                "Markdown contains too many links".into(),
                            ));
                        }
                        validate_link_destination(&dest_url)?;
                    }
                    Tag::Image { .. } => {
                        return Err(AppError::BadRequest(
                            "Markdown images require a bound platform asset".into(),
                        ));
                    }
                    _ => {}
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Html(_) | Event::InlineHtml(_) => {
                return Err(AppError::BadRequest("raw HTML is not allowed in Markdown".into()));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_link_destination(destination: &str) -> AppResult<()> {
    if destination.starts_with('#')
        || (destination.starts_with('/') && !destination.starts_with("//"))
    {
        return Ok(());
    }
    let uri = destination
        .parse::<axum::http::Uri>()
        .map_err(|_| AppError::BadRequest("invalid Markdown link".into()))?;
    if matches!(uri.scheme_str(), Some("http" | "https")) && uri.host().is_some() {
        return Ok(());
    }
    Err(AppError::BadRequest("Markdown links must be internal paths or HTTP(S) URLs".into()))
}

/// Build bounded plain text for search and notification projections.
pub(crate) fn plain_text_projection(body: &str, format: ContentFormat, max_chars: usize) -> String {
    if format == ContentFormat::PlainV1 {
        return body.chars().take(max_chars).collect();
    }
    let mut output = String::new();
    let mut output_chars = 0usize;
    let mut code_depth = 0usize;
    for event in Parser::new_ext(body, markdown_options()) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_depth = code_depth.saturating_sub(1),
            Event::Text(text) if code_depth == 0 => {
                if !output.is_empty() && !output.ends_with(char::is_whitespace) {
                    output.push(' ');
                    output_chars += 1;
                }
                for character in text.chars().take(max_chars.saturating_sub(output_chars)) {
                    output.push(character);
                    output_chars += 1;
                }
            }
            Event::SoftBreak | Event::HardBreak if code_depth == 0 && output_chars < max_chars => {
                output.push(' ');
                output_chars += 1;
            }
            _ => {}
        }
        if output_chars >= max_chars {
            break;
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(max_chars).collect()
}

/// Extract mention handles only from visible text nodes, excluding Markdown code.
pub(crate) fn mention_handles(body: &str, format: ContentFormat, own_handle: &str) -> Vec<String> {
    let visible_text = if format == ContentFormat::PlainV1 {
        body.to_owned()
    } else {
        plain_text_projection(body, format, body.chars().count())
    };
    let mut seen = HashSet::new();
    MENTION_PATTERN
        .captures_iter(&visible_text)
        .map(|capture| capture[1].to_owned())
        .filter(|handle| {
            (3..=30).contains(&handle.len())
                && handle.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
                })
        })
        .filter(|handle| !handle.eq_ignore_ascii_case(own_handle) && seen.insert(handle.clone()))
        .take(10)
        .collect()
}

fn normalize_and_validate_tags(tags: &mut Option<Vec<String>>) -> AppResult<()> {
    let Some(tags) = tags else {
        return Ok(());
    };
    if tags.len() > 3 {
        return Err(AppError::BadRequest("tags must not contain more than 3 items".into()));
    }

    let mut unique_tags = HashSet::new();
    for tag in tags.iter_mut() {
        *tag = tag.trim().to_owned();
        if tag.is_empty() || tag.chars().count() > 64 {
            return Err(AppError::BadRequest("each tag must be 1–64 characters".into()));
        }
        if !unique_tags.insert(tag.clone()) {
            return Err(AppError::BadRequest("tags must be unique".into()));
        }
    }
    Ok(())
}

fn moderate_field(value: &mut String, is_queued: &mut bool) -> AppResult<()> {
    let moderated = crate::watched_words::moderate_text(value)?;
    *value = moderated.canonical;
    *is_queued |= moderated.is_queued;
    Ok(())
}

fn moderate_optional_field(value: &mut Option<String>, is_queued: &mut bool) -> AppResult<()> {
    if let Some(value) = value {
        moderate_field(value, is_queued)?;
    }
    Ok(())
}

/// Validate and canonicalize a new thread before any database write.
pub(crate) fn prepare_thread_create(
    mut input: ThreadInput,
) -> AppResult<PreparedContent<ThreadInput>> {
    input.title = input.title.trim().to_owned();
    validate_title(&input.title)?;
    validate_thread_body(input.body.as_deref(), input.content_format)?;
    normalize_and_validate_tags(&mut input.tags)?;

    if let Some(poll) = input.poll.as_mut() {
        poll.question = poll.question.trim().to_owned();
        let question_length = poll.question.chars().count();
        if poll.question.is_empty() || question_length > 500 {
            return Err(AppError::BadRequest("poll question must be 1–500 characters".into()));
        }
        if !(2..=20).contains(&poll.options.len()) {
            return Err(AppError::BadRequest("poll requires 2–20 options".into()));
        }
        let mut normalized_options = HashSet::new();
        for option in &mut poll.options {
            *option = option.trim().to_owned();
            let normalized = option.to_lowercase();
            if option.is_empty() || option.chars().count() > 200 {
                return Err(AppError::BadRequest("poll options must be 1–200 characters".into()));
            }
            if !normalized_options.insert(normalized) {
                return Err(AppError::BadRequest("poll options must be unique".into()));
            }
        }
        if let Some(closes_at) = poll.closes_at {
            if chrono::DateTime::from_timestamp(closes_at, 0).is_none()
                || closes_at <= chrono::Utc::now().timestamp()
            {
                return Err(AppError::BadRequest(
                    "poll closesAt must be a future timestamp".into(),
                ));
            }
        }
    }

    let mut is_queued = false;
    moderate_field(&mut input.title, &mut is_queued)?;
    moderate_optional_field(&mut input.body, &mut is_queued)?;
    if let Some(poll) = input.poll.as_mut() {
        moderate_field(&mut poll.question, &mut is_queued)?;
        let mut canonical_options = HashSet::new();
        for option in &mut poll.options {
            moderate_field(option, &mut is_queued)?;
            if !canonical_options.insert(option.to_lowercase()) {
                return Err(AppError::BadRequest(
                    "poll options must remain unique after moderation".into(),
                ));
            }
        }
    }

    Ok(PreparedContent { input, is_queued })
}

/// Validate and canonicalize the fields supplied by a thread edit.
pub(crate) fn prepare_thread_update(
    mut input: ThreadUpdateInput,
) -> AppResult<PreparedContent<ThreadUpdateInput>> {
    if input.title.is_none() && input.body.is_none() && input.tags.is_none() {
        return Err(AppError::BadRequest(
            "at least one of title, body, or tags is required".into(),
        ));
    }
    if let Some(title) = input.title.as_mut() {
        *title = title.trim().to_owned();
        validate_title(title)?;
    }
    if input.content_format.is_some() && input.body.is_none() {
        return Err(AppError::BadRequest(
            "contentFormat can only change together with body".into(),
        ));
    }
    if input.body.is_some() && input.content_format.is_none() {
        input.content_format = Some(ContentFormat::PlainV1);
    }
    validate_thread_body(
        input.body.as_deref(),
        input.content_format.unwrap_or(ContentFormat::PlainV1),
    )?;
    normalize_and_validate_tags(&mut input.tags)?;

    let mut is_queued = false;
    moderate_optional_field(&mut input.title, &mut is_queued)?;
    moderate_optional_field(&mut input.body, &mut is_queued)?;
    Ok(PreparedContent { input, is_queued })
}

/// Validate and canonicalize a new comment before any database write.
pub(crate) fn prepare_comment_create(
    mut input: CommentInput,
) -> AppResult<PreparedContent<CommentInput>> {
    validate_comment_body(&input.body, input.content_format)?;
    let mut is_queued = false;
    moderate_field(&mut input.body, &mut is_queued)?;
    Ok(PreparedContent { input, is_queued })
}

/// Validate and canonicalize a comment edit using the same rules as creation.
pub(crate) fn prepare_comment_update(
    mut input: CommentUpdateInput,
) -> AppResult<PreparedContent<CommentUpdateInput>> {
    validate_comment_body(&input.body, input.content_format)?;
    let mut is_queued = false;
    moderate_field(&mut input.body, &mut is_queued)?;
    Ok(PreparedContent { input, is_queued })
}

#[cfg(test)]
mod tests {
    use super::{mention_handles, plain_text_projection, validate_format_profile};
    use crate::dto::ContentFormat;

    #[test]
    fn markdown_profile_rejects_html_images_and_unsafe_links() {
        assert!(validate_format_profile(
            "<script>alert(1)</script>",
            ContentFormat::MarkdownV1,
            50,
            5
        )
        .is_err());
        assert!(validate_format_profile(
            "![alt](https://example.com/a.png)",
            ContentFormat::MarkdownV1,
            50,
            5
        )
        .is_err());
        assert!(validate_format_profile(
            "[bad](javascript:alert(1))",
            ContentFormat::MarkdownV1,
            50,
            5
        )
        .is_err());
        assert!(validate_format_profile(
            "[safe](/forum/threads/1)",
            ContentFormat::MarkdownV1,
            50,
            5
        )
        .is_ok());
    }

    #[test]
    fn markdown_projection_and_mentions_ignore_markup_and_code() {
        let source =
            "Hello **@alice.name** @OWNER `@inline`\n\n```rust\n@block\n```\n[同济](/courses)";
        assert_eq!(
            plain_text_projection(source, ContentFormat::MarkdownV1, 100),
            "Hello @alice.name @OWNER 同济"
        );
        assert_eq!(mention_handles(source, ContentFormat::MarkdownV1, "owner"), vec!["alice.name"]);
    }
}

//! Canonical validation and watched-word policy for public forum content.

use std::collections::HashSet;

use shared::{AppError, AppResult};

use crate::dto::{CommentInput, CommentUpdateInput, ThreadInput, ThreadUpdateInput};

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

fn validate_thread_body(body: Option<&str>) -> AppResult<()> {
    if body.is_some_and(|body| body.chars().count() > 50_000) {
        return Err(AppError::BadRequest("body must not exceed 50000 characters".into()));
    }
    Ok(())
}

fn validate_comment_body(body: &str) -> AppResult<()> {
    let body_length = body.chars().count();
    if body.trim().is_empty() || body_length > 16_000 {
        return Err(AppError::BadRequest("body must be 1–16000 characters".into()));
    }
    Ok(())
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
    validate_thread_body(input.body.as_deref())?;
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
    if let Some(title) = input.title.as_mut() {
        *title = title.trim().to_owned();
        validate_title(title)?;
    }
    validate_thread_body(input.body.as_deref())?;
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
    validate_comment_body(&input.body)?;
    let mut is_queued = false;
    moderate_field(&mut input.body, &mut is_queued)?;
    Ok(PreparedContent { input, is_queued })
}

/// Validate and canonicalize a comment edit using the same rules as creation.
pub(crate) fn prepare_comment_update(
    mut input: CommentUpdateInput,
) -> AppResult<PreparedContent<CommentUpdateInput>> {
    validate_comment_body(&input.body)?;
    let mut is_queued = false;
    moderate_field(&mut input.body, &mut is_queued)?;
    Ok(PreparedContent { input, is_queued })
}

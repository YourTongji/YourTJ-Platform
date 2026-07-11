//! Canonical, privacy-safe presentation metadata for federated search results.

use std::collections::HashMap;

use serde::Serialize;

/// One character range in a canonical result field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHighlightRangeDto {
    pub start: usize,
    pub end: usize,
}

/// Highlight ranges for one field of one visible, owner-rehydrated search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHighlightDto {
    pub scope: String,
    pub id: String,
    pub field: String,
    pub ranges: Vec<SearchHighlightRangeDto>,
}

pub(crate) struct SearchQuality {
    pub highlights: Vec<SearchHighlightDto>,
    pub suggested_query: Option<String>,
}

struct SearchField<'a> {
    scope: &'static str,
    id: &'a str,
    field: &'static str,
    value: &'a str,
}

#[allow(clippy::too_many_arguments)] // reason: each slice is one typed federated result section.
pub(crate) fn build_search_quality(
    query: &str,
    courses: &[courses::public_search::CourseSearchHit],
    reviews: &[reviews::search::ReviewSearchHit],
    threads: &[forum::meili::ForumThreadDoc],
    users: &[forum::discovery::UserSearchHit],
    boards: &[forum::discovery::BoardSearchHit],
    tags: &[forum::discovery::TagSearchHit],
) -> SearchQuality {
    let mut fields = Vec::new();
    for course in courses {
        push_field(&mut fields, "course", &course.id, "name", &course.name);
        push_field(&mut fields, "course", &course.id, "code", &course.code);
        if let Some(teacher_name) = course.teacher_name.as_deref() {
            push_field(&mut fields, "course", &course.id, "teacherName", teacher_name);
        }
        if let Some(department) = course.department.as_deref() {
            push_field(&mut fields, "course", &course.id, "department", department);
        }
    }
    for review in reviews {
        push_field(&mut fields, "review", &review.id, "courseName", &review.course_name);
        if let Some(comment) = review.comment.as_deref() {
            push_field(&mut fields, "review", &review.id, "comment", comment);
        }
    }
    for thread in threads {
        push_field(&mut fields, "thread", &thread.id, "title", &thread.title);
        push_field(&mut fields, "thread", &thread.id, "bodyExcerpt", &thread.body_excerpt);
        push_field(&mut fields, "thread", &thread.id, "board", &thread.board);
        push_field(&mut fields, "thread", &thread.id, "authorHandle", &thread.author_handle);
    }
    for user in users {
        push_field(&mut fields, "user", &user.id, "handle", &user.handle);
        if let Some(display_name) = user.display_name.as_deref() {
            push_field(&mut fields, "user", &user.id, "displayName", display_name);
        }
    }
    for board in boards {
        push_field(&mut fields, "board", &board.id, "name", &board.name);
        push_field(&mut fields, "board", &board.id, "slug", &board.slug);
        if let Some(description) = board.description.as_deref() {
            push_field(&mut fields, "board", &board.id, "description", description);
        }
    }
    for tag in tags {
        push_field(&mut fields, "tag", &tag.id, "name", &tag.name);
        push_field(&mut fields, "tag", &tag.id, "slug", &tag.slug);
        if let Some(description) = tag.description.as_deref() {
            push_field(&mut fields, "tag", &tag.id, "description", description);
        }
    }

    let suggested_query = suggest_query(query, fields.iter().map(|field| field.value));
    let mut terms = highlight_terms(query);
    if let Some(suggestion) = suggested_query.as_deref() {
        terms.extend(highlight_terms(suggestion));
        terms.sort();
        terms.dedup();
    }
    let highlights = fields
        .into_iter()
        .filter_map(|field| {
            let ranges = highlight_ranges(field.value, &terms);
            (!ranges.is_empty()).then(|| SearchHighlightDto {
                scope: field.scope.into(),
                id: field.id.into(),
                field: field.field.into(),
                ranges,
            })
        })
        .collect();

    SearchQuality { highlights, suggested_query }
}

fn push_field<'a>(
    fields: &mut Vec<SearchField<'a>>,
    scope: &'static str,
    id: &'a str,
    field: &'static str,
    value: &'a str,
) {
    if !value.is_empty() {
        fields.push(SearchField { scope, id, field, value });
    }
}

fn highlight_terms(value: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    for character in value.chars().chain(std::iter::once(' ')) {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if current.chars().count() >= 2 {
            terms.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn folded_chars(value: &str) -> (Vec<char>, Vec<usize>) {
    let mut folded = Vec::new();
    let mut original_indices = Vec::new();
    for (original_index, character) in value.chars().enumerate() {
        for folded_character in character.to_lowercase() {
            folded.push(folded_character);
            original_indices.push(original_index);
        }
    }
    (folded, original_indices)
}

fn highlight_ranges(value: &str, terms: &[String]) -> Vec<SearchHighlightRangeDto> {
    let (haystack, original_indices) = folded_chars(value);
    let mut candidates = Vec::new();
    for term in terms {
        let needle = term.chars().collect::<Vec<_>>();
        if needle.is_empty() || needle.len() > haystack.len() {
            continue;
        }
        for start in 0..=haystack.len() - needle.len() {
            if haystack[start..start + needle.len()] != needle {
                continue;
            }
            let original_start = original_indices[start];
            let original_end = original_indices[start + needle.len() - 1] + 1;
            candidates.push(SearchHighlightRangeDto { start: original_start, end: original_end });
        }
    }
    candidates.sort_by_key(|range| (range.start, std::cmp::Reverse(range.end)));
    let mut ranges: Vec<SearchHighlightRangeDto> = Vec::new();
    for candidate in candidates {
        if ranges.last().is_some_and(|previous| candidate.start < previous.end) {
            continue;
        }
        ranges.push(candidate);
        if ranges.len() == 8 {
            break;
        }
    }
    ranges
}

#[derive(Debug)]
struct AsciiToken {
    start: usize,
    end: usize,
    normalized: String,
}

fn ascii_tokens(value: &str) -> Vec<AsciiToken> {
    let bytes = value.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_alphabetic() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
            index += 1;
        }
        tokens.push(AsciiToken {
            start,
            end: index,
            normalized: value[start..index].to_ascii_lowercase(),
        });
    }
    tokens
}

fn suggest_query<'a>(query: &str, values: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut corpus: HashMap<String, (String, usize)> = HashMap::new();
    for value in values {
        for token in ascii_tokens(value) {
            let display = value[token.start..token.end].to_owned();
            corpus.entry(token.normalized).and_modify(|entry| entry.1 += 1).or_insert((display, 1));
        }
    }
    let query_tokens = ascii_tokens(query);
    let mut replacements = Vec::new();
    for token in query_tokens {
        if token.normalized.len() < 3 || corpus.contains_key(&token.normalized) {
            continue;
        }
        let max_distance = if token.normalized.len() >= 7 { 2 } else { 1 };
        let mut matches = corpus
            .iter()
            .filter_map(|(candidate, (display, count))| {
                let length_difference = candidate.len().abs_diff(token.normalized.len());
                if length_difference > max_distance {
                    return None;
                }
                let distance = levenshtein(&token.normalized, candidate);
                (distance <= max_distance).then_some((distance, *count, candidate, display))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.2.cmp(right.2))
        });
        let Some(best) = matches.first() else {
            continue;
        };
        if matches.get(1).is_some_and(|other| other.0 == best.0 && other.1 == best.1) {
            continue;
        }
        replacements.push((token.start, token.end, best.3.clone()));
    }
    if replacements.is_empty() {
        return None;
    }
    let mut suggestion = String::with_capacity(query.len());
    let mut previous_end = 0;
    for (start, end, replacement) in replacements {
        suggestion.push_str(&query[previous_end..start]);
        suggestion.push_str(&replacement);
        previous_end = end;
    }
    suggestion.push_str(&query[previous_end..]);
    (suggestion != query).then_some(suggestion)
}

fn levenshtein(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    for (left_index, left_character) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(right_chars.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_character) in right_chars.iter().enumerate() {
            current.push(
                (previous[right_index + 1] + 1)
                    .min(current[right_index] + 1)
                    .min(previous[right_index] + usize::from(left_character != *right_character)),
            );
        }
        previous = current;
    }
    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::{highlight_ranges, highlight_terms, levenshtein, suggest_query};

    #[test]
    fn highlights_use_unicode_character_offsets_without_html() {
        let ranges = highlight_ranges("一起学算法与 Algorithm", &highlight_terms("算法 algorithm"));
        assert_eq!(ranges.len(), 2);
        assert_eq!((ranges[0].start, ranges[0].end), (3, 5));
        assert_eq!((ranges[1].start, ranges[1].end), (7, 16));
    }

    #[test]
    fn suggestions_are_derived_only_from_unambiguous_visible_canonical_words() {
        assert_eq!(
            suggest_query("algoritm design", ["Algorithm Design", "Algorithm course"].into_iter()),
            Some("Algorithm design".into())
        );
        assert_eq!(suggest_query("cat", ["cut", "cot"].into_iter()), None);
        assert_eq!(suggest_query("private", std::iter::empty()), None);
        assert_eq!(levenshtein("algoritm", "algorithm"), 1);
    }
}

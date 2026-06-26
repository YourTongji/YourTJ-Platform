//! Pinyin helpers for Meilisearch indexing. Converts Chinese course/teacher names
//! into pinyin and initials for fuzzy search. Also builds a composite
//! `search_keywords` column with original text, pinyin, and initials.

use pinyin::ToPinyin;
use sqlx::PgPool;

use crate::error::CoursesError;

/// Compute (full_pinyin, initials) for a Chinese string. Non-Chinese characters
/// are lowercased and treated as-is. The full pinyin is space-separated syllables;
/// initials are space-separated first letters.
pub fn to_pinyin(chinese: &str) -> (String, String) {
    let mut full: Vec<String> = Vec::new();
    let mut initials: Vec<String> = Vec::new();

    for ch in chinese.chars() {
        if let Some(py) = ch.to_pinyin() {
            full.push(py.plain().to_string());
            initials.push(py.first_letter().to_string());
        } else {
            // Non-Chinese: lowercase if ASCII alpha, keep as-is otherwise
            let lower: String = ch.to_lowercase().collect();
            full.push(lower.clone());
            initials.push(lower);
        }
    }

    (full.join(" "), initials.join(""))
}

/// Build the `search_keywords` text for a course: original name + pinyin + initials.
fn build_search_keywords(name: &str) -> String {
    let (pinyin, initials) = to_pinyin(name);
    format!("{name} {pinyin} {initials}")
}

/// Compute pinyin/initials/search_keywords for a single course and UPDATE the row.
/// This is idempotent — safe to call repeatedly.
pub async fn sync_course_pinyin(pool: &PgPool, course_id: i64) -> Result<(), CoursesError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM courses.courses WHERE id = $1")
        .bind(course_id)
        .fetch_optional(pool)
        .await?;

    let (name,) = match row {
        Some(r) => r,
        None => return Err(CoursesError::CourseNotFound),
    };

    let (name_pinyin, name_initials) = to_pinyin(&name);
    let search_keywords = build_search_keywords(&name);

    sqlx::query(
        "UPDATE courses.courses \
         SET name_pinyin = $2, name_initials = $3, search_keywords = $4 \
         WHERE id = $1",
    )
    .bind(course_id)
    .bind(&name_pinyin)
    .bind(&name_initials)
    .bind(&search_keywords)
    .execute(pool)
    .await?;

    Ok(())
}

/// Batch-compute pinyin for every course that has a `NULL` name_pinyin column.
/// Designed to run as a background maintenance job after bulk imports.
pub async fn sync_all_courses_pinyin(pool: &PgPool) -> Result<u64, CoursesError> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM courses.courses WHERE name_pinyin IS NULL")
            .fetch_all(pool)
            .await?;

    let mut count = 0u64;
    for (id, name) in &rows {
        let (name_pinyin, name_initials) = to_pinyin(name);
        let search_keywords = build_search_keywords(name);

        sqlx::query(
            "UPDATE courses.courses \
             SET name_pinyin = $2, name_initials = $3, search_keywords = $4 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(&name_pinyin)
        .bind(&name_initials)
        .bind(&search_keywords)
        .execute(pool)
        .await?;

        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinyin_simple() {
        let (full, initials) = to_pinyin("数据结构");
        assert!(!full.is_empty());
        assert!(!initials.is_empty());
    }

    #[test]
    fn pinyin_ascii_fallback() {
        let (full, initials) = to_pinyin("CS101");
        assert!(full.contains("c"));
        assert!(initials.contains("c"));
    }

    #[test]
    fn pinyin_mixed() {
        let (full, initials) = to_pinyin("大学计算机");
        assert!(!full.is_empty());
        assert!(!initials.is_empty());
    }
}

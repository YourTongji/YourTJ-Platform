//! Repository layer for the courses domain — all functions accept a `&PgPool`
//! and return concrete types from `crate::models`. Errors are mapped through
//! `CoursesError` → `AppError` in the handler layer.

use sqlx::PgPool;

use crate::error::CoursesError;
use crate::models::{DepartmentRow, TeacherRow};

/// A virtual row product of the JOIN query used by `find_course_by_id` and `find_course_by_code`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CourseWithTeacherRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_id: Option<i64>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
    pub name_pinyin: Option<String>,
    pub name_initials: Option<String>,
    pub search_keywords: Option<String>,
    pub teacher_name: Option<String>,
}

// Reusable column list for CourseWithTeacherRow queries (avoids column mismatch
// with `SELECT c.*` which would pick up `is_legacy` / `is_icu`).
const COURSE_COLS: &str = "c.id, c.code, c.name, c.credit, c.department, c.teacher_id, \
     c.review_count, CASE WHEN c.review_count > 0 THEN c.review_avg END AS review_avg, \
     c.name_pinyin, c.name_initials, c.search_keywords";

const LIST_COURSE_SQL_BASE: &str = " \
     SELECT {course_cols}, t.name AS teacher_name \
     FROM courses.courses c \
     LEFT JOIN courses.teachers t ON c.teacher_id = t.id \
    ";

/// Compose a concrete SELECT by substituting the column list.
fn list_course_sql() -> String {
    LIST_COURSE_SQL_BASE.replace("{course_cols}", COURSE_COLS)
}

/// List distinct, non-null departments from `courses.courses`.
pub async fn list_departments(pool: &PgPool) -> Result<Vec<DepartmentRow>, CoursesError> {
    let rows = sqlx::query_as::<_, DepartmentRow>(
        "SELECT DISTINCT department FROM courses.courses WHERE department IS NOT NULL ORDER BY department",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Cursor-paginated course listing. The cursor is a base64-encoded `id`; when
/// provided, rows with a strictly lower `id` are returned (for `id DESC` sorts).
/// `limit` is capped at 100.
pub async fn list_courses(
    pool: &PgPool,
    dept: Option<&str>,
    sort: &str,
    cursor: Option<i64>,
    limit: i64,
) -> Result<(Vec<CourseWithTeacherRow>, Option<i64>), CoursesError> {
    let limit = limit.min(100);
    let fetch_limit = limit + 1; // fetch one extra to detect has_more

    let base_sql = list_course_sql();

    let rows: Vec<CourseWithTeacherRow> = match sort {
        "hot" => {
            if let (Some(dept), Some(cursor)) = (dept, cursor) {
                let sql = format!("{base_sql} WHERE c.department = $1 AND c.id < $2 ORDER BY c.review_count DESC, c.id DESC LIMIT $3");
                sqlx::query_as(&sql)
                    .bind(dept)
                    .bind(cursor)
                    .bind(fetch_limit)
                    .fetch_all(pool)
                    .await?
            } else if let Some(dept) = dept {
                let sql = format!("{base_sql} WHERE c.department = $1 ORDER BY c.review_count DESC, c.id DESC LIMIT $2");
                sqlx::query_as(&sql).bind(dept).bind(fetch_limit).fetch_all(pool).await?
            } else if let Some(cursor) = cursor {
                let sql = format!(
                    "{base_sql} WHERE c.id < $1 ORDER BY c.review_count DESC, c.id DESC LIMIT $2"
                );
                sqlx::query_as(&sql).bind(cursor).bind(fetch_limit).fetch_all(pool).await?
            } else {
                let sql = format!("{base_sql} ORDER BY c.review_count DESC, c.id DESC LIMIT $1");
                sqlx::query_as(&sql).bind(fetch_limit).fetch_all(pool).await?
            }
        }
        "rating" => {
            if let (Some(dept), Some(cursor)) = (dept, cursor) {
                let sql = format!("{base_sql} WHERE c.department = $1 AND c.id < $2 ORDER BY c.review_avg DESC NULLS LAST, c.id DESC LIMIT $3");
                sqlx::query_as(&sql)
                    .bind(dept)
                    .bind(cursor)
                    .bind(fetch_limit)
                    .fetch_all(pool)
                    .await?
            } else if let Some(dept) = dept {
                let sql = format!("{base_sql} WHERE c.department = $1 ORDER BY c.review_avg DESC NULLS LAST, c.id DESC LIMIT $2");
                sqlx::query_as(&sql).bind(dept).bind(fetch_limit).fetch_all(pool).await?
            } else if let Some(cursor) = cursor {
                let sql = format!("{base_sql} WHERE c.id < $1 ORDER BY c.review_avg DESC NULLS LAST, c.id DESC LIMIT $2");
                sqlx::query_as(&sql).bind(cursor).bind(fetch_limit).fetch_all(pool).await?
            } else {
                let sql =
                    format!("{base_sql} ORDER BY c.review_avg DESC NULLS LAST, c.id DESC LIMIT $1");
                sqlx::query_as(&sql).bind(fetch_limit).fetch_all(pool).await?
            }
        }
        _ => {
            // "new" — default
            if let (Some(dept), Some(cursor)) = (dept, cursor) {
                let sql = format!(
                    "{base_sql} WHERE c.department = $1 AND c.id < $2 ORDER BY c.id DESC LIMIT $3"
                );
                sqlx::query_as(&sql)
                    .bind(dept)
                    .bind(cursor)
                    .bind(fetch_limit)
                    .fetch_all(pool)
                    .await?
            } else if let Some(dept) = dept {
                let sql = format!("{base_sql} WHERE c.department = $1 ORDER BY c.id DESC LIMIT $2");
                sqlx::query_as(&sql).bind(dept).bind(fetch_limit).fetch_all(pool).await?
            } else if let Some(cursor) = cursor {
                let sql = format!("{base_sql} WHERE c.id < $1 ORDER BY c.id DESC LIMIT $2");
                sqlx::query_as(&sql).bind(cursor).bind(fetch_limit).fetch_all(pool).await?
            } else {
                let sql = format!("{base_sql} ORDER BY c.id DESC LIMIT $1");
                sqlx::query_as(&sql).bind(fetch_limit).fetch_all(pool).await?
            }
        }
    };

    let has_more = rows.len() > limit as usize;
    let next_cursor = if has_more { rows.get(limit as usize - 1).map(|r| r.id) } else { None };

    let items: Vec<CourseWithTeacherRow> =
        if has_more { rows.into_iter().take(limit as usize).collect() } else { rows };

    Ok((items, next_cursor))
}

/// Look up a single course by primary key, including teacher name.
pub async fn find_course_by_id(
    pool: &PgPool,
    id: i64,
) -> Result<Option<CourseWithTeacherRow>, CoursesError> {
    let base_sql = list_course_sql();
    let sql = format!("{base_sql} WHERE c.id = $1");
    let row = sqlx::query_as::<_, CourseWithTeacherRow>(&sql).bind(id).fetch_optional(pool).await?;
    Ok(row)
}

/// Look up a single course by its unique code, including teacher name.
pub async fn find_course_by_code(
    pool: &PgPool,
    code: &str,
) -> Result<Option<CourseWithTeacherRow>, CoursesError> {
    let base_sql = list_course_sql();
    let sql = format!("{base_sql} WHERE c.code = $1");
    let row =
        sqlx::query_as::<_, CourseWithTeacherRow>(&sql).bind(code).fetch_optional(pool).await?;
    Ok(row)
}

/// Related courses: same department or same teacher, excluding the given
/// course, ordered by review count descending, limited to 5.
pub async fn list_related_courses(
    pool: &PgPool,
    id: i64,
) -> Result<Vec<CourseWithTeacherRow>, CoursesError> {
    let base_sql = list_course_sql();
    let sql = format!(
        "{base_sql} \
         WHERE c.id != $1 \
           AND (c.department = (SELECT department FROM courses.courses WHERE id = $2) \
                OR c.teacher_id = (SELECT teacher_id FROM courses.courses WHERE id = $3)) \
         ORDER BY c.review_count DESC \
         LIMIT 5"
    );
    let rows = sqlx::query_as::<_, CourseWithTeacherRow>(&sql)
        .bind(id)
        .bind(id)
        .bind(id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// All aliases for a course.
pub async fn find_aliases(pool: &PgPool, course_id: i64) -> Result<Vec<String>, CoursesError> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT alias FROM courses.course_aliases WHERE course_id = $1")
            .bind(course_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(a,)| a).collect())
}

/// All teachers for a course — uses explicit column list to avoid the `tid` column
/// that exists in the DB but not in our struct.
pub async fn find_teachers_by_course(
    pool: &PgPool,
    course_id: i64,
) -> Result<Vec<TeacherRow>, CoursesError> {
    let rows = sqlx::query_as::<_, TeacherRow>(
        "SELECT t.id, t.name, t.title, t.department, t.name_pinyin, t.name_initials \
         FROM courses.teachers t \
         JOIN courses.courses c ON c.teacher_id = t.id \
         WHERE c.id = $1",
    )
    .bind(course_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

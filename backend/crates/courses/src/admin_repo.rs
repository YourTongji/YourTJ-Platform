//! Repository layer for admin course CRUD.
//!
//! These functions provide a wide admin view over courses, including teacher
//! name joins and dynamic updates.

use shared::Page;
use sqlx::FromRow;
use sqlx::PgPool;

use crate::error::CoursesError;

/// Raw row returned by admin course queries.
#[derive(Debug, Clone, FromRow)]
pub(super) struct AdminCourseRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub teacher_name: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
}

/// Raw row for a RETURNING clause that does not join teacher_name.
#[derive(Debug, Clone, FromRow)]
pub(super) struct AdminBareCourseRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub credit: Option<f64>,
    pub department: Option<String>,
    pub review_count: i32,
    pub review_avg: Option<f64>,
}

/// Raw row for teacher upsert lookups.
#[derive(Debug, Clone, FromRow)]
struct TeacherIdRow {
    pub id: i64,
}

/// List all courses with teacher name, cursor-paginated by id ascending.
pub async fn admin_list_courses(
    pool: &PgPool,
    cursor: Option<i64>,
    limit: i64,
) -> Result<Page<AdminCourseRow>, CoursesError> {
    let limit = limit.min(100);
    let fetch_limit = limit + 1; // fetch one extra to detect has_more

    let since_id = cursor.unwrap_or(0);
    let rows = sqlx::query_as::<_, AdminCourseRow>(
        "SELECT c.id, c.code, c.name, c.credit, c.department, \
         t.name AS teacher_name, c.review_count, c.review_avg \
         FROM courses.courses c \
         LEFT JOIN courses.teachers t ON c.teacher_id = t.id \
         WHERE c.id > $1 \
         ORDER BY c.id \
         LIMIT $2",
    )
    .bind(since_id)
    .bind(fetch_limit)
    .fetch_all(pool)
    .await?;

    let has_more = rows.len() > limit as usize;
    let items: Vec<AdminCourseRow> =
        if has_more { rows.into_iter().take(limit as usize).collect() } else { rows };
    let next_cursor = items.last().map(|r| r.id.to_string());

    Ok(Page::new(items, next_cursor))
}

/// Upsert a teacher by name and return the id.
async fn upsert_teacher(pool: &PgPool, name: &str) -> Result<i64, CoursesError> {
    // INSERT ... ON CONFLICT DO NOTHING, then SELECT id
    let inserted = sqlx::query_as::<_, TeacherIdRow>(
        "INSERT INTO courses.teachers (name) VALUES ($1) \
         ON CONFLICT DO NOTHING \
         RETURNING id",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = inserted {
        Ok(row.id)
    } else {
        let row =
            sqlx::query_as::<_, TeacherIdRow>("SELECT id FROM courses.teachers WHERE name = $1")
                .bind(name)
                .fetch_one(pool)
                .await?;
        Ok(row.id)
    }
}

/// Create a new course and return the created row.
pub async fn admin_create_course(
    pool: &PgPool,
    code: &str,
    name: &str,
    credit: Option<f64>,
    department: Option<&str>,
    teacher_name: Option<&str>,
) -> Result<AdminBareCourseRow, CoursesError> {
    let teacher_id: Option<i64> =
        if let Some(tn) = teacher_name { Some(upsert_teacher(pool, tn).await?) } else { None };

    let row = sqlx::query_as::<_, AdminBareCourseRow>(
        "INSERT INTO courses.courses (code, name, credit, department, teacher_id) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, code, name, credit, department, \
         0 AS review_count, 0.0::float8 AS review_avg",
    )
    .bind(code)
    .bind(name)
    .bind(credit)
    .bind(department)
    .bind(teacher_id)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Update fields of a course dynamically. Only non-None fields are updated.
/// Returns the updated row, or `None` if the course was not found.
pub async fn admin_update_course(
    pool: &PgPool,
    id: i64,
    code: Option<&str>,
    name: Option<&str>,
    credit: Option<f64>,
    department: Option<&str>,
    teacher_name: Option<&str>,
) -> Result<Option<AdminBareCourseRow>, CoursesError> {
    let mut set_clauses = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    let mut idx = 0u32;

    if let Some(v) = code {
        idx += 1;
        set_clauses.push(format!("code = ${idx}"));
        bind_values.push(v.to_string());
    }
    if let Some(v) = name {
        idx += 1;
        set_clauses.push(format!("name = ${idx}"));
        bind_values.push(v.to_string());
    }
    if let Some(v) = credit {
        idx += 1;
        set_clauses.push(format!("credit = ${idx}"));
        bind_values.push(v.to_string());
    }
    if let Some(v) = department {
        idx += 1;
        set_clauses.push(format!("department = ${idx}"));
        bind_values.push(v.to_string());
    }
    if let Some(tn) = teacher_name {
        let teacher_id = upsert_teacher(pool, tn).await?;
        idx += 1;
        set_clauses.push(format!("teacher_id = ${idx}"));
        bind_values.push(teacher_id.to_string());
    }

    if set_clauses.is_empty() {
        // No fields to update — just fetch current
        let row = sqlx::query_as::<_, AdminBareCourseRow>(
            "SELECT id, code, name, credit, department, review_count, review_avg \
             FROM courses.courses WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        return Ok(row);
    }

    let parts: Vec<&str> = set_clauses.iter().map(|s| s.as_str()).collect();
    let mut sql = String::from("UPDATE courses.courses SET ");
    sql.push_str(&parts.join(", "));
    idx += 1;
    sql.push_str(&format!(
        " WHERE id = ${idx} RETURNING id, code, name, credit, department, review_count, review_avg"
    ));

    let mut q = sqlx::query_as::<_, AdminBareCourseRow>(&sql);
    for val in &bind_values {
        q = q.bind(val);
    }
    let row = q.bind(id).fetch_optional(pool).await?;
    Ok(row)
}

/// Look up the teacher name for a course, if any.
pub async fn find_teacher_name_by_course(
    pool: &PgPool,
    course_id: i64,
) -> Result<Option<String>, CoursesError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT t.name FROM courses.teachers t \
         JOIN courses.courses c ON c.teacher_id = t.id WHERE c.id = $1",
    )
    .bind(course_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(n,)| n))
}

/// Delete a course by id. Returns `true` if a row was deleted.
pub async fn admin_delete_course(pool: &PgPool, id: i64) -> Result<bool, CoursesError> {
    let rows = sqlx::query("DELETE FROM courses.courses WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

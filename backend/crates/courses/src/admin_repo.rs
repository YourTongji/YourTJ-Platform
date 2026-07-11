//! Repository layer for admin course CRUD.
//!
//! These functions provide a wide admin view over courses, including teacher
//! name joins and dynamic updates.

use shared::Page;
use sqlx::FromRow;
use sqlx::{PgConnection, PgPool};

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
async fn find_or_create_teacher(tx: &mut PgConnection, name: &str) -> Result<i64, CoursesError> {
    let existing = sqlx::query_as::<_, TeacherIdRow>(
        "SELECT id FROM courses.teachers WHERE lower(name) = lower($1) ORDER BY id LIMIT 1",
    )
    .bind(name)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(row) = existing {
        Ok(row.id)
    } else {
        let row = sqlx::query_as::<_, TeacherIdRow>(
            "INSERT INTO courses.teachers (name) VALUES ($1) RETURNING id",
        )
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;
        Ok(row.id)
    }
}

/// Create a new course and return the created row.
pub async fn admin_create_course(
    tx: &mut PgConnection,
    code: &str,
    name: &str,
    credit: Option<f64>,
    department: Option<&str>,
    teacher_name: Option<&str>,
) -> Result<AdminBareCourseRow, CoursesError> {
    let teacher_id: Option<i64> = if let Some(teacher_name) = teacher_name {
        Some(find_or_create_teacher(tx, teacher_name).await?)
    } else {
        None
    };

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
    .fetch_one(tx)
    .await?;

    Ok(row)
}

/// Update fields of a course dynamically. Only non-None fields are updated.
/// Returns the updated row, or `None` if the course was not found.
pub async fn admin_update_course(
    tx: &mut PgConnection,
    id: i64,
    code: Option<&str>,
    name: Option<&str>,
    credit: Option<f64>,
    department: Option<&str>,
    teacher_name: Option<&str>,
) -> Result<Option<AdminBareCourseRow>, CoursesError> {
    let teacher_id = if let Some(teacher_name) = teacher_name {
        Some(find_or_create_teacher(tx, teacher_name).await?)
    } else {
        None
    };
    let row = sqlx::query_as::<_, AdminBareCourseRow>(
        "UPDATE courses.courses SET \
           code = COALESCE($1, code), name = COALESCE($2, name), \
           credit = COALESCE($3, credit), department = COALESCE($4, department), \
           teacher_id = COALESCE($5, teacher_id) \
         WHERE id = $6 \
         RETURNING id, code, name, credit, department, review_count, review_avg",
    )
    .bind(code)
    .bind(name)
    .bind(credit)
    .bind(department)
    .bind(teacher_id)
    .bind(id)
    .fetch_optional(tx)
    .await?;
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
pub async fn admin_delete_course(tx: &mut PgConnection, id: i64) -> Result<bool, CoursesError> {
    sqlx::query("DELETE FROM courses.course_aliases WHERE course_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let result =
        sqlx::query("DELETE FROM courses.courses WHERE id = $1").bind(id).execute(&mut *tx).await;
    let rows = match result {
        Ok(result) => result.rows_affected(),
        Err(sqlx::Error::Database(error))
            if error.code().as_deref() == Some("23503")
                && error.constraint() == Some("reviews_course_id_fkey") =>
        {
            return Err(CoursesError::CourseHasReviews);
        }
        Err(error) => return Err(error.into()),
    };
    Ok(rows > 0)
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::admin_delete_course;
    use crate::error::CoursesError;

    static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

    #[tokio::test]
    async fn hidden_review_fk_maps_course_deletion_to_conflict_error() {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&database_url).await.expect("course integrity database");
        MIGRATOR.run(&pool).await.expect("course integrity migrations");
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let account_id: i64 = sqlx::query_scalar(
            "INSERT INTO identity.accounts (email, handle) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("course-integrity-{suffix}@tongji.edu.cn"))
        .bind(format!("course-integrity-{suffix}"))
        .fetch_one(&pool)
        .await
        .expect("seed course integrity account");
        let course_id: i64 = sqlx::query_scalar(
            "INSERT INTO courses.courses (code, name, review_count) \
             VALUES ($1, 'Course deletion guard', 0) RETURNING id",
        )
        .bind(format!("COURSE-INTEGRITY-{suffix}"))
        .fetch_one(&pool)
        .await
        .expect("seed guarded course");
        sqlx::query(
            "INSERT INTO reviews.reviews (course_id, account_id, rating, status) \
             VALUES ($1, $2, 3, 'hidden')",
        )
        .bind(course_id)
        .bind(account_id)
        .execute(&pool)
        .await
        .expect("seed hidden review");

        let mut tx = pool.begin().await.expect("begin guarded deletion");
        let result = admin_delete_course(&mut tx, course_id).await;
        assert!(matches!(result, Err(CoursesError::CourseHasReviews)));
        tx.rollback().await.expect("rollback rejected deletion");

        let still_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM courses.courses WHERE id = $1)")
                .bind(course_id)
                .fetch_one(&pool)
                .await
                .expect("guarded course still exists");
        assert!(still_exists);

        sqlx::query("DELETE FROM reviews.reviews WHERE course_id = $1")
            .bind(course_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM courses.courses WHERE id = $1")
            .bind(course_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM identity.accounts WHERE id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .ok();
    }
}

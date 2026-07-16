//! Shared test helpers for courses integration tests.

use sqlx::PgPool;
use std::env;

/// Connect to the database from DATABASE_URL, or return None if not configured.
pub async fn try_connect() -> Option<PgPool> {
    let url = env::var("DATABASE_URL").ok()?;
    if url.is_empty() {
        return None;
    }
    let pool = PgPool::connect(&url).await.ok()?;
    Some(pool)
}

/// Seed minimal test data into the courses schema. Idempotent — uses ON CONFLICT
/// DO NOTHING so repeated calls are safe. Tables use GENERATED ALWAYS AS IDENTITY
/// so we use OVERRIDING SYSTEM VALUE for explicit primary-key inserts.
#[allow(dead_code)]
pub async fn seed_courses_data(pool: &PgPool) {
    // Insert teachers
    sqlx::query(
        r#"
        INSERT INTO courses.teachers (id, name, title, department)
        OVERRIDING SYSTEM VALUE
        VALUES (1, '张老师', '教授', '计算机科学与技术系')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO courses.teachers (id, name, title, department)
        OVERRIDING SYSTEM VALUE
        VALUES (2, '李老师', '副教授', '数学系')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    // Insert courses
    sqlx::query(
        r#"
        INSERT INTO courses.courses (id, code, name, credit, department, teacher_id, review_count, review_avg)
        OVERRIDING SYSTEM VALUE
        VALUES
            (1, 'CS101', '数据结构', 4.0, '计算机科学与技术系', 1, 42, 4.5),
            (2, 'CS102', '操作系统', 3.0, '计算机科学与技术系', 1, 38, 4.2),
            (3, 'MATH201', '高等数学', 5.0, '数学系', 2, 55, 4.8)
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    // Insert aliases
    sqlx::query(
        r#"
        INSERT INTO courses.course_aliases (course_id, alias)
        VALUES (1, 'DS'), (1, '数据结构与算法')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();
}

/// Seed minimal test data into the selection schema.
#[allow(dead_code)]
pub async fn seed_selection_data(pool: &PgPool) {
    sqlx::query(
        r#"
        INSERT INTO selection.calendars (id, name, is_current)
        VALUES (1, '2024-2025 第一学期', true)
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.campuses (id, name)
        VALUES (1, '四平路校区'), (2, '嘉定校区')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.faculties (id, name, campus_id)
        VALUES (1, '电子与信息工程学院', 2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.majors (id, name, faculty_id, grade)
        VALUES (1, '计算机科学与技术', 1, '2024')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.course_natures (id, name)
        VALUES (1, '必修'), (2, '选修')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.courses (
            id, code, teaching_class_code, name, credit, nature_id, calendar_id,
            campus_id, teacher_name, teacher_names, start_week, end_week,
            weeks_unknown, schedule_unknown
        )
        VALUES (
            1, 'SEL101', 'SEL101.01', '选课测试课', 2.0, 1, 1, 1,
            '李老师', ARRAY['李老师'], 1, 16, false, false
        )
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.major_courses (major_id, course_id, grade)
        VALUES (1, 1, '2024')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        r#"
        INSERT INTO selection.timeslots (
            course_id, teacher_name, weekday, start_slot, end_slot, weeks,
            week_numbers, weeks_unknown, location, location_unknown
        )
        VALUES (
            1, '李老师', 1, 1, 2, '1-16',
            ARRAY(SELECT generate_series(1, 16)), false, '四平路校区 南楼101', false
        )
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await
    .ok();
}

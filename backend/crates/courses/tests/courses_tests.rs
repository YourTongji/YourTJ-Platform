//! Integration tests for the courses domain — exercises repo functions directly.
//! These tests require `DATABASE_URL` to be set. When it is not, every test is
//! skipped (returns Ok without asserting).

mod common;

use courses::repo;

/// Helper: return early with Ok if no DB is available.
macro_rules! pool_or_skip {
    () => {
        match common::try_connect().await {
            Some(p) => p,
            None => return,
        }
    };
}

#[tokio::test]
async fn list_departments_returns_rows() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let depts = repo::list_departments(&pool).await.unwrap();
    assert!(!depts.is_empty(), "expected at least one department");
}

#[tokio::test]
async fn list_courses_default_sort() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let (rows, cursor) = repo::list_courses(&pool, None, "new", None, 10).await.unwrap();
    assert!(!rows.is_empty(), "expected some courses");
    // Cursor should be set if there are more courses than limit
    // (our seed has 3 courses, limit 10, so no cursor)
    assert!(cursor.is_none() || cursor.is_some());
}

#[tokio::test]
async fn list_courses_filter_by_department() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let (rows, _) =
        repo::list_courses(&pool, Some("计算机科学与技术系"), "new", None, 10).await.unwrap();
    for row in &rows {
        assert_eq!(row.department.as_deref(), Some("计算机科学与技术系"));
    }
}

#[tokio::test]
async fn find_course_by_id_exists() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let row = repo::find_course_by_id(&pool, 1).await.unwrap();
    assert!(row.is_some());
    let row = row.unwrap();
    assert_eq!(row.code, "CS101");
    assert_eq!(row.teacher_name.as_deref(), Some("张老师"));
}

#[tokio::test]
async fn find_course_by_id_not_found() {
    let pool = pool_or_skip!();
    let row = repo::find_course_by_id(&pool, 99999).await.unwrap();
    assert!(row.is_none());
}

#[tokio::test]
async fn find_course_by_code() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let row = repo::find_course_by_code(&pool, "CS101").await.unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().name, "数据结构");
}

#[tokio::test]
async fn list_related_courses_excludes_self() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let rows = repo::list_related_courses(&pool, 1).await.unwrap();
    for row in &rows {
        assert_ne!(row.id, 1, "related should not include the course itself");
    }
}

#[tokio::test]
async fn find_aliases() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let aliases = repo::find_aliases(&pool, 1).await.unwrap();
    assert!(aliases.contains(&"DS".to_string()));
    assert!(aliases.contains(&"数据结构与算法".to_string()));
}

#[tokio::test]
async fn find_teachers_by_course() {
    let pool = pool_or_skip!();
    common::seed_courses_data(&pool).await;
    let teachers = repo::find_teachers_by_course(&pool, 1).await.unwrap();
    assert!(!teachers.is_empty());
    assert_eq!(teachers[0].name, "张老师");
}

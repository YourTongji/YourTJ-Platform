//! Integration tests for the selection (选课) domain — exercises repo functions
//! directly. These tests require `DATABASE_URL` to be set. When it is not, every
//! test is skipped.

mod common;

use courses::selection_repo;

macro_rules! pool_or_skip {
    () => {
        match common::try_connect().await {
            Some(p) => p,
            None => return,
        }
    };
}

#[tokio::test]
async fn list_calendars() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_calendars(&pool).await.unwrap();
    assert!(!rows.is_empty());
    // Current should be first
}

#[tokio::test]
async fn list_campuses() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_campuses(&pool).await.unwrap();
    assert!(rows.len() >= 2);
}

#[tokio::test]
async fn list_faculties() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_faculties(&pool).await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn list_grades() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let grades = selection_repo::list_grades(&pool, 1).await.unwrap();
    assert!(grades.contains(&"2024".to_string()));
}

#[tokio::test]
async fn list_majors() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_majors(&pool, "2024").await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn list_course_natures() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_course_natures(&pool).await.unwrap();
    assert!(rows.len() >= 2);
}

#[tokio::test]
async fn list_courses_by_major() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_courses_by_major(&pool, 1, "2024").await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].code, "SEL101");
}

#[tokio::test]
async fn list_courses_by_nature() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_courses_by_nature(&pool, 1).await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
async fn find_selection_course_by_code() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let row = selection_repo::find_selection_course_by_code(&pool, "SEL101").await.unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().name, "选课测试课");
}

// search_selection_courses was moved from DB ILIKE to Meilisearch
// (courses::meili::search_selection_courses). Integration tests for
// the Meilisearch path require a running Meilisearch instance and are
// covered by end-to-end testing.

#[tokio::test]
async fn list_timeslots() {
    let pool = pool_or_skip!();
    common::seed_selection_data(&pool).await;
    let rows = selection_repo::list_timeslots(&pool, 1).await.unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0].weekday, 1);
}

#[tokio::test]
async fn find_latest_update_none_when_empty() {
    let pool = pool_or_skip!();
    // Don't seed fetchlog — should return None
    let result = selection_repo::find_latest_update(&pool).await.unwrap();
    assert!(result.is_none());
}

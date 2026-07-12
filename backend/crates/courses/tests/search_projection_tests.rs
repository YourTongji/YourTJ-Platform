//! Integration tests for database-backed course search result projection.

mod common;

#[tokio::test]
async fn course_projection_preserves_candidate_rank_and_uses_numeric_ids() {
    let Some(pool) = common::try_connect().await else {
        return;
    };
    common::seed_courses_data(&pool).await;

    let hits = courses::public_search::load_course_hits(&pool, &[3, 999_999, 1, 3], 10)
        .await
        .expect("load course search hits");

    assert_eq!(hits.iter().map(|hit| hit.id.as_str()).collect::<Vec<_>>(), ["3", "1"]);
    assert_eq!(hits[0].code, "MATH201");
    assert_eq!(hits[1].teacher_name.as_deref(), Some("张老师"));
}

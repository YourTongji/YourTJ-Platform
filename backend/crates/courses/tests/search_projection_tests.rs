//! Integration tests for database-backed course search result projection.

mod common;

use std::time::Duration;

use meilisearch_sdk::client::Client;

#[tokio::test]
async fn course_projection_preserves_candidate_rank_and_uses_numeric_ids() {
    let Some(pool) = common::try_connect().await else {
        return;
    };
    let fixture_base = (uuid::Uuid::new_v4().as_u128() & ((1_u128 << 62) - 1)) as i64 + 1;
    let teacher_id = fixture_base;
    let ranked_id = fixture_base + 1;
    let teacher_course_id = fixture_base + 2;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let teacher_name = format!("Projection teacher {suffix}");
    sqlx::query("INSERT INTO courses.teachers (id, name) OVERRIDING SYSTEM VALUE VALUES ($1, $2)")
        .bind(teacher_id)
        .bind(&teacher_name)
        .execute(&pool)
        .await
        .expect("seed projection teacher");
    let ranked_code = format!("PROJECTION-MATH-{suffix}");
    sqlx::query(
        "INSERT INTO courses.courses (id, code, name) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, 'Projection ranked')",
    )
    .bind(ranked_id)
    .bind(&ranked_code)
    .execute(&pool)
    .await
    .expect("seed ranked projection course");
    let teacher_code = format!("PROJECTION-CS-{suffix}");
    sqlx::query(
        "INSERT INTO courses.courses (id, code, name, teacher_id) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, 'Projection teacher course', $3)",
    )
    .bind(teacher_course_id)
    .bind(&teacher_code)
    .bind(teacher_id)
    .execute(&pool)
    .await
    .expect("seed teacher projection course");

    let hits = courses::public_search::load_course_hits(
        &pool,
        &[ranked_id, i64::MAX, teacher_course_id, ranked_id],
        10,
    )
    .await
    .expect("load course search hits");

    assert_eq!(
        hits.iter()
            .map(|hit| hit.id.parse::<i64>().expect("numeric course id"))
            .collect::<Vec<_>>(),
        [ranked_id, teacher_course_id]
    );
    assert_eq!(hits[0].code, ranked_code);
    assert_eq!(hits[1].teacher_name.as_deref(), Some(teacher_name.as_str()));

    sqlx::query("DELETE FROM courses.courses WHERE id = ANY($1)")
        .bind(&[ranked_id, teacher_course_id][..])
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM courses.teachers WHERE id = $1")
        .bind(teacher_id)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
async fn projection_reconciliation_recovers_index_loss_and_supports_selection_filters() {
    let Some(pool) = common::try_connect().await else {
        return;
    };
    let Ok(meili_url) = std::env::var("MEILI_URL") else {
        return;
    };
    if meili_url.trim().is_empty() {
        return;
    }
    let meili_key = std::env::var("MEILI_MASTER_KEY").unwrap_or_default();
    common::seed_courses_data(&pool).await;
    common::seed_selection_data(&pool).await;

    courses::meili::reconcile_search_projections(&pool, &meili_url, &meili_key)
        .await
        .expect("reconcile stale search projections");
    assert!(courses::meili::projection_is_ready(&pool, "catalogue")
        .await
        .expect("read catalogue readiness"));
    assert!(courses::meili::projection_is_ready(&pool, "selection")
        .await
        .expect("read selection readiness"));

    let api_key = (!meili_key.is_empty()).then_some(meili_key.as_str());
    let client = Client::new(&meili_url, api_key).expect("create Meilisearch test client");
    let deletion = client
        .index("selection_courses")
        .delete_all_documents()
        .await
        .expect("enqueue simulated selection index loss")
        .wait_for_completion(
            &client,
            Some(Duration::from_millis(20)),
            Some(Duration::from_secs(30)),
        )
        .await
        .expect("wait for simulated selection index loss");
    assert!(deletion.is_success());

    courses::meili::reconcile_search_projections(&pool, &meili_url, &meili_key)
        .await
        .expect("rebuild externally cleared selection index");

    let filter = courses::selection_repo::OfferingFilter {
        calendar_id: Some(1),
        major_id: Some(1),
        grade: Some("2024".into()),
        weekday: Some(1),
        start_slot: Some(1),
        end_slot: Some(2),
        week: Some(3),
        include_unknown_schedule: false,
        ..courses::selection_repo::OfferingFilter::default()
    };
    for query in ["SEL101", "李老师", "xuanke"] {
        let result = courses::meili::search_selection_offering_ids(
            &meili_url, &meili_key, query, &filter, 0, 20,
        )
        .await
        .expect("search selection offering");
        assert!(result.ids.contains(&1), "query {query} should find the seeded offering");
        assert!(result.consumed >= 1);
    }

    let course_hits =
        courses::public_search::search_courses(&pool, &meili_url, &meili_key, "数据结构", 20)
            .await
            .expect("search reconciled catalogue index");
    assert!(course_hits.iter().any(|course| course.code == "CS101"));

    let new_course_id = (uuid::Uuid::new_v4().as_u128() & ((1_u128 << 62) - 1)) as i64 + 1;
    let new_course_code = format!("RECONCILE-{}", uuid::Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO courses.courses (id, code, name) OVERRIDING SYSTEM VALUE \
         VALUES ($1, $2, '投影行数恢复')",
    )
    .bind(new_course_id)
    .bind(&new_course_code)
    .execute(&pool)
    .await
    .expect("seed catalogue source row drift");
    courses::meili::reconcile_search_projections(&pool, &meili_url, &meili_key)
        .await
        .expect("rebuild catalogue after source row addition");
    let added_hits =
        courses::public_search::search_courses(&pool, &meili_url, &meili_key, &new_course_code, 20)
            .await
            .expect("search catalogue after source row addition");
    assert!(added_hits.iter().any(|course| course.id == new_course_id.to_string()));

    sqlx::query("DELETE FROM courses.courses WHERE id = $1")
        .bind(new_course_id)
        .execute(&pool)
        .await
        .expect("remove catalogue source row");
    courses::meili::reconcile_search_projections(&pool, &meili_url, &meili_key)
        .await
        .expect("rebuild catalogue after source row removal");
    let removed_hits =
        courses::public_search::search_courses(&pool, &meili_url, &meili_key, &new_course_code, 20)
            .await
            .expect("search catalogue after source row removal");
    assert!(removed_hits.is_empty());
}

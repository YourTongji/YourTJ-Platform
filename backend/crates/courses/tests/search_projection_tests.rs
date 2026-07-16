//! Integration tests for database-backed course search result projection.

mod common;

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
async fn selection_reindex_supports_code_teacher_pinyin_and_time_filters() {
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
    common::seed_selection_data(&pool).await;

    courses::meili::setup_selection_index(&meili_url, &meili_key)
        .await
        .expect("setup selection index");
    let indexed = courses::meili::sync_selection_courses_to_meili(&meili_url, &meili_key, &pool)
        .await
        .expect("reindex selection offerings");
    assert!(indexed >= 1);

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
}

-- materialize_courses.sql — Materialize courses.* from selection.pk_* raw tables.
-- Idempotent: safe to re-run. Run AFTER step 2 (d1_import → selection.pk_*),
-- before materialize_selection.sql.
BEGIN;

-- Teachers: deduplicate by name from raw teacher data
INSERT INTO courses.teachers (tid, name, title, department, name_pinyin, name_initials)
SELECT DISTINCT ON (t.teacher_name)
       t.teacher_code AS tid,
       t.teacher_name AS name,
       NULL AS title,
       NULL AS department,
       NULL AS name_pinyin,
       NULL AS name_initials
FROM selection.pk_teachers_raw t
WHERE TRIM(COALESCE(t.teacher_name, '')) != ''
ON CONFLICT DO NOTHING;

-- Courses: aggregate pk_course_details by course_code (canonical course, not teaching class)
INSERT INTO courses.courses (id, code, name, credit, department, review_count, review_avg,
                              name_pinyin, name_initials, search_keywords, is_legacy)
SELECT ROW_NUMBER() OVER (ORDER BY COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code)) + 1000000,
       COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code),
       COALESCE(NULLIF(TRIM(cd.course_name), ''), NULLIF(TRIM(cd.name), ''), cd.code),
       AVG(cd.credit) FILTER (WHERE cd.credit IS NOT NULL),
       NULL AS department,
       0,
       0,
       NULL, NULL, NULL,
       1
FROM selection.pk_course_details cd
WHERE COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code) IS NOT NULL
GROUP BY COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code),
         COALESCE(NULLIF(TRIM(cd.course_name), ''), NULLIF(TRIM(cd.name), ''), cd.code)
ON CONFLICT DO NOTHING;

-- Course aliases: map all code variants (code, course_code, new_code, new_course_code) to courses
INSERT INTO courses.course_aliases (course_id, alias)
SELECT c.id, cd.code AS alias
FROM selection.pk_course_details cd
JOIN courses.courses c ON c.code = COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code)
WHERE TRIM(COALESCE(cd.code, '')) != ''
ON CONFLICT (course_id, alias) DO NOTHING;

INSERT INTO courses.course_aliases (course_id, alias)
SELECT c.id, cd.new_code AS alias
FROM selection.pk_course_details cd
JOIN courses.courses c ON c.code = COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code)
WHERE NULLIF(TRIM(cd.new_code), '') IS NOT NULL
ON CONFLICT (course_id, alias) DO NOTHING;

INSERT INTO courses.course_aliases (course_id, alias)
SELECT c.id, cd.new_course_code AS alias
FROM selection.pk_course_details cd
JOIN courses.courses c ON c.code = COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code)
WHERE NULLIF(TRIM(cd.new_course_code), '') IS NOT NULL
ON CONFLICT (course_id, alias) DO NOTHING;

-- Self-alias for the canonical code
INSERT INTO courses.course_aliases (course_id, alias)
SELECT c.id, c.code AS alias
FROM courses.courses c
WHERE c.code IS NOT NULL
ON CONFLICT (course_id, alias) DO NOTHING;

COMMIT;

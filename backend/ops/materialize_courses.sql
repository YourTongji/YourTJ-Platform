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

-- Courses: aggregate to exactly one canonical row per course code. When historical
-- teaching classes disagree on the display name, prefer the newest calendar/class.
WITH normalized_courses AS (
  SELECT COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code) AS code,
         COALESCE(NULLIF(TRIM(cd.course_name), ''), NULLIF(TRIM(cd.name), ''), cd.code) AS name,
         cd.credit,
         cd.calendar_id,
         cd.id
  FROM selection.pk_course_details cd
  WHERE COALESCE(NULLIF(TRIM(cd.course_code), ''), cd.code) IS NOT NULL
), canonical_courses AS (
  SELECT code,
         (ARRAY_AGG(name ORDER BY calendar_id DESC NULLS LAST, id DESC))[1] AS name,
         AVG(credit) FILTER (WHERE credit IS NOT NULL) AS credit
  FROM normalized_courses
  GROUP BY code
)
INSERT INTO courses.courses (id, code, name, credit, department, review_count, review_avg,
                              name_pinyin, name_initials, search_keywords, is_legacy)
OVERRIDING SYSTEM VALUE
SELECT ROW_NUMBER() OVER (ORDER BY course.code) + 1000000,
       course.code,
       course.name,
       course.credit,
       NULL AS department,
       0,
       0,
       NULL, NULL, NULL,
       1
FROM canonical_courses course
ON CONFLICT DO NOTHING;

SELECT setval(
  pg_get_serial_sequence('courses.courses', 'id'),
  COALESCE((SELECT MAX(id) FROM courses.courses), 1),
  EXISTS (SELECT 1 FROM courses.courses)
);

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

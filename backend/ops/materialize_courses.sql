-- materialize_courses.sql — Reconcile the catalogue projection from PK raw data.
-- Existing community-owned courses and review history remain authoritative.
BEGIN;

SELECT pg_advisory_xact_lock(hashtextextended('selection.materialize', 0));

-- Teachers have no natural-key constraint in the legacy schema, so explicitly
-- insert only names that are not already represented.
WITH canonical_teachers AS (
  SELECT DISTINCT ON (BTRIM(teacher_name))
         NULLIF(BTRIM(teacher_code), '') AS tid,
         BTRIM(teacher_name) AS name
  FROM selection.pk_teachers_raw
  WHERE NULLIF(BTRIM(teacher_name), '') IS NOT NULL
  ORDER BY BTRIM(teacher_name), id DESC
)
INSERT INTO courses.teachers (tid, name, title, department, name_pinyin, name_initials)
SELECT teacher.tid, teacher.name, NULL, NULL, NULL, NULL
FROM canonical_teachers AS teacher
WHERE NOT EXISTS (
  SELECT 1 FROM courses.teachers AS existing
  WHERE LOWER(existing.name) = LOWER(teacher.name)
);

-- One catalogue row per canonical course code. Update rows previously imported
-- by this projection, preserve curated/community rows, and allocate new IDs from
-- the owning identity sequence rather than a ROW_NUMBER that shifts over time.
WITH normalized_courses AS (
  SELECT COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')) AS code,
         COALESCE(
           NULLIF(BTRIM(detail.course_name), ''),
           NULLIF(BTRIM(detail.name), ''),
           NULLIF(BTRIM(detail.course_code), ''),
           BTRIM(detail.code)
         ) AS name,
         detail.credit,
         detail.calendar_id,
         detail.id
  FROM selection.pk_course_details AS detail
  WHERE COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), ''))
        IS NOT NULL
), canonical_courses AS (
  SELECT code,
         (ARRAY_AGG(name ORDER BY calendar_id DESC NULLS LAST, id DESC))[1] AS name,
         AVG(credit) FILTER (WHERE credit IS NOT NULL) AS credit
  FROM normalized_courses
  GROUP BY code
)
UPDATE courses.courses AS course
SET name = source.name,
    credit = source.credit,
    name_pinyin = CASE WHEN course.name = source.name THEN course.name_pinyin END,
    name_initials = CASE WHEN course.name = source.name THEN course.name_initials END,
    search_keywords = CASE WHEN course.name = source.name THEN course.search_keywords END
FROM canonical_courses AS source
WHERE course.is_legacy = 1
  AND course.code = source.code;

WITH normalized_courses AS (
  SELECT COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')) AS code,
         COALESCE(
           NULLIF(BTRIM(detail.course_name), ''),
           NULLIF(BTRIM(detail.name), ''),
           NULLIF(BTRIM(detail.course_code), ''),
           BTRIM(detail.code)
         ) AS name,
         detail.credit,
         detail.calendar_id,
         detail.id
  FROM selection.pk_course_details AS detail
  WHERE COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), ''))
        IS NOT NULL
), canonical_courses AS (
  SELECT code,
         (ARRAY_AGG(name ORDER BY calendar_id DESC NULLS LAST, id DESC))[1] AS name,
         AVG(credit) FILTER (WHERE credit IS NOT NULL) AS credit
  FROM normalized_courses
  GROUP BY code
)
INSERT INTO courses.courses (
  code, name, credit, department, review_count, review_avg,
  name_pinyin, name_initials, search_keywords, is_legacy
)
SELECT source.code, source.name, source.credit, NULL, 0, 0, NULL, NULL, NULL, 1
FROM canonical_courses AS source
WHERE NOT EXISTS (
  SELECT 1 FROM courses.courses AS existing
  WHERE existing.code = source.code
);

-- Retire stale imported catalogue rows only when no review history owns them.
DELETE FROM courses.course_aliases AS alias
USING courses.courses AS course
WHERE alias.course_id = course.id
  AND course.is_legacy = 1
  AND NOT EXISTS (
    SELECT 1 FROM selection.pk_course_details AS detail
    WHERE COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), ''))
          = course.code
  )
  AND NOT EXISTS (
    SELECT 1 FROM reviews.reviews AS review WHERE review.course_id = course.id
  );
DELETE FROM courses.courses AS course
WHERE course.is_legacy = 1
  AND NOT EXISTS (
    SELECT 1 FROM selection.pk_course_details AS detail
    WHERE COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), ''))
          = course.code
  )
  AND NOT EXISTS (
    SELECT 1 FROM reviews.reviews AS review WHERE review.course_id = course.id
  );

-- Resolve every alias to one deterministic catalogue owner, preferring a
-- curated/community row over a legacy projection with the same code.
WITH course_owner AS (
  SELECT DISTINCT ON (code) id, code
  FROM courses.courses
  ORDER BY code, is_legacy ASC NULLS FIRST, id
), aliases AS (
  SELECT owner.id AS course_id, NULLIF(BTRIM(detail.code), '') AS alias
  FROM selection.pk_course_details AS detail
  JOIN course_owner AS owner ON owner.code = COALESCE(
    NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')
  )
  UNION
  SELECT owner.id, NULLIF(BTRIM(detail.new_code), '')
  FROM selection.pk_course_details AS detail
  JOIN course_owner AS owner ON owner.code = COALESCE(
    NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')
  )
  UNION
  SELECT owner.id, NULLIF(BTRIM(detail.new_course_code), '')
  FROM selection.pk_course_details AS detail
  JOIN course_owner AS owner ON owner.code = COALESCE(
    NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')
  )
  UNION
  SELECT owner.id, owner.code FROM course_owner AS owner
)
INSERT INTO courses.course_aliases (course_id, alias)
SELECT course_id, alias
FROM aliases
WHERE alias IS NOT NULL
ON CONFLICT (course_id, alias) DO NOTHING;

COMMIT;

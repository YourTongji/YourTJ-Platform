-- materialize_courses.sql — Reconcile the catalogue projection from PK raw data.
-- Existing community-owned courses and review history remain authoritative.
BEGIN;

SELECT pg_advisory_xact_lock(hashtextextended('selection.materialize', 0));

LOCK TABLE
  selection.pk_calendars,
  selection.pk_languages,
  selection.pk_course_natures,
  selection.pk_course_natures_by_calendar,
  selection.pk_assessments,
  selection.pk_campuses,
  selection.pk_faculties,
  selection.pk_majors,
  selection.pk_course_details,
  selection.pk_teachers_raw,
  selection.pk_teacher_timeslots,
  selection.pk_major_courses,
  selection.pk_fetch_logs,
  courses.pk_legacy_teachers,
  courses.pk_legacy_courses,
  courses.pk_legacy_course_aliases
IN SHARE MODE;

SELECT selection.assert_materialization_source();
SELECT courses.assert_legacy_materialization_source();

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
  SELECT DISTINCT ON (code) code, name, credit
  FROM normalized_courses
  ORDER BY code, calendar_id DESC NULLS LAST, id DESC
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
  SELECT DISTINCT ON (code) code, name, credit
  FROM normalized_courses
  ORDER BY code, calendar_id DESC NULLS LAST, id DESC
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

-- Catalogue rows and aliases outlive any one selection snapshot. This reconcile
-- only upserts; retirement requires an explicit Courses-owned lifecycle and may
-- not depend on Reviews-private SQL.

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

-- Reconcile the historical public aggregate without overwriting community
-- reviews created after the previous import. Only a legacy course that resolves
-- to exactly one catalogue owner contributes; ambiguous aliases fail closed by
-- remaining unmapped instead of contaminating two courses.
WITH legacy_identifiers AS (
  SELECT legacy.id AS legacy_course_id, NULLIF(BTRIM(legacy.code), '') AS identifier
  FROM courses.pk_legacy_courses AS legacy
  UNION
  SELECT alias.course_id, NULLIF(BTRIM(alias.alias), '')
  FROM courses.pk_legacy_course_aliases AS alias
  WHERE alias.system = 'onesystem'
), candidate_mappings AS (
  SELECT DISTINCT identifier.legacy_course_id, current_alias.course_id
  FROM legacy_identifiers AS identifier
  JOIN courses.course_aliases AS current_alias
    ON current_alias.alias = identifier.identifier
  WHERE identifier.identifier IS NOT NULL
), unique_mappings AS (
  SELECT legacy_course_id, MIN(course_id) AS course_id
  FROM candidate_mappings
  GROUP BY legacy_course_id
  HAVING COUNT(DISTINCT course_id) = 1
), legacy_aggregates AS (
  SELECT mapping.course_id,
         SUM(legacy.review_count)::INTEGER AS review_count,
         CASE WHEN SUM(legacy.review_count) > 0
           THEN SUM(legacy.review_avg * legacy.review_count) / SUM(legacy.review_count)
           ELSE 0
         END AS review_avg
  FROM unique_mappings AS mapping
  JOIN courses.pk_legacy_courses AS legacy ON legacy.id = mapping.legacy_course_id
  GROUP BY mapping.course_id
), reconciled AS (
  SELECT course.id,
         GREATEST(course.review_count - course.legacy_review_count, 0) AS community_count,
         LEAST(
           GREATEST(
             course.review_avg * course.review_count
               - course.legacy_review_avg * course.legacy_review_count,
             0
           ),
           GREATEST(course.review_count - course.legacy_review_count, 0) * 5.0
         ) AS community_points,
         COALESCE(legacy.review_count, 0) AS legacy_count,
         COALESCE(legacy.review_avg, 0) AS legacy_avg
  FROM courses.courses AS course
  LEFT JOIN legacy_aggregates AS legacy ON legacy.course_id = course.id
)
UPDATE courses.courses AS course
SET review_count = reconciled.community_count + reconciled.legacy_count,
    review_avg = CASE
      WHEN reconciled.community_count + reconciled.legacy_count = 0 THEN 0
      ELSE (
        reconciled.community_points + reconciled.legacy_avg * reconciled.legacy_count
      ) / (reconciled.community_count + reconciled.legacy_count)
    END,
    legacy_review_count = reconciled.legacy_count,
    legacy_review_avg = reconciled.legacy_avg
FROM reconciled
WHERE reconciled.id = course.id;

UPDATE courses.search_projection_state
SET source_generation = source_generation + 1,
    source_rows = (SELECT COUNT(*) FROM courses.courses),
    indexed_generation = NULL,
    indexed_rows = NULL,
    status = 'stale',
    updated_at = now()
WHERE projection = 'catalogue';

COMMIT;

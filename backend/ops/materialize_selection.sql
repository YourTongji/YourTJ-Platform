-- materialize_selection.sql — Reconcile selection.* from selection.pk_* raw tables.
-- Idempotent and transactionally visible: readers see either the old complete
-- snapshot or the new complete snapshot, never a half-materialized schedule.
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

-- Replace dependent facts first; dimensions retain stable natural-key IDs.
DELETE FROM selection.timeslots;
DELETE FROM selection.major_courses;
DELETE FROM selection.courses;
DELETE FROM selection.fetchlog;
ALTER SEQUENCE IF EXISTS selection.timeslots_id_seq RESTART;

-- Calendars. Upstream ids are monotonic in every audited snapshot; the maximum
-- available id is therefore the only current-semester signal we can prove.
UPDATE selection.calendars SET is_current = false WHERE is_current;
INSERT INTO selection.calendars (id, name, is_current)
SELECT calendar_id,
       COALESCE(NULLIF(BTRIM(calendar_name), ''), calendar_id::TEXT),
       calendar_id = (SELECT MAX(calendar_id) FROM selection.pk_calendars)
FROM selection.pk_calendars
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    is_current = EXCLUDED.is_current;
DELETE FROM selection.calendars AS calendar
WHERE NOT EXISTS (
  SELECT 1 FROM selection.pk_calendars AS raw WHERE raw.calendar_id = calendar.id
);

-- Campuses and faculties use stable natural-key sequences introduced in 0012.
-- The current upstream snapshot does not expose faculty/campus relationships;
-- clear legacy links before reconciling dimensions so stale foreign keys cannot
-- block removal of a no-longer-present faculty or campus.
UPDATE selection.majors SET faculty_id = NULL WHERE faculty_id IS NOT NULL;
UPDATE selection.faculties SET campus_id = NULL WHERE campus_id IS NOT NULL;

INSERT INTO selection.campuses (name)
SELECT DISTINCT COALESCE(NULLIF(BTRIM(campus_i18n), ''), BTRIM(campus))
FROM selection.pk_campuses
WHERE COALESCE(NULLIF(BTRIM(campus_i18n), ''), NULLIF(BTRIM(campus), '')) IS NOT NULL
ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name;

INSERT INTO selection.faculties (name, campus_id)
SELECT DISTINCT COALESCE(NULLIF(BTRIM(faculty_i18n), ''), BTRIM(faculty)), NULL::BIGINT
FROM selection.pk_faculties
WHERE COALESCE(NULLIF(BTRIM(faculty_i18n), ''), NULLIF(BTRIM(faculty), '')) IS NOT NULL
ON CONFLICT (name) DO UPDATE
SET name = EXCLUDED.name,
    campus_id = EXCLUDED.campus_id;
DELETE FROM selection.faculties AS faculty
WHERE NOT EXISTS (
  SELECT 1
  FROM selection.pk_faculties AS raw
  WHERE COALESCE(NULLIF(BTRIM(raw.faculty_i18n), ''), BTRIM(raw.faculty)) = faculty.name
);
DELETE FROM selection.campuses AS campus
WHERE NOT EXISTS (
  SELECT 1
  FROM selection.pk_campuses AS raw
  WHERE COALESCE(NULLIF(BTRIM(raw.campus_i18n), ''), BTRIM(raw.campus)) = campus.name
);

INSERT INTO selection.majors (id, name, faculty_id, grade)
SELECT id, BTRIM(name), NULL::BIGINT, grade::TEXT
FROM selection.pk_majors
WHERE NULLIF(BTRIM(name), '') IS NOT NULL
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    faculty_id = EXCLUDED.faculty_id,
    grade = EXCLUDED.grade;
DELETE FROM selection.majors AS major
WHERE NOT EXISTS (
  SELECT 1 FROM selection.pk_majors AS raw WHERE raw.id = major.id
);

WITH raw_natures AS (
  SELECT course_label_id, NULLIF(BTRIM(course_label_name), '') AS name
  FROM selection.pk_course_natures
  UNION ALL
  SELECT course_label_id, NULLIF(BTRIM(course_label_name), '') AS name
  FROM selection.pk_course_natures_by_calendar
), canonical_natures AS (
  SELECT course_label_id, MAX(name) FILTER (WHERE name IS NOT NULL) AS name
  FROM raw_natures
  GROUP BY course_label_id
)
INSERT INTO selection.course_natures (id, name)
SELECT course_label_id, COALESCE(name, course_label_id::TEXT)
FROM canonical_natures
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;
DELETE FROM selection.course_natures AS nature
WHERE NOT EXISTS (
  SELECT 1 FROM selection.pk_course_natures AS raw
  WHERE raw.course_label_id = nature.id
  UNION ALL
  SELECT 1 FROM selection.pk_course_natures_by_calendar AS raw
  WHERE raw.course_label_id = nature.id
);

-- Exactly one normalized offering per upstream teachingClassId. Course code is
-- catalogue identity; teaching_class_code and id distinguish its offerings.
INSERT INTO selection.courses (
  id, code, teaching_class_code, name, credit, nature_id, calendar_id, campus_id,
  faculty_name, teaching_language, teacher_name, teacher_names,
  start_week, end_week, weeks_unknown, schedule_unknown, status,
  catalogue_course_id, updated_at
)
SELECT detail.id,
       COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')),
       NULLIF(BTRIM(detail.code), ''),
       COALESCE(
         NULLIF(BTRIM(detail.course_name), ''),
         NULLIF(BTRIM(detail.name), ''),
         NULLIF(BTRIM(detail.course_code), ''),
         BTRIM(detail.code)
       ),
       detail.credit,
       nature.id,
       calendar.id,
       campus.id,
       COALESCE(NULLIF(BTRIM(raw_faculty.faculty_i18n), ''), NULLIF(BTRIM(detail.faculty), '')),
       COALESCE(
         NULLIF(BTRIM(raw_language.teaching_language_i18n), ''),
         NULLIF(BTRIM(detail.teaching_language), '')
       ),
       teachers.first_name,
       COALESCE(teachers.names, ARRAY[]::TEXT[]),
       CASE WHEN detail.start_week BETWEEN 1 AND 30
                  AND detail.end_week BETWEEN detail.start_week AND 30
            THEN detail.start_week END,
       CASE WHEN detail.start_week BETWEEN 1 AND 30
                  AND detail.end_week BETWEEN detail.start_week AND 30
            THEN detail.end_week END,
       NOT COALESCE(
         detail.start_week BETWEEN 1 AND 30
         AND detail.end_week BETWEEN detail.start_week AND 30,
         false
       ),
       true,
       'unknown',
       catalogue.id,
       now()
FROM selection.pk_course_details AS detail
JOIN selection.calendars AS calendar ON calendar.id = detail.calendar_id
LEFT JOIN selection.course_natures AS nature ON nature.id = detail.course_label_id
LEFT JOIN selection.pk_campuses AS raw_campus
  ON raw_campus.campus = NULLIF(BTRIM(detail.campus), '')
LEFT JOIN selection.campuses AS campus
  ON campus.name = COALESCE(
    NULLIF(BTRIM(raw_campus.campus_i18n), ''),
    NULLIF(BTRIM(detail.campus), '')
  )
LEFT JOIN selection.pk_faculties AS raw_faculty
  ON raw_faculty.faculty = NULLIF(BTRIM(detail.faculty), '')
LEFT JOIN selection.pk_languages AS raw_language
  ON raw_language.teaching_language = NULLIF(BTRIM(detail.teaching_language), '')
LEFT JOIN LATERAL (
  SELECT (array_agg(name ORDER BY teacher_id))[1] AS first_name,
         array_agg(DISTINCT name ORDER BY name) AS names
  FROM (
    SELECT teacher.id AS teacher_id, NULLIF(BTRIM(teacher.teacher_name), '') AS name
    FROM selection.pk_teachers_raw AS teacher
    WHERE teacher.teaching_class_id = detail.id
      AND NULLIF(BTRIM(teacher.teacher_name), '') IS NOT NULL
  ) AS normalized_teachers
) AS teachers ON true
LEFT JOIN LATERAL (
  SELECT course.id
  FROM courses.courses AS course
  WHERE course.code = COALESCE(
    NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), '')
  )
  ORDER BY course.is_legacy ASC NULLS FIRST, course.id
  LIMIT 1
) AS catalogue ON true
WHERE COALESCE(NULLIF(BTRIM(detail.course_code), ''), NULLIF(BTRIM(detail.code), ''))
      IS NOT NULL;

-- Attach the closest auditable legacy rating aggregate to each teaching class.
-- Teacher code/name matches outrank a course-only alias match. Multiple legacy
-- rows at the same best rank are weighted together because the historical
-- system split some course/teacher combinations across several rows.
WITH offering_identifiers AS (
  SELECT detail.id AS offering_id,
         identifier.value AS identifier,
         identifier.rank AS identifier_rank
  FROM selection.pk_course_details AS detail
  CROSS JOIN LATERAL (
    VALUES
      (NULLIF(BTRIM(detail.code), ''), 0),
      (NULLIF(BTRIM(detail.new_code), ''), 0),
      (NULLIF(BTRIM(detail.course_code), ''), 1),
      (NULLIF(BTRIM(detail.new_course_code), ''), 1)
  ) AS identifier(value, rank)
  WHERE identifier.value IS NOT NULL
), legacy_identifiers AS (
  SELECT legacy.id AS legacy_course_id, NULLIF(BTRIM(legacy.code), '') AS identifier
  FROM courses.pk_legacy_courses AS legacy
  UNION
  SELECT alias.course_id, NULLIF(BTRIM(alias.alias), '')
  FROM courses.pk_legacy_course_aliases AS alias
  WHERE alias.system = 'onesystem'
), candidates AS (
  SELECT DISTINCT offering.offering_id,
         legacy.id AS legacy_course_id,
         legacy.review_count,
         legacy.review_avg,
         offering.identifier_rank,
         CASE
           WHEN NULLIF(BTRIM(legacy_teacher.tid), '') IS NOT NULL
            AND EXISTS (
              SELECT 1
              FROM selection.pk_teachers_raw AS current_teacher
              WHERE current_teacher.teaching_class_id = offering.offering_id
                AND NULLIF(BTRIM(current_teacher.teacher_code), '') =
                    NULLIF(BTRIM(legacy_teacher.tid), '')
            ) THEN 0
           WHEN NULLIF(BTRIM(legacy_teacher.name), '') IS NOT NULL
            AND EXISTS (
              SELECT 1
              FROM selection.pk_teachers_raw AS current_teacher
              WHERE current_teacher.teaching_class_id = offering.offering_id
                AND LOWER(NULLIF(BTRIM(current_teacher.teacher_name), '')) =
                    LOWER(NULLIF(BTRIM(legacy_teacher.name), ''))
            ) THEN 1
           ELSE 2
         END AS teacher_rank
  FROM offering_identifiers AS offering
  JOIN legacy_identifiers AS identifier ON identifier.identifier = offering.identifier
  JOIN courses.pk_legacy_courses AS legacy ON legacy.id = identifier.legacy_course_id
  LEFT JOIN courses.pk_legacy_teachers AS legacy_teacher ON legacy_teacher.id = legacy.teacher_id
  WHERE legacy.review_count > 0
), ranked AS (
  SELECT candidate.*,
         MIN(candidate.teacher_rank) OVER (
           PARTITION BY candidate.offering_id
         ) AS best_teacher_rank
  FROM candidates AS candidate
), best_ranked AS (
  SELECT ranked.*,
         MIN(ranked.identifier_rank) OVER (
           PARTITION BY ranked.offering_id, ranked.teacher_rank
         ) AS best_identifier_rank
  FROM ranked
  WHERE ranked.teacher_rank = ranked.best_teacher_rank
), selected AS (
  SELECT DISTINCT offering_id, legacy_course_id, review_count, review_avg, teacher_rank
  FROM best_ranked
  WHERE identifier_rank = best_identifier_rank
), aggregate AS (
  SELECT offering_id,
         SUM(review_count)::INTEGER AS review_count,
         SUM(review_avg * review_count) / SUM(review_count) AS review_avg,
         CASE WHEN MIN(teacher_rank) < 2 THEN 'teacher' ELSE 'course' END AS review_scope
  FROM selected
  GROUP BY offering_id
)
UPDATE selection.courses AS course
SET review_count = aggregate.review_count,
    review_avg = aggregate.review_avg,
    review_scope = aggregate.review_scope
FROM aggregate
WHERE aggregate.offering_id = course.id;

INSERT INTO selection.major_courses (major_id, course_id, grade)
SELECT binding.major_id, binding.course_id, major.grade::TEXT
FROM selection.pk_major_courses AS binding
JOIN selection.pk_majors AS major ON major.id = binding.major_id
JOIN selection.courses AS course ON course.id = binding.course_id
ON CONFLICT (major_id, course_id) DO UPDATE SET grade = EXCLUDED.grade;

-- Primary schedule source: strict parsing of the complete arrangement lines.
-- Upstream repeats the complete arrangement once for every teacher attached to
-- some offerings. Collapse identical schedule facts across those rows; retain
-- a teacher only when exactly one teacher owns that fact. Exact week sets are
-- retained only when they do not contradict a valid course range. Location and
-- week uncertainty remain independent facts.
WITH arrangement_lines AS (
  SELECT teacher.teaching_class_id,
         COALESCE(
           parsed.teacher_name,
           (
             SELECT MIN(identity_teacher.teacher_name)
             FROM selection.pk_teachers_raw AS identity_teacher
             WHERE identity_teacher.teaching_class_id = teacher.teaching_class_id
               AND NULLIF(BTRIM(identity_teacher.teacher_code), '') = parsed.teacher_code
             HAVING COUNT(DISTINCT NULLIF(BTRIM(identity_teacher.teacher_name), '')) = 1
           ),
           NULLIF(BTRIM(teacher.teacher_name), '')
         ) AS teacher_name,
         detail.start_week AS raw_start_week,
         detail.end_week AS raw_end_week,
         parsed.weekday,
         parsed.start_slot,
         parsed.end_slot,
         parsed.week_numbers AS parsed_week_numbers,
         parsed.location
  FROM selection.pk_teachers_raw AS teacher
  JOIN selection.pk_course_details AS detail ON detail.id = teacher.teaching_class_id
  CROSS JOIN LATERAL regexp_split_to_table(
    COALESCE(teacher.arrange_info_text, ''), E'\\r?\\n'
  ) AS line
  CROSS JOIN LATERAL selection.parse_arrangement_line(line) AS parsed
), safe_arrangements AS (
  SELECT teaching_class_id,
         teacher_name,
         weekday,
         start_slot,
         end_slot,
         CASE
           WHEN parsed_week_numbers IS NULL THEN ARRAY[]::INTEGER[]
           WHEN raw_start_week BETWEEN 1 AND 30
            AND raw_end_week BETWEEN raw_start_week AND 30
            AND EXISTS (
              SELECT 1 FROM unnest(parsed_week_numbers) AS week
              WHERE week < raw_start_week OR week > raw_end_week
            ) THEN ARRAY[]::INTEGER[]
           ELSE parsed_week_numbers
         END AS week_numbers,
         CASE
           WHEN parsed_week_numbers IS NULL THEN true
           WHEN raw_start_week BETWEEN 1 AND 30
            AND raw_end_week BETWEEN raw_start_week AND 30
            AND EXISTS (
              SELECT 1 FROM unnest(parsed_week_numbers) AS week
              WHERE week < raw_start_week OR week > raw_end_week
            ) THEN true
           ELSE false
         END AS weeks_unknown,
         location
  FROM arrangement_lines
)
INSERT INTO selection.timeslots (
  course_id, teacher_name, weekday, start_slot, end_slot, weeks, location,
  week_numbers, weeks_unknown, location_unknown
)
SELECT teaching_class_id,
       CASE WHEN COUNT(DISTINCT teacher_name) = 1 THEN MIN(teacher_name) END,
       weekday,
       start_slot,
       end_slot,
       CASE WHEN weeks_unknown THEN NULL ELSE array_to_string(week_numbers, ',') END,
       location,
       week_numbers,
       weeks_unknown,
       location IS NULL
FROM safe_arrangements
WHERE EXISTS (
  SELECT 1 FROM selection.courses AS course WHERE course.id = teaching_class_id
)
GROUP BY teaching_class_id, weekday, start_slot, end_slot,
         week_numbers, weeks_unknown, location;

-- Auxiliary day/section rows fill only periods not already represented by a
-- parsed arrangement. This preserves usable fallback facts when one line in a
-- mixed-format arrangement is not understood.
WITH auxiliary_sections AS (
  SELECT raw.teaching_class_id,
         NULLIF(BTRIM(raw.teacher_name), '') AS teacher_name,
         raw.occupy_day AS weekday,
         raw.occupy_section,
         raw.occupy_section - ROW_NUMBER() OVER (
           PARTITION BY raw.teaching_class_id, raw.occupy_day,
                        NULLIF(BTRIM(raw.teacher_name), '')
           ORDER BY raw.occupy_section
         )::INTEGER AS section_group
  FROM selection.pk_teacher_timeslots AS raw
  JOIN selection.courses AS course
    ON course.id = raw.teaching_class_id
   AND course.calendar_id = raw.calendar_id
  WHERE raw.occupy_day BETWEEN 1 AND 7
    AND raw.occupy_section BETWEEN 1 AND 20
    AND NOT EXISTS (
      SELECT 1 FROM selection.timeslots AS existing
      WHERE existing.course_id = raw.teaching_class_id
        AND existing.weekday = raw.occupy_day
        AND raw.occupy_section BETWEEN existing.start_slot AND existing.end_slot
    )
), auxiliary_teacher_ranges AS (
  SELECT teaching_class_id,
         teacher_name,
         weekday,
         MIN(occupy_section) AS start_slot,
         MAX(occupy_section) AS end_slot
  FROM auxiliary_sections
  GROUP BY teaching_class_id, teacher_name, weekday, section_group
), auxiliary_ranges AS (
  SELECT teaching_class_id,
         CASE WHEN COUNT(DISTINCT teacher_name) = 1 THEN MIN(teacher_name) END AS teacher_name,
         weekday,
         start_slot,
         end_slot
  FROM auxiliary_teacher_ranges
  GROUP BY teaching_class_id, weekday, start_slot, end_slot
)
INSERT INTO selection.timeslots (
  course_id, teacher_name, weekday, start_slot, end_slot, weeks, location,
  week_numbers, weeks_unknown, location_unknown
)
SELECT range.teaching_class_id,
       range.teacher_name,
       range.weekday,
       range.start_slot,
       range.end_slot,
       CASE WHEN course.weeks_unknown THEN NULL
            ELSE course.start_week::TEXT || '-' || course.end_week::TEXT END,
       NULL,
       CASE WHEN course.weeks_unknown THEN ARRAY[]::INTEGER[]
            ELSE ARRAY(SELECT generate_series(course.start_week, course.end_week)) END,
       course.weeks_unknown,
       true
FROM auxiliary_ranges AS range
JOIN selection.courses AS course ON course.id = range.teaching_class_id;

WITH materialized_offerings AS (
  SELECT DISTINCT slot.course_id
  FROM selection.timeslots AS slot
), offerings_with_unparsed_lines AS (
  SELECT DISTINCT teacher.teaching_class_id
  FROM selection.pk_teachers_raw AS teacher
  CROSS JOIN LATERAL regexp_split_to_table(
    COALESCE(teacher.arrange_info_text, ''), E'\\r?\\n'
  ) AS line
  LEFT JOIN LATERAL selection.parse_arrangement_line(line) AS parsed ON true
  WHERE NULLIF(BTRIM(line), '') IS NOT NULL
    AND parsed.weekday IS NULL
), schedule_status AS (
  SELECT course.id,
         materialized.course_id IS NULL OR unparsed.teaching_class_id IS NOT NULL
           AS schedule_unknown
  FROM selection.courses AS course
  LEFT JOIN materialized_offerings AS materialized ON materialized.course_id = course.id
  LEFT JOIN offerings_with_unparsed_lines AS unparsed ON unparsed.teaching_class_id = course.id
)
UPDATE selection.courses AS course
SET schedule_unknown = status.schedule_unknown,
    updated_at = now()
FROM schedule_status AS status
WHERE status.id = course.id;

DO $validate_timeslot_limit$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM selection.timeslots
    GROUP BY course_id
    HAVING COUNT(*) > 100
  ) THEN
    RAISE EXCEPTION
      'selection materialization blocked: offering exceeds 100 timeslots'
      USING ERRCODE = 'integrity_constraint_violation';
  END IF;
END
$validate_timeslot_limit$;

-- Raw fetch time is Unix seconds. `jcourse-db` is the canonical upstream
-- producer label; backup transport provenance lives in selection.import_runs.
-- Import freshness must never be substituted for source-data freshness.
INSERT INTO selection.fetchlog (source, fetched_at)
SELECT 'jcourse-db', to_timestamp(fetch_time)
FROM selection.pk_fetch_logs
WHERE fetch_time IS NOT NULL
ORDER BY fetch_time;

UPDATE courses.search_projection_state
SET source_generation = source_generation + 1,
    source_rows = (SELECT COUNT(*) FROM selection.courses),
    indexed_generation = NULL,
    indexed_rows = NULL,
    status = 'stale',
    updated_at = now()
WHERE projection = 'selection';

COMMIT;

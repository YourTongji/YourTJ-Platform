-- 0071_courses_data_pipeline_integrity.sql
-- Preserve legacy rating aggregates, parse real teaching arrangements, and
-- make search projection readiness explicit.

CREATE TABLE courses.pk_legacy_teachers (
  id          BIGINT PRIMARY KEY,
  tid         TEXT,
  name        TEXT NOT NULL,
  title       TEXT,
  department  TEXT
);

CREATE TABLE courses.pk_legacy_courses (
  id              BIGINT PRIMARY KEY,
  code            TEXT NOT NULL,
  name            TEXT NOT NULL,
  credit          DOUBLE PRECISION,
  department      TEXT,
  teacher_id      BIGINT REFERENCES courses.pk_legacy_teachers(id),
  review_count    INTEGER NOT NULL DEFAULT 0 CHECK (review_count >= 0),
  review_avg      DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (review_avg BETWEEN 0 AND 5),
  search_keywords TEXT,
  is_legacy       INTEGER NOT NULL DEFAULT 0,
  is_icu          INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX courses_pk_legacy_courses_code_idx ON courses.pk_legacy_courses (code);
CREATE INDEX courses_pk_legacy_courses_teacher_idx ON courses.pk_legacy_courses (teacher_id);

CREATE TABLE courses.pk_legacy_course_aliases (
  system      TEXT NOT NULL,
  alias       TEXT NOT NULL,
  course_id   BIGINT NOT NULL REFERENCES courses.pk_legacy_courses(id),
  created_at  BIGINT,
  PRIMARY KEY (system, alias)
);
CREATE INDEX courses_pk_legacy_alias_course_idx
  ON courses.pk_legacy_course_aliases (course_id);

CREATE TABLE courses.legacy_import_runs (
  id                   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  snapshot_sha256      CHAR(64) NOT NULL UNIQUE
    REFERENCES selection.import_runs(snapshot_sha256),
  source_database      TEXT NOT NULL CHECK (char_length(source_database) BETWEEN 1 AND 128),
  source_table_counts  JSONB NOT NULL,
  target_table_counts  JSONB NOT NULL,
  validation           JSONB NOT NULL DEFAULT '{}'::JSONB,
  imported_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX courses_legacy_import_runs_imported_idx
  ON courses.legacy_import_runs (imported_at DESC, id DESC);

CREATE FUNCTION courses.assert_legacy_materialization_source()
RETURNS VOID
LANGUAGE plpgsql
AS $$
DECLARE
  latest_run courses.legacy_import_runs%ROWTYPE;
  current_counts JSONB;
  approval_mode TEXT;
  baseline_counts JSONB;
  expected_count_decreases JSONB;
  recorded_count_decreases JSONB;
BEGIN
  SELECT *
  INTO latest_run
  FROM courses.legacy_import_runs
  ORDER BY imported_at DESC, id DESC
  LIMIT 1;

  IF NOT FOUND THEN
    RAISE EXCEPTION
      'course materialization blocked: no validated legacy course import exists'
      USING ERRCODE = 'integrity_constraint_violation';
  END IF;

  current_counts := jsonb_build_object(
    'teachers', (SELECT COUNT(*) FROM courses.pk_legacy_teachers),
    'courses', (SELECT COUNT(*) FROM courses.pk_legacy_courses),
    'course_aliases', (SELECT COUNT(*) FROM courses.pk_legacy_course_aliases)
  );

  IF latest_run.source_database <> 'jcourse-db-backup'
     OR latest_run.source_table_counts IS DISTINCT FROM latest_run.target_table_counts
     OR current_counts IS DISTINCT FROM latest_run.target_table_counts
     OR NOT (latest_run.validation @> '{
       "rowCountsMatched": true,
       "sourceSchemaValidated": true,
       "completenessApproved": true
     }'::JSONB) THEN
    RAISE EXCEPTION
      'course materialization blocked: legacy course import is not validated'
      USING ERRCODE = 'integrity_constraint_violation';
  END IF;

  IF (current_counts ->> 'teachers')::BIGINT = 0
     OR (current_counts ->> 'courses')::BIGINT = 0
     OR (current_counts ->> 'course_aliases')::BIGINT = 0 THEN
    RAISE EXCEPTION
      'course materialization blocked: legacy course source tables must be non-empty'
      USING ERRCODE = 'integrity_constraint_violation';
  END IF;

  approval_mode := COALESCE(latest_run.validation ->> 'legacyCourseApprovalMode', '');
  baseline_counts := latest_run.validation -> 'baselineLegacyCourseCounts';
  IF approval_mode NOT IN (
       'baselineCompared', 'unbaselined', 'countDecreaseOverride'
     )
     OR (
       approval_mode IN ('unbaselined', 'countDecreaseOverride')
       AND char_length(BTRIM(COALESCE(latest_run.validation ->> 'approvalReason', '')))
         NOT BETWEEN 10 AND 500
     )
     OR (
       approval_mode IN ('baselineCompared', 'countDecreaseOverride')
       AND (
         COALESCE(latest_run.validation ->> 'baselineSnapshotSha256', '')
           !~ '^[0-9a-f]{64}$'
         OR jsonb_typeof(baseline_counts) IS DISTINCT FROM 'object'
         OR (SELECT COUNT(*) FROM jsonb_object_keys(baseline_counts)) <> 3
         OR NOT (baseline_counts ?& ARRAY['teachers', 'courses', 'course_aliases'])
       )
     )
     OR (
       approval_mode = 'unbaselined'
       AND latest_run.validation -> 'approvedLegacyCourseCounts' IS DISTINCT FROM
         current_counts
     ) THEN
    RAISE EXCEPTION
      'course materialization blocked: legacy snapshot completeness approval is invalid'
      USING ERRCODE = 'integrity_constraint_violation';
  END IF;

  IF approval_mode IN ('baselineCompared', 'countDecreaseOverride') THEN
    IF EXISTS (
      SELECT 1
      FROM jsonb_each(baseline_counts) AS entry(key, value)
      WHERE jsonb_typeof(entry.value) IS DISTINCT FROM 'number'
         OR entry.value::TEXT !~ '^(0|[1-9][0-9]*)$'
    ) THEN
      RAISE EXCEPTION
        'course materialization blocked: legacy snapshot completeness approval is invalid'
        USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    IF EXISTS (
      SELECT 1
      FROM jsonb_each(baseline_counts) AS entry(key, value)
      WHERE (entry.value #>> '{}')::NUMERIC > 9223372036854775807
    ) THEN
      RAISE EXCEPTION
        'course materialization blocked: legacy snapshot completeness approval is invalid'
        USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    SELECT COALESCE(
      jsonb_object_agg(
        entry.key,
        jsonb_build_object(
          'before', (entry.value #>> '{}')::BIGINT,
          'after', (current_counts ->> entry.key)::BIGINT
        )
      ),
      '{}'::JSONB
    )
    INTO expected_count_decreases
    FROM jsonb_each(baseline_counts) AS entry(key, value)
    WHERE (entry.value #>> '{}')::BIGINT > (current_counts ->> entry.key)::BIGINT;

    recorded_count_decreases := COALESCE(
      latest_run.validation -> 'legacyCourseCountDecreases',
      '{}'::JSONB
    );
    IF jsonb_typeof(recorded_count_decreases) IS DISTINCT FROM 'object'
       OR (
         approval_mode = 'baselineCompared'
         AND (
           expected_count_decreases <> '{}'::JSONB
           OR recorded_count_decreases <> '{}'::JSONB
         )
       )
       OR (
         approval_mode = 'countDecreaseOverride'
         AND (
           expected_count_decreases = '{}'::JSONB
           OR recorded_count_decreases IS DISTINCT FROM expected_count_decreases
         )
       ) THEN
      RAISE EXCEPTION
        'course materialization blocked: legacy snapshot completeness approval is invalid'
        USING ERRCODE = 'integrity_constraint_violation';
    END IF;
  END IF;
END;
$$;

ALTER TABLE courses.courses
  ADD COLUMN legacy_review_count INTEGER NOT NULL DEFAULT 0
    CHECK (legacy_review_count >= 0),
  ADD COLUMN legacy_review_avg DOUBLE PRECISION NOT NULL DEFAULT 0
    CHECK (legacy_review_avg BETWEEN 0 AND 5);

ALTER TABLE selection.courses
  ADD COLUMN review_count INTEGER NOT NULL DEFAULT 0 CHECK (review_count >= 0),
  ADD COLUMN review_avg DOUBLE PRECISION,
  ADD COLUMN review_scope TEXT NOT NULL DEFAULT 'none'
    CHECK (review_scope IN ('none', 'course', 'teacher')),
  ADD CONSTRAINT selection_courses_review_fact_check CHECK (
    (review_count = 0 AND review_avg IS NULL AND review_scope = 'none')
    OR (
      review_count > 0
      AND review_avg IS NOT NULL
      AND review_avg BETWEEN 0 AND 5
      AND review_scope IN ('course', 'teacher')
    )
  );

DROP FUNCTION selection.parse_arrangement_line(TEXT);

-- Real upstream lines normally begin with `teacher name(code) `. Keeping the
-- parsed identity with the schedule fact prevents a repeated multi-teacher raw
-- row from assigning every period to an arbitrary owning row.
CREATE FUNCTION selection.parse_arrangement_line(line TEXT)
RETURNS TABLE (
  teacher_name TEXT,
  teacher_code TEXT,
  weekday INTEGER,
  start_slot INTEGER,
  end_slot INTEGER,
  week_numbers INTEGER[],
  location TEXT
)
LANGUAGE plpgsql
IMMUTABLE
STRICT
AS $$
DECLARE
  captures TEXT[];
  identity_captures TEXT[];
  identity_prefix TEXT;
BEGIN
  captures := regexp_match(
    BTRIM(line),
    '^(?:(.*)[[:space:]]+)?星期([一二三四五六日])([0-9]{1,2})-([0-9]{1,2})节[[:space:]]*\[([^]]+)\]([[:space:]]+(.+))?$'
  );
  IF captures IS NULL THEN
    RETURN;
  END IF;

  identity_prefix := NULLIF(BTRIM(captures[1]), '');
  identity_captures := regexp_match(COALESCE(identity_prefix, ''), '^\(([^()]*)\)$');
  IF identity_captures IS NOT NULL THEN
    teacher_code := NULLIF(BTRIM(identity_captures[1]), '');
  ELSE
    identity_captures := regexp_match(
      COALESCE(identity_prefix, ''),
      '^([^(),]+)\(([^()]*)\)$'
    );
    IF identity_captures IS NOT NULL THEN
      teacher_name := NULLIF(BTRIM(identity_captures[1]), '');
      teacher_code := NULLIF(BTRIM(identity_captures[2]), '');
    END IF;
  END IF;

  weekday := CASE captures[2]
    WHEN '一' THEN 1 WHEN '二' THEN 2 WHEN '三' THEN 3 WHEN '四' THEN 4
    WHEN '五' THEN 5 WHEN '六' THEN 6 WHEN '日' THEN 7
  END;
  start_slot := captures[3]::INTEGER;
  end_slot := captures[4]::INTEGER;
  IF start_slot < 1 OR end_slot > 20 OR end_slot < start_slot THEN
    RETURN;
  END IF;
  week_numbers := selection.parse_week_expression(captures[5]);
  location := NULLIF(BTRIM(captures[7]), '');
  RETURN NEXT;
END;
$$;

CREATE TABLE courses.search_projection_state (
  projection          TEXT PRIMARY KEY CHECK (projection IN ('catalogue', 'selection')),
  source_generation   BIGINT NOT NULL DEFAULT 0 CHECK (source_generation >= 0),
  indexed_generation  BIGINT CHECK (indexed_generation >= 0),
  source_rows         BIGINT NOT NULL DEFAULT 0 CHECK (source_rows >= 0),
  indexed_rows        BIGINT CHECK (indexed_rows >= 0),
  status              TEXT NOT NULL DEFAULT 'stale'
    CHECK (status IN ('stale', 'rebuilding', 'ready')),
  updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (
    status <> 'ready'
    OR (
      indexed_generation = source_generation
      AND indexed_rows = source_rows
    )
  )
);

INSERT INTO courses.search_projection_state (
  projection, source_generation, indexed_generation, source_rows, indexed_rows, status
)
VALUES
  (
    'catalogue', 1, NULL,
    (SELECT COUNT(*) FROM courses.courses), NULL, 'stale'
  ),
  (
    'selection', 1, NULL,
    (SELECT COUNT(*) FROM selection.courses), NULL, 'stale'
  );

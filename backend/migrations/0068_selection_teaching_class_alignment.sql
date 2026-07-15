-- 0068_selection_teaching_class_alignment.sql
-- Make teaching-class identity explicit, preserve schedule uncertainty, and add
-- durable/auditable selection import and synchronization operations.

ALTER TABLE selection.courses
  ADD COLUMN teaching_class_code TEXT,
  ADD COLUMN faculty_name TEXT,
  ADD COLUMN teaching_language TEXT,
  ADD COLUMN start_week INTEGER,
  ADD COLUMN end_week INTEGER,
  ADD COLUMN weeks_unknown BOOLEAN NOT NULL DEFAULT true,
  ADD COLUMN schedule_unknown BOOLEAN NOT NULL DEFAULT true,
  ADD COLUMN status TEXT NOT NULL DEFAULT 'unknown'
    CHECK (status IN ('unknown', 'active', 'cancelled')),
  ADD COLUMN catalogue_course_id BIGINT REFERENCES courses.courses(id) ON DELETE SET NULL,
  ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ADD CONSTRAINT selection_courses_week_range_check CHECK (
    (start_week IS NULL AND end_week IS NULL)
    OR (start_week BETWEEN 1 AND 30 AND end_week BETWEEN start_week AND 30)
  );

ALTER TABLE selection.timeslots
  ADD COLUMN week_numbers INTEGER[] NOT NULL DEFAULT ARRAY[]::INTEGER[],
  ADD COLUMN weeks_unknown BOOLEAN NOT NULL DEFAULT true,
  ADD COLUMN location_unknown BOOLEAN NOT NULL DEFAULT true,
  ADD CONSTRAINT selection_timeslots_week_numbers_check CHECK (
    week_numbers <@ ARRAY[
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
      11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      21, 22, 23, 24, 25, 26, 27, 28, 29, 30
    ]::INTEGER[]
  ),
  ADD CONSTRAINT selection_timeslots_unknown_week_check CHECK (
    weeks_unknown OR cardinality(week_numbers) > 0
  ),
  ADD CONSTRAINT selection_timeslots_unknown_location_check CHECK (
    location_unknown OR NULLIF(BTRIM(location), '') IS NOT NULL
  );

CREATE INDEX selection_courses_calendar_code_id_idx
  ON selection.courses (calendar_id, code, id);
CREATE INDEX selection_courses_nature_calendar_code_id_idx
  ON selection.courses (nature_id, calendar_id, code, id);
CREATE INDEX selection_courses_campus_calendar_code_id_idx
  ON selection.courses (campus_id, calendar_id, code, id);
CREATE INDEX selection_courses_catalogue_idx
  ON selection.courses (catalogue_course_id)
  WHERE catalogue_course_id IS NOT NULL;
CREATE INDEX selection_major_courses_major_grade_course_idx
  ON selection.major_courses (major_id, grade, course_id);
CREATE INDEX selection_major_courses_course_idx
  ON selection.major_courses (course_id) INCLUDE (major_id, grade);
CREATE INDEX selection_majors_grade_id_idx
  ON selection.majors (grade, id);
CREATE INDEX selection_timeslots_filter_idx
  ON selection.timeslots (weekday, start_slot, end_slot, course_id);
CREATE INDEX selection_timeslots_weeks_idx
  ON selection.timeslots USING GIN (week_numbers);
CREATE INDEX selection_fetchlog_latest_idx
  ON selection.fetchlog (fetched_at DESC, id DESC);

-- Parse upstream week expressions such as `1-16`, `1-15周(单)`, or
-- `2-14双 15-16`. Invalid or out-of-domain input returns NULL rather than
-- inventing schedule certainty.
CREATE FUNCTION selection.parse_week_expression(expression TEXT)
RETURNS INTEGER[]
LANGUAGE plpgsql
IMMUTABLE
STRICT
AS $$
DECLARE
  token TEXT;
  cleaned TEXT;
  captures TEXT[];
  first_week INTEGER;
  last_week INTEGER;
  current_week INTEGER;
  wants_odd BOOLEAN;
  wants_even BOOLEAN;
  parsed INTEGER[] := ARRAY[]::INTEGER[];
BEGIN
  IF BTRIM(expression) = '' THEN
    RETURN NULL;
  END IF;

  FOR token IN
    SELECT value FROM regexp_split_to_table(BTRIM(expression), '[[:space:]]+') AS value
  LOOP
    wants_odd := POSITION('单' IN token) > 0;
    wants_even := POSITION('双' IN token) > 0;
    IF wants_odd AND wants_even THEN
      RETURN NULL;
    END IF;

    cleaned := regexp_replace(token, '[周单双()（）]', '', 'g');
    captures := regexp_match(cleaned, '^([0-9]{1,2})(-([0-9]{1,2}))?$');
    IF captures IS NULL THEN
      RETURN NULL;
    END IF;

    first_week := captures[1]::INTEGER;
    last_week := COALESCE(captures[3], captures[1])::INTEGER;
    IF first_week < 1 OR last_week > 30 OR last_week < first_week THEN
      RETURN NULL;
    END IF;

    FOR current_week IN first_week..last_week LOOP
      IF (wants_odd AND current_week % 2 = 0)
        OR (wants_even AND current_week % 2 = 1) THEN
        CONTINUE;
      END IF;
      parsed := array_append(parsed, current_week);
    END LOOP;
  END LOOP;

  SELECT array_agg(DISTINCT value ORDER BY value)
  INTO parsed
  FROM unnest(parsed) AS value;
  RETURN parsed;
END;
$$;

-- Parse exactly one upstream arrangement line. Teacher identity is not parsed;
-- it comes from the owning raw row. A non-matching line yields no
-- row and is therefore represented as an unknown schedule by materialization.
CREATE FUNCTION selection.parse_arrangement_line(line TEXT)
RETURNS TABLE (
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
BEGIN
  captures := regexp_match(
    BTRIM(line),
    '星期([一二三四五六日])([0-9]{1,2})-([0-9]{1,2})节[[:space:]]*\[([^]]+)\]([[:space:]]+(.+))?$'
  );
  IF captures IS NULL THEN
    RETURN;
  END IF;

  weekday := CASE captures[1]
    WHEN '一' THEN 1 WHEN '二' THEN 2 WHEN '三' THEN 3 WHEN '四' THEN 4
    WHEN '五' THEN 5 WHEN '六' THEN 6 WHEN '日' THEN 7
  END;
  start_slot := captures[2]::INTEGER;
  end_slot := captures[3]::INTEGER;
  IF start_slot < 1 OR end_slot > 20 OR end_slot < start_slot THEN
    RETURN;
  END IF;
  week_numbers := selection.parse_week_expression(captures[4]);
  location := NULLIF(BTRIM(captures[6]), '');
  RETURN NEXT;
END;
$$;

-- Preserve facts already present in the raw mirror during rolling deployment.
UPDATE selection.courses AS course
SET teaching_class_code = NULLIF(BTRIM(raw.code), ''),
    faculty_name = NULLIF(BTRIM(raw.faculty), ''),
    teaching_language = NULLIF(BTRIM(raw.teaching_language), ''),
    start_week = CASE
      WHEN raw.start_week BETWEEN 1 AND 30
       AND raw.end_week BETWEEN raw.start_week AND 30 THEN raw.start_week
    END,
    end_week = CASE
      WHEN raw.start_week BETWEEN 1 AND 30
       AND raw.end_week BETWEEN raw.start_week AND 30 THEN raw.end_week
    END,
    weeks_unknown = NOT (
      raw.start_week BETWEEN 1 AND 30
      AND raw.end_week BETWEEN raw.start_week AND 30
    ),
    updated_at = now()
FROM selection.pk_course_details AS raw
WHERE raw.id = course.id;

CREATE TABLE selection.import_runs (
  id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  snapshot_sha256     CHAR(64) NOT NULL UNIQUE
    CHECK (snapshot_sha256 ~ '^[0-9a-f]{64}$'),
  snapshot_bytes      BIGINT NOT NULL CHECK (snapshot_bytes > 0),
  source_database     TEXT NOT NULL CHECK (char_length(source_database) BETWEEN 1 AND 128),
  snapshot_exported_at TIMESTAMPTZ,
  imported_by         TEXT NOT NULL CHECK (char_length(imported_by) BETWEEN 1 AND 200),
  source_table_counts JSONB NOT NULL,
  target_table_counts JSONB NOT NULL,
  validation          JSONB NOT NULL DEFAULT '{}'::JSONB,
  imported_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX selection_import_runs_imported_idx
  ON selection.import_runs (imported_at DESC, id DESC);

CREATE TABLE selection.sync_jobs (
  id                  UUID PRIMARY KEY,
  requested_by        BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason              TEXT NOT NULL CHECK (char_length(reason) BETWEEN 3 AND 500),
  idempotency_key_hash CHAR(64) NOT NULL CHECK (idempotency_key_hash ~ '^[0-9a-f]{64}$'),
  request_fingerprint CHAR(64) NOT NULL CHECK (request_fingerprint ~ '^[0-9a-f]{64}$'),
  status              TEXT NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued', 'running', 'succeeded', 'dead', 'cancelled')),
  step                TEXT NOT NULL DEFAULT 'queued'
    CHECK (step IN ('queued', 'materialize', 'catalogue', 'search', 'cache', 'complete')),
  attempts            SMALLINT NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 8),
  progress_current    INTEGER NOT NULL DEFAULT 0 CHECK (progress_current BETWEEN 0 AND 4),
  progress_total      INTEGER NOT NULL DEFAULT 4 CHECK (progress_total = 4),
  next_attempt_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  locked_at           TIMESTAMPTZ,
  lease_token         UUID,
  lease_expires_at    TIMESTAMPTZ,
  last_error_code     TEXT CHECK (
    last_error_code IS NULL OR last_error_code IN (
      'worker_lease_expired', 'materialize_failed', 'catalogue_failed',
      'search_failed', 'cache_failed', 'worker_iteration_failed'
    )
  ),
  result              JSONB NOT NULL DEFAULT '{}'::JSONB,
  started_at          TIMESTAMPTZ,
  completed_at        TIMESTAMPTZ,
  created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (requested_by, idempotency_key_hash),
  CHECK ((status = 'running') = (
    locked_at IS NOT NULL AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL
  )),
  CHECK (status = 'running' OR (
    locked_at IS NULL AND lease_token IS NULL AND lease_expires_at IS NULL
  )),
  CHECK ((status IN ('succeeded', 'dead', 'cancelled')) = (completed_at IS NOT NULL)),
  CHECK (status <> 'succeeded' OR progress_current = progress_total),
  CHECK (status <> 'dead' OR (attempts = 8 AND last_error_code IS NOT NULL))
);

CREATE UNIQUE INDEX selection_sync_jobs_one_active_idx
  ON selection.sync_jobs ((1))
  WHERE status IN ('queued', 'running');
CREATE INDEX selection_sync_jobs_due_idx
  ON selection.sync_jobs (next_attempt_at, created_at, id)
  WHERE status = 'queued';
CREATE INDEX selection_sync_jobs_history_idx
  ON selection.sync_jobs (created_at DESC, id DESC);

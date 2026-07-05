-- 0010_selection_raw_normalized.sql — Add missing review columns and selection PK materialization

-- ============================ reviews.reviews extensions ============================
-- D1 has reviewer_name, reviewer_avatar, wallet_user_hash, edit_token that PG is missing
ALTER TABLE reviews.reviews
  ADD COLUMN IF NOT EXISTS reviewer_name   TEXT NOT NULL DEFAULT '',
  ADD COLUMN IF NOT EXISTS reviewer_avatar TEXT NOT NULL DEFAULT '',
  ADD COLUMN IF NOT EXISTS wallet_user_hash TEXT,
  ADD COLUMN IF NOT EXISTS edit_token      TEXT;

-- ============================ selection.courses extensions ============================
-- Add index for code lookups (used by selection handlers)
CREATE INDEX IF NOT EXISTS idx_selection_courses_code ON selection.courses (code);
CREATE INDEX IF NOT EXISTS idx_selection_courses_calendar ON selection.courses (calendar_id);

-- ============================ pk indexes for query speed ============================
CREATE INDEX IF NOT EXISTS idx_pk_course_details_course_code_calendar
  ON selection.pk_course_details (course_code, calendar_id);

-- 0026_forum_flag_attempts.sql — preserve terminal report history across later reports.
-- Append-only: never edit an applied migration.

ALTER TABLE forum.flags
  DROP CONSTRAINT IF EXISTS flags_target_type_target_id_reporter_id_key;

CREATE UNIQUE INDEX flags_one_open_report_per_reporter
  ON forum.flags (target_type, target_id, reporter_id)
  WHERE status = 'open';

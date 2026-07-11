-- 0029_review_report_open_uniqueness.sql — retain report history while
-- allowing a reporter to open a new case after a terminal decision.

ALTER TABLE reviews.review_reports
  DROP CONSTRAINT IF EXISTS review_reports_review_id_reporter_account_id_key;

CREATE UNIQUE INDEX review_reports_one_open_per_reporter_idx
  ON reviews.review_reports (review_id, reporter_account_id)
  WHERE status = 'open';

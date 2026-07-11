-- 0023_review_moderation_decisions.sql — explicit terminal decisions for review reports.
-- Historical generic resolutions cannot be safely inferred as policy violations,
-- so they migrate to the neutral ignored decision.

UPDATE reviews.review_reports SET status = 'ignored' WHERE status = 'resolved';

ALTER TABLE reviews.review_reports
  ADD CONSTRAINT review_reports_status_check
  CHECK (status IN ('open', 'upheld', 'rejected', 'ignored')) NOT VALID;

ALTER TABLE reviews.review_reports VALIDATE CONSTRAINT review_reports_status_check;

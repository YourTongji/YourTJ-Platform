-- 0028_review_course_restrict.sql — preserve review history on course deletion.
-- A stale aggregate must never allow hidden or pending reviews to be removed
-- through an ON DELETE CASCADE course mutation.

ALTER TABLE reviews.reviews
  DROP CONSTRAINT reviews_course_id_fkey;

ALTER TABLE reviews.reviews
  ADD CONSTRAINT reviews_course_id_fkey
  FOREIGN KEY (course_id) REFERENCES courses.courses(id) ON DELETE RESTRICT;

-- Preserve legacy D1 interaction identities without treating client IDs as accounts.

CREATE TABLE reviews.legacy_review_likes (
  review_id BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  client_id TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  PRIMARY KEY (review_id, client_id)
);

CREATE TABLE reviews.legacy_review_reports (
  id BIGINT PRIMARY KEY,
  review_id BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  client_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  status TEXT NOT NULL,
  admin_note TEXT,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL,
  resolved_at BIGINT
);

CREATE INDEX legacy_review_reports_status_idx
  ON reviews.legacy_review_reports (status, created_at DESC);

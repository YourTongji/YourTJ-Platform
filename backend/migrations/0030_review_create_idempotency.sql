-- 0030_review_create_idempotency.sql — durable replay for review publication.

CREATE TABLE reviews.review_create_idempotency (
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id) ON DELETE CASCADE,
  idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 128),
  request_hash    TEXT NOT NULL CHECK (length(request_hash) = 64),
  review_id       BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  response        JSONB NOT NULL,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, idempotency_key)
);

CREATE INDEX review_create_idempotency_created_idx
  ON reviews.review_create_idempotency (created_at);

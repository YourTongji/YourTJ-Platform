-- 0004_review_remediation.sql — Review remediation: wallet claim, forum votes, comment path guard.
-- Append-only. This migration adds tables and columns required by the remediation blueprint.
-- Old code can safely ignore the new tables/columns/indexes.

-- ============================ identity.wallet_claim_challenges ============================
CREATE TABLE identity.wallet_claim_challenges (
  id         TEXT PRIMARY KEY,
  account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  nonce      TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  used_at    TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON identity.wallet_claim_challenges (account_id, expires_at);

-- ============================ identity.legacy_wallet_links extensions ============================
ALTER TABLE identity.legacy_wallet_links
  ADD COLUMN IF NOT EXISTS legacy_public_key TEXT,
  ADD COLUMN IF NOT EXISTS legacy_balance    BIGINT NOT NULL DEFAULT 0 CHECK (legacy_balance >= 0),
  ADD COLUMN IF NOT EXISTS imported_metadata JSONB;

-- ============================ forum.votes ============================
CREATE TABLE forum.votes (
  post_type   TEXT NOT NULL CHECK (post_type IN ('thread', 'comment')),
  post_id     BIGINT NOT NULL,
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  value       SMALLINT NOT NULL CHECK (value IN (-1, 1)),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (post_type, post_id, account_id)
);

-- ============================ forum.comments path uniqueness ============================
CREATE UNIQUE INDEX IF NOT EXISTS forum_comments_thread_path_unique
  ON forum.comments(thread_id, path) WHERE path IS NOT NULL;

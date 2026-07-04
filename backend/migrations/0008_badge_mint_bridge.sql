-- 0008_badge_mint_bridge.sql — Pending mint queue for badge credit bridge
CREATE TABLE platform.pending_mints (
  id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id),
  amount          BIGINT NOT NULL CHECK (amount > 0),
  idempotency_key TEXT UNIQUE NOT NULL,
  badge_slug      TEXT NOT NULL,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  minted_at       TIMESTAMPTZ
);
CREATE INDEX ON platform.pending_mints (minted_at) WHERE minted_at IS NULL;

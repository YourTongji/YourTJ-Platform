-- 0014_credit_signing_intents.sql — one-time authorization for user-initiated value movement.

CREATE TABLE credit.signing_intents (
  id              UUID PRIMARY KEY,
  account_id      BIGINT NOT NULL REFERENCES identity.accounts(id),
  public_key      TEXT NOT NULL,
  action          TEXT NOT NULL,
  request_hash    TEXT NOT NULL,
  snapshot        JSONB NOT NULL,
  idempotency_key TEXT NOT NULL,
  signing_bytes   TEXT NOT NULL,
  expires_at      TIMESTAMPTZ NOT NULL,
  consumed_at     TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (account_id, idempotency_key)
);
CREATE INDEX ON credit.signing_intents (expires_at) WHERE consumed_at IS NULL;

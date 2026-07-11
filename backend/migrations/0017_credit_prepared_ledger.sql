-- 0017_credit_prepared_ledger.sql — bind wallet signatures to exact ledger payloads.

ALTER TABLE credit.signing_intents
  ADD COLUMN ledger_entry JSONB,
  ADD COLUMN ledger_canonical TEXT;

ALTER TABLE credit.signing_intents
  ADD CONSTRAINT signing_intents_ledger_pair_check CHECK (
    (ledger_entry IS NULL AND ledger_canonical IS NULL)
    OR (ledger_entry IS NOT NULL AND ledger_canonical IS NOT NULL)
  );

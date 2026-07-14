-- Freeze the key that the pre-migration signing-intent query treated as current, then enforce
-- one active wallet key per account at the database boundary. Revoked keys remain available for
-- historical ledger verification.

WITH ranked_active_keys AS (
  SELECT
    account_id,
    public_key,
    row_number() OVER (
      PARTITION BY account_id
      ORDER BY created_at DESC, public_key DESC
    ) AS active_rank
  FROM identity.account_keys
  WHERE revoked_at IS NULL
)
UPDATE identity.account_keys account_key
SET revoked_at = now()
FROM ranked_active_keys ranked
WHERE ranked.active_rank > 1
  AND account_key.account_id = ranked.account_id
  AND account_key.public_key = ranked.public_key;

CREATE UNIQUE INDEX account_keys_one_active_per_account_idx
  ON identity.account_keys (account_id)
  WHERE revoked_at IS NULL;

COMMENT ON INDEX identity.account_keys_one_active_per_account_idx IS
  'Only the frozen canonical wallet key may authorize new signing intents; rotation requires a separately audited protocol.';

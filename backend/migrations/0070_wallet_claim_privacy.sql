-- 0070_wallet_claim_privacy.sql — bound challenge storage and retire claimed legacy credentials.

UPDATE reviews.reviews
SET wallet_user_hash = NULL,
    edit_token = NULL
WHERE account_id IS NOT NULL
  AND (wallet_user_hash IS NOT NULL OR edit_token IS NOT NULL);

ALTER TABLE reviews.reviews
  ADD CONSTRAINT reviews_claimed_legacy_credentials_cleared
  CHECK (account_id IS NULL OR (wallet_user_hash IS NULL AND edit_token IS NULL))
  NOT VALID;

ALTER TABLE reviews.reviews
  VALIDATE CONSTRAINT reviews_claimed_legacy_credentials_cleared;

DELETE FROM identity.wallet_claim_challenges
WHERE used_at IS NOT NULL OR expires_at <= clock_timestamp();

WITH ranked AS (
  SELECT id,
         row_number() OVER (
           PARTITION BY account_id
           ORDER BY created_at DESC, id DESC
         ) AS account_position
  FROM identity.wallet_claim_challenges
)
DELETE FROM identity.wallet_claim_challenges challenge
USING ranked
WHERE challenge.id = ranked.id
  AND ranked.account_position > 1;

CREATE UNIQUE INDEX wallet_claim_challenges_one_per_account_idx
  ON identity.wallet_claim_challenges (account_id);

-- 0024_invitation_expiry.sql — expiring, single-use staff account invitations.

ALTER TABLE identity.accounts
  ADD COLUMN invitation_expires_at TIMESTAMPTZ,
  ADD COLUMN invitation_accepted_at TIMESTAMPTZ;

UPDATE identity.accounts
SET invitation_expires_at = invited_at + interval '7 days'
WHERE invited_at IS NOT NULL AND invitation_expires_at IS NULL;

ALTER TABLE identity.accounts
  ADD CONSTRAINT invited_account_expiry_check CHECK (
    (invited_at IS NULL AND invitation_expires_at IS NULL)
    OR (invited_at IS NOT NULL AND invitation_expires_at > invited_at)
  );

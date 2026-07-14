-- Bind full-login sessions to a first-party installation without storing the browser identifier.
-- Existing clients remain valid because the digest is nullable. During a rolling deployment the
-- trigger carries a digest across refresh rotation even when an older application writer omits it.

ALTER TABLE identity.sessions
  ADD COLUMN client_installation_hash BYTEA,
  ADD CONSTRAINT sessions_client_installation_hash_length
    CHECK (client_installation_hash IS NULL OR octet_length(client_installation_hash) = 32);

CREATE INDEX sessions_live_installation_idx
  ON identity.sessions (account_id, client_installation_hash, last_used_at DESC, id DESC)
  WHERE client_installation_hash IS NOT NULL AND revoked_at IS NULL;

CREATE FUNCTION identity.inherit_session_installation_hash()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
  IF NEW.client_installation_hash IS NULL AND NEW.rotated_from_id IS NOT NULL THEN
    SELECT predecessor.client_installation_hash
      INTO NEW.client_installation_hash
      FROM identity.sessions predecessor
      WHERE predecessor.id = NEW.rotated_from_id;
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER sessions_inherit_installation_hash
BEFORE INSERT ON identity.sessions
FOR EACH ROW EXECUTE FUNCTION identity.inherit_session_installation_hash();

COMMENT ON COLUMN identity.sessions.client_installation_hash IS
  'Account-scoped SHA-256 digest of a first-party random installation UUID; never the raw browser value.';

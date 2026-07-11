-- 0055_governance_append_only_truncate.sql — close the statement-level append-only gap.
-- Append-only: never edit an applied migration.

CREATE TRIGGER governance_audit_events_no_truncate
  BEFORE TRUNCATE ON governance.audit_events
  FOR EACH STATEMENT EXECUTE FUNCTION governance.reject_audit_event_mutation();

CREATE TRIGGER governance_appeal_events_no_truncate
  BEFORE TRUNCATE ON governance.appeal_events
  FOR EACH STATEMENT EXECUTE FUNCTION governance.reject_appeal_event_mutation();

-- Production must additionally use a non-owner runtime role. This removes any
-- accidental grant inherited through PUBLIC, while the statement trigger also
-- protects migration-owner maintenance sessions that do not disable triggers.
REVOKE UPDATE, DELETE, TRUNCATE
  ON governance.audit_events, governance.appeal_events FROM PUBLIC;

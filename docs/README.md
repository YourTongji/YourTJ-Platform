# YourTJ documentation

> **Status:** DELIVERED IN THIS PR — documentation index and authority policy
>
> **Owner:** Platform maintainers
>
> **Last verified:** 2026-07-11 against this PR, including migrations `0020`–`0031`
>
> **Authoritative sources:** `AGENTS.md`, `contract/openapi.yaml`, `backend/migrations/`, domain source code

This directory separates current product rules from implementation inventories and historical plans.
Every normative document must state its owner, verification baseline, authoritative sources, and the
implementation state of each material behavior.

## Authority

When sources disagree, use this order and fix the disagreement in the same PR:

1. Legal/compliance and security invariants in `AGENTS.md`.
2. `contract/openapi.yaml` for the public HTTP wire contract.
3. Numbered migrations for the deployed database shape.
4. Current product/security documents for business rules and acceptance criteria.
5. Source code for what the current binary actually does.

No lower source silently overrides a higher one. A mismatch between contract, migration, documentation,
and code is a bug, not an alternative implementation.

## Status vocabulary

Normative documents use exactly two implementation labels:

- **DELIVERED IN THIS PR:** implemented in this branch and backed by the cited contract, migration, source,
  and verification baseline.
- **FOLLOW-UP:** not implemented by this PR. It remains a product/security requirement or backlog item and
  must not be described as deployed.

“Historical” describes a document's authority class, not an implementation status. Do not introduce
alternative implementation labels or unqualified future-tense requirements that readers could mistake
for delivered behavior.

## Normative documents

- [Community governance](product/community-governance.md) — account and content lifecycle, moderation,
  sanctions, appeals, and retention.
- [Activity scoring](product/activity-scoring.md) — daily contributions, policy weights, APIs, and
  heatmap behavior.
- [Profiles and messaging](product/profile-and-messaging.md) — public/private fields, profile behavior,
  direct-message privacy, blocking, reporting, and retention.
- [RBAC and audit](security/rbac-and-audit.md) — role hierarchy, capabilities, staff protections, and
  cross-domain audit requirements.
- [Admin console](operations/admin-console.md) — information architecture, workflows, safety rules, and
  delivery criteria.
- [Email delivery](operations/email-delivery.md) — provider configuration, failure semantics, secret
  isolation, deployment, and incident response.

## Migrations delivered in this PR

- `0020_activity.sql` — append-only activity transitions, daily projections, and versioned score policy.
- `0021_dm_moderation.sql` — canonical DM pairs, read pointers, reserved participant lifecycle fields, and
  message reports.
- `0022_governance.sql` — central governance audit events and invitation provenance.
- `0023_review_moderation_decisions.sql` — explicit review-report terminal decisions.
- `0024_invitation_expiry.sql` — expiring, accepted-once invitation metadata.
- `0025_moderation_state.sql` — automatic forum-hide provenance, resolution notes, and system-issued
  sanction support.
- `0026_forum_flag_attempts.sql` — preserves terminal report decisions while allowing one later open
  report attempt per reporter and target.
- `0027_activity_backfill.sql` — idempotently projects existing visible community contributions into the
  activity event and daily-count model.
- `0028_review_course_restrict.sql` — prevents course deletion from cascading into retained review
  history.
- `0029_review_report_open_uniqueness.sql` — retains terminal review-report history while allowing one
  later open report per reporter and review.
- `0030_review_create_idempotency.sql` — stores account-scoped review request hashes and original
  responses for durable publication replay.
- `0031_forum_board_thread_count_reconcile.sql` — reconciles existing board counters with the visible,
  non-hidden, non-deleted, non-archived thread definition maintained on future state transitions.

## Supporting and historical documents

- [`REWRITE_V2_DESIGN.md`](REWRITE_V2_DESIGN.md) is a historical architecture baseline. Use OpenAPI and
  migrations for current interfaces and schema.
- [`FORUM_DISCOURSE_PARITY.md`](FORUM_DISCOURSE_PARITY.md) is the historical forum parity plan. Its phase
  matrix is not a live implementation inventory.
- [`ARCH_REVIEW_AND_E2E_PLAN.md`](ARCH_REVIEW_AND_E2E_PLAN.md) records an architecture review and an E2E
  proposal; verify checklist items against current CI before relying on them.
- [`D1_LOCAL_IMPORT.md`](D1_LOCAL_IMPORT.md) is the operational D1 import procedure.

## Maintenance rules

- Do not duplicate full OpenAPI paths or migration DDL in prose.
- Do not hard-code path/schema/operation counts; derive them in CI when useful.
- Change the contract first for HTTP changes, then implementation, generated clients, docs, and tests.
- A behavior is only labelled `DELIVERED IN THIS PR` after its migration/contract (when applicable),
  backend authorization, frontend surface (when applicable), and verification baseline agree.
- Known missing behavior is labelled `FOLLOW-UP`; schema placeholders alone do not make a feature
  delivered.
- Historical plans remain available for rationale but must carry a non-authoritative banner.

# YourTJ documentation

> **Status:** Current documentation index and authority policy
>
> **Owner:** Platform maintainers
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
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

- **Current:** normative behavior already expected of the product.
- **Proposed in this PR:** normative target that must not be described as deployed until its code and
  contract land and pass verification.
- **Future follow-up:** explicitly outside the current PR.
- **Historical:** context only; never an authority for current behavior.

The current governance PR uses inline labels — `CURRENT`, `PR TARGET`, and `FOLLOW-UP` — so readers can
tell shipped behavior from intended behavior without consulting git history.

## Current normative documents

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
- A behavior is only relabeled `CURRENT` after its migration, contract, backend authorization, frontend,
  and rejection-path tests are complete.
- Historical plans remain available for rationale but must carry a non-authoritative banner.

# RBAC and staff audit

> **Status:** Current normative specification; expanded hierarchy and central audit are proposed in this PR
>
> **Owner:** Security + Identity maintainers
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
>
> **Authoritative sources:** `AGENTS.md`, `contract/openapi.yaml`, `shared::AuthAccount`, identity/forum/reviews/courses/media/admin source

Role checks protect backend operations. UI visibility is convenience only and never an authorization
boundary.

## Implementation baseline

- **CURRENT:** `user`, `mod`, and `admin` roles exist; `require_mod` accepts both staff roles and
  `require_admin` exists.
- **CURRENT GAP:** most management endpoints use `require_mod`, so moderators can currently reach account
  suspension, settings, course mutation, jobs, and policy-like operations. Audit is limited to forum
  actions and is not transactionally guaranteed.
- **PR TARGET:** capability-based guards derived from roles, hierarchy protection, admin-only sensitive
  operations, and one append-only cross-domain audit stream.
- **FOLLOW-UP:** separate privacy/security operator roles if operational staffing grows enough to justify
  them. Until then, narrow capabilities are preferable to adding broad roles.

## Role hierarchy

```text
user < mod < admin
system/service is an actor type, not a human role
```

- A moderator may act only on ordinary users and community content.
- An admin may act on users and moderators but not silently override an equal admin.
- No one may sanction themselves, promote themselves, demote the final active admin, or approve their own
  privileged invitation/role change.
- Role and suspension changes revoke refresh sessions and invalidate authorization caches immediately.

## Capability matrix

| Capability | user | mod | admin |
|---|:---:|:---:|:---:|
| create/edit own content and profile | yes | yes | yes |
| review community/review/media report queues | — | yes | yes |
| hide/remove/restore ordinary-user content | — | yes | yes |
| view reported DM evidence only | — | yes | yes |
| silence an ordinary user for a bounded period | — | yes | yes |
| suspend, deactivate, or delete an account | — | — | yes |
| invite an account or assign/revoke staff role | — | — | yes |
| reveal a masked campus email | — | — | separate audited admin capability |
| manage boards, tags, watched words, badges | — | read/operate as delegated | yes |
| mutate course catalogue or platform settings | — | — | yes |
| change activity weights | — | — | yes |
| trigger sync/reindex/retention jobs | — | — | yes |
| read domain-scoped staff log | — | yes | yes |
| read/export full audit log | — | — | yes |
| directly change wallet balance or ledger history | — | — | — |
| run read-only credit verify/reconciliation | — | — | yes |

Delegation to moderators is explicit per capability, not implied by access to `/admin` routes.

## Authorization rules

- Define named server capabilities such as `moderation.content`, `moderation.dm_evidence`,
  `users.silence`, `users.suspend`, `users.roles`, `platform.settings`, `activity.policy`, and
  `operations.jobs`.
- Each handler checks one named capability and target hierarchy before reading sensitive data or starting
  a transaction.
- List endpoints filter fields and rows by capability; authorization is not limited to mutation routes.
- Sensitive actions require recent authentication or an explicit confirmation challenge. Access tokens
  alone are insufficient for role changes, account deletion, email reveal, or indefinite suspension.
- A 403 response uses the normal error envelope and reveals no target existence beyond what the caller may
  already list.

## Central audit model proposed in this PR

One platform-owned append-only audit table covers every staff and automated management action. Domain
tables may keep operational histories, but they do not replace this record.

Required fields:

- immutable id and timestamp;
- actor kind (`account`, `system`, `service`) and optional actor account id;
- actor role/capability snapshot;
- action and target type/id;
- mandatory reason for mutations;
- request id and source surface;
- result (`succeeded`, `rejected`, `failed`);
- non-sensitive before/after summary or hashes;
- restricted metadata for policy category, related reports, and job id.

The model must represent system actors without inventing account id `0`. Secrets, raw email, tokens,
signatures, full request bodies, and unrestricted DM content never enter general audit metadata.

### Atomicity

- A successful business mutation and its success audit event commit in the same transaction.
- If audit insertion fails, the protected mutation fails.
- Rejected privileged attempts record a bounded security event without leaking request secrets.
- Background jobs record requested, started, succeeded/failed states linked by job id.
- Audit rows are never updated or deleted by ordinary application flows. Corrections are new events.

## Staff safety and transparency

- Destructive and high-impact UI actions show the target, effect, duration, and required reason before
  confirmation.
- The subject receives a notification with action category, duration, and appeal guidance unless a
  documented exception applies.
- Staff can see only the minimum PII needed. Email reveal and DM evidence view are themselves audited.
- Audit export is admin-only, watermarked, rate-limited, and excludes encrypted secrets.
- Retention follows [Community governance](../product/community-governance.md); access is reviewed at least
  quarterly.

## Acceptance criteria

- Every management route maps to exactly one named capability and has user/mod/admin rejection tests.
- Mods cannot suspend staff, modify roles/settings/activity policy/courses, or trigger operational jobs.
- Admin self-action and final-admin protections are tested under concurrent requests.
- Every successful management mutation produces one atomic audit event; forced audit failure rolls the
  mutation back.
- System jobs use a legal system actor rather than a fake account foreign key.
- PII reveal and DM evidence access are independently capability-checked and auditable.
- Frontend controls render from capabilities, but direct API calls remain correctly denied.

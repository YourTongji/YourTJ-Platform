# RBAC and staff audit

> **Status:** DELIVERED IN THIS PR — security hardening is follow-up
>
> **Owner:** Security + Identity maintainers
>
> **Last verified:** 2026-07-11 against this PR, migration `0022_governance.sql`, OpenAPI, and current source
>
> **Authoritative sources:** `AGENTS.md`, `contract/openapi.yaml`, `shared::AuthAccount`, `backend/crates/governance`, identity/forum/reviews/courses/media/admin source

Backend checks authorize every delivered staff operation. Capability-based UI rendering is a usability and
data-minimization measure, never an authorization boundary.

## Implementation baseline

- **DELIVERED IN THIS PR:** named capabilities are derived from persisted `user`, `mod`, and `admin` roles,
  included in account responses, checked by admin handlers, and used to build Web navigation/actions.
- **DELIVERED IN THIS PR:** identity user-management mutations reject self and equal/higher-role targets;
  role changes and suspension/session actions revoke active refresh sessions as applicable.
- **DELIVERED IN THIS PR:** forum and review content actions resolve the author's role through the
  privacy-safe identity boundary; moderators cannot act on moderator/admin authors and administrators
  cannot act on administrator authors.
- **DELIVERED IN THIS PR:** `governance.audit_events` is an append-only cross-domain record with account,
  system, and service actor kinds. Integrated management mutations write it transactionally; reported-DM
  evidence listing is also audited.
- **FOLLOW-UP:** recent-authentication challenges, standardized request id/source/result audit fields,
  rejected-attempt audit, audit export controls, notification/appeal flows, and privacy-owner retention.
- **FOLLOW-UP:** a complete deactivated/deleted account lifecycle and conflict-of-interest policy beyond
  the delivered role hierarchy, including assignment, recusal, and independent appeal review.

## Delivered role and capability mapping

```text
user < mod < admin
system/service is an audit actor kind, not a human role
```

The mapping in `shared::auth` is currently static by role; per-account capability delegation is not
implemented.

| Named capability | user | mod | admin | Delivered use |
|---|:---:|:---:|:---:|---|
| `moderation.content` | — | yes | yes | forum/review/media queues, reported-DM evidence, forum inline actions |
| `users.search` | — | yes | yes | privacy-safe user directory and sanction history |
| `users.silence` | — | yes | yes | silence/revoke silence for lower-role targets |
| `audit.read` | — | yes | yes | central audit list and filters |
| `users.invite` | — | — | yes | expiring campus invitation |
| `users.roles` | — | — | yes | lower-role role changes |
| `users.suspend` | — | — | yes | suspension, revoke suspension, session revoke |
| `community.manage` | — | — | yes | boards, tags, watched words, badge backend operations |
| `courses.manage` | — | — | yes | course catalogue management |
| `platform.settings` | — | — | yes | generic setting updates |
| `activity.policy` | — | — | yes | scoring policy publish/history |
| `announcements.manage` | — | — | yes | announcement management |
| `operations.jobs` | — | — | yes | selection sync and reindex triggers |

No delivered capability allows direct wallet-balance mutation, arbitrary ledger append, general DM inbox
browsing, or campus-email reveal.

## Authorization rules delivered in this PR

- Handlers require the named capability before sensitive list or mutation work.
- The identity user directory returns public operational fields only; it does not return campus email.
- Identity account mutations lock the target, reject self/equal/higher-role actions, and require a bounded
  reason.
- An admin cannot change or sanction another admin under the delivered `require_lower_role` rule. This is
  stricter than a separate “final admin only” guard.
- Interactive role changes are limited to `user` and `mod`. Administrator provisioning stays out of band
  until a separate super-admin/recovery policy can make promotion and demotion reversible.
- Moderator-issued silence must end within 30 days; admins retain the separately authorized longer or
  indefinite path. Both still require a lower-role target and an audit reason.
- Role changes revoke the target's active refresh sessions. Suspension creates a sanction and revokes
  sessions; authentication checks active suspend sanctions on every authenticated request.
- A missing capability returns the platform error envelope. Backend denial remains effective even when a
  caller manually constructs the request.
- DM moderation exposes only reported-message evidence and records an audit event when that queue is read.

### Authorization follow-up

- Recent authentication or an explicit challenge for role changes, indefinite suspension, account
  lifecycle deletion, PII reveal, and other high-impact actions. JWT `iat` is not currently used as a
  recent-auth proof.
- Dedicated PII-reveal capability and audited workflow; no email-reveal endpoint is delivered now.
- Optional per-account capability grants if static role mapping becomes too broad.

## Central audit model delivered in this PR

Migration `0022` stores these fields:

- `id` and `created_at`;
- `actor_kind` (`account`, `system`, or `service`);
- nullable `actor_account_id` and `actor_role`;
- `action`;
- `target_type` and `target_id`;
- nullable `reason`;
- nullable `metadata` JSON; writers are responsible for keeping it non-sensitive and purpose-limited.

The account actor invariant requires an account id exactly when `actor_kind = 'account'`. Actor handle is
joined for the admin response and is not copied into the immutable event. The writer API also supports a
system actor without inventing account id `0`.

Secrets, raw campus email, tokens, signatures, full request bodies, and unrestricted DM content must not
enter audit metadata.

### Atomicity delivered in this PR

- Integrated business mutations call `record_account_event_tx` or `record_system_event_tx` inside their
  existing PostgreSQL transaction, so a failed audit insert prevents that transaction from committing.
- Audit rows are append-only in ordinary application flows. Revocation and correction create new events.
- Reported-DM evidence listing writes a separate reasoned read event because it is a read rather than a
  domain mutation.
- Operational trigger handlers audit the request, but no durable job lifecycle exists.

### Audit fields and behaviors not delivered

- **FOLLOW-UP:** first-class `request_id` and source-surface columns.
- **FOLLOW-UP:** first-class result state such as `succeeded`, `rejected`, or `failed`.
- **FOLLOW-UP:** standardized before/after hashes and actor capability snapshot. Current domain metadata is
  action-specific and actor role is stored, but capabilities are not snapshotted.
- **FOLLOW-UP:** bounded rejected/failed privileged-attempt events. Current writers primarily record
  successful integrated operations.
- **FOLLOW-UP:** persistent job requested/started/succeeded/failed events linked by a durable job id.
- **FOLLOW-UP:** watermark/rate-limit/export workflow. The delivered console is a paginated read-only list.

## Staff safety and transparency

### Delivered

- Risky Web actions show effect and target context, require a reason, and disable duplicate submission.
- User-management responses and profile deep links use public identifiers rather than exposing campus
  email.
- Staff cannot browse arbitrary DMs; only participant-reported evidence is available.
- Audit metadata is displayed only as the presence of structured context in the general console,
  not dumped as an unrestricted request body.

### Follow-up

- Subject notification with action category, duration, and appeal guidance.
- User-visible appeals and independent review rules.
- Recent-authentication challenges and exceptional-action dual approval.
- Audit/evidence retention policy, automated deletion, legal holds, and periodic access review.
- Transactional/outbox index delivery and audit correlation for asynchronous repairs; public forum search
  already revalidates candidate visibility against PostgreSQL.

## Delivered verification baseline

- Account responses include the capability names for the persisted role; Web navigation/actions consume
  those capabilities.
- Moderators lack invitation, role, suspension, course, settings, activity-policy, announcement, and job
  capabilities; direct backend calls are denied.
- Identity user mutations reject self and equal/higher-role targets and revoke sessions where documented.
- Forum/review moderation rejects equal/higher-role authors; media moderation rejects self-review.
- Integrated management mutations append central audit events without logging campus email or DM bodies.
- Reported-DM evidence listing is capability-checked and audited, with no general DM browsing route.
- Frontend visibility and backend authorization are independently enforced.

Recent auth, account deletion lifecycle, standardized request/result audit fields, subject notifications,
appeals, retention, and transactional search-index delivery are explicitly excluded from this delivered
baseline.

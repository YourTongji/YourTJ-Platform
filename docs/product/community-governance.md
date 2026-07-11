# Community governance

> **Status:** Current normative specification; implementation state is labelled per section
>
> **Owner:** Community operations + Identity/Forum/Reviews maintainers
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
>
> **Authoritative sources:** `AGENTS.md` §5, `contract/openapi.yaml`, migrations `0001`, `0005`, `0006`, identity/forum/reviews source

This document defines how accounts and community content move through their lifecycles. It is the
business-policy source for moderation; [RBAC and audit](../security/rbac-and-audit.md) defines who may
perform each action.

## Implementation baseline

- **CURRENT:** campus-email self-registration, `user/mod/admin` roles, forum soft deletion, forum flags,
  review reports, silence/suspend sanctions, watched words, and forum-only staff action records exist.
- **PR TARGET:** invitation-based manual account creation, explicit role hierarchy, consistent reversible
  moderation across forum/reviews/media, central audit events, activity policy, and complete admin UI.
- **FOLLOW-UP:** user-facing appeals case management, self-service data export, and automated retention
  workers beyond the minimum deletion hooks delivered here.

## Governing principles

1. User speech is not silently rewritten. Staff normally hide, remove, restore, or request an author
   correction. `redact` is exceptional, reasoned, and revision-preserving.
2. Reversible action comes before destructive action. Permanent purge is a retention operation, not a
   moderator button.
3. Every staff mutation requires a reason and an atomic audit event.
4. The subject is notified unless doing so would create a documented safety or legal risk.
5. Public identity is the handle; campus email remains private and is only exposed to specifically
   authorized account operations.
6. Credit ledger history is never deleted or rewritten. Account erasure replaces identity links with a
   non-identifying tombstone while preserving ledger verification.

## Account lifecycle

Account lifecycle and sanctions are separate dimensions:

```text
invited -> active -> deactivated -> deleted
               \-> active after reactivation

active + silence  = can sign in and read, cannot create community content
active + suspend  = cannot authenticate or refresh sessions
```

The current database does not yet represent `invited` or `deactivated` separately and has both an account
`suspended` status and suspend sanctions. **PR TARGET:** sanctions are the authority for time-bounded
silence/suspension; lifecycle status is used for activation and deletion only. Code must not infer both.

### Registration

- **CURRENT:** a verified `@tongji.edu.cn` email can create an account.
- **PR TARGET:** manual registration means creating a one-time, expiring invitation for a campus email.
  The user still proves mailbox ownership and chooses credentials. Staff never set or view a plaintext
  password and cannot bypass campus ownership verification.
- Invitation creation is admin-only, rate-limited, idempotent per email, and audited. The UI displays only
  a masked email after submission.
- Role assignment is a separate admin-only action; invitations create ordinary users by default.

### Suspension, deactivation, and deletion

- `silence` is a temporary or indefinite write restriction. It must cover forum posts/comments/votes,
  reviews, DMs, and user-authored marketplace descriptions.
- `suspend` blocks access-token use and refresh immediately, revokes active refresh sessions, and prevents
  new sessions. Only admins may suspend; moderators may issue silence to ordinary users.
- `deactivated` is a reversible account-owner/admin lifecycle action, not a misconduct label.
- `deleted` begins a 30-day recovery window. Public identity is immediately replaced by a tombstone,
  sessions and wallet keys are revoked, and email is inaccessible to normal staff. After the window,
  encrypted email and mutable profile data are purged.
- Public content is not automatically erased when doing so would break a discussion or ledger invariant;
  its author is anonymized. A separate content-erasure request follows the content retention rules below.

## Content lifecycle

| State | Public behavior | Reversible | Typical actor |
|---|---|---:|---|
| visible | Included in feeds/search/profile | — | author/system |
| pending | Not generally discoverable; awaiting review | yes | system/mod |
| hidden | Temporarily unavailable; evidence retained | yes | mod/system |
| removed | Tombstone shown where structure matters | yes during retention window | author/mod |
| redacted | Only prohibited fragment replaced; revision retained | yes by admin | admin |
| purged | Payload physically removed after retention | no | retention worker |

Forum threads/comments preserve structural tombstones. Reviews use hide/restore rather than physical
deletion. Media is quarantined before object deletion. Search and caches must follow the same visibility
state as PostgreSQL.

An author may edit their own content and receives normal revision history. A moderator may not use author
edit endpoints to change meaning. Admin redaction is limited to exposed PII, illegal material, or a valid
legal/safety request and records before/after hashes in restricted audit metadata.

## Reports and moderation decisions

Forum flags, review reports, media reports, and future DM reports remain domain-owned but use the same
decision vocabulary:

- `open`: waiting for staff review.
- `upheld`: policy violation confirmed; apply a specified reversible content action and, if warranted, a
  sanction.
- `rejected`: no violation; undo automatic hiding.
- `ignored`: insufficient/actionless report; undo automatic hiding without rewarding or penalizing either
  party.

A decision records policy category, free-text reason, actor, target, resulting action, timestamps, and
related report IDs. “Resolve” without a decision is not sufficient. Duplicate reports by one account do
not increase weight. Automated hiding is temporary until a human decision.

## Sanctions and escalation

- Moderators may silence ordinary users for a bounded period.
- Admins may silence or suspend ordinary users and moderators.
- No staff member may sanction themselves, an equal/higher role, or the final active administrator.
- Repeated violations may escalate from warning -> silence -> suspension. The reason and end time are
  mandatory; indefinite suspension requires an admin confirmation step.
- Revocation creates a new audit event and never overwrites the original sanction history.
- **FOLLOW-UP:** expose a user appeal case with status, evidence, reviewer, decision, and one independent
  reviewer for sanctions longer than 30 days.

## Retention and erasure

These are maximum product targets and require privacy-owner review before production rollout. A shorter
legal or user-requested period wins unless security evidence must be preserved.

| Data | Target retention |
|---|---|
| Email verification/reset codes | delete within 24 hours after expiry |
| Expired/revoked refresh sessions | 30 days; IP/user-agent fields at most 90 days |
| Removed content payload | 30-day recovery, then redact; upheld-report evidence at most 180 days |
| Unreported DM content | until participant deletion rules apply; 30-day recovery after both delete |
| Reported DM excerpt/evidence | at most 180 days, access logged |
| Fine-grained activity events | 400 days; daily totals follow account erasure policy |
| Sanctions and staff audit events | 2 years, then aggregate or purge non-required metadata |
| Credit ledger | permanent append-only record with deleted-account tombstone |

Retention workers must be idempotent, observable, and auditable. Backups follow the same expiry intent and
must not be used to silently reactivate erased production records.

## Acceptance criteria

- Every staff write has an allowed capability, mandatory reason, atomic audit event, and rejection tests.
- Hide/remove actions can be restored inside their recovery window without losing revisions or counts.
- A moderator cannot act on staff accounts or change roles/configuration.
- Suspension revokes refresh sessions and blocks access immediately; expiry/revocation restores only the
  intended capability.
- Deleted accounts expose no email or former handle publicly and cannot authenticate.
- Forum, reviews, media, search, cache, profiles, and activity totals agree on content visibility.
- Report queues require `upheld`, `rejected`, or `ignored`; a generic resolved state is not the terminal
  business decision.

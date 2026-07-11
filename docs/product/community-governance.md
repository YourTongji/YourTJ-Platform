# Community governance

> **Status:** DELIVERED IN THIS PR — lifecycle and transparency follow-ups are explicit
>
> **Owner:** Community operations + Identity/Forum/Reviews maintainers
>
> **Last verified:** 2026-07-11 against this PR, migrations `0022`–`0031`, OpenAPI, and current source
>
> **Authoritative sources:** `AGENTS.md` §5, `contract/openapi.yaml`, migrations
> `0022_governance.sql` through `0031_forum_board_thread_count_reconcile.sql`,
> identity/forum/reviews/media source

This document defines the delivered community-governance baseline and separates it from future account
lifecycle, transparency, and retention work. [RBAC and audit](../security/rbac-and-audit.md) defines who
may perform each delivered action.

## Implementation baseline

- **DELIVERED IN THIS PR:** expiring staff-created campus invitations, named capabilities, target-role
  hierarchy checks, reasoned user sanctions and revocation, refresh-session revocation, and a
  capability-driven admin UI.
- **DELIVERED IN THIS PR:** reversible forum thread/comment moderation, explicit forum/review report
  decisions, media approve/block, scoped reported-DM review, and cross-domain governance audit events for
  the management mutations integrated in this PR.
- **DELIVERED IN THIS PR:** automatic forum report hiding is represented separately from a staff hide;
  reject/ignore restores only the automatic transition, while uphold keeps the contribution withdrawn and
  soft-deletes the target.
- **DELIVERED IN THIS PR:** content moderation enforces author-role hierarchy (`user < mod < admin`), and
  staff cannot use the ordinary user-report endpoint as an unreasoned privileged moderation shortcut.
- **DELIVERED IN THIS PR:** public forum search revalidates every Meilisearch candidate against PostgreSQL;
  state transitions reconcile documents and full reindex waits for clear-before-add completion.
- **FOLLOW-UP:** coherent `deactivated` and `deleted` account lifecycle actions, recovery, anonymization,
  erasure, and purge workers. The existing account status enum is not a complete lifecycle implementation.
- **FOLLOW-UP:** subject notifications, user-facing appeals, recent-authentication challenges, automated
  retention, and transactional/outbox search-index delivery.

## Governing principles

These principles govern current and follow-up work; only behavior explicitly labelled delivered below is
implemented today.

1. Staff do not silently rewrite user speech. Delivered actions hide, soft-delete, restore, or change
   workflow state.
2. Reversible action comes before permanent destruction. No delivered moderator control performs a
   retention purge.
3. Delivered staff mutations require a reason and write domain/governance audit records as part of their
   protected transaction where applicable.
4. Public identity is the handle; campus email is excluded from public and delivered staff-directory DTOs.
5. Credit ledger history is never deleted or rewritten. Governance work must not add balance editors or
   arbitrary ledger appends.
6. **FOLLOW-UP:** subjects should receive action/duration/appeal guidance unless a documented safety or
   legal exception applies.

## Account lifecycle: delivered boundary

The database currently has `active`, `suspended`, and `deleted` account-status values. This PR does not add
`deactivated`, nor does it deliver admin endpoints for deactivate/reactivate/delete/recover/purge.

Invitations are represented by account metadata (`invited_by`, `invited_at`, `invitation_expires_at`, and
`invitation_accepted_at`) rather than a separate `invited` status. Suspension performed by the new admin
workflow is an append-only sanction dimension; authentication checks both account status and active
suspend sanctions.

The intended lifecycle remains **FOLLOW-UP**:

```text
active -> deactivated -> deleted -> purged mutable identity
   ^           |
   +-----------+ reactivation during the allowed window

active + silence = authenticate/read, but protected community writes are denied
active + suspend = authentication and refresh are denied
```

Do not infer that `deactivated` or the recovery/purge behavior exists merely because the legacy status enum
contains `deleted`.

### Registration and invitation delivered in this PR

- Existing campus-email self-registration remains available.
- Authorized admins can create a one-time, seven-day campus invitation with handle and mandatory reason.
  Invitations always start as `user`; staff may grant `mod` only through the separate audited role-change
  workflow after mailbox verification.
- Invitation creation stores an encrypted/blind-indexed email through the identity repository, records the
  inviter, and writes a governance audit event in the same transaction.
- The invited person must still prove mailbox ownership through the email-code flow. Staff cannot set or
  view a plaintext password.
- Expired invitations are rejected; successful verification records acceptance.

### Sanctions delivered in this PR

- `silence` blocks protected forum writes/votes, review writes, and DM creation/sending while allowing
  authentication and reads.
- `suspend` prevents authentication/refresh and revokes active sessions when issued.
- Moderator capability permits silence of lower-role targets for at most 30 days; suspension is admin
  capability only. Admins may issue longer or indefinite sanctions when policy requires it.
- Self-action and equal/higher-role targets are rejected. Role changes also revoke active sessions.
- Sanctions are appended with reason and optional end time; revocation marks the sanction revoked and
  appends a new governance event rather than deleting history.
- **FOLLOW-UP:** recent authentication or a separate confirmation challenge for indefinite/high-impact
  sanctions and role changes.
- **FOLLOW-UP:** subject notifications and appeal guidance.

## Content lifecycle delivered in this PR

| State/action | Delivered behavior | Reversible now |
|---|---|---:|
| visible | included in the owning domain's normal reads | — |
| pending review/media | withheld until decision where the domain supports pending state | yes |
| hidden | excluded from public reads while evidence remains | yes |
| soft-deleted/removed | payload and structural record retained; normal public reads exclude it | yes |
| restored/unhidden | public visibility and activity contribution return only when no other hidden/deleted state remains | yes |
| archived thread | removed from active discussion flow | yes through unarchive |
| redacted | **FOLLOW-UP:** no general revision-preserving redaction workflow delivered | — |
| purged | **FOLLOW-UP:** no retention purge worker or moderator button delivered | — |

Forum thread controls support pin/unpin, close/reopen, archive/unarchive, hide/unhide, soft-delete/restore,
and move. Comments support hide/unhide and soft-delete/restore. Review moderation supports explicit report
decisions and visibility/removal operations. Media supports approve/block; block removes the OSS object
before committing the blocked state, while pending URLs are limited to the owner or staff. Staff cannot
approve or block an upload they own.

PostgreSQL is the authoritative state. Public forum search reconstructs results from current visible
database rows, so a stale index document cannot disclose hidden, deleted, archived, or non-visible forum
content. Forum state changes reconcile the affected document and full reindex clears before rebuilding.
Transactional/outbox delivery and equivalent review/media index repair remain follow-up work.

## Reports and moderation decisions delivered in this PR

- Forum flags: `open -> upheld | rejected | ignored`.
- Review reports: `open -> upheld | rejected | ignored`; migration `0023` maps legacy generic `resolved`
  rows to neutral `ignored`.
- DM reports: `open -> upheld | rejected`; there is no `ignored` DM status in the delivered schema.
- Media uses approve/block actions rather than the report decision enum.

Forum flag weight can cross the automatic-hide threshold. That transition sets `auto_hidden_at`, hides the
target, and reverses its activity contribution immediately. A rejected or ignored decision removes the
hide only when it matches the recorded automatic transition and reactivates an otherwise-visible target.
An upheld decision soft-deletes the target and keeps the activity contribution inactive.
Moderators and administrators are rejected by the ordinary flag-submission endpoint; staff must use a
reasoned, audited moderation action.

Decision handlers require bounded notes/reasons, reject repeated resolution, and write the relevant domain
history plus governance audit event. One reporter may have only one open report per target, but a later
report creates a new row instead of erasing a prior terminal decision.

## Captcha and abuse controls delivered in this PR

- Web uses the official YourTJCaptcha challenge API for campus email-code requests, review publication,
  and review reporting. `VITE_CAPTCHA_URL` may select the deployment; the default is the existing YourTJ
  service.
- The browser sends the resulting opaque pass token to the platform API. It does not send the campus
  email or review body to the captcha service.
- The backend verifies the pass token with the configured provider and atomically consumes its hash in
  Redis under an operation-specific purpose. The runtime default is the official YourTJCaptcha
  `/api/siteverify` endpoint and `CAPTCHA_SITEVERIFY_URL` may override it. Tokens are single-use per purpose
  and oversized/empty tokens are rejected.
- Protected operations fail closed when captcha verification or replay protection is unavailable.
- Review publication stores account-scoped `Idempotency-Key`, request hash, review id, and the original
  response atomically. A matching retry replays before captcha consumption; reuse for different review
  content returns conflict.
- The external captcha service necessarily receives ordinary network metadata and challenge-image
  requests. Its deployment and retention policy must be covered by the platform privacy notice.
- **FOLLOW-UP:** forum thread/comment captcha enforcement is still represented in the historical target
  architecture but is not delivered by this PR; current forum writes use authentication, sanctions,
  trust-level rate limits, and watched-word controls.

## Retention and erasure are follow-up

No automated retention worker, account recovery window, message purge, content redaction worker, backup
expiry enforcement, or legal-hold workflow is delivered in this PR. The following remain policy targets
subject to privacy-owner review, not claims about current automation:

| Data | Follow-up target |
|---|---|
| Expired verification/reset material | remove promptly after expiry |
| Revoked sessions and device metadata | bounded security retention |
| Soft-deleted public content | recovery window followed by redaction/purge where lawful |
| Unreported DM content | participant-lifecycle policy and delayed purge after both delete |
| Reported DM evidence | bounded evidence retention with access logging |
| Fine-grained activity events | bounded projection/reconciliation retention |
| Review publication idempotency records | bounded retry window, then purge; account/review deletion cascades |
| Sanctions and staff audit | policy-defined security/governance retention |
| Credit ledger | permanent append-only verification with a non-identifying account tombstone |

Workers must be idempotent, observable, and audited when implemented.

## Follow-up transparency and consistency

- User notification for sanctions and content actions, with duration/category and appeal guidance.
- Appeal cases with evidence, status, reviewer, decision, and conflict-of-interest controls.
- Recent-authentication challenges for high-impact staff actions.
- Deactivate/delete/recovery/anonymization/purge lifecycle and self-service export.
- Retention workers and legal holds.
- Transactional/outbox synchronization for search-index delivery and equivalent review/media repair.

## Delivered verification baseline

- Delivered staff writes are mapped to named capabilities, require reasons, enforce target hierarchy, and
  append audit records.
- Forum hide/delete and comment/thread restoration preserve activity-count visibility invariants.
- Automatic forum hide, reject/ignore, and uphold produce the documented contribution state.
- Suspension revokes sessions and blocks authentication; sanction revocation preserves history.
- Public/profile/admin directory responses do not expose campus email or arbitrary DM content.
- Delivered report queues use only their documented terminal states and reject repeated decisions.
- Forum/review moderation rejects equal/higher-role authors, and media moderation rejects self-review.

Account deletion lifecycle, subject notifications, appeals, retention, and transactional search-index
delivery are explicitly excluded from this delivered baseline.

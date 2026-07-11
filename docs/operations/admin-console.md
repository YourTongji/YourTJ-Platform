# Admin console

> **Status:** Current normative information architecture; expanded console is proposed in this PR
>
> **Owner:** Community operations + Web/API maintainers
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
>
> **Authoritative sources:** `contract/openapi.yaml`, [RBAC and audit](../security/rbac-and-audit.md), `web/src/pages/admin-page.tsx`, admin handlers

The admin console is a capability-driven operations surface, not a collection of raw database editors.
It must make safe common actions fast while keeping sensitive and irreversible operations deliberate.

## Implementation baseline

- **CURRENT:** `/admin` exposes review visibility, review reports, string settings, selection sync, and
  review/forum reindex controls to both mod and admin roles.
- **CURRENT GAP:** backend capabilities for forum, courses, media, badges, sanctions, and logs are not
  represented in the Web console; existing review actions lack a complete decision workflow.
- **PR TARGET:** navigation and workflows below, inline staff actions on profiles/content, correct role
  separation, activity policy administration, and central audit visibility.
- **FOLLOW-UP:** analytics dashboards, bulk moderation, appeal case management, and dual approval for
  exceptional security operations.

## Information architecture

| Area | Primary workflows | Minimum role |
|---|---|---|
| Overview | open queue counts, active sanctions, failed jobs, service health | mod |
| Users | search, inspect public context, invite, silence/suspend, sessions, role history | mixed |
| Moderation | Forum / Reviews / Media / DM report tabs with shared decisions | mod |
| Content | boards, tags, watched words, badges, announcements | delegated mod/admin |
| Courses | catalogue CRUD, import provenance, selection sync | admin |
| Activity | current formula, version history, weight update preview | admin |
| Platform | typed settings and feature switches | admin |
| Jobs | sync/reindex/retention runs, progress, logs, retry | admin |
| Audit | staff actions and sensitive reads, filter/export | scoped mod/admin |
| Credit integrity | ledger verify and reconciliation status, read-only | admin |

The navigation displays only areas granted by server-issued capabilities. A single generic “mod/admin”
gate is insufficient.

## User management

- Search by handle or exact account id; email search is admin-only and returns a masked result.
- User detail shows public contributions, active sanctions, report history, role history, sessions summary,
  and audit timeline without exposing unrelated DM content.
- `邀请用户` creates an expiring campus-email invitation. It does not create a verified account or set a
  password. The form requires reason and defaults to user role.
- Moderator actions: bounded silence on ordinary users.
- Admin actions: suspend/unsuspend, deactivate/reactivate, deletion workflow, revoke sessions, assign role.
- Staff targets, self-actions, final-admin changes, indefinite sanctions, and deletion receive explicit
  hierarchy validation and confirmation.

## Moderation workspace

All queues share filters, target preview, reporter context, author history, policy category, decision, and
reason. Domain-specific evidence stays in its owning tab.

- Forum: flags, automatically hidden content, thread/comment restore.
- Reviews: reports, pending/hidden reviews, hide/restore; no physical delete button.
- Media: pending/quarantined uploads, approve/block, object-cleanup status.
- DM reports: only the reported message and minimum surrounding context; every evidence view is audited.

Terminal decisions are `upheld`, `rejected`, or `ignored`. The UI previews the resulting content action
and sanction before commit, then offers restore/revoke where policy permits. It never silently edits user
speech; exceptional redaction is admin-only and revision-preserving.

## Inline staff controls

Authenticated staff see a compact action menu on profiles, threads, comments, and reviews when they hold
the corresponding capability:

- profile: open admin record, silence/suspend where permitted, view sanctions, start normal DM;
- thread/comment: hide/remove/restore, pin/close/move where permitted, open reports;
- review: hide/restore, open report history;
- media: open moderation record.

Menus show state-aware actions only and deep-link into the console for reason, evidence, and confirmation.
The public component must not contain privileged data in hidden markup.

## Activity policy editor

The page displays the current version and formula, three bounded integer inputs, a sample-day score
preview, effective time, required reason, and version history. Save uses optimistic concurrency and makes
clear that current weights reinterpret historical daily counts. Only admins receive the mutation
capability.

## Settings and jobs

- Platform settings use typed, described forms with validation; arbitrary key/value editing is not the
  normal UI.
- Jobs return a persisted job id and expose queued/running/succeeded/failed state, timestamps, actor,
  summary, and safe retry. A 202 response alone is not completion.
- Reindex/sync buttons require confirmation, prevent duplicate active runs, and link to audit and logs.
- Credit tools are verification/reconciliation only. No balance editor is permitted.

## Interaction and accessibility rules

- Every list has loading, empty, error, pagination, and stale-data states.
- Mutations disable duplicate submission, report exact success/failure, and refresh only affected queries.
- Risky actions require a reason; destructive actions additionally require explicit confirmation.
- Tables work at keyboard and screen-reader level; status is never color-only.
- Desktop may use dense tables, but moderation and urgent user actions remain usable on mobile.
- Chinese terminology is consistent: `禁言` = silence, `封禁` = suspend, `移除` = reversible removal,
  `清除` = retention purge.

## Delivery sequence

1. Capability and audit infrastructure.
2. Users and unified moderation queues.
3. Activity policy and inline staff controls.
4. Content/course/platform management.
5. Persistent jobs and full audit explorer.

Each area remains labelled partial until its backend guard, contract, UI, success path, rejection path,
audit assertion, and responsive/accessibility checks pass.

## Acceptance criteria

- User, mod, and admin see the correct navigation and direct unauthorized API calls are denied.
- An admin can invite a user without bypassing mailbox proof; a mod cannot invite or assign roles.
- Forum, review, media, and DM report decisions use the shared decision model and create atomic audit
  events.
- Staff actions on profiles/content are hierarchy-aware, require reasons, and expose restore/revoke where
  applicable.
- Activity weight edits are versioned, admin-only, previewed, and audited.
- Jobs expose durable status rather than only a toast; retries do not create concurrent duplicate jobs.
- No console control can directly mutate a credit balance or append arbitrary ledger entries.

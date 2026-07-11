# Admin console

> **Status:** DELIVERED IN THIS PR — remaining operations work is labelled follow-up
>
> **Owner:** Community operations + Web/API maintainers
>
> **Last verified:** 2026-07-11 against this PR, OpenAPI, admin handlers, and current Web source
>
> **Authoritative sources:** `contract/openapi.yaml`, [RBAC and audit](../security/rbac-and-audit.md), `web/src/pages/admin-page.tsx`, `web/src/components/admin`, admin handlers

The admin console is a capability-driven operations surface, not a raw database editor. Backend
capability and target-hierarchy checks remain authoritative; hiding a control in Web is not authorization.

## Implementation baseline

- **DELIVERED IN THIS PR:** capability-filtered navigation and actions for overview, user invitation and
  search, role changes, sanctions, sanction history/revocation, session revocation, moderation, resources,
  activity policy, announcements, audit, string settings, and operational task triggers.
- **DELIVERED IN THIS PR:** five moderation workspaces cover forum reports, review reports, review
  visibility/status, reported-DM evidence, and direct staff recovery lookup for hidden/deleted forum
  content. Media review is delivered under content resources.
- **DELIVERED IN THIS PR:** course catalogue management, media approve/block, and community-structure
  management for boards, tags, and watched words.
- **DELIVERED IN THIS PR:** inline capability-aware links/actions on profiles and forum content, with
  state-aware actions and mandatory reason dialogs.
- **FOLLOW-UP:** durable job state and retry, typed settings, badge management UI, read-only credit
  integrity UI, appeals, and bulk moderation. These are not delivered by this PR.

## Delivered information architecture

| Area | Delivered workflows | Capability boundary |
|---|---|---|
| Overview | user/status totals, report queues, pending media, today's activity | `users.search` |
| Users | handle/id search, filters, pagination, invitation, role, silence/suspend, sanction history/revoke, session revoke | `users.*` capabilities |
| Moderation | forum reports, review reports, review status, reported-DM evidence | `moderation.content` |
| Content resources | media approve/block, course CRUD, boards/tags/watched words | `moderation.content`, `courses.manage`, `community.manage` |
| Activity | current weights, sample preview, history, publish new version | `activity.policy` |
| Announcements | list, create, edit, delete with reason | `announcements.manage` |
| Audit | cursor list and actor/action/target-type filters | `audit.read` |
| Platform | current generic string setting editor | `platform.settings` |
| Operations | trigger selection sync and review/forum reindex | `operations.jobs` |

Navigation is built from server-issued capabilities. Direct requests are checked again by each backend
handler.

## User management delivered in this PR

- Search is privacy-safe and accepts handle text or exact account id, with role/status filters and cursor
  pagination. It does not expose campus email.
- `邀请用户` creates an expiring account invitation for a campus email. The console does not set a
  password or bypass mailbox ownership verification, and the form requires an audit reason.
- Role changes revoke active sessions, enforce target hierarchy, and only assign `user` or `mod`.
  Administrator provisioning remains out of band until a reversible super-admin/recovery policy exists.
- Moderators can issue silence of at most 30 days to lower-role users; admins can suspend lower-role users
  or issue longer sanctions. The UI lists sanction history and supports reasoned revocation.
- Authorized admins can revoke all refresh sessions for a lower-role target.
- Self-action and equal/higher-role actions are rejected by the backend.
- **FOLLOW-UP:** complete account `deactivated`/`deleted` lifecycle workflows, recovery, and purge are not
  present in this console.
- **FOLLOW-UP:** recent-authentication challenges for high-impact role/session/suspension operations are
  not yet implemented.

## Moderation workspace delivered in this PR

The console provides five distinct queues/views so each domain keeps its own evidence boundary:

1. Forum reports support `uphold`, `reject`, and `ignore`, with temporary automatic hiding and reasoned
   terminal decisions.
2. Review reports support `uphold`, `reject`, and `ignore`.
3. Review status supports reasoned visibility changes and soft removal.
4. DM reports expose the reported message excerpt and report metadata only; they support `uphold` or
   `reject` and do not provide a general inbox browser.
5. Content recovery accepts an exact thread/comment ID, reads the retained staff-only evidence, and exposes
   the same reasoned restore/unhide controls used inline. It is not a broad private-content search surface.

Every paginated queue exposes previous/next navigation. Forum and review report cards include bounded
target evidence (author, context/status, and excerpt) so staff do not have to decide from an opaque ID.

Pending media is reviewed separately under Content resources with approve/block actions; staff cannot
review their own upload. All delivered mutations require an explicit reason where the backend contract
requires one and invalidate affected queries after success.

Course deletion is rejected when any visible, hidden, or pending review history exists. The database
foreign key is restrictive as a final invariant, so a stale visible-review aggregate cannot cascade-delete
retained community speech.

## Inline staff controls delivered in this PR

- Profiles expose a capability-aware deep link into the matching user-management search rather than
  embedding private account data or sanction forms in public markup.
- Thread/comment detail exposes state-aware pin/unpin, close/reopen, archive/unarchive, hide/unhide,
  soft-delete/restore, and move actions as supported by the target type.
- Every inline content mutation opens the shared reason dialog before calling the generic admin endpoint.
- User sanctions remain in the user-management workflow so hierarchy, history, duration, and reason can
  be reviewed together.
- Staff do not silently rewrite user-authored forum text. Authors retain the ordinary revision path; staff
  use visible state changes and retained audit history. A future legal/safety redaction flow must preserve
  the original as restricted evidence instead of impersonating the author.
- The same rule applies to course reviews: staff may decide reports, hide, restore, or soft-remove a review,
  but cannot overwrite its author rating or text through an admin endpoint.

## Activity policy editor delivered in this PR

The page shows the current version and formula, three bounded integer weights, a sample-day score preview,
the required reason, and cursor-backed version history. Save uses `expectedVersion` optimistic concurrency
and makes clear that current weights reinterpret historical daily counts. There is no effective-time
field: a successfully appended policy becomes current immediately.

## Settings and operations: delivered boundary

- **DELIVERED IN THIS PR:** settings can be listed and updated as generic string key/value pairs with a
  mandatory reason and governance audit event.
- **FOLLOW-UP — typed settings:** field types, descriptions, domain validation, and setting-version
  concurrency are not implemented. The UI explicitly identifies the current string editor limitation.
- **DELIVERED IN THIS PR:** selection sync and review/forum reindex can be confirmed and triggered with a
  reason. The response only confirms submission.
- **FOLLOW-UP — durable jobs:** there is no persisted job id, queued/running/succeeded/failed model,
  progress, failure log, duplicate-run lock, or safe retry UI. A success toast does not prove completion.
- **FOLLOW-UP — badges UI:** badge backend operations are not surfaced in the console.
- **FOLLOW-UP — credit integrity:** no ledger verification/reconciliation panel is delivered. No balance
  editor or arbitrary ledger append is permitted now or in follow-up work.

## Interaction and accessibility rules

- Delivered lists include loading, empty, error, and pagination states where their API is paginated.
- Mutations disable duplicate submission, show exact success/failure feedback, and invalidate affected
  queries.
- Risky actions require a reason; destructive actions use explicit confirmation styling and copy.
- Status is communicated with text as well as color; navigation and form controls are keyboard accessible.
- Chinese terminology is consistent: `禁言` = silence, `封禁` = suspend, `移除` = reversible removal,
  `清除` = retention purge.

## Follow-up backlog

1. Persisted operation jobs with progress, logs, deduplication, and retry.
2. Typed/versioned settings instead of generic string editing.
3. Badge creation, award/revoke, and history UI.
4. Read-only credit verification and reconciliation status.
5. User-facing appeals and reviewer workflow.
6. Bounded bulk moderation with preview, per-item results, and audit correlation.

## Delivered verification baseline

- User, mod, and admin navigation is capability-driven, and backend routes reject missing capabilities.
- Invitation preserves mailbox proof; moderators cannot invite or assign roles.
- Delivered moderation decisions and mutations require reasons and write governance/domain audit records.
- User actions enforce role hierarchy and expose sanction/session history without revealing unrelated PII
  or private messages.
- Activity policy edits are versioned, admin-capability-only, previewed, and audited.
- The console cannot directly mutate a credit balance or append arbitrary ledger entries.

The follow-up backlog above is intentionally excluded from this delivered verification baseline.

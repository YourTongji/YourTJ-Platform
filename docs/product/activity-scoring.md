# Daily activity scoring

> **Status:** DELIVERED IN THIS PR — follow-up work is labelled below
>
> **Owner:** Activity/Forum/Reviews maintainers + Community operations
>
> **Last verified:** 2026-07-11 against this PR, migrations `0020_activity.sql` and
> `0027_activity_backfill.sql`, OpenAPI, and current source
>
> **Authoritative sources:** `contract/openapi.yaml`, `backend/migrations/0020_activity.sql`, `backend/crates/activity`, forum/reviews write paths

The home-page square grid represents the authenticated user's daily community activity, similar to a
contribution heatmap. It is not a trust-level progress bar and does not grant or transfer credit.

## Implementation baseline

- **DELIVERED IN THIS PR:** an `activity` domain with idempotent activation/reversal events, projected
  daily counts, a versioned scoring policy, authenticated calendar and policy APIs, write-path integration,
  and the real home-page heatmap.
- **DELIVERED IN THIS PR:** forum threads, comments, positive forum votes, and review likes contribute;
  removal, moderation visibility changes, vote changes, and unlike operations reverse the original day.
- **DELIVERED IN THIS PR:** deployment backfills existing visible threads, comments, current positive
  forum votes, and likes on visible reviews. The migration inserts only missing event keys, so reruns do
  not double-count history.
- **FOLLOW-UP:** public-profile heatmaps require a per-user visibility control. Additional contribution
  kinds require an explicit contract, migration, and policy revision.
- **FOLLOW-UP:** an observable reconciliation worker may verify and repair the projection from events.
  No such scheduled worker is delivered in this PR.

## Definitions

The initial policy is:

```text
score = threads * 10 + comments * 3 + likes * 1
```

These are seeded policy values, not constants in application code. `likes` means positive reactions the
user gives: an up-vote on a forum thread/comment or a like on a course review. It does not mean likes
received. Down-votes, views, edits, login events, DMs, and moderation actions do not count.

Each source relationship has at most one active positive event:

| Source transition | Projected result |
|---|---:|
| visible thread created | `thread +1` |
| visible comment created | `comment +1` |
| positive forum vote/review like created | `like +1` |
| up-vote changed to down-vote, or review like removed | matching `-1` on the original activity date |
| report threshold automatically hides content | matching `-1` immediately on the original activity date |
| automatically hidden report is rejected or ignored | matching `+1` on the original date when the target is otherwise visible |
| automatically hidden report is upheld | contribution remains reversed while the target is soft-deleted |
| staff hides or soft-deletes counted content | matching `-1` on the original date |
| staff restores/unhides content so it is visible again | matching `+1` on the original date |

Positive reactions to content that becomes unavailable are reversed as well, so users cannot retain
activity credit from content removed for abuse. Restoration reactivates still-current positive reactions
using their original reaction timestamp.

Activation and reversal are idempotent. Repeated requests and visibility toggles cannot manufacture
activity, and self-votes/self-likes are rejected at the source write path.

## Day boundary and range

- The canonical community day uses `Asia/Shanghai` regardless of browser timezone.
- Source timestamps remain UTC; the projection stores the corresponding Shanghai `activity_date`.
- The default response covers 365 continuous days ending today; a request may cover at most 371 days.
- Empty dates are returned explicitly with zero counts so clients never infer timezone gaps.

## Data ownership and model

Activity is a cross-domain read projection and owns the `activity` schema/crate. Forum and reviews call
its public transaction-aware API; they do not reach into activity tables with foreign SQL.

Migration `0027_activity_backfill.sql` is a deployment-only coordination exception: it reads the existing
forum/reviews source tables once to seed the new projection before application traffic starts. Runtime
calendar reads and writes continue to respect crate APIs and never repeat that cross-domain aggregation.

### `activity.events`

The append-only event table stores:

- `event_key`: globally unique transition key;
- `source_key`: stable source relationship, for example one content item or one user's vote relationship;
- `generation`: increments when a previously reversed source becomes active again;
- `account_id` and `kind`, where `kind` is `thread`, `comment`, or `like`;
- `delta`, constrained to `1` or `-1`;
- `activity_date`: the original contribution's Shanghai calendar date;
- `occurred_at` and `created_at`;
- `reverses_event_id`: required for a `-1` event and unique, linking it to the positive event it reverses.

`(source_key, generation, delta)` is unique. A positive event has no `reverses_event_id`; a negative event
must reference exactly one prior positive event. No content body, email, handle, IP, or DM material is
stored here.

### `activity.daily_counts`

One row per `(account_id, activity_date)` stores non-negative `threads_created`, `comments_created`, and
`likes_given`, plus `updated_at`. The projection updates this row in the same transaction as the source
mutation and activity event. Calendar reads use this table and never aggregate forum/review source tables.

### `activity.score_policies`

Policies are append-only rows containing `version`, `thread_weight`, `comment_weight`, `like_weight`,
mandatory `reason`, nullable `changed_by`, and `created_at`. There is no `effective_at` column. Each weight
is validated in `0..1000`; `expectedVersion` is an optimistic-concurrency input, not a stored policy field.

The newest version is active immediately and reinterprets displayed history. Raw daily counts are not
rewritten. Policy publication and its governance audit event commit in the same transaction.

## Consistency

- `source_key` is advisory-locked before transitions; generation and reversal constraints provide the
  idempotency boundary.
- A source write, activity event, and daily-count update share one database transaction.
- Daily counters cannot become negative; an unmatched deactivation is an idempotent no-op.
- Policy updates take an advisory lock, compare `expectedVersion`, append the new version, and return a
  conflict when the observed version is stale.
- **FOLLOW-UP:** scheduled reconciliation, drift metrics, quarantine, and repair operations are not yet
  implemented.

## HTTP contract delivered in this PR

### User heatmap

`GET /api/v2/me/activity?from=YYYY-MM-DD&to=YYYY-MM-DD`

```json
{
  "timezone": "Asia/Shanghai",
  "from": "2025-07-12",
  "to": "2026-07-11",
  "policyVersion": 1,
  "weights": {
    "thread": 10,
    "comment": 3,
    "like": 1
  },
  "days": [
    {
      "date": "2026-07-11",
      "threads": 1,
      "comments": 2,
      "likes": 4,
      "score": 20
    }
  ]
}
```

The endpoint is authenticated. Public `GET /users/{handle}/activity` is **FOLLOW-UP** and remains absent
until profile visibility controls are specified and implemented.

### Policy administration

- `GET /api/v2/admin/activity-policy`
- `PUT /api/v2/admin/activity-policy` with `expectedVersion`, `weights`, and `reason`
- `GET /api/v2/admin/activity-policy/history`

All three require the `activity.policy` capability, which is currently issued to admins. Policy responses
contain `version`, `timezone`, `weights`, `reason`, `changedBy`, and `createdAt`; no effective time is
accepted or returned.

## Heatmap UI delivered in this PR

- The card is titled `活跃度`, never `等级成长`.
- Tooltips show the date, score, and exact thread/comment/like counts.
- Intensity has five levels including zero and is derived from returned scores.
- Color is not the only signal: cells expose accessible labels and keyboard focus.
- Logged-out users see an explanatory state instead of synthetic data.
- The same heatmap card is available below 1240 px instead of disappearing with the desktop sidebar.
- Trust level and activity remain separate concepts.

## Delivered verification baseline

- Contributions use the correct Shanghai date and continuous calendar response.
- Duplicate writes, unlike/vote changes, manual moderation, automatic hiding, report decisions, and
  restoration preserve idempotent counts.
- Down-votes and received likes do not increase activity; self-votes and self-likes are rejected.
- Policy edits are versioned, capability-checked, reasoned, immediately active, and atomically audited.
- Ranges longer than 371 days are rejected and reads do not aggregate source tables.
- Existing contributions are backfilled idempotently before the new application version accepts writes.
- The home grid consumes the activity API and contains no trust-level-derived placeholder cells.

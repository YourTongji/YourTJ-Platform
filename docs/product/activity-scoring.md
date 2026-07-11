# Daily activity scoring

> **Status:** Current normative specification for the activity feature proposed in this PR
>
> **Owner:** Activity/Forum/Reviews maintainers + Community operations
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
>
> **Authoritative sources:** `contract/openapi.yaml` once updated, the new activity migration, forum/reviews write paths

The home-page square grid represents a user's daily community activity, similar to a contribution
heatmap. It is not a trust-level progress bar and does not grant or transfer credit.

## Implementation state

- **CURRENT:** the home sidebar renders synthetic cells derived from trust level. No daily activity table,
  policy, or API exists.
- **PR TARGET:** persist idempotent activity events and daily counts, expose the authenticated user's
  heatmap, let admins version the three weights, and replace the synthetic grid.
- **FOLLOW-UP:** optional public-profile heatmaps after a per-user privacy control is available, plus
  additional contribution kinds only through a versioned policy change.

## Definitions

The default formula is:

```text
score = threadCount * 10 + commentCount * 3 + likeCount * 1
```

The numbers are initial policy values, not constants in application code. `likeCount` means positive
reactions the user gives: an up-vote on a forum thread/comment or a like on a course review. It does not
mean likes received. Down-votes, views, edits, login events, DMs, and moderation actions do not count.

Each source action contributes at most once:

| Source transition | Event delta |
|---|---:|
| visible thread created | `thread +1` |
| visible comment created | `comment +1` |
| positive forum vote/review like created | `like +1` |
| up-vote changed to down-vote or like removed | `like -1` on original activity date |
| counted content removed or an upheld report makes it invalid | matching `-1` on original date |
| content restored | matching `+1` on original date |

Pending automatic hiding does not change the score until a moderation decision. Restore and reversal
events must be idempotent, so toggling cannot manufacture activity.

## Day boundary and range

- The canonical community day uses `Asia/Shanghai` regardless of browser timezone.
- Source timestamps remain UTC; the projection converts `occurredAt` to a Shanghai calendar date.
- The default response covers the trailing 53 weeks ending today; a request may cover at most 371 days.
- Empty dates are returned explicitly with zero counts so clients never infer timezone gaps.

## Data ownership and model

Activity is a cross-domain read projection and owns a new `activity` schema/crate. Forum and reviews call
its public transaction-aware API; they must not write its tables with foreign SQL.

### `activity.events`

Minimal append-only source for deduplication and reconciliation:

- `id`
- `event_key` — globally unique business id, such as `forum:thread:42:created`
- `account_id`
- `event_type` — `thread`, `comment`, or `like`
- `source_type` and `source_id`
- `delta` — `1` or `-1`
- `occurred_at` — time of the original contribution; reversals retain the original activity date
- `created_at` — time this event was recorded

No content body, email, handle, IP, or DM material is stored here.

### `activity.daily_counts`

One row per `(account_id, activity_date)` with non-negative `thread_count`, `comment_count`, and
`like_count`, plus `updated_at`. It is updated in the same transaction as the idempotent event whenever
possible. A reconciliation job can rebuild it from events; public reads never scan forum/reviews tables.

### `activity.score_policies`

Append-only versions containing `version`, the three non-negative integer weights, `effective_at`,
`created_by`, mandatory `reason`, and `created_at`. At least one weight must be positive. Recommended
validation is `0..1000` per weight.

Changing the active policy intentionally reinterprets the displayed history using current weights. Raw
daily counts never change because of a policy edit. The response returns the policy version so UI and
audit records remain explainable.

## Consistency

- `event_key` is the idempotency boundary; replay is a no-op.
- A source write and its activity event use one database transaction. If a domain cannot participate in
  the same transaction, it writes a transactional outbox record and retries projection.
- Daily counters may not become negative. An unmatched reversal is logged and quarantined for
  reconciliation rather than clamped silently.
- A scheduled reconciliation compares event-derived totals with daily rows and emits a metric; it does
  not mutate source content.
- Activity-policy updates are optimistic-concurrency controlled by version and atomically audited.

## HTTP contract proposed in this PR

### User heatmap

`GET /api/v2/me/activity?from=YYYY-MM-DD&to=YYYY-MM-DD`

```json
{
  "timezone": "Asia/Shanghai",
  "from": "2025-07-06",
  "to": "2026-07-11",
  "policy": {
    "version": 1,
    "threadWeight": 10,
    "commentWeight": 3,
    "likeWeight": 1
  },
  "days": [
    {
      "date": "2026-07-11",
      "threadCount": 1,
      "commentCount": 2,
      "likeCount": 4,
      "score": 20
    }
  ]
}
```

The endpoint is authenticated. Public `GET /users/{handle}/activity` is **FOLLOW-UP** and must remain
absent until profile visibility controls are specified and implemented.

### Policy administration

- `GET /api/v2/admin/activity-policy`
- `PUT /api/v2/admin/activity-policy` with three weights, expected version, effective time, and reason
- `GET /api/v2/admin/activity-policy/history`

Reads require staff access; mutation is admin-only. A moderator cannot change scoring policy. The API
returns the created policy version and a conflict when the expected version is stale.

## Heatmap UI

- Title: `活跃度`; never `等级成长`.
- Tooltip shows date, score, and the three raw counts.
- Intensity has five levels including zero. Thresholds are derived from the returned non-zero score
  distribution, while tooltips always show exact values.
- Color is not the only signal: cells have accessible labels and keyboard focus.
- Logged-out state explains that activity is personal and links to login; it must not render fake data.
- Trust level and activity are separate cards/labels if both are displayed.

## Acceptance criteria

- A thread, comment, forum up-vote, and review like appear on the correct Shanghai date.
- Duplicate requests do not change counts; unlike, vote changes, removal, uphold, and restore produce the
  documented reversible result.
- Down-votes and received likes never increase activity.
- Policy changes require admin role, reason, valid version, and an audit event; they immediately update
  calculated scores without rewriting raw counts.
- The API returns a continuous date series, rejects ranges over 371 days, and performs no source-table
  aggregation on the read path.
- The home grid uses API data, displays exact accessible tooltips, and contains no trust-level-derived
  placeholder cells.

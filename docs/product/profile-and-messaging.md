# Profiles and direct messaging

> **Status:** Current normative specification; implementation state is labelled per section
>
> **Owner:** Identity/Forum/Web maintainers + Privacy owner
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
>
> **Authoritative sources:** `contract/openapi.yaml`, migrations `0001`, `0005`, `0006`, identity/forum source, `web/src/pages/profile-page.tsx`, `web/src/pages/messages-page.tsx`

This document defines public profile boundaries and the minimum safe 1:1 direct-message product. Campus
email is an authentication attribute, not a community profile field.

## Implementation baseline

- **CURRENT:** public profile, public thread/comment endpoints, 1:1 DM conversations/messages, ignore
  relations, and a basic Web profile/messages page exist.
- **CURRENT GAP:** profile list responses and DM conversation responses do not match their OpenAPI page
  schemas; the current UI therefore cannot be considered complete. DM has no read state, report flow,
  archive/delete state, or post-creation block enforcement.
- **PR TARGET:** align responses and clients, complete profile states, add capability-driven staff actions,
  and provide a safe, usable conversation experience with unread/read, block, and report semantics.
- **FOLLOW-UP:** attachments, message search, public activity heatmaps, and user data export.

## Profile views

| Field | Public profile | Account owner | Authorized staff |
|---|---:|---:|---:|
| handle, avatar | yes | yes | yes |
| join date, trust level, badges, public counts | yes | yes | yes |
| public threads/comments | yes | yes | yes |
| daily activity heatmap | no in this PR | yes | only for a documented moderation need |
| role | display only `mod/admin` staff badge | yes | yes |
| account id | no | yes | yes |
| campus email | never | masked | masked by default; reveal is separate audited capability |
| sanctions/reports | no | own active sanctions and notices | role-scoped |
| sessions/wallet keys | no | own only | security-admin workflow only |

Deleted, removed, or hidden content is absent from public activity lists. A removed account renders a
stable `已注销用户` tombstone and never leaks the former handle through profile URLs or API payloads.

### Profile API behavior

- `GET /users/{handle}` returns one documented schema containing handle, avatar, staff badge where
  applicable, trust level, badges, counts, and join timestamp. It never returns email.
- `GET /users/{handle}/threads` and `/comments` use the platform cursor `Page<T>` envelope and a bounded
  limit; Web renders loading, empty, error, and pagination states.
- `PATCH /me` edits only owner-controlled public fields. Role, trust, sanctions, and activity counts are
  never accepted from this endpoint.
- Staff controls on a profile are shown from capabilities, not by checking role strings alone. Backend
  authorization remains decisive.

## Starting a conversation

- The primary entry is a `私信` action on a user profile or handle search. Requiring users to know a
  numeric account id is not an acceptable final UI.
- DMs are 1:1. The database enforces one canonical conversation per unordered account pair so concurrent
  creation cannot produce duplicates.
- A user cannot message themselves, deleted/suspended recipients, or an account that has blocked them.
- TL0 cannot initiate a conversation but may reply to an existing staff/system conversation when policy
  permits.
- Creating or sending is rate-limited. Active silence blocks DM writes.

## Conversation and message lifecycle

Each participant has independent `last_read_message_id`, `archived_at`, and `deleted_at` state. Listing
conversations returns the other user's public identity, last-message excerpt/time, and exact unread count.

- Sending creates an immutable message. Ordinary messages are not edited in place.
- `POST /forum/dm/conversations/{id}/read` only advances the participant's read pointer.
- Archive hides a conversation from the default inbox but preserves it for the other participant.
- Delete removes it from that participant's view after a recovery window; it does not erase the other
  participant's copy.
- When both participants delete, unreported message bodies are eligible for purge after 30 days.
- Blocking applies immediately to existing conversations: the blocked party can no longer send, while
  both parties retain access to prior messages according to retention policy.

Messages have a documented non-empty length bound, plain-text/Markdown rendering rules, and the same
malicious-link and media safety boundary as public content. Notification payloads contain only a short
escaped excerpt, never the full message.

## Privacy and moderation

- Staff cannot browse DM inboxes. Normal admin endpoints expose conversation metadata only.
- A participant may report a specific message with category and note. The report captures the target
  message and the minimum surrounding context needed for review.
- Only a moderator handling that report may access its evidence; every evidence view and action is
  audited. Unrelated messages remain hidden.
- Report decisions use `upheld/rejected/ignored`. Upheld abuse may remove the message from both views and
  apply a sanction under [Community governance](community-governance.md).
- Transport and database encryption, backup access, and staff evidence access are operational security
  controls; the product does not claim end-to-end encryption.

## Retention

- Active, unreported messages remain available until participant deletion rules apply.
- Both-participant deletion starts a 30-day recovery period, then removes message bodies.
- Reported excerpts and required surrounding context are retained at most 180 days after final decision,
  unless a documented legal hold applies.
- Conversation metadata is minimized and removed/anonymized with the account lifecycle.
- DM content is excluded from search, activity scoring, public profile counts, digests, and model-training
  datasets.

## Web experience proposed in this PR

- Two-pane desktop inbox and single-pane mobile flow with stable conversation URLs.
- Handle/avatar, last excerpt/time, unread badge, loading/error/empty/pagination states.
- Profile `私信` button prefills the recipient; handle search replaces account-id input.
- Messages are visually separated by sender, ordered consistently, preserve line breaks, and scroll to
  unread/latest without losing pagination position.
- Send supports Enter/Shift+Enter intentionally, disables while pending, prevents duplicate submission,
  and announces errors accessibly.
- Conversation menu exposes archive, block/unblock, and report where supported.

## Acceptance criteria

- Profile DTOs, OpenAPI-generated types, Rust responses, and Web consumption agree exactly.
- Public profile lists paginate and exclude hidden/removed content and private fields.
- Concurrent conversation creation yields one conversation; non-participants receive 403 without data.
- Read pointers only advance and unread counts are correct across two sessions.
- Blocking an existing sender prevents the next message; silence and recipient lifecycle are enforced.
- Staff cannot query arbitrary DM bodies; report evidence access and decisions are capability-checked and
  audited.
- Deletion and retention behavior is tested for one participant, both participants, reported messages,
  and deleted accounts.

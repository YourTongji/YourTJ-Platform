# Profiles and direct messaging

> **Status:** DELIVERED IN THIS PR — profile and core 1:1 messaging flows; lifecycle extensions are follow-up
>
> **Owner:** Identity/Forum/Web maintainers + Privacy owner
>
> **Last verified:** 2026-07-11 against this PR, migration `0021_dm_moderation.sql`, OpenAPI, and current source
>
> **Authoritative sources:** `contract/openapi.yaml`, `backend/migrations/0021_dm_moderation.sql`, identity/forum source, `web/src/pages/profile-page.tsx`, `web/src/pages/messages-page.tsx`

This document defines public profile boundaries and the delivered minimum safe 1:1 direct-message
product. Campus email is an authentication attribute, not a community profile field.

## Implementation baseline

- **DELIVERED IN THIS PR:** one aligned public profile DTO, paginated public thread/comment lists, profile
  loading/empty/error/pagination states, profile-to-DM entry, and capability-aware user-governance links.
- **DELIVERED IN THIS PR:** conversation creation by public handle, one database-enforced canonical
  conversation per unordered pair, paginated inbox/messages, monotonic read pointers, exact unread counts,
  bidirectional block enforcement, message reports, and a scoped admin evidence queue.
- **DELIVERED IN THIS PR:** the responsive Web inbox supports new conversations, older-page loading,
  sending, read marking, block/unblock, and reporting a specific message.
- **FOLLOW-UP:** participant archive/delete actions and their recovery/retention worker are not delivered,
  even though migration `0021` reserves participant columns for those states.
- **FOLLOW-UP:** attachments, message search, public-profile heatmaps/privacy controls, and user data export.

## Profile views delivered in this PR

| Field | Public profile API/UI | Account owner | Staff console |
|---|---:|---:|---:|
| handle, avatar | yes | yes | yes |
| opaque account id | returned for relationship actions | yes | yes |
| join date, trust level, badges, public counts | yes | yes | yes |
| public threads/comments | yes, paginated | yes | yes |
| role | displayed as community/staff badge | yes | yes |
| daily activity heatmap | no | available only from authenticated `/me/activity` | no profile heatmap |
| campus email | never | not exposed by profile APIs | not exposed by delivered directory UI |
| sanctions and session controls | no | no self-service view in this PR | capability-scoped |

Hidden and soft-deleted forum content is excluded from public profile activity lists. Complete account
deactivation/deletion tombstones and former-handle erasure are **FOLLOW-UP** with the account lifecycle.

### Profile API behavior

- `GET /users/{handle}` returns the documented `UserProfile`: id, handle, avatar, role, trust level,
  badges, public counts, and join timestamp. It never returns email.
- `GET /users/{handle}/threads` and `/comments` return bounded cursor `Page<T>` envelopes. Web consumes
  those envelopes and exposes load-more states.
- `PATCH /me` accepts only owner-controlled public fields. Role, trust, sanctions, and counts are not
  writable through this endpoint.
- Profile staff UI is capability-derived and deep-links to the user directory; backend authorization is
  still decisive.

## Starting a conversation delivered in this PR

- The primary entry is `私信` on a profile or the new-conversation handle form. Users never need to enter
  a numeric account id.
- `POST /forum/dm/conversations` accepts `recipientHandle`.
- The database stores `account_low_id` and `account_high_id`, enforces `low < high`, and applies a unique
  constraint to the pair. Concurrent creation returns the same canonical conversation.
- Users cannot message themselves, inactive/suspended recipients, or an account where either side has
  blocked the other.
- Trust-level 0 cannot initiate a conversation. Active silence blocks conversation creation and sending.
- Sending rechecks participant lifecycle, sanctions, and both directions of the block relationship, then
  applies a token-bucket rate limit.

## Read, unread, and message behavior delivered in this PR

- Inbox and message lists are cursor-paginated and participant-scoped.
- The inbox returns the other user's public identity, last-message excerpt/time, and exact unread count.
- `POST /forum/dm/conversations/{id}/read` advances only the authenticated participant's
  `last_read_message_id`; it never moves the pointer backwards.
- Messages are immutable, must contain 1–16000 characters, and are available only to active participants.
- The Web inbox marks the newest visible message read and refreshes the conversation unread count.
- A new message creates a recipient notification containing a bounded excerpt, conversation id, and
  sender handle rather than the full conversation.

### Participant lifecycle not yet delivered

Migration `0021` adds participant `archived_at` and `deleted_at`, but this PR does not expose archive or
delete endpoints or UI. Therefore the following remain **FOLLOW-UP**:

- independent archive/unarchive and delete/recover actions;
- both-participant deletion semantics;
- delayed body purge and legal-hold handling;
- an idempotent, observable retention worker.

The presence of columns alone must not be described as a shipped lifecycle.

## Blocking delivered in this PR

- `/me/ignores` lists block relationships with cursor pagination; PUT/DELETE add or remove a relationship.
- Blocking is checked in both directions when opening or sending in an existing conversation.
- The profile and conversation UI offer block/unblock with explicit confirmation.
- Blocking does not grant either participant access to new data and does not expose unrelated private
  messages.

## Reporting and staff evidence delivered in this PR

- A participant can report a specific accessible message with a fixed category and optional bounded note;
  duplicate reports of the same message by the same participant conflict.
- Staff have no general DM inbox or message-browsing endpoint.
- `GET /admin/dm/reports` is capability-checked and returns only report metadata, public reporter/sender
  handles, and a bounded excerpt of the reported message. Listing evidence records a governance audit
  event.
- Staff may resolve an open DM report as `upheld` or `rejected`; the decision writes forum and governance
  audit records in the same transaction.
- The product does not claim end-to-end encryption. Database, transport, backup, and operator access are
  separate operational controls.

## Web experience delivered in this PR

- Two-pane desktop inbox and single-pane mobile flow use stable `?conversation=` URLs.
- Conversation rows show handle/avatar, last excerpt/time, unread badge, and pagination states.
- The profile `发私信` action opens the canonical conversation; the new-conversation form accepts the
  exact public handle.
- Messages are separated by sender, ordered consistently, preserve line breaks, and load older pages.
- Sending disables duplicate submission and exposes errors; users can report a selected message or change
  the block relationship from the conversation.

## Follow-up privacy and retention work

- Archive/delete/recovery APIs and UI, both-participant purge eligibility, and the retention worker.
- Attachment storage, malware/content-type validation, authorization, deletion, and CDN policy.
- Private message search. DM content remains excluded from the existing public search surface.
- Public-profile activity visibility controls and data export.
- Account-deletion anonymization and retention behavior across profile URLs, DMs, reports, and backups.

## Delivered verification baseline

- Profile DTOs, OpenAPI-generated types, Rust responses, and Web consumption agree.
- Public lists paginate and exclude hidden/soft-deleted content and campus email.
- Concurrent conversation creation yields one canonical pair; non-participants cannot list messages or
  report inaccessible messages.
- Read pointers advance monotonically and unread counts are derived per participant.
- Blocking an existing sender prevents the next message; silence, trust, and recipient availability are
  enforced.
- Staff cannot query arbitrary DM bodies; reported evidence listing and decisions are capability-checked
  and audited.

Archive/delete/retention behavior is explicitly excluded from this baseline until its follow-up work
lands.

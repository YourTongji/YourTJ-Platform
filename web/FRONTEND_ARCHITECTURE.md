# YourTJ Web Frontend Architecture

> **Status:** Current frontend inventory; partial/proposed areas are labelled below
>
> **Owner:** Web maintainers
>
> **Last verified:** 2026-07-11 against `origin/main@06a8898`
>
> **Authoritative sources:** `contract/openapi.yaml` for the intended HTTP contract and `web/src` for the current client implementation

Product rules live in the [documentation index](../docs/README.md), especially
[activity scoring](../docs/product/activity-scoring.md),
[profiles and messaging](../docs/product/profile-and-messaging.md), and the
[admin console](../docs/operations/admin-console.md). This file records frontend structure and must not
promote a partial screen to a complete product capability.

## Stack

- Vite + React + TypeScript
- React Router for page routing
- TanStack Query for request caching and invalidation
- Zustand for local schedule persistence
- shadcn/ui style Radix primitives in `src/components/ui`
- API schema generated from `../contract/openapi.yaml` via `pnpm generate:api`

## Runtime

Default API base is `/api/v2`. During local development, Vite proxies `/api` to
`http://localhost:8080`. Set `VITE_API_BASE_URL` when deploying behind another
gateway.

TongjiCaptcha defaults to `https://captcha.07211024.xyz`. Set `VITE_CAPTCHA_URL`
when a deployment uses another compatible service base. The reusable browser flow
loads real challenge images from `/api/captcha`, verifies selected indices through
`/api/verify`, and passes only the returned single-use token to protected API writes.

## Feature Map

| Area | Routes | State | Backend APIs |
|---|---|---|---|
| Home | `/` | **Partial:** feed works; right grid is still synthetic trust-level data until this PR's activity API lands | announcements, forum feed; proposed `/me/activity` |
| Auth | `/login` | Current, subject to contract conformance tests | email/password auth, refresh, logout, `/me` |
| Forum | `/forum`, `/forum/threads/:id`, `/bookmarks` | Current core, with moderation UI expansion proposed | boards, tags, threads, comments, votes, flags, bookmarks, subscriptions, polls |
| Messages | `/messages` | **Partial:** basic 1:1 list/send only; no read state, report/archive flow, or handle-first composer | DM conversations and messages |
| Courses | `/courses`, `/courses/:id` | Current core | departments, courses, search, details, AI summary, reviews, likes, reports |
| Selection | `/schedule` | Current client uses implementation aliases; contract mismatch documented below | calendars, grades, majors, natures, course search/timeslots |
| Wallet | `/wallet` | Current core; signing uses one-time intents | wallet, legacy claim, ledger verify, tip, tasks, products, purchases |
| Notifications | `/notifications` | Current core | list, unread count, mark read, account notification prefs |
| Profile | `/profile/:handle` | **Partial:** screen exists; list response contract currently drifts from Rust | public profile, user threads/comments |
| Settings | `/settings` | Current profile and backend-persisted notification preferences | `/me`, `/me/notification-prefs` |
| Admin | `/admin` | **Partial:** only reviews/reports/settings/job triggers are surfaced | expanded capability-driven console is proposed in this PR |

## Selection Adaptation

The OpenAPI contract lists several selection paths in the normalized form
`/selection/courses/by-major`, `/selection/courses/by-nature`,
`/selection/courses/by-code/{code}`, and `/selection/courses/by-time`.

The current Rust router exposes:

- `/selection/courses-by-major`
- `/selection/courses-by-nature`
- `/selection/courses/{code}`
- `/selection/courses/{code}/timeslots`

The current frontend uses the Rust router aliases so it can operate against the current binary. This is
an acknowledged contract defect, not a new source of truth: the implementation, OpenAPI, and generated
client must converge before the mismatch can be marked resolved.

## Wallet Signing

The frontend implements a local Ed25519 wallet:

- private seed: browser localStorage only
- public key: sent to `/wallet/bind`
- signing helper: `src/lib/wallet.ts`

Current backend contract uses `POST /credit/signing-intents` to return exact `signingBytes`, followed by
the write request carrying `X-Wallet-Intent`, `X-Wallet-Sig`, and the same `Idempotency-Key`. Any screen
that has not completed this two-step flow must stay labelled unavailable rather than falling back to an
unsigned request.

## Verification

Run before delivery:

```bash
pnpm generate:api
pnpm lint
pnpm typecheck
pnpm build
```

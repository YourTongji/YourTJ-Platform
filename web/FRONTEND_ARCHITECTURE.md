# YourTJ Web Frontend Architecture

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

## Feature Map

| Area | Routes | Backend APIs |
|---|---|---|
| Home | `/` | announcements, hot forum feed, hot courses, wallet summary |
| Auth | `/login` | `/auth/email/request-code`, `/auth/email/verify`, `/auth/refresh`, `/auth/logout`, `/me` |
| Forum | `/forum`, `/forum/threads/:id`, `/bookmarks` | boards, tags, thread feed, create thread, comments, votes, flags, bookmarks, subscriptions, polls |
| Messages | `/messages` | DM conversations and messages |
| Courses | `/courses`, `/courses/:id` | departments, courses, search, details, AI summary, reviews, likes, reports |
| Selection | `/schedule` | calendars, grades, majors, course natures, courses by major/nature/search, course timeslots, latest sync |
| Wallet | `/wallet` | wallet, bind public key, legacy claim challenge/claim, ledger, ledger verify, tip, tasks, products, purchases |
| Notifications | `/notifications` | notifications, unread count, mark read, notification prefs via settings |
| Profile | `/profile/:handle` | public user profile, user threads, user comments |
| Settings | `/settings` | `/me` profile update; local notification preference placeholder |
| Admin | `/admin` | review queue, report queue, settings, selection sync, search reindex |

## Selection Adaptation

The OpenAPI contract lists several selection paths in the normalized form
`/selection/courses/by-major`, `/selection/courses/by-nature`,
`/selection/courses/by-code/{code}`, and `/selection/courses/by-time`.

The current Rust router exposes:

- `/selection/courses-by-major`
- `/selection/courses-by-nature`
- `/selection/courses/{code}`
- `/selection/courses/{code}/timeslots`

The frontend uses the Rust router paths because they are authoritative for the
current backend binary.

## Wallet Signing

The frontend implements a local Ed25519 wallet:

- private seed: browser localStorage only
- public key: sent to `/wallet/bind`
- signing helper: `src/lib/wallet.ts`

Current backend status:

- Task/product escrow endpoints accept or ignore `X-Wallet-Sig` and are wired.
- `/credit/tip` generates `tx_id`, `nonce`, and `timestamp` on the server before
  signature verification. A browser cannot pre-sign the exact payload. The UI
  therefore documents the backend protocol gap instead of pretending tips work.

To make tipping fully usable, add a backend intent/challenge endpoint that returns
the canonical payload to sign, or let the client provide the nonce/timestamp/tx id
that the backend verifies.

## Verification

Run before delivery:

```bash
pnpm generate:api
pnpm lint
pnpm typecheck
pnpm build
```

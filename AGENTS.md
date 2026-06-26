# AGENTS.md — YourTJ Platform

Operating guide for anyone (human or AI agent) writing code in this repository.
Read this **and** [`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md) before making changes.

---

## 1. What this is

YourTJ Platform is the unified backend + web monorepo for a **campus community
platform**. The forum is the headline product; course selection (选课), course
reviews (评课), and the Web2.5 points system (积分) are sub-domains that share one
identity, one database, and one deployment.

- Backend: **Rust** — Axum + Tokio, a Cargo workspace split by domain.
- Database: **PolarDB** (PostgreSQL-compatible), one schema per domain.
- Search: **Meilisearch**. Cache/counters/rate-limit/hot-rank: **Redis**. Media: **OSS + CDN**.
- Deploy: **Aliyun 华东** (ICP-filed) on **SAE** serverless containers; same image
  runs on SLB + ECS later.
- Identity: campus-email code login + JWT; each account binds an **Ed25519** key,
  used only to sign money operations.
- Points: **Web2.5 closed-loop** — central ledger + Ed25519 signatures + hash chain.

iOS and Flutter clients live in **separate repos** and consume types generated from
`contract/openapi.yaml`. They are not in this monorepo.

---

## 2. Repository layout & crate boundaries

```
backend/crates/
  api/        Axum gateway binary — process startup + router composition only. No business logic.
  identity/   accounts, email auth, sessions, Ed25519 public keys.
  courses/    catalogue, 选课 mirror tables, search surface.
  reviews/    reviews, likes, reports, moderation queue.
  credit/     Web2.5 points ledger.
  forum/      boards, threads, comments, votes, notifications (Phase B).
  shared/     config, the AppError type, pagination. Dependency-light; compiled by everyone.
```

**Boundary rules**
- A domain crate owns its tables. Cross-domain access goes through that crate's
  public API, never by reaching into another domain's tables from foreign SQL.
- `shared` must not depend on any domain crate (no cycles). Domain crates may depend
  on `shared`. `api` depends on everything and wires it together.
- Put new HTTP routes in the owning domain crate's `routes()`; `api` only `.merge()`s them.

---

## 3. Local development

```bash
cd backend
cp .env.example .env
cargo run --bin api      # http://localhost:8080/health
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Postgres / Redis / Meilisearch run as local containers (compose file added later).

### Definition of done — run these before you call a change finished
1. `cargo fmt --all` — no diffs.
2. `cargo clippy --all-targets --all-features -- -D warnings` — clean.
3. `cargo test --all` — green.
4. If you changed the HTTP surface, update `contract/openapi.yaml` in the same change.
5. If you changed the schema, add a **new** migration (never edit an applied one).
6. Public items have rustdoc; new invariants are noted in the design doc.

---

## 4. Rust coding conventions

These are enforced in CI (`fmt` + `clippy -D warnings`). Match the style already in
`crates/shared` and `crates/api`.

### 4.1 Toolchain & formatting
- Pinned by `rust-toolchain.toml` (stable). Edition **2021**.
- `rustfmt` is authoritative — never hand-format. Config in `backend/rustfmt.toml`
  (`max_width = 100`). Order imports std → external → crate as a convention.
- No `#[rustfmt::skip]` without a one-line reason comment.

### 4.2 Linting
- `clippy` runs with `-D warnings`; the build fails on any lint. Fix the cause, do not
  blanket-`#[allow(...)]`. A local `#[allow(clippy::lint)]` is acceptable only with a
  comment explaining why, scoped as tightly as possible.
- No compiler warnings either — unused imports/vars must go.

### 4.3 Naming
- `snake_case` for functions, variables, modules, crates; `CamelCase` for types and
  traits; `SCREAMING_SNAKE_CASE` for consts/statics.
- Names say what, not how: `verify_ledger`, not `do_check`. Booleans read as
  predicates: `is_visible`, `has_more`.
- Acronyms are one capital then lower in CamelCase: `JwtClaims`, `HttpClient`,
  `OssClient` — not `JWTClaims`.

### 4.4 Module organization
- One concern per file; group with `mod`. Keep `lib.rs`/`main.rs` thin (re-exports +
  wiring). Prefer `mod foo;` files over giant single modules.
- Expose the minimum: default to private, `pub(crate)` for cross-module-internal,
  `pub` only for the crate's real API. No `pub use` re-export sprawl.

### 4.5 Error handling — this is strict
- Handlers and services return `shared::AppResult<T>` (`Result<T, AppError>`). Convert
  with `?`. Render errors only through `AppError`'s `IntoResponse`.
- **No `.unwrap()` / `.expect()` / `panic!` in non-test code.** The only exception is
  unrecoverable startup wiring in `api/main.rs`, and even there prefer `?`. Use
  `.expect("reason")` (with a real reason) over `.unwrap()` if you truly must.
- Library/domain errors: `thiserror` enums. Application glue: `anyhow`. Map internal
  failures into a specific `AppError` variant at the boundary — **never leak DB strings,
  IDs, or stack traces to clients**; `AppError::Internal` logs server-side and returns a
  generic 500.
- Validate inputs early and return `AppError::BadRequest(...)`; don't let bad data reach
  the DB.

```rust
// good
let account = repo.find(id).await?.ok_or(AppError::NotFound)?;

// bad — panics on a normal "not found", and on any DB error
let account = repo.find(id).await.unwrap().unwrap();
```

### 4.6 Async & concurrency
- Everything is `async` on Tokio. **Never block the runtime**: no `std::thread::sleep`,
  no blocking file/CPU work on an async task — use `tokio::time::sleep` and
  `tokio::task::spawn_blocking` for CPU-bound work (e.g. PBKDF2/Argon2, pinyin batch).
- **Do not hold a `Mutex`/`RwLock` guard across `.await`.** Prefer message passing or
  scope the lock tightly. For shared state use `Arc<…>`; for hot counters use Redis, not
  an in-process lock.
- Don't spawn unbounded tasks for per-request work; use the request task. Background jobs
  are explicit and supervised.

### 4.7 HTTP / Axum
- Routes are versioned under `/api/v2/...`. Path params use Axum 0.8 syntax `{id}`.
- Handler signature: `async fn handler(...) -> AppResult<Json<Dto>>` (or `impl
  IntoResponse`). Extract with typed extractors (`State`, `Path`, `Query`, `Json`); do
  not parse raw bodies by hand.
- Shared state travels via `State<AppState>` where `AppState` is cheap to clone
  (`Arc` inside). No global mutable statics.
- Auth, rate-limit, and request-id are middleware/`tower` layers — not copy-pasted into
  handlers. Money operations additionally verify `X-Wallet-Sig` (see §5).
- Responses use the platform envelope: success returns the DTO; errors return
  `{ "error": { "code", "message" } }` via `AppError`. Lists return `shared::Page<T>`.

### 4.8 Database / sqlx
- All SQL goes through `sqlx` with **bound parameters** (`$1, $2`). Never format user
  input into SQL strings.
- Reads that can tolerate slight staleness use the read replica (`Config::read_url()`);
  writes and read-your-write paths use the primary.
- Money and multi-row invariants run in a **transaction**. Ledger appends take an
  advisory lock to keep the hash chain linear (see §5).
- Schema changes are migrations in `backend/migrations/NNNN_name.sql`, append-only.
  Never edit a migration that has run anywhere. Keep DDL reviewable and reversible in
  intent.
- Separate DB row structs from API DTOs — don't serialize a DB row straight to the client.

### 4.9 Serialization / DTOs
- JSON is `camelCase`. Put `#[serde(rename_all = "camelCase")]` on every
  request/response struct.
- DTOs are explicit, named structs — not `serde_json::Value` (placeholders excepted).
  Make illegal states unrepresentable: prefer enums over stringly-typed status fields.
- Timestamps are Unix seconds (`i64`) over the wire.

### 4.10 Logging & tracing
- Use `tracing`, never `println!`/`eprintln!`. Add `#[tracing::instrument(skip(...))]`
  to service entry points; `skip` anything large or sensitive.
- Levels: `error` (needs attention), `warn` (recoverable anomaly), `info` (lifecycle),
  `debug`/`trace` (dev detail). Structured fields, not string interpolation:
  `tracing::info!(account_id, "review published")`.
- **Never log secrets or PII**: no email addresses, codes, tokens, private keys,
  signatures-as-credentials, raw request bodies. When in doubt, omit.

### 4.11 Security
- Server stores only Ed25519 **public** keys and password/secret **hashes** — never
  private keys, PINs, or plaintext codes.
- Verify every money/escrow write's `X-Wallet-Sig` against the account's public key
  before mutating the ledger. Reject on missing/invalid signature or replayed nonce.
- Use constant-time comparison for secrets/codes/MACs. Hash verification codes and
  refresh tokens at rest.
- Validate and bound all input (lengths, ranges, enums). Rate-limit writes and search.
- Secrets come from the environment / secret manager. Never commit a real `.env`,
  key, or token. `CREDIT_SYSTEM_PRIVATE_KEY` and `JWT_SECRET` are loaded at runtime only.

### 4.12 Testing
- Unit-test pure logic next to it (`#[cfg(test)] mod tests`). Integration tests under
  `crates/<crate>/tests/`. DB/integration tests use ephemeral containers
  (testcontainers) — never a shared/real database.
- The ledger, signatures, balance derivation, and escrow state machine **must** have
  tests covering tampering, replay, and edge amounts. Treat money code as high-assurance.
- Test names describe behavior: `rejects_review_when_rating_out_of_range`.

### 4.13 Dependencies
- Pin versions in the workspace `[workspace.dependencies]`; crates reference them with
  `<dep>.workspace = true`. One version per dependency across the workspace.
- Add a dependency only when it clearly beats std + a little code. Prefer mature, widely
  used crates. Justify anything heavy or unusual in the PR.
- A workspace dependency is free until a crate uses it — declare intended deps centrally,
  enable per-crate when the code lands.

### 4.14 Performance & caching
- Honor the cache model in the design doc (§4): L1 client debounce → L2 edge SWR → L3
  Redis; **version-bump invalidation**, not blind deletes.
- Never recompute aggregates on the read path. `review_count` / `review_avg` and similar
  are maintained incrementally on write.
- Realtime search is **Meilisearch only** — no `LIKE %q%` over the DB on the hot path.
- Avoid N+1 queries; batch. Measure before optimizing further — current scale is small,
  correctness and latency-to-campus matter more than micro-throughput.

### 4.15 Documentation
- Every public item gets a rustdoc line saying what it does and any invariant it upholds.
- Module-level `//!` docs state the domain's responsibility and its hard rules (see the
  existing crate headers).
- Comments explain **why**, not what the code already says. Keep them current.

---

## 5. Domain invariants — do not break these

### Identity
- The public **handle** is shown to users; the real **email** is server-only (moderation,
  anti-abuse). Don't expose email in any public response.
- Sessions are JWT access + revocable refresh. Old wallets are merged via a signed
  challenge (`/wallet/claim`), never by importing a secret.

### 积分 Web2.5 — 合规红线（HARD COMPLIANCE LINE）
闭环虚拟权益，**不是**虚拟货币。无论需求怎么提，下面这些不许做：
- **无充值入口、无提现、不与法币双向兑换、不可套现。** No recharge, no withdrawal, no
  fiat on/off-ramp, no cashout.
- **不开放无理由自由转账。** Value moves only inside controlled flows: `mint` (system),
  `escrow_hold` / `escrow_release`, `tip`, `bounty`. Do **not** add a free peer
  `transfer` endpoint.
- 积分**纯靠贡献赚取**（系统签名 `mint`）。
- Ledger (`credit.ledger`) is **append-only**, monotonic `seq`, `prev_hash` chained,
  every entry Ed25519-signed. Balance (`credit.wallets.balance`) is a **derived cache** —
  never the source of truth; reconcile against the ledger.
- Appends are serialized (advisory lock) so the chain stays linear. Verification
  (`/wallet/ledger/verify`) recomputes hashes and checks every signature.

If a feature request seems to require crossing this line, stop and escalate — it is a
legal boundary, not a preference.

### Privacy / PIPL
- Store the minimum PII; encrypt the email at rest; support account deletion. Don't add
  new PII columns without a reason and a retention/deletion answer.

---

## 6. Workflows

### API contract
The HTTP surface is owned by `contract/openapi.yaml`. Change the contract first, then
implement; client type bindings are generated from it. A route that isn't in the
contract isn't done.

### Migrations
Add `backend/migrations/NNNN_descriptive_name.sql` (next number). Append-only. Test the
up-path against a fresh database. Update the design doc's DDL section if the change is
structural.

### Git & PRs
- **Do not commit or push unless explicitly asked.** When asked, never commit on `main` —
  branch first.
- Conventional commits: `feat(credit): ...`, `fix(reviews): ...`, `chore(ci): ...`.
- Small, focused PRs. CI (fmt + clippy + test + build) must be green. No secrets, no
  generated artifacts, no `target/`.

---

## 7. Agent working agreement (for AI agents)

- Read this file and the design doc before editing. Stay inside the relevant domain crate;
  don't refactor unrelated code in the same change.
- Keep changes scoped and reviewable. Match surrounding style. Don't introduce a new
  pattern when an existing one fits.
- Before declaring done, run the Definition-of-Done checklist (§3). Report honestly if a
  step failed — never claim green tests you didn't run.
- Don't add dependencies, change the public API, or touch migrations casually — those have
  review weight. Call them out.
- If a request conflicts with a §5 invariant (especially the credit compliance line),
  stop and flag it instead of implementing it.
- Don't commit or push unless the task explicitly says to.

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
  blanket-`#[allow(...)]`.
- Every `#[allow(clippy::lint_name)]` must carry a `// reason: ...` comment on the
  same line explaining why the lint is intentionally waived rather than fixed.
  Scope the attribute as tightly as possible (single item, not whole module).
- No compiler warnings — unused imports/vars must go.

### 4.3 Comments
- Comments explain **why**, not what. The code already says what it does.
- Doc comments on public APIs are required only when the purpose is not obvious from
  the name and signature alone.
- No inline comments that restate the next line of code.
- No decay comments (e.g. `// defaults to 60`, `// expires in 5 minutes`). Types
  and const names are the source of truth; stale value-comments are worse than none.
- Section dividers like `// ── Foo ──` are not allowed in files under 200 lines.
  Split into modules instead.
- No `TODO` comments in committed code — use real issue tracking.

### 4.4 Naming
- `snake_case` for functions, variables, modules, crates; `CamelCase` for types and
  traits; `SCREAMING_SNAKE_CASE` for consts/statics.
- Names say what, not how: `verify_ledger`, not `do_check`. Booleans read as
  predicates: `is_visible`, `has_more`.
- Acronyms are one capital then lower in CamelCase: `JwtClaims`, `HttpClient`,
  `OssClient` — not `JWTClaims`.
- **No single-letter variable names** (`a`, `b`, `c`, `e`, `r`, `s`, `v`, etc.).
  The only exceptions: `i` / `j` for loop indices, and `f` in closures like
  `|f| f.id == id`.
- Abbreviated names are acceptable when domain-standard and unambiguous across the
  codebase: `ctx` (context), `req` (request), `tx` (transaction), `repo`
  (repository). If in doubt, spell it out.
- Enum variants and function names must be self-explanatory without a doc comment.
  If a variant needs a comment to explain its purpose, rename it.
- Avoid `XxxInfo`, `XxxDetail`, `XxxData` — find a real name.
- The same concept must be named consistently across all files. If `repo.rs` calls
  it `course_id`, `handlers.rs` must not call it `class_id` for the same thing.

### 4.5 Module organization
- One concern per file; group with `mod`. Keep `lib.rs`/`main.rs` thin (re-exports +
  wiring). Prefer `mod foo;` files over giant single modules.
- Expose the minimum: default to private, `pub(crate)` for cross-module-internal,
  `pub` only for the crate's real API. No `pub use` re-export sprawl.

### 4.6 Error handling — this is strict
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

### 4.7 Async & concurrency
- Everything is `async` on Tokio. **Never block the runtime**: no `std::thread::sleep`,
  no blocking file/CPU work on an async task — use `tokio::time::sleep` and
  `tokio::task::spawn_blocking` for CPU-bound work (e.g. PBKDF2/Argon2, pinyin batch).
- **Do not hold a `Mutex`/`RwLock` guard across `.await`.** Prefer message passing or
  scope the lock tightly. For shared state use `Arc<…>`; for hot counters use Redis, not
  an in-process lock.
- Don't spawn unbounded tasks for per-request work; use the request task. Background jobs
  are explicit and supervised.

### 4.8 HTTP / Axum
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

### 4.9 Database / sqlx
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

### 4.10 Serialization / DTOs
- JSON is `camelCase`. Put `#[serde(rename_all = "camelCase")]` on every
  request/response struct.
- DTOs are explicit, named structs — not `serde_json::Value` (placeholders excepted).
  Make illegal states unrepresentable: prefer enums over stringly-typed status fields.
- Timestamps are Unix seconds (`i64`) over the wire.

### 4.11 Logging & tracing
- Use `tracing`, never `println!`/`eprintln!`. Add `#[tracing::instrument(skip(...))]`
  to service entry points; `skip` anything large or sensitive.
- Levels: `error` (needs attention), `warn` (recoverable anomaly), `info` (lifecycle),
  `debug`/`trace` (dev detail). Structured fields, not string interpolation:
  `tracing::info!(account_id, "review published")`.
- Every failed outcome must log at `warn!` with structured fields (`?error` or named
  fields), not a bare string.
- **Never log secrets or PII**: no email addresses, codes, tokens, private keys,
  signatures-as-credentials, raw request bodies. When in doubt, omit.

### 4.12 Security
- Server stores only Ed25519 **public** keys and password/secret **hashes** — never
  private keys, PINs, or plaintext codes.
- Verify every money/escrow write's `X-Wallet-Sig` against the account's public key
  before mutating the ledger. Reject on missing/invalid signature or replayed nonce.
- Use constant-time comparison for secrets/codes/MACs. Hash verification codes and
  refresh tokens at rest.
- Validate and bound all input (lengths, ranges, enums). Rate-limit writes and search.
- Secrets come from the environment / secret manager. Never commit a real `.env`,
  key, or token. `CREDIT_SYSTEM_PRIVATE_KEY` and `JWT_SECRET` are loaded at runtime only.

### 4.13 Testing
- **Unit tests** for pure logic go in the same file (`#[cfg(test)] mod tests`).
  **Integration tests** go under `crates/<crate>/tests/`. Prefer integration tests
  over inline modules when DB or multiple crate boundaries are involved.
- Integration tests use ephemeral containers (testcontainers) — never a shared/real
  database.
- Test tools (helpers, builders, fixtures) live in the test directory (`tests/helpers/`),
  not in `src/`. Production code must not carry test-only utilities.
- Never test mock infrastructure (helpers, builders, replay policies). Tests must exercise
  the real crate API — handler → repo → DB.
- Never test simple getters or trivial struct field access. Test behaviour: "does this
  produce the right side-effect?" not "does this struct field match what I just set?"
- The ledger, signatures, balance derivation, and escrow state machine **must** have
  tests covering tampering, replay, and edge amounts. Treat money code as high-assurance.
- Test names describe behavior: `rejects_review_when_rating_out_of_range`.
- Prefer edge cases over CRUD enumeration. One test for "remove and verify count" is
  enough — don't write three variations.
- Every `#[should_panic]` test must verify the panic message (`expected = "..."`).
- Tests must be fast. No test may depend on external services (LLM APIs) or long
  timeouts. DB integration tests use local ephemeral containers.
- Tests must be process-safe. Do not assume shared static state, global counters, or
  filesystem paths that are unique within-process but collide across processes.
  Use random prefixes for temp paths when needed.

### 4.14 Dependencies
- Pin versions in the workspace `[workspace.dependencies]`; crates reference them with
  `<dep>.workspace = true`. One version per dependency across the workspace.
- Add a dependency only when it clearly beats std + a little code. Prefer mature, widely
  used crates. Justify anything heavy or unusual in the PR.
- A workspace dependency is free until a crate uses it — declare intended deps centrally,
  enable per-crate when the code lands.
- Library crates must not depend on `tracing-subscriber` unless they initialize a
  subscriber in test-only or explicitly owned runtime code. Libraries use `tracing`;
  binaries initialize subscribers.
- Workspace crates are private unless explicitly prepared for publishing. Set
  `publish = false` in each crate manifest.
- Internal path dependencies (`foo = { path = "crates/foo" }`) must also include a
  version (`foo = { version = "0.1.0", path = "crates/foo" }`) so dependency policy
  tools do not treat them as wildcard requirements.
- Use `cargo machete --with-metadata` to check for unused dependencies. Do not remove
  a dependency solely because `cargo machete` flagged it — verify with source search
  and `cargo tree` first. Confirmed false positives must be documented in the crate's
  `[package.metadata.cargo-machete] ignored = [...]` table.
- Use `cargo deny check` to enforce dependency policy: advisories, yanked crates,
  license allowlists/exceptions, duplicate-version warnings, and source restrictions.
  Do not silence a deny finding without documenting why the dependency is required
  and why the policy exception is safe.

### 4.15 Performance & caching
- Honor the cache model in the design doc (§4): L1 client debounce → L2 edge SWR → L3
  Redis; **version-bump invalidation**, not blind deletes.
- Never recompute aggregates on the read path. `review_count` / `review_avg` and similar
  are maintained incrementally on write.
- Realtime search is **Meilisearch only** — no `LIKE %q%` over the DB on the hot path.
- Avoid N+1 queries; batch. Measure before optimizing further — current scale is small,
  correctness and latency-to-campus matter more than micro-throughput.

### 4.16 Documentation
- Every public item gets a rustdoc line saying what it does and any invariant it upholds.
  Skip the doc when the name and signature already make the purpose obvious.
- Module-level `//!` docs state the domain's responsibility and its hard rules (see the
  existing crate headers).

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
- All development happens on feature/personal branches. **Never commit directly to
  `main`**, never `git push origin main`, never `git checkout main`.
- Changes land on `main` only through pull requests — never by direct commit or local
  merge into `main`.
- When asked to merge something into `main`, open a PR from the working branch and let
  the user handle the merge.
- Never discard uncommitted changes. If you need to switch branches, `git stash` the
  work first. Do not `git reset --hard` or `git checkout -f` for changes that still
  need to land.
- Conventional commits: `feat(credit): ...`, `fix(reviews): ...`, `chore(ci): ...`.
- Small, focused PRs. CI (fmt + clippy + test + build) must be green. No secrets, no
  generated artifacts, no `target/`.
- Only commit changes you authored. Do not include, revert, or modify other people's
  work (e.g. `Cargo.lock` updates, dependency bumps, files created by other agents or
  users) unless explicitly instructed.

---

## 7. Agent working agreement (for AI agents)

- Read this file and the design doc before editing. Stay inside the relevant domain crate;
  don't refactor unrelated code in the same change.
- **Do not autonomously modify code unless the user explicitly asks.** When the user
  asks for a change, wait for their confirmation before starting — do not pre-emptively
  implement.
- Keep changes scoped and reviewable. Match surrounding style. Don't introduce a new
  pattern when an existing one fits.
- Commit each logical change atomically before moving to an unrelated topic. Do not
  squash unrelated refactors together.
- Before declaring done, run the Definition-of-Done checklist (§3). Report honestly if a
  step failed — never claim green tests you didn't run.
- Don't add dependencies, change the public API, or touch migrations casually — those have
  review weight. Call them out.
- If a request conflicts with a §5 invariant (especially the credit compliance line),
  stop and flag it instead of implementing it.
- Don't commit or push unless the task explicitly says to.

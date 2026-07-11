---
name: yourtj-development
description: Develop, fix, refactor, review, test, document, or publish changes in the YourTJ Platform repository. Use for any task involving Rust backend, React Web, OpenAPI, migrations, CI/deployment, repository documentation, commits, pushes, or pull requests so domain boundaries, verification, documentation impact, and publication authority are handled consistently.
---

# YourTJ Development

Use this workflow for repository work from initial scope through verified handoff. Keep product behavior,
wire contracts, database shape, implementation, tests, and documentation synchronized.

## 1. Establish authority and workspace

Classify the request before changing state:

- **Read-only:** analysis, review, diagnosis, or status. Inspect and report; do not implement or publish.
- **Change:** fix, build, update, or create. Implement and verify only the requested scope.
- **Publish:** commit, push, open/update a PR, deploy, or mutate an external system. Require explicit
  authorization for the requested publication action; change authorization alone is insufficient. An
  explicit request to open a PR includes the necessary commit and feature-branch push, but never merge,
  production deployment, or unrelated external mutation.

Run `git status --short --branch`, inspect worktrees, and preserve existing changes. Never work directly on
`main`. Create a branch from current `origin/main`; use `codex/<topic>` by default. Prefer a new worktree
when the current checkout is dirty. Never discard or include unrelated work.

## 2. Read the governing sources

Read completely before implementation:

1. repository `AGENTS.md`;
2. [`docs/README.md`](../../../docs/README.md);
3. [`docs/development/README.md`](../../../docs/development/README.md);
4. the directly affected product, architecture, security, and operations documents;
5. `contract/openapi.yaml`, relevant migrations, source, and tests as needed.

Do not use deleted historical plans, old PR descriptions, or chat messages as a second source of truth.

## 3. Build an impact matrix

Before editing, state whether the change affects:

- backend owner domain and cross-domain reads/writes;
- Web behavior and generated types;
- HTTP/OpenAPI compatibility;
- PostgreSQL migration/backfill/concurrency;
- auth, sessions, capabilities, PII, privacy, retention, or audit;
- credit compliance/signatures/replay;
- media/OSS, search, cache, counters, notifications, or background jobs;
- deployment/config/provider secrets;
- product, architecture, development, operations, and security documents.

Stop and escalate if the request crosses the credit compliance line, needs new PII without lifecycle
answers, or requires a product decision that changes access or data semantics.

For content-format changes, explicitly inspect canonical rows, revisions/history, drafts, public/admin
DTOs, search/cache projections, notification/moderation excerpts, export, and legacy client behavior.
Every Markdown change must declare its image/embed policy; remote/data images stay disabled unless clean
asset binding and authorization are implemented and tested.

## 4. Implement in dependency order

Use this order where applicable:

1. product semantics and acceptance criteria;
2. OpenAPI contract;
3. append-only migration and compatibility plan;
4. owner-domain repository/service/handler implementation;
5. generated Web types and user/admin surfaces;
6. focused tests, then scope-wide verification;
7. documentation and operational runbooks.

Keep `api` to composition/wiring and use domain public APIs. Reuse existing validation, transaction,
authorization, error, and audit patterns. Never leak DB errors, secrets, email, tokens, or private content.

## 5. Verify proportionally

Read and follow [`docs/development/testing.md`](../../../docs/development/testing.md). Always run:

```bash
python3 scripts/check_docs.py
git diff --check
```

Then run the exact gates for changed paths:

- Backend: fmt, clippy, unit tests, dedicated-DB serial integration tests.
- OpenAPI: regenerate `web/src/lib/api/schema.ts`, inspect diff, run backend and Web gates.
- Web: generate types, lint, typecheck, build, and manually verify desktop/mobile interaction states.
- Migration: fresh database up-path plus affected data/concurrency tests.
- Auth/PII/governance/credit/search/media: include documented negative, replay, privacy, failure, and
  reconciliation cases.
- Content/Markdown: automate legacy-format compatibility, create/edit parity, raw HTML and unsafe URL
  rejection, sanitizer/XSS corpus, AST/output resource limits, image/embed policy, and safe plain-text
  search/notification projections. Manual preview alone is not sufficient for a security-critical renderer.

Report commands that failed, skipped, or were not run. Never infer CI success from a local subset.

## 6. Synchronize documentation

Follow [`docs/development/documentation.md`](../../../docs/development/documentation.md). Update the owning
product document and current-state inventory when user-visible capability changes. Update architecture,
security, operations, `.env.example`, or development guides when their facts change.

Every PR must contain either linked documentation changes or `Docs impact: none` with a concrete reason.
Do not use PR-relative permanent status labels or copy OpenAPI/DDL into prose.

## 7. Review the final diff

Before handoff or publication:

- inspect `git status`, full diff, and diff stats;
- confirm only in-scope/authored files are present;
- search for credentials, real PII, dumps, local paths, generated build output, and stale document links;
- verify contract/schema/docs/current-state claims match the implementation;
- record known limitations and safe next steps without leaving production `TODO` comments.

## 8. Commit, push, and open a PR only when authorized

Follow [`docs/development/pull-requests.md`](../../../docs/development/pull-requests.md). Use logical
Conventional Commits, push the feature branch, and fill `.github/PULL_REQUEST_TEMPLATE.md` with actual
test results, impact, migration/deployment notes, docs impact, preview journey, and rollback.

After pushing, wait for CI. When runtime paths changed, verify that PR preview deployed both frontend and
backend and manually exercise the feature. Do not merge into `main`; the user or maintainer handles merge.

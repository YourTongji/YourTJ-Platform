# 测试策略与命令

> 文档类型：开发与测试指南
>
> 状态：Active
>
> 负责人：Platform maintainers、Domain maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

先跑最小 focused test 获得快速反馈，再按 changed scope 跑与 CI 一致的完整 gate。没有运行的检查
不能在 PR 或交付中写成通过。

## 文档检查

所有 PR 都运行：

```bash
python3 scripts/check_docs.py
git diff --check
```

Pull request CI 还会用 `scripts/check_pr_docs.py` 对比 base/head 与 PR 的“文档影响”段；它由 workflow
提供 SHA 和 body，普通 local check 不需要伪造这些环境变量。

## Backend quick gates

在 `backend/`：

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- --deny warnings
cargo test --lib
```

Focused test 示例：

```bash
cargo test -p identity password
cargo test -p forum --test dm_tests
cargo test -p activity --test contribution_projection
```

选择会真正覆盖修改行为的 test target，不按文件名猜测成功。

## Backend CI-parity integration

CI 使用专用 PostgreSQL 17 service 和 Redis 7，并串行运行会共享数据库的 integration tests。本地
可使用独立 `yourtj_test`；不要指向开发或线上数据：

```bash
cd backend
DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj_test \
REDIS_URL=redis://localhost:6379 \
  sqlx migrate run --source migrations

DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj_test \
REDIS_URL=redis://localhost:6379 \
  cargo test --all -- --nocapture --test-threads=1
```

普通 `cargo test --all` 不带 DB/serial 参数不等价于 CI：部分 suite 会跳过或使用 fallback，多个
helper 还会清共享表。

## Web 与 contract gates

在 `web/`，Node 22 + pnpm 11.11.0：

```bash
pnpm install --frozen-lockfile
pnpm run generate:api
pnpm run lint
pnpm run typecheck
pnpm run build
```

OpenAPI 变化后必须提交生成的 `web/src/lib/api/schema.ts`。生成后审查 diff，确认没有手写覆盖：

```bash
git diff -- ../contract/openapi.yaml src/lib/api/schema.ts
```

当前 Web 尚无 Vitest/RTL/Playwright/axe suite。视觉或交互变更至少人工验证 desktop + mobile：

- loading、empty、error、success、permission-denied；
- 键盘导航、focus、读屏 label 和不只依赖颜色的状态；
- 浏览器 console/network 无新增错误；
- refresh/deep link、登录返回、API base 与 PR subpath；
- mutation 重复提交、失败恢复和 query invalidation。

Markdown/sanitizer、auth storage 或其他安全关键 browser logic 不能只靠人工 QA；相关 PR 必须先建立
可重复的最小自动化 test harness，并覆盖 XSS/unsafe URL/resource-limit corpus。

截图/录屏放 PR，不提交临时 `/tmp` 路径报告。补前端单元/组件/浏览器旅程测试是产品 P1 质量工作。

## Migration 验证

- 只新增下一个编号 migration，不编辑 applied 文件。
- 在 fresh dedicated database 运行 `sqlx migrate run --source migrations`。
- 对有数据升级写 fixture，验证 backfill、constraint、index 与并发行为。
- 确认应用旧/新版本滚动窗口是否兼容；记录 forward/rollback intent。
- 不使用 production/shared database 跑测试或手工 destructive SQL。

## 按变更范围选择检查

| Scope | 必需检查 |
|---|---|
| Docs/skill only | docs checker（含 repo skill 结构）、official skill validator（如可用）、`git diff --check` |
| Rust logic | backend quick + focused tests；完成前跑 CI-parity integration |
| Migration | backend gates + fresh migration + affected integration tests |
| OpenAPI | backend handler tests + generate types + Web gates + contract diff review |
| Web UI | generate types + lint/typecheck/build + desktop/mobile manual QA |
| Cross-stack feature | 全部 backend integration + Web gates + preview journey |
| Auth/PII/governance | 权限/枚举/重放/审计/retention 负向测试 |
| Credit | tamper/replay/edge amount/full ledger verification，高保证全套 |
| Search/cache/media | visibility、stale projection、reindex/reconcile、failure/cleanup tests |

纯内部 refactor 可以不制造无意义的产品文档 diff，但仍要在 PR 说明为什么行为、contract、schema、
security 和 operations 都不受影响。

## 关键旅程矩阵

旧的阶段性 E2E 计划已删除，但以下稳定旅程必须作为测试路线图保留：

| Suite | 必须覆盖 |
|---|---|
| S1 Contract | OpenAPI 可解析、生成类型无 diff、关键错误/分页/unauthorized shape 与 handler 一致 |
| S2 Identity | 注册、密码/验证码登录、purpose/replay、找回、refresh rotation、session revoke、sanction |
| S3 Selection data | fresh migration、Raw→catalogue/selection 物化不变量、重复运行、关键 API/search |
| S4 Reviews | publish idempotency、edit/like/unlike/report/decision、visibility、course-delete restriction |
| S5 Community | board policy、thread/comment create/edit/delete/restore、interaction、activity、follow/privacy、notification、DM |
| S6 Credit | signing intent、tamper/replay、ledger chain、balance reconciliation、escrow edge states |
| S7 Search/media | typed federated results、visibility/stale index、reindex；upload intent/callback/scan/binding/delete/GC |
| S8 Final reconciliation | counters、activity、search、unread、jobs、audit 与 source-of-truth 终态一致 |

当前 `crates/e2e` 没有覆盖这张完整矩阵，也未进入 CI；Web 也没有浏览器 suite。不要在文档或 PR
声称“E2E 完整”。每个功能 PR 先补受影响旅程，平台后续再把 S1–S8 编排成可重复的隔离环境 gate。

## PR 记录

PR body 写实际命令和结果，例如 `cargo test -p forum --test dm_tests (12 passed)`。不要只勾选
“tests passed”；失败、skip、未运行及原因同样必须记录。CI 与 preview 仍需在 push 后独立确认。

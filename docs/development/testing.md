# 测试策略与命令

> 文档类型：开发与测试指南
>
> 状态：Active
>
> 负责人：Platform maintainers、Domain maintainers
>
> 最近核验：2026-07-12，durable notification、account lifecycle/export focused suites 与 Onebox TLS fixture

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
cargo test -p identity --test recent_auth -- --test-threads=1
cargo test -p identity --test account_lifecycle -- --test-threads=1
cargo test -p identity --test admin_governance lifecycle_dead_letter -- --test-threads=1
cargo test -p api account_data::tests::partial_owner_cleanup_failure -- --test-threads=1
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

Shared integration helper 不得使用 `TRUNCATE identity.accounts CASCADE`：它会级联到治理历史，
在保护不足的 schema 上破坏证据，在完整 schema 上则应被 append-only truncate trigger 拒绝。当前
helper 要求数据库名以 `_test` 结尾，通过退休旧账号的 identifier/status 释放测试 fixture，并只清理
本 suite 所需的 mutable owner-domain rows，不修改 immutable audit/appeal events。新 suite 优先使用
每测试 fixture 或 fresh disposable database，不继续扩大全局 cascade cleanup。

## Web 与 contract gates

在 `web/`，Node 22 + pnpm 11.11.0：

```bash
pnpm install --frozen-lockfile
pnpm run generate:api
pnpm run test:run
pnpm run lint
pnpm run typecheck
pnpm run build
```

OpenAPI 变化后必须提交生成的 `web/src/lib/api/schema.ts`。生成后审查 diff，确认没有手写覆盖：

```bash
git diff -- ../contract/openapi.yaml src/lib/api/schema.ts
```

Web 使用 Vitest + Testing Library + axe-core 做 component 与基础无障碍回归：

```bash
pnpm run test:run       # CI/一次性运行
pnpm run test           # 开发 watch mode
```

测试环境入口为 `web/src/test/setup.ts`；共用无障碍断言放在 `web/src/test/accessibility.ts`。
新建可交互 shared component 时至少覆盖用户可见行为、keyboard/accessibility name，以及 axe 能稳定
检查的静态规则。不要只断言 implementation detail 或 class list；class 断言仅用于 motion/reduced-motion
等没有 jsdom layout engine 的约束。

当前仍无 Playwright/browser journey suite，现有 component/axe smoke test 也不等价于 screen reader、
contrast 或真实 layout 验收。视觉或交互变更至少人工验证 desktop + mobile：

- loading、empty、error、success、permission-denied；
- 键盘导航、focus、读屏 label 和不只依赖颜色的状态；
- 浏览器 console/network 无新增错误；
- refresh/deep link、登录返回、API base 与 PR subpath；
- mutation 重复提交、失败恢复和 query invalidation。

Markdown/sanitizer、auth storage 或其他安全关键 browser logic 不能只靠人工 QA；相关 PR 必须先建立
可重复的最小自动化 test harness，并覆盖 XSS/unsafe URL/resource-limit corpus。

Forum 图片变更还必须运行 `cargo test -p forum --test forum_media_attachment_tests --
--test-threads=1`，覆盖 `yourtj-asset` AST/ordered set、owner/usage/clean、stale CAS、revision、
delete/restore/GC grace；Web 运行 Markdown renderer/editor 与 Forum attachment component tests。测试
只向数据库写合成 metadata，不调用真实 OSS、CDN 或生产 credential。

Onebox 网络边界变更运行 `cargo test -p api onebox::network`。该 suite 启动运行时生成证书的本地 TLS
fixture，仍穿透生产 redirect、逐块 body limit 和 DNS pin 状态机，覆盖 host/SNI、逐跳 allowlist/DNS、
rebinding、MIME/charset、Content-Length/chunked 超限和两层 timeout；`cfg(test)` transport 只负责把
合成公网 pin 映射到 loopback 和增加测试 root，不得跳过策略状态机或访问公网。

Media binding/retention/删除变更还必须在 fresh database 运行
`cargo test -p media --test retention_gc -- --test-threads=1` 和 upload quarantine/profile binding 用例，
覆盖 profile/promotion/draft reference、clean approval age、pending 不按年龄进入 GC、未 callback intent
digest/exact-key cleanup、callback plaintext 不落库、10/100/512 MiB/500/2,000 quota 边界与 48 小时 attempt
retention、preview expiry+1 天、detached-grace cleanup、30 天 synthetic tombstone 及 hold/retry history 延长、
operational hold CAS/release、recent-auth/capability、rollout gate、account-purge held enqueue/missing-job terminal、
lease fence、provider-success redaction、system inventory every-read audit/dead-letter retry、history-purge flag 和
系统/账号 audit。测试 object store 必须是进程内 fake，不连接真实 OSS。

Migration `0057` 还要覆盖从前一版 schema 升级：source trigger 必须在 backfill 前保护引用 snapshot，
callback digest backfill 后 plaintext column 不存在，旧 API 不得与新 schema 混跑；malformed legacy draft
payload 不能中断 migration，历史 clean row 的 `cleaned_at` 从 rollout 时刻开始，且 DB preflight 在
drift/anomaly 非零时 fail closed。启用 GC 前从仓库根目录运行：

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/check_media_retention_references.sql
```

该命令只能用于获批的目标环境连接；不能在 shared/production 数据上运行 destructive test。它不检查
published Markdown/`asset_usages` 或 OSS object inventory；这两项 reconciliation 必须另行通过，三项共同
构成启用硬门槛。代码合并或测试通过不代表 `MEDIA_RETENTION_GC_ENABLED` 已在任何环境启用。

截图/录屏放 PR，不提交临时 `/tmp` 路径报告。补完整前端单元覆盖和浏览器旅程测试仍是产品 P1
质量工作。

## Migration 验证

- 只新增下一个编号 migration，不编辑 applied 文件。
- 在 fresh dedicated database 运行 `sqlx migrate run --source migrations`。
- 对有数据升级写 fixture，验证 backfill、constraint、index 与并发行为。
- Append-only migration 既测试 row `UPDATE/DELETE`，也执行真实 direct/cascaded `TRUNCATE` 并确认
  statement trigger 拒绝；随后验证正常 append 仍可用。
- 确认应用旧/新版本滚动窗口是否兼容；记录 forward/rollback intent。
- 不使用 production/shared database 跑测试或手工 destructive SQL。

## 按变更范围选择检查

| Scope | 必需检查 |
|---|---|
| Docs/skill only | docs checker（含 repo skill 结构）、official skill validator（如可用）、`git diff --check` |
| Rust logic | backend quick + focused tests；完成前跑 CI-parity integration |
| Migration | backend gates + fresh migration + affected integration tests |
| OpenAPI | backend handler tests + generate types + Web gates + contract diff review |
| Web UI | generate types + Vitest/axe + lint/typecheck/build + desktop/mobile manual QA |
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
| S2 Identity | 注册/onboarding/当前条款、密码/验证码登录、purpose/replay、找回、credential-version recent-auth race、refresh rotation、session revoke、sanction、停用/删除/恢复、deadline-locked irreversible purge、partial owner failure、dead-letter list/requeue/audit、owner export/grant/lease |
| S3 Selection data | fresh migration、Raw→catalogue/selection 物化不变量、重复运行、关键 API/search |
| S4 Reviews | publish idempotency、edit/like/unlike/report/decision、visibility、course-delete restriction |
| S5 Community | board policy、thread/comment create/edit/delete/restore、interaction、activity、follow/privacy、notification、DM；通知 outbox 的同事务 producer、lease/SKIP LOCKED、幂等 receipt、policy 与 source-reversal 竞态、dead-letter/retry；处置通知与 owner-domain appeal reversal |
| S6 Governance | suspended appeal-only access、普通 route scope isolation、仅本人原事件、idempotency conflict、30 天窗口、独立 reviewer/hierarchy、stale claim、原子 overturn/amend/history/notice |
| S7 Credit | signing intent、tamper/replay、ledger chain、balance reconciliation、escrow edge states |
| S8 Search/media | typed federated results、visibility/stale index、reindex；upload intent/callback/scan/binding/delete/GC |
| S9 Final reconciliation | counters、activity、search、unread、jobs、audit 与 source-of-truth 终态一致 |

当前 `crates/e2e` 没有覆盖这张完整矩阵，也未进入 CI；Web 只有最小 component/axe harness，仍没有
浏览器 suite。不要在文档或 PR
声称“E2E 完整”。每个功能 PR 先补受影响旅程，平台后续再把 S1–S9 编排成可重复的隔离环境 gate。

## PR 记录

PR body 写实际命令和结果，例如 `cargo test -p forum --test dm_tests (12 passed)`。不要只勾选
“tests passed”；失败、skip、未运行及原因同样必须记录。CI 与 preview 仍需在 push 后独立确认。

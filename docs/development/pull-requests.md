# 分支、提交与 Pull Request

> 文档类型：开发流程
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

所有变更通过 feature branch 和 PR 进入 main。维护者负责 merge；开发者和 Agent 不直接 commit、
push 或本地 merge 到 main。

## 分支与 worktree

```bash
git fetch origin main
git switch -c <type>/<topic> origin/main
```

Codex 默认使用 `codex/<topic>`。如果当前 checkout dirty，优先新 worktree；不得 `reset --hard`、
force checkout 或丢弃不属于本任务的改动。

分支从最新 `origin/main` 创建。只 stage 本任务作者明确负责的文件；不要顺带提交他人的 lockfile、
artifact、secret、dump 或无关格式化。

## 提交

使用 Conventional Commits：

```text
feat(forum): add user follow relationships
fix(identity): consume verification codes once
docs(product): define community capability model
chore(ci): validate documentation links
```

- 一个 commit 表达一个逻辑变化；不把无关 refactor、依赖升级和功能混在一起。
- 提交前 review `git diff`、`git diff --check`、状态和 secret/PII。
- 不用 commit message 宣称未验证或未实现的能力。

## PR 必填内容

仓库模板要求：

- 需求/Issue 与行为摘要；
- 关键产品语义、状态机、权限和不变量；
- 影响矩阵：product docs、OpenAPI/generated types、schema、auth/PII、credit、media/search、deploy；
- 实际测试命令与结果，包含未运行/skip；
- migration rollout/backfill/compatibility；
- UI desktop/mobile evidence；
- preview URL 与验证旅程；
- 已知限制、rollback 和 reviewer focus。

`Docs impact` 不能为空。更新受影响文档；确实只有内部实现时写 `Docs impact: none` 并解释为何没有
用户行为、contract、schema、安全、运维或开发流程变化。

CI 会比较 PR base/head：存在 canonical 文档变更时禁止声明 none；没有文档变更时必须使用上述
精确声明并给出具体理由。OpenAPI、migration、workflow、Compose 或 `.env.example` 变化强制要求
canonical 文档 diff，不能声明 none。只保留模板 HTML 注释会被视为空内容。

## 开 PR 前

1. Rebase/merge latest main 的方式由团队约定；不要 rewrite 他人公开分支。
2. 跑[测试策略](testing.md)中对应的 CI-parity gates。
3. 生成并提交 OpenAPI types；确认 migration 只追加。
4. 更新 current-state/领域/架构/安全/运维文档。
5. 检查 diff scope、secret、PII、generated artifact 和 commit history。
6. 推送并开 draft 或 ready PR；状态必须符合实际完成度。

Agent 只有在用户明确要求 commit/push/开 PR 时执行这些外部变更。用户只要求实现或分析时，停在
已验证的工作树并报告状态。

## CI 与 Preview

- Push 后等待 backend CI、integration 和 Web job。
- 运行代码或 contract 变化时，PR preview 同时部署前后端到 `/pr-<N>/`。
- Docs-only PR 没有 runtime preview；运行 docs check 即可。
- Preview health 后仍需执行本次用户旅程并检查 console/network。
- Fork PR 和当前超过两位数的 PR number 可能无法部署，PR 中如实标注。

详见[部署与 PR Preview](../operations/deployment-and-previews.md)。

## Review 与合并

- Reviewer 先检查产品/权限/数据边界，再检查代码风格。
- PII、credit、auth、governance、migration、external provider 变更请求领域 owner review。
- 处理 review 后重跑受影响 checks，更新 PR body 和 docs，不只回复“已修”。
- CI/preview 失败时诊断并修复；不因超时或接近完成而宣称 green。
- Merge 后由 main workflow 部署，维护者验证 smoke；未完成项进入 Issue/roadmap，不留 `TODO` 注释。

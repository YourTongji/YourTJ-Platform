# 部署与 PR Preview

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-12，`contract/openapi.yaml` 与 deploy workflows

本文件描述仓库当前 GitHub Actions 行为，不把目标 Aliyun 架构写成已上线生产事实。Workflow
或服务器脚本变化时必须在同一 PR 更新本 runbook。

## 环境

| 环境 | 当前入口 | 用途 |
|---|---|---|
| Local | Web `localhost:5173`、API `localhost:8080` | 开发与测试 |
| PR preview | `http://<preview-host>:8080/pr-<N>/` | 同仓 PR 集成预览 |
| Main staging | `http://<preview-host>:8080/` | main 当前共享测试部署 |
| Target production | Aliyun 无状态容器 + PolarDB/Redis/Meili/OSS | 尚未由仓库 IaC 交付 |

Preview/main host 不是已经完成 SLO、备份、域名和 secret manager 的正式生产声明。

## CI 与部署是两条独立流水线

- `.github/workflows/ci.yml` 对 PR 和 main 运行 backend lint/tests 与 Web generated types/lint/build。
- `.github/workflows/pr-preview.yml` 构建并部署 PR preview。
- `.github/workflows/deploy-main.yml` 在 main 的运行代码路径变化时部署 main staging。

当前 deploy workflow 没有显式依赖 CI。交付人必须同时确认 CI 和 deploy jobs 真实成功，不能只看
GitHub summary 文案或可打开的旧页面。

## PR preview 触发与构建

以下路径变化会触发 preview：

- `web/**`
- `backend/**`
- `contract/openapi.yaml`
- preview workflow 本身

无论只改 Web 还是只改 backend，preview 都同时构建并部署前端和后端，避免前端对着 main backend
或 backend 对着旧前端产生假预览。Docs-only PR 不触发 runtime preview。

Workflow 当前只接受不超过两位数的 PR number，这是已知临时限制，应在达到上限前移除。Fork PR
通常拿不到 deployment secrets，不能承诺自动 preview。

## Preview 流程

1. Checkout PR revision 并验证 PR number。
2. 构建 backend Docker image `yourtj-api:pr-<N>`。
3. 安装 Web 依赖，以 `/pr-<N>/api/v2` 和 `/pr-<N>/` base path 构建静态资源。
4. 通过 SSH 传输 image 和 frontend dist。
5. 服务器 `/opt/yourtj-preview/deploy-pr.sh` 创建/更新对应 preview。
6. 从服务器本机验证 frontend root 与 `/api/v2/health`。

Review 时还要从浏览器实际访问页面、检查 API 请求、console、登录态和本次功能。Health 只证明进程
可响应，不证明 migration、search、email、OSS 或业务旅程正确。

## 数据与 secret 隔离

- Preview server contract 要求每个 PR 使用独立数据库/容器命名空间，不连接生产数据库或对象存储。
  `/opt/yourtj-preview/deploy-pr.sh` 不在本仓库，operator 必须验证实际脚本满足该约束，不能只凭
  workflow 的命名和 cleanup 命令推断隔离已经成立。
- Preview backend 不注入生产 Cloudflare email token；邮件使用 redacted log/test provider。
- 不把 SSH、数据库、邮件、OSS 或 JWT credential 写入 workflow、文档、PR、日志或截图。
- 当前 workflow 中仍有需要迁入 GitHub secrets 的 preview database credential，并且 main SSH
  host-key policy 需要收紧；在完成前把它们视为 P0 deployment hardening。
- Preview 不导入真实 D1/用户/DM 数据；测试资料使用合成 fixture。

## Main staging 部署

`deploy-main.yml` 在 main 的 `web/**`、`backend/**`、contract 或 workflow 变化时构建前后端并通过
SSH 部署。Docs-only 合并不会重新部署。部署后应验证：

```text
GET /                 -> 当前 Web revision
GET /api/v2/health    -> backend health
```

还需执行本次变更的关键 smoke journey。当前 workflow 缺完整 main health gate、自动 rollback 和
release manifest；失败时不要仅依赖 summary 判断“已部署”。

## PostgreSQL migration owner 与 runtime role

正式环境必须使用两个不同角色：migration/table owner 只在受控 rollout 中执行 sqlx migration；应用
runtime DSN 使用非 owner login。不要因为当前 shared test server 可能复用一个账号，就把该做法复制到
PolarDB production。

- Runtime 只获得数据库 `CONNECT`、所需 schema `USAGE`、业务表最小 DML 和 sequence 权限；不授予
  schema/table ownership、`CREATE`、`ALTER`、`DROP`、`TRUNCATE`、replication-role 或 disable-trigger。
- 对 `governance.audit_events`、`governance.appeal_events`，runtime 只需 `SELECT/INSERT`，必须显式
  `REVOKE UPDATE, DELETE, TRUNCATE`。Migration `0055` 还从 `PUBLIC` 撤销这些权限，并用 statement
  trigger 拒绝 direct/cascaded truncate。
- Table owner/superuser 仍能人为 disable trigger，因此 trigger 与 least privilege 是两道独立边界。
  普通部署、cleanup、retention 和测试不得用 owner 连接；灾备 restore 在隔离环境按批准 runbook 执行。
- 上线前核对 runtime 不是表 owner，并用 `has_table_privilege` 验证上述 deny；以 runtime credential
  执行 update/delete/truncate 负向 smoke，以 migration credential 只验证 migration ledger，不在 live
  数据上试 destructive statement。

新增或改变社区搜索文档后，部署完成还需要由具备 `operations.jobs` 的管理员以
`{"reason":"deploy <revision> community search schema"}` 请求体触发
`POST /api/v2/admin/forum/reindex`。该任务会重建 thread、public user、board 和 tag index；返回
`202` 只表示已入当前进程任务，operator 必须检查后端日志并实际搜索已有账号/板块/tag，不能把
queued 当成功。现阶段没有 durable job status/retry，因此多实例发布时只触发一次，并在失败后显式重试。

## 关闭、过期与清理

- 未合并 PR 关闭时 workflow 停止容器、删除 image/frontend 和 preview database。
- 合并 PR 依赖 main deployment 接管，但 repository workflow 未提供完整 orphan/TTL reconciliation。
- Workflow summary 声称 preview 24 小时过期，但仓库没有 schedule 保证这一点；不能依赖该文案。
- 需要补定时清理：枚举 open PR 与服务器资源，dry-run 后移除 orphan，并输出审计/指标。

## 变更与回滚

- Migration 必须 forward-compatible；preview 成功不代表 shared main data 可以安全回滚。
- `0055` 只增加 append-only truncate trigger/privilege deny，无数据 backfill。回退应用时保留 trigger；
  不通过 drop trigger 或清空 audit/appeal history 伪造 schema rollback。
- Web/API breaking change 使用 additive contract、双读/双写或明确 cutover，避免前后端窗口不兼容。
- 当前回滚依赖重新部署已知良好 revision；没有自动 release promotion/rollback。执行前确认 migration
  和外部副作用允许回退。
- Email 与 OSS 分别按[邮件 runbook](email-delivery.md)和[媒体存储 runbook](media-storage.md)处理；
  Meili/Redis 按架构中的事实源/投影语义降级。后两者的独立 incident runbook 仍需补齐，不能临时
  绕过可见性、授权或数据完整性。

## 部署完成条件

- CI 与 deploy jobs 真实通过。
- Frontend 和 backend revision 对应同一 commit。
- Health、关键 API 和本次用户旅程已验证，无新增 console/server error。
- Migration、search index、scheduled/outbox work 的状态可观察。
- Secret/PII 未进入 artifact 或日志，preview 与 main 数据隔离。
- PR 中记录 preview URL、验证步骤、已知限制和回滚方法。

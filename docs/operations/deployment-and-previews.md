# 部署与 PR Preview

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-12，`contract/openapi.yaml`、deploy workflows 与 `ops/deploy/deploy-main.sh`

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
- Preview backend 不读取或注入 main 的 `OSS_*` Repository Secrets，也不连接 main bucket。媒体协议使用
  fake provider/本地测试边界；需要真实 provider E2E 时必须使用独立测试账号、bucket/prefix 和最短生命周期。
- 不把 SSH、数据库、邮件、OSS 或 JWT credential 写入 workflow、文档、PR、日志或截图。
- 当前 workflow 中仍有需要迁入 GitHub secrets 的 preview database credential，并且 main SSH
  host-key policy 需要收紧；在完成前把它们视为 P0 deployment hardening。
- Preview 不导入真实 D1/用户/DM 数据；测试资料使用合成 fixture。

## Main staging 部署

`deploy-main.yml` 在 main 的 `web/**`、`backend/**`、contract、`ops/deploy/**` 或 workflow 变化时构建
前后端并通过 SSH 部署。Docs-only 合并不会重新部署。所有 build/deploy jobs 都要求
`github.ref == refs/heads/main`；从 PR/feature branch 手动 dispatch 会全部跳过，不能把任意分支部署到
main staging。Deploy job 还绑定 GitHub `main-staging` Environment；其 deployment branch policy 只允许
`main`，作为 workflow 内 ref gate 之外的 repository-setting 边界。删除/放宽该 policy 必须按部署配置
变更审查并同步本 runbook。

Main 使用仓库内版本化的 `ops/deploy/deploy-main.sh`，workflow 每次把该 revision 的脚本传到受限临时
路径执行，不依赖 `/opt/yourtj-preview/deploy-main.sh` 这类可漂移副本。服务器仍需维护权限为 `0600`
的 `/opt/yourtj-preview/shared/main-runtime.env` 与 `email-main.env`；前者保存数据库、JWT、积分签名等
main runtime secret，后者保存邮件 provider secret。OSS 六项 Repository Secrets 由 runner 写入本次 run
专用的 `0600` 临时 env 文件，通过 stdin/文件传输而不是 SSH command line 传递，并在退出时删除。

脚本在停止旧容器前执行以下 fail-closed preflight：

- 六项 OSS 配置完整、格式合法，callback base 为无内嵌 credential 的 HTTPS URL；
- configured bucket endpoint 可达，缺失 bucket/错误 region 不继续发布；
- callback `/api/v2/media/callback` 可通过 HTTPS 到达应用且会拒绝未签名请求；
- 使用与后端相同的 exact-key `PutObject` policy 调用一次 STS `AssumeRole`，只核验 temporary credential
  issuance，不上传 object，也不输出 provider response/credential。

前端按 commit SHA 上传到不可变 release directory，避免传输中清空正在服务的目录。部署时旧 frontend/
backend container 先保留为 rollback container；新容器的 direct/public health、revision label 与六项 OSS
runtime env presence 全部通过后才删除旧容器，任一步失败自动恢复旧容器。Migration 仍可能 forward-only，
所以 container rollback 不代表 schema rollback。

部署后应验证：

```text
GET /                 -> 当前 Web revision
GET /api/v2/health    -> backend health
```

还需使用真实 main 测试账号执行一次 upload intent → browser direct upload → OSS callback → pending
状态的关键 smoke journey。自动 preflight 已验证配置、网络、bucket、callback 和 STS，但不能替代
Browser CORS、server-side prevent-overwrite、实际 PutObject/callback body 或 moderation scanner。当前仍
缺 release manifest、跨 migration 自动 rollback 和正式域名外部 probe；失败时不要仅依赖 summary 判断
“已部署”。

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

## Media/lifecycle migrations `0057`–`0058` 的 gated rollout

`0057` 的 profile/promotion/draft trigger 在 backfill 前安装，保护 source snapshot 的引用完整性；它还
允许 deletion job 使用 system actor，并把 upload-intent callback secret 从 plaintext column 切换为
SHA-256 digest-only。旧 API 仍访问 plaintext column，旧 deletion/lifecycle worker 也不理解新 actor/gate/
terminal progress，因此 `0057` 是 application-level breaking cutover，不能新旧进程混跑。Hash backfill
允许新版验证迁移前已经签发的 callback token；maintenance gap 内已写 OSS 但 callback 未落库的 object
仍可能 orphan，必须由 exact-key cleanup 和发布后 reconciliation 收敛。

`0058` 为 account lifecycle job 增加每次 claim 唯一的 UUID lease token。complete/fail/defer/block 都按
token CAS，complete 先锁 job 再锁 account，防止过期 worker 覆盖新 worker 的 Media 等待或人工阻断。
旧 lifecycle binary 已持有的 job 不认识该 token，不能与迁移并行；它与 `0057` 共用同一次 maintenance
cutover，而不是额外 rolling step。

通用 media GC 与账号 purge system enqueue 由 `MEDIA_RETENTION_GC_ENABLED` 共同控制，默认 `false`。
代码合并、migration 成功或 main staging 部署都不等于已经启用。部署必须：

1. 保持该 flag 为 `false`，先停止签发新 upload credential，同时保持旧 callback endpoint 可用。等待至少
   15 分钟 STS/intent TTL + 10 分钟 cleanup safety buffer；或用数据库 active intent、gateway in-flight
   callback 与 provider callback 指标权威确认 outstanding intents/callbacks 为零。
2. 再 drain/停止全部旧 callback/API/writer image 与旧 deletion/lifecycle worker；确认没有进程仍访问
   callback plaintext column、写业务引用或消费队列。
3. 以 migration owner 顺序执行 `0057`、`0058`，再部署全部 binding-aware writer 与 lease-fenced 新版
   worker。确认 lifecycle running row 都带非空 token，迁移回收的旧 running row 只会重新排队。
4. 从仓库根目录运行 DB preflight：

   ```bash
   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/check_media_retention_references.sql
   ```

5. SQL 中 profile/promotion/draft drift、deletion anomaly 和 redaction anomaly 必须为零，但这只是 DB
   preflight。另行对所有 retained thread/comment canonical Markdown 与 `asset_usages` 做 exact reference
   reconciliation，并对 DB exact key 与 OSS object inventory 做双向 reconciliation；两项也必须无未处置
   drift。单个 SQL 通过不得作为启用依据。
6. 三项硬门槛全部通过后才设置 `MEDIA_RETENTION_GC_ENABLED=true` 并重启/滚动新版进程。核对 startup
   log 与 clean candidate、queue/lease/succeeded/dead-letter、账号 purge progress，并抽样确认 live
   reference、future grace 与 operational hold 行为。

新版 moderation deletion worker 和未 callback intent 的 exact-key housekeeping 独立运行，不受 GC flag
影响；已消费 intent credential 30 天、preview grant expiry+1 天、detached binding grace、credential
attempt 48 小时和 synthetic cleanup tombstone 的清理也独立。Operations hold/retry/succeeded-job/redacted-evidence
的 365 天 purge 使用另一个默认关闭的 `MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED`，只有 privacy/legal
owner 批准后才能启用，不能与 GC rollout 捆绑授权。详细回滚和核验见[媒体存储 runbook](media-storage.md)。

## 关闭、过期与清理

- 未合并 PR 关闭时 workflow 停止容器、删除 image/frontend 和 preview database。
- 合并 PR 依赖 main deployment 接管，但 repository workflow 未提供完整 orphan/TTL reconciliation。
- Workflow summary 声称 preview 24 小时过期，但仓库没有 schedule 保证这一点；不能依赖该文案。
- 需要补定时清理：枚举 open PR 与服务器资源，dry-run 后移除 orphan，并输出审计/指标。

## 变更与回滚

- Migration 必须 forward-compatible；preview 成功不代表 shared main data 可以安全回滚。
- `0055` 只增加 append-only truncate trigger/privilege deny，无数据 backfill。回退应用时保留 trigger；
  不通过 drop trigger 或清空 audit/appeal history 伪造 schema rollback。
- `0057` 删除 callback plaintext column 并扩展 media deletion job 以支持 system actor，不能回滚到旧 API。
  运行时回滚先把
  `MEDIA_RETENTION_GC_ENABLED=false`，由新版 deletion worker 排空或保留已存在的 system job。队列仍有
  system actor 时不得恢复旧 worker；trigger/backfill/redacted tombstone 保持 forward-only。
- `0058` 的 lifecycle lease token 与 job/account 锁序同样 forward-only。回滚应用时仍必须使用理解 token
  的 worker；不得恢复按 job id 无条件 complete/fail 的旧 binary。
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
- Main 容器 revision label 与 workflow SHA 一致，六项 OSS runtime env 存在；preflight 的 bucket、HTTPS
  callback 与 STS AssumeRole 均通过，随后完成一次真实上传旅程。
- PR 中记录 preview URL、验证步骤、已知限制和回滚方法。

# 部署与 PR Preview

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-12，deploy workflows、`ops/deploy/` versioned scripts 与 migrations `0060`–`0062`

本文件描述仓库当前 GitHub Actions 行为，不把目标 Aliyun 架构写成已上线生产事实。Workflow
或服务器脚本变化时必须在同一 PR 更新本 runbook。

## 环境

| 环境 | 当前入口 | 用途 |
|---|---|---|
| Local | Web `localhost:5173`、API `localhost:8080` | 开发与测试 |
| PR preview | `http://<preview-host>:8080/pr-<N>/` | 同仓 PR 集成预览；不接真实邮件/OSS/CDN provider |
| Main staging | `https://pf-dev.yourtj.de/` | main 当前共享测试部署 |
| Target production | Aliyun 无状态容器 + PolarDB/Redis/Meili/OSS/CDN | 尚未由仓库 IaC 交付 |

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
- `ops/deploy/deploy-pr.sh`、`cleanup-pr.sh`、`frontend-nginx.conf.template`
- preview workflow 本身

无论只改 Web 还是只改 backend，preview 都同时构建并部署前端和后端，避免前端对着 main backend
或 backend 对着旧前端产生假预览。Docs-only PR 不触发 runtime preview。

Workflow 和服务器 proxy 只接受 `1`–`999` 的 PR number。端口是固定的零填充映射：PR 1 使用
frontend/backend `15001/16001`，PR 24 使用 `15024/16024`，PR 100 使用 `15100/16100`；不要另建一套
手工端口约定。Fork PR 通常拿不到 deployment secrets，不能承诺自动 preview。

## Preview 流程

1. Checkout PR revision 并验证 PR number 为 `1`–`999`。
2. 构建 backend Docker image `yourtj-api:pr-<N>`。
3. 安装 Web 依赖，以 `/pr-<N>/api/v2` 和 `/pr-<N>/` base path 构建静态资源。
4. 通过 SSH 传输 image，并把 frontend dist 写到
   `/opt/yourtj-preview/pr/<N>/releases/<sha>-<run-id>-<attempt>/frontend` 不可变 release；传输或部署失败
   删除本次未采用目录，不覆盖当前容器的 bind mount。
5. Workflow 把当前 revision 的 `ops/deploy/deploy-pr.sh` 和 `frontend-nginx.conf.template` 传到临时
   受限路径并执行；不调用服务器上可漂移的脚本副本。
6. Backend container 先由 `docker create` 建立，再在 `docker start` 前记录 fail-forward 边界；一旦 start
   被尝试就不能可靠证明 migration 从未执行。后续失败
   绝不重启旧 backend。新 backend 未 ready 时停止但保留该 container 供诊断；已经 ready 后发生 frontend/
   public probe 失败则让它继续运行。Frontend 是独立的静态发布单元，失败时恢复旧 frontend；修复 backend
   只能提交并部署新 revision。Workflow 最后再验证 public health/readiness。

Preview backend 显式注入 `BIND_ADDRESS=127.0.0.1`，frontend Docker publish 使用
`127.0.0.1:<15xxx>:80`；脚本读取 container env/port mapping 后 fail closed。PR backend/frontend port
只应由同 host Nginx 访问，不能绑定 `0.0.0.0`。

Preview operator 必须预先创建 regular、non-symlink、mode-`0600` 的 `$HOME/.pgpass`。当前默认连接
匹配行形如 `127.0.0.1:5433:*:yourtj_preview:<password>`；真实 password 由受限 secret channel 写入，
不要把完整行放进 shell history、Actions log 或工单。设置后用相同 `PGPASSFILE`、host、port、user 对
`postgres` 数据库执行只读 `SELECT 1` 验证，再允许 workflow 部署。Host/port/user 任一改变时必须同步
`PREVIEW_POSTGRES_*` 环境、cleanup 和本 runbook，不能把 password 重新塞回 DSN 兼容。

Review 时还要从浏览器实际访问页面、检查 API 请求、console、登录态和本次功能。Health 只证明进程
可响应，不证明 migration、search、email、OSS 或业务旅程正确。

## 数据与 secret 隔离

- Preview 由仓库内 versioned script 创建独立 `yourtj_pr_<N>` PostgreSQL database、frontend/backend
  container、随机且持久的 mode-`0600` backend secret file。数据库密码只从 preview operator 的
  mode-`0600`、非 symlink `$HOME/.pgpass` 由 libpq 读取；DSN、脚本和 container environment 不包含密码。
- Redis/Meilisearch 是共享测试服务，preview 不是处理真实 PII、私信或生产数据的安全边界；fixture
  必须是合成数据。每个 preview 的 PostgreSQL/container 隔离不能被扩写为所有依赖完全独占。
- Preview backend 不注入生产 Cloudflare email token；邮件使用 redacted log/test provider。
- Preview backend 不读取或注入 main 的 Ingest、Delivery、CDN signing 或 purge secrets，也不连接 main
  bucket/CDN。媒体 runtime 整组未配置是合法的 provider-free preview 状态；上传/处理/CDN 旅程不在该
  环境宣称可用，协议由 fake provider/integration tests 验证。需要真实 provider E2E 时必须使用独立
  preview 账号、双 bucket、domain、callback 和最短生命周期。
- 不把 SSH、数据库、邮件、OSS 或 JWT credential 写入 workflow、文档、PR、日志或截图。
- 任何曾进入聊天、Issue、终端回显、Actions log 或截图的 bearer/password/key 一律按已泄露处理：先创建
  最小权限替代值并验证，再撤销旧值、检查访问记录；release sign-off 只记录“旧值已 disabled、新值仅在
  secret store 可用”，不得复述 credential。
- SSH 当前仍使用 `StrictHostKeyChecking accept-new`；应迁移为预置、轮换受控的 host key pin。不要以
  降级为关闭 host-key 检查的方式解决连接故障。
- Preview 不导入真实 D1/用户/DM 数据；测试资料使用合成 fixture。

## Main staging 部署

`deploy-main.yml` 在 main 的 `web/**`、`backend/**`、contract、`ops/deploy/**` 或 workflow 变化时构建
前后端并通过 SSH 部署。Docs-only 合并不会重新部署。所有 build/deploy jobs 都要求
`github.ref == refs/heads/main`；从 PR/feature branch 手动 dispatch 会全部跳过，不能把任意分支部署到
main staging。Deploy job 还绑定 GitHub `main-staging` Environment；其 deployment branch policy 只允许
`main`，作为 workflow 内 ref gate 之外的 repository-setting 边界。删除/放宽该 policy 必须按部署配置
变更审查并同步本 runbook。

Main 使用仓库内版本化的 `ops/deploy/deploy-main.sh`、OSS verifier、frontend Nginx template 和 host
`preview-proxy.conf`。Workflow 每次把当前 revision 传到受限临时路径，不依赖
`/opt/yourtj-preview/deploy-main.sh` 一类可漂移副本。Host proxy 更新先备份现有配置，经 `nginx -t`
成功后 reload；校验失败恢复备份并终止发布。

服务器仍需维护权限为 `0600` 的 `/opt/yourtj-preview/shared/main-runtime.env` 与 `email-main.env`；前者
保存数据库、JWT、积分签名以及邮箱加密版本/AEAD/blind-index key，并必须设置
`EMAIL_ENCRYPTION_STRICT=true`；后者保存邮件 provider secret。Main preflight 拒绝缺失、非 32-byte hex、
重复的邮箱加密 key 或 strict=false，应用启动后还会 backfill 并确认不存在明文邮箱。Ingest、Delivery、
CDN signing/purge 配置由 runner 写入本次 run 专用的 `0600` 临时 env file，通过 stdin/文件传输而不是
SSH command line 传递，并在退出时删除。

脚本在停止旧容器前执行以下 fail-closed preflight：

- Ingest/Delivery/CDN 全部必需配置完整且无未知 key；两个 private bucket、三个 provider principal
  和 CDN 双签名 key 必须满足隔离约束，URL TTL 固定为 300 秒；
- 两个 configured bucket endpoint 可达且匿名 HEAD 必须精确返回 403；返回 200 会按 public bucket
  fail closed，缺失 bucket/错误 Region 也不继续发布；
- callback `/api/v2/media/callback` 可通过 HTTPS 到达应用且会拒绝未签名请求；
- 使用与后端相同的 exact-key `PutObject` policy 调用一次 STS `AssumeRole`，以 temporary credential 首次
  写入随机 `uploads/deploy-smoke/` key，再省略 `x-oss-forbid-overwrite` 尝试覆盖；只有 Ingest provider 的
  path-level prevent-overwrite rule 返回 HTTP 409 `FileAlreadyExists` 才通过，最后用 Ingest caller 删除对象。
  这项检查不输出 provider response/credential/object locator；
- Delivery writer 以带 `x-oss-forbid-overwrite:true` 的 PUT 写入一个固定微型 WebP 到随机
  `assets/deploy-smoke/<uuid>.webp`，再以 signed HEAD/GET 核对长度、摘要 metadata、MIME 和完整 body；
- 对这个确认存在的同一路径，unsigned CDN GET 必须精确为 403（404 也失败），当前 slot 的 Type-A URL
  必须为 200 且 body 与源 fixture 完全一致；
- 独立 CDN purge principal 以 POST 提交 `RefreshObjectCaches`，轮询
  `DescribeRefreshTaskById` 直到全部 `Complete`，总等待上限十分钟；无论中途成功或失败都尽力按
  purge → DELETE 清理合成对象，清理失败也会让 preflight fail closed；
- frontend CSP template 只含一个 CDN-origin placeholder，渲染后以 `nginx:alpine nginx -t` 验证。
- backend container 必须精确注入 `BIND_ADDRESS=127.0.0.1`，frontend publish 必须精确为
  `127.0.0.1:15000`；脚本在 public probe 前检查二者，防止 Docker 默认暴露到所有 interface。

这段合成 preflight 使用 Python 标准库实现与 Rust runtime 相同的 OSS V4 Authorization header：
HMAC-SHA256、date/Region/`oss` scope、canonical URI/空 query/headers、
`x-oss-content-sha256:UNSIGNED-PAYLOAD` 和分层 signing key。OSS V1 自 2025-09-01 起不再对新增 bucket
开放，部署和 runtime 都不得回退。脚本使用短单请求 timeout、有界 response/body 和禁止 redirect 的
opener；Actions 输出不包含 provider body、Authorization、signed URL、purge task ID 或对象 locator。它
证明 main 配置的 Delivery 正向写读、CDN 认证和 purge 链路，不替代 Ingest/Delivery 越权负向、Browser
CORS、callback 或真实 processing journey。

前端按 commit SHA 上传到不可变 release directory，避免传输中清空正在服务的目录。Frontend 失败独立恢复
旧静态 release。Backend 只有在新 container 尚未创建且 start 未尝试时才可恢复旧 container；脚本在
`docker create` 成功后、`docker start` 之前先记录 fail-forward 状态，因此 start 命令本身失败也不会误启
旧 revision。一旦新 backend start 被尝试，任何
失败都按 fail-forward 处理：未 ready 则停止并保留新 container 诊断，已 ready 则保留运行，旧 backend
backup 保持停止。Revision label inspection 本身也必须成功；Docker template 解析失败、label 缺失或与
部署 revision 不一致都会使发布失败，但不能据此越过 migration 边界重启旧 backend。

部署后应验证：

```text
GET /                 -> 当前 Web revision
GET /api/v2/health    -> backend health
GET /api/v2/ready     -> PostgreSQL 可达且 migration ledger 到达当前 binary 期望版本
```

Workflow 随后从 GitHub runner 对 `MAIN_PUBLIC_BASE_URL` 的 exact HTTPS origin 验证 Web、health 与
readiness，避免只有服务器 loopback 正常却公网 DNS/TLS/proxy 失效。还需使用真实 main 测试账号执行
upload intent → browser direct upload → callback → owner preview → approve → processing → published CDN
URL 的关键 smoke journey。自动 preflight 已覆盖合成 Delivery Put/Head/Get、当前 Type-A auth 与 purge，
但不能替代 Browser CORS、Ingest callback body、runtime V4、实际变体处理、篡改/过期 URL 或跨权限负向
测试。当前仍缺 release manifest；backend 跨 migration 失败必须 forward-fix，不能依赖自动 rollback 或
summary 文案判断“已部署”。

### GitHub `main-staging` Environment

部署 job 从 `main-staging` Environment 读取下列配置；名字必须与 workflow 完全一致。真实值只写入
GitHub Environment 或服务器受限 env file，不写进仓库、Issue、PR 或 Actions 输出。

| 类型 | 名称 |
|---|---|
| Secrets：Ingest | `OSS_REGION`、`OSS_BUCKET`、`OSS_ACCESS_KEY_ID`、`OSS_ACCESS_KEY_SECRET`、`OSS_ROLE_ARN`、`OSS_CALLBACK_BASE_URL` |
| Secrets：Delivery | `MEDIA_DELIVERY_OSS_BUCKET`、`MEDIA_DELIVERY_OSS_ACCESS_KEY_ID`、`MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET` |
| Secrets：CDN | `MEDIA_CDN_PRIMARY_KEY`、`MEDIA_CDN_SECONDARY_KEY`、`CDN_ACCESS_KEY_ID`、`CDN_ACCESS_KEY_SECRET` |
| Variables | `MEDIA_CDN_BASE_URL`、`MEDIA_CDN_SIGNING_KEY_SLOT`、`MAIN_PUBLIC_BASE_URL` |

当前 workflow 把 Region、bucket、role ARN 和 callback base 也作为 Environment secrets 读取，这是部署
契约而不是这些值天然都属于 credential。`MEDIA_CDN_URL_TTL_SECONDS` 由 workflow 固定注入 `300`，不要
另建一个可漂移的 GitHub value。`MEDIA_CDN_BASE_URL` 与 `MAIN_PUBLIC_BASE_URL` 都必须是无 path/query/
credential 的 exact HTTPS origin；signing slot 只能为 `primary` 或 `secondary`。

部署用的 `PREVIEW_SSH_KEY`、`PREVIEW_HOST`、`PREVIEW_PORT`、`PREVIEW_USER` 继续由 GitHub secret 提供；
`main-staging` 的 deployment branch policy 只允许 `main`。Provider secret 不应改成 Repository variable，
也不得让 PR workflow 引用该 Environment。

### Cloudflare 真实客户端 IP 边界

`ops/deploy/preview-proxy.conf` 的 trusted proxy allowlist 最近于 2026-07-12 对照 Cloudflare 官方
[IPv4 ranges](https://www.cloudflare.com/ips-v4/) 和
[IPv6 ranges](https://www.cloudflare.com/ips-v6/) 核验。Nginx 只对这些 source CIDR 信任
`CF-Connecting-IP`，然后把恢复后的 `$remote_addr` 同时覆盖写入上游 `X-Forwarded-For` 与
`X-Real-IP`。它不会把客户端自带的 XFF chain 继续传给 backend，因此直接访问 origin 并伪造 header
不能改变应用看到的 IP。Cloudflare 对恢复源 IP 的说明见
[官方 Nginx 指南](https://developers.cloudflare.com/support/troubleshooting/restoring-visitor-ips/restoring-original-visitor-ips/#nginx)。

这项边界服务于 IP rate limit、滥用调查和安全日志，不把 IP 变成账号身份，也不替代账号/设备限流。
Origin 当前还承载直接访问的 PR preview；trusted-real-IP 配置不会自动把非 Cloudflare 流量挡在 firewall
外，只保证它不能冒充另一客户端。如生产决定只允许 Cloudflare 回源，应另建 firewall/security-group
规则并保留健康检查/运维来源，不能靠 `real_ip` 模块假装已阻断。

Cloudflare 发布网段变更或至少每季度执行一次更新：

1. 通过受信网络分别读取上述官方纯文本 IPv4/IPv6 endpoint；TLS/HTTP 失败时停止，不沿用第三方列表。
2. 把规范化 CIDR 集合与 versioned `set_real_ip_from` 全量比较；任何新增、删除或非法 CIDR 都由两人
   review，PR 记录获取时间和 diff，不在 workflow 运行时远程下载后直接改生产配置。
3. 只修改 `ops/deploy/preview-proxy.conf`，保持 `real_ip_header CF-Connecting-IP`、`real_ip_recursive on`
   和 upstream header 覆盖不变；绝不加入 `0.0.0.0/0`、`::/0` 或信任任意 XFF。
4. 运行 static workflow tests，并在隔离 Nginx/container 执行 `nginx -t`；合并后 main deploy 先备份 host
   config、安装 versioned file、再次 `nginx -t`，成功才 reload，失败恢复备份。
5. 从 Cloudflare 域名发起合成请求，确认受控日志/rate-limit 看到测试客户端 IP；再直接访问 origin 并
   携带伪造 `CF-Connecting-IP`/XFF，确认后端仍看到实际 source。证据只记录 hash/判定，不把测试 IP
   长期写入 PR 或公开日志。

若 Cloudflare `Pseudo IPv4` 设为 `Overwrite Headers`，`CF-Connecting-IP` 可能变成 pseudo IPv4；当前
配置不读取 `CF-Connecting-IPv6`。改变该 dashboard 设置或改用 IPv6 header 前必须同步 rate-limit/privacy
语义、测试与本 runbook。

### Merge blocker：应用直连端口绑定所有 interface

2026-07-12 在 shared staging 通过服务器 `ss` 与 Docker mapping 核验：旧 main/PR frontend `15xxx` 和
backend `16xxx` 绑定 `0.0.0.0`，host iptables `INPUT` policy 为 ACCEPT；是否另被当前 cloud NSG 阻断
不能作为应用安全边界。PostgreSQL `5433`、Redis `6380`、Meilisearch `7701` 实际已绑定
`127.0.0.1`，此前从本机发起的 TCP 探针不代表公网可达，不应误报为数据服务暴露。

本 revision 将它启动的 backend 注入 `BIND_ADDRESS=127.0.0.1`，frontend Docker publish 也精确限制到
loopback，并在脚本内读取 env/mapping fail closed。Main rollout 在 provider preflight 前和发布完成前
枚举所有运行中的 `pr-<N>-fe/be`：任一 frontend mapping 或 backend bind/port 不是零填充规则对应的
loopback 值，就停止该 PR 的前后端容器并保留其数据库/image 供安全重部署，不把不安全 preview 自动恢复。
Workflow 随后从外部并发负向探测完整保留区间 `15000`–`16999` 以及数据服务端口，任一可达都使发布失败。
该修复及 live 负向复测是 release gate。Cloud NSG/host firewall 仍应作为 defense-in-depth，不能依赖未知
或未记录的默认规则。

Operator 按以下顺序处置：

1. Review 当前 cloud NSG/security group 与 host firewall，拒绝公网到 main/PR `15xxx/16xxx`；只保留
   产品明确公开的 edge/preview/受限 SSH 端口。内部服务如未来确需跨机访问，只允许精确 private
   CIDR/source security group。
2. 保留并复核 PostgreSQL/Redis/Meilisearch 的 loopback bind；不得为了排查连接而改回 wildcard。
   PostgreSQL `pg_hba.conf`、Redis protected mode/auth、Meilisearch master key 继续作为独立
   defense-in-depth，但本次无需把已经 loopback 的服务误改或重建。
3. 部署本 revision 后在服务器检查应用和数据服务：

   ```bash
   docker port main-fe 80/tcp
   docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' main-be | grep -Fx 'BIND_ADDRESS=127.0.0.1'
   sudo ss -lntp
   ```

   Frontend 必须显示 `127.0.0.1:15000`，backend listener 必须是 `127.0.0.1:16000`；每个运行中 PR 的
   `15xxx/16xxx` 也只能为 loopback。`5433/6380/7701` 继续精确显示 `127.0.0.1`，不能出现 wildcard。
4. 在不位于该 host/VPC 的外部主机设置 `PUBLIC_HOST` 后，对完整 reserved app range 执行无 credential
   TCP 负向探针；repository main workflow 已使用有界线程池覆盖 `15000`–`16999`，手工复核不得只抽查
   main 两个端口。任一被自动停止的 PR 必须从包含新版 `deploy-pr.sh` 的 revision 重新部署后再恢复预览。

   ```bash
   python3 - "$PUBLIC_HOST" <<'PY'
   from concurrent.futures import ThreadPoolExecutor
   import socket
   import sys

   host = sys.argv[1]
   ports = range(15000, 17000)

   def reachable(port):
       try:
           with socket.create_connection((host, port), timeout=0.75):
               return True
       except OSError:
           return False

   with ThreadPoolExecutor(max_workers=128) as executor:
       exposed = [port for port, is_open in zip(ports, executor.map(reachable, ports)) if is_open]
   if exposed:
       raise SystemExit(f"FAIL: direct app ports remain public: {exposed}")
   print("OK: direct app ports are not reachable from the public Internet")
   PY
   ```

5. 再验证 `https://pf-dev.yourtj.de/`、health、readiness、登录/Redis 限流和搜索仍正常；PR 的 public
   `:8080/pr-<N>/` 应由 Nginx 到达，但对应 `15xxx/16xxx` 直连失败。在 PR/变更单记录时间、来源网络和
   pass/fail，不记录 credential 或服务响应正文。

若任一 app 直连端口仍可达，停止完成 deployment；不要用应用层 401/页面可用或“可能有 NSG”把 TCP
暴露判成通过。仓库脚本只能约束它启动的新 container，旧/手工 container 和 cloud firewall 仍需 operator
核验。

### Migrations `0060`–`0062` rollout 摘要

- `0060` 增加不可逆单日签到事实、每日 score、账号 score projection 与 durable trust evaluator。
  它不需要外部 provider，但上线后必须核对签到幂等、策略重投影与 evaluator queue/lease。
- `0063` 将 trust evaluator 收敛为 50-account batch、持久 cursor、token-fenced lease 续租、退避与
  8 次 dead-letter，并增加逐账号 failure inventory。发布后检查当日 run 最终为 `completed`；`failed/dead`
  时先按 failure account 修复 projection，再由下一次 due retry 接管，不能直接改等级或删除审计事件。
- `0061` 是 private Ingest → sanitized private Delivery → signed CDN 的 forward-only cutover，并扩展
  processing/cleanup job。旧单对象 worker 不能与新版长期混跑；按[媒体存储 runbook](media-storage.md)
  的维护窗口、双 bucket/RAM/CDN 配置、backfill、真实 smoke 和 reconciliation 执行。
- `0062` 增加 password security event 与 durable identity email-delivery job。Migration 成功不等于邮件
  已投递；新版进程必须运行 worker，并按[邮件发送 runbook](email-delivery.md)观察 retry/dead-letter。

三项 migration 都必须先在 disposable fresh database 完成全量 up-path，再进入 staging。新 backend
container 启动后可能已执行 migration，因此 deploy 不再恢复旧 backend；失败时保留现场并部署理解新
schema/状态的修复 revision。Frontend 可独立恢复，不改变 backend 的 forward-only 边界。

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

- 未合并 PR 关闭时 workflow 把当前 revision 的 `ops/deploy/cleanup-pr.sh` 通过 stdin 执行，停止容器、
  删除 image/frontend 和 preview database；数据库删除同样只从 operator 的 mode-`0600` `.pgpass` 取密钥。
- 合并 PR 依赖 main deployment 接管，但 repository workflow 未提供完整 orphan/TTL reconciliation。
- Preview 没有定时 TTL；workflow summary 明确说明它会保留到 PR close cleanup。若 cleanup 失败，按本节
  的 inventory/手工清理流程处置，不能假设 24 小时后自动回收。
- 需要补定时清理：枚举 open PR 与服务器资源，dry-run 后移除 orphan，并输出审计/指标。

## 变更与 fail-forward 恢复

- Migration 必须 forward-compatible；preview 成功不代表 shared main data 可以安全回退。新 backend
  container 一旦启动，main/PR 脚本都不会恢复旧 backend；frontend rollback 不授权 backend rollback。
- `0055` 只增加 append-only truncate trigger/privilege deny，无数据 backfill。修复应用时保留 trigger；
  不通过 drop trigger 或清空 audit/appeal history 伪造 schema rollback。
- `0057` 删除 callback plaintext column 并扩展 media deletion job 以支持 system actor，不能回滚到旧 API。
  故障恢复先把
  `MEDIA_RETENTION_GC_ENABLED=false`，由新版 deletion worker 排空或保留已存在的 system job。队列仍有
  system actor 时不得恢复旧 worker；trigger/backfill/redacted tombstone 保持 forward-only。
- `0058` 的 lifecycle lease token 与 job/account 锁序同样 forward-only。修复应用仍必须使用理解 token
  的 worker；不得恢复按 job id 无条件 complete/fail 的旧 binary。
- `0061` 的 publication completeness、variant processing 与 ordered cleanup step 同样 forward-only。
  失败时停止新 approve/processing/GC，保留 publication/variant/job 数据，用理解 multi-step cleanup 的
  新 revision forward-fix；不得恢复 direct OSS URL、旧 backend 或让旧 worker 把仅删除 Ingest 当成完成。
- `0062` 故障时可停止新 identity email enqueue/worker，但保留 security event、job 与 credential version
  事实；不能通过清表恢复旧密码或伪造通知已投递。
- Web/API breaking change 使用 additive contract、双读/双写或明确 cutover，避免前后端窗口不兼容。
- 当前只有 frontend 的自动恢复；backend 没有自动 release promotion/rollback。失败后依据保留的新
  container 日志和 migration ledger 构建 forward-fix，不能启动已停止的旧 backup 来缩短故障窗口。
- Email 与 OSS 分别按[邮件 runbook](email-delivery.md)和[媒体存储 runbook](media-storage.md)处理；
  Meili/Redis 按架构中的事实源/投影语义降级。后两者的独立 incident runbook 仍需补齐，不能临时
  绕过可见性、授权或数据完整性。

## 部署完成条件

- CI 与 deploy jobs 真实通过。
- Frontend 和 backend revision 对应同一 commit。
- Health、关键 API 和本次用户旅程已验证，无新增 console/server error。
- Migration、search index、scheduled/outbox work 的状态可观察。
- Secret/PII 未进入 artifact 或日志，preview 与 main 数据隔离。
- Main 容器 revision label 与 workflow SHA 一致，完整 Ingest/Delivery/CDN runtime env 存在；preflight 的
  双 bucket、HTTPS callback、STS、Delivery 合成 Put/Head/Get、同路径 unsigned 403、当前 Type-A body、
  purge Complete、DELETE cleanup 与 frontend Nginx 均通过，随后完成一次真实
  upload/process/signed-delivery/block-cleanup 旅程。
- GitHub runner 对 canonical `MAIN_PUBLIC_BASE_URL` 的 Web、health、readiness 外部探针通过。
- Host proxy 的 Cloudflare IPv4/IPv6 allowlist 与官方列表一致；Cloudflare 路径恢复真实 IP，direct-origin
  伪造 `CF-Connecting-IP`/XFF 的负向 smoke 不改变 backend source IP。
- External TCP probe 证明 main 和全部运行中 PR 的 `15xxx/16xxx` 直连端口不可公网访问；server-side
  app listener/port mapping 为 loopback。`ss` 证明 DB/Redis/Meili 保持 `127.0.0.1` bind，cloud firewall
  通过 operator review。
- PR preview 明确保持 provider-free；未把“上传不可用”误报成媒体 provider 已验收。
- PR 中记录 preview URL、验证步骤、已知限制和回滚方法。

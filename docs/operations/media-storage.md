# OSS 媒体存储

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Media maintainers、Platform maintainers、Security owner
>
> 最近核验：2026-07-12，migration `0057`、Media/Forum/Platform tests 与部署配置

本 runbook 描述当前 Alibaba Cloud STS/OSS 代码边界和上线前配置要求。代码支持不代表 main/production
已经配置；当前部署的 bucket、RAM、CORS、CDN 与 scanner 状态必须由 operator 独立核验。

媒体引用、删除与保留链路的实现状态是 `Partial`：migration、owner-domain 写路径、管理 API/UI 和
自动化测试已经存在，但通用 retention GC 与账号 purge 的 system enqueue 由默认关闭的 rollout flag
保护。合并或部署代码本身不代表 GC 已启用，也不能据此声称某个环境的 object 已开始自动清理。

## 当前实现

- Authenticated client 请求一次 upload intent，服务端生成 account/kind/UUID-scoped exact object key。
- STS credential 约 15 分钟过期，RAM policy 只允许该 exact key 的 `oss:PutObject`。当前 20 MiB
  由 Web 预检、intent/数据库 reservation 和 callback metadata 校验共同执行；OSS PutObject 的 RAM
  condition keys 不支持 Content-Length，因此 provider 在接收 object 前强制限制大小仍为 `Partial`。
  绕过 Web 的恶意客户端仍只能写该 exact key，超限 callback 会拒绝，未消费 intent 的 exact-key
  housekeeping 负责后续删除。要获得 provider-side hard cap，需改为支持 `content-length-range` 的
  PostObject policy；该协议迁移是 `Planned`，不能把当前链路描述成已由 STS 强制 20 MiB。
- Upload credential issuance 在 account row lock 下由 PostgreSQL fail closed：每账号最多 10 个 active
  intent、rolling 24 小时 100 次 issuance attempt、stored + reserved 512 MiB、live object + active intent
  500 条、全部 retained upload record + active intent 2,000 条。Redis 不可用不会放宽这些界限；每次
  attempt 事实保留 48 小时，以覆盖完整 rolling window 和故障复核。
- Web SDK 每次 PutObject 都发送 `x-oss-forbid-overwrite: true`；生产 bucket 还必须对 `uploads/` prefix 和
  upload RAM role 配置 server-side prevent-overwrite rule。客户端 header 只是纵深防御，不能替代 bucket
  规则；否则恶意客户端可在 callback/preview 后、STS 到期前覆盖同一 key，审核 evidence 不可信。
- 当前允许 JPEG、PNG、GIF、WebP 和 PDF；SVG、视频和其他文件拒绝。
- OSS callback public-key URL 只允许官方 host、禁 redirect、有 5 秒 timeout 和 16 KiB key document limit。
- Callback 锁定 intent，核对 key/MIME/bytes/SHA-256 shape，原子创建 `pending` upload 并消费 intent。
  Callback bearer token 只在签发响应/OSS callback 中以明文短暂流转；数据库仅保存 SHA-256 digest，
  constant-time 验证 presented token。Migration `0057` backfill 既有 digest 后删除 plaintext column。
- Authenticated URL endpoint 对 clean asset 允许任意登录用户，pending 只允许 owner，quarantined/blocked 不返回；staff
  pending evidence 必须走下述一次性 audit proxy，不能回退到该 direct URL。当前 owner/clean URL 仍是
  direct OSS URL，不是 private signed/CDN URL。
- Staff approve 仅允许 `pending image -> clean`，并要求同一审核员已经完成一次可信同源图片预览；PDF/file
  在 malware scanner 与 sandbox renderer 证据接入前 fail closed。Block 支持 pending 和已发布 clean asset：
  先在短事务中提交 `quarantined` 与 durable deletion job，使所有公开派生立即停止，再在数据库事务外删除
  OSS object；成功后原子提交 `blocked` 和 audit，失败则保持不可公开的 quarantined、指数退避重试，8 次后
  进入 dead letter 并允许授权审核员人工重新排队。
- Platform promotion 保存可空 `asset_id`，创建或替换时只接受当前管理员拥有的 clean image；公开卡片
  不保存或接受任意图片 URL。登录用户仍通过 media URL authorization 读取，匿名素材交付尚未开放。
- Avatar/banner binding endpoint 只接受当前账号拥有、`kind=image`、`status=clean` 的 asset id；验证与
  Identity reference update 在同一事务，pending/blocked/他人 asset 返回 not found。解绑不会接受 URL。
- Profile 上传 intent/asset 保存可选 intended usage；owner-only recent list/单项 status API 只返回最小
  moderation metadata，不返回 object key、hash、account id 或 object URL。Web 在存在 pending 项时有界
  轮询，只有 clean 项出现绑定操作，blocked 项提示重新上传。
- Profile disclosure 每次只为仍为 clean 的 asset 派生 URL；后续被 block 的 object 即使旧 reference
  尚未清理，也不会继续出现在 profile DTO。
- Forum thread/comment upload intent 使用独立 `forum_thread/forum_comment` usage；Web 持久恢复审核状态并
  插入 `yourtj-asset:<id>`，但 pending/blocked 只能留在 owner draft，不能建立公开 binding。
- Media-owned `asset_usages` 在 Forum create/edit 事务中重验 owner、image、usage、clean 和正文有序集合；
  soft delete 只 detach usage 并设置 30 天 GC grace，restore 重新解析正文并 rebind。公开投影只返回派生
  URL、alt、position 和可信时的尺寸，不返回 object key、hash 或 owner metadata。
- Profile avatar/banner 与 Platform promotion 使用 media-owned 单槽 `asset_bindings`；Forum 云草稿使用
  `draft_asset_references`，已发布内容继续使用 version-aware `asset_usages`。Migration `0057` 先安装
  profile/promotion/draft source trigger，再读取 backfill snapshot，使旧 writer 在 cutover 前提交的引用
  也进入 Media 事实表；新 writer 仍显式同步，不依赖 trigger 代替 owner-domain 校验。
- 归档/隐藏不等于删除，仍保留 active usage 以支持长期恢复；作者、staff、举报 uphold 的软删除都会在
  同一事务 detach，staff restore 和申诉 overturn 都从 canonical Markdown 重新验证 clean owner asset
  后 rebind。恢复验证失败会整体回滚，不能出现正文已恢复但图片进入 GC 的半完成状态。
- Comment 申诉 overturn 先锁 parent thread、再锁 comment，并在锁后读取 parent visibility。并发 parent
  hide/delete 先提交时，comment/media 可恢复为 retained 状态，但 activity/vote 不会被旧 parent
  snapshot 重新激活；该锁序与普通作者/管理 comment mutation 一致，避免 thread↔comment deadlock。
- Admin queue 同样不返回 object key、hash 或持久 URL，并可按 pending/clean/quarantined/blocked 浏览。
  服务端在分页前执行严格层级过滤：mod 只能审核 user，admin 可审核 user/mod，任何人不能自审或审核同级。
  持有 `moderation.content` 的合格独立审核员
  必须填写读取原因，先取得 60 秒、仅当前账号可用、一次性 token，再以 header 交给同源 preview
  endpoint；服务端重验 pending/image/MIME/声明字节数，在流出首个 byte 前解析 JPEG/PNG/GIF/WebP
  header（header scan 最多 1 MiB），限制单边 20,000 px、总像素 40 MP，并以 20 MiB hard limit 继续
  流式代理；可信 dimensions 回写 upload metadata。响应返回 `no-store`、`nosniff`、same-origin/CSP
  headers。token 只存 SHA-256；一次性 claim 在短事务提交后才进行 provider I/O，失败会消耗 token 并要求
  重新申请，避免在外部读取期间持有数据库锁。成功读取后，可信 dimensions、media-owned evidence 与
  `media.upload.previewed` audit 在第二个短事务原子提交；audit 只记 upload id、固定 purpose、reason、
  MIME/声明字节数，不记 token、provider URL 或 key。发放 grant、成功 finalize、approve、block/requeue
  每次都重新读取 uploader 当前 role，避免 grant 发放后晋升造成越权。
- 当前同源 evidence proxy 只开放 allowlisted raster image；PDF/file 不回退到 vendor URL，管理 UI 明确
  显示“文件预览未开放”。PDF 要等独立的 scanner/sandbox renderer 后再开放人工内容预览。
- 通用 GC scheduler 只选择 `status=clean` 且 `cleaned_at` 已满 30 天、没有 live binding/usage/draft
  reference、没有未结束 grace、没有 active operational hold 的 asset；候选锁定后再次校验再隔离。
  `pending` upload 永不因年龄进入通用 GC。未 callback 的 exact object key 由 upload-intent housekeeping
  处理；账号 purge 走单独、受 rollout gate 保护的 system enqueue。

管理 UI 必须如实说明 block 会先隔离再永久删除 object、失败会重试且 dead letter 可人工重新排队，不能把
`202 Accepted` 误写成已经完成 provider 删除。

## Runtime 配置

| Variable | 用途 |
|---|---|
| `OSS_REGION` | Bucket 通用 Region ID，例如 `cn-shanghai`；不含 `oss-` 前缀 |
| `OSS_BUCKET` | Object bucket name |
| `OSS_ACCESS_KEY_ID` / `OSS_ACCESS_KEY_SECRET` | Backend AssumeRole/delete credential |
| `OSS_ROLE_ARN` | Upload-only RAM role |
| `OSS_CALLBACK_BASE_URL` | OSS 可访问的 HTTPS callback base |
| `MEDIA_RETENTION_GC_ENABLED` | 通用 clean-object GC 与账号 purge system enqueue；默认 `false` |
| `MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED` | 365 天 media operations metadata purge；默认 `false`，需批准后启用 |

任一必要值缺失时 media route fail closed。Credential 只在 main/production secret store 中注入，不进入
`.env.example`、PR preview、workflow source、日志或截图。Main staging 的 GitHub Actions 把六项
Repository Secrets 写入本次 run 专用的 `0600` 临时 env 文件，仓库内 `ops/deploy/deploy-main.sh` 通过
Docker `--env-file` 注入并在发布后只验证变量存在，不回显值；PR preview workflow 有测试约束，禁止引用
这些 production/main secrets。优先使用可轮换的 workload/RAM identity，减少长期 AccessKey；当前环境
变量模型迁移前需限制权限和 host access。

Backend 与 deployment preflight 使用通用 Region ID 拼出 `oss-<region>.aliyuncs.com` endpoint；因此
`OSS_REGION` 不能改成 Browser SDK 使用的 `oss-cn-shanghai` 形式，否则会产生重复 `oss-` 前缀。Web
在初始化 Alibaba Browser SDK 时负责增加且仅增加一次 `oss-` 前缀。生产 smoke 必须确认浏览器请求的
bucket host 为 `<bucket>.oss-<region>.aliyuncs.com`，不能只凭 CORS rule 或 STS preflight 判断可直传。

Main 部署 preflight 会在停止旧容器前验证 bucket endpoint、HTTPS callback reachability 和一次受 exact-key
`PutObject` policy 限制的 STS `AssumeRole`。该检查不实际上传 object，因此不能证明 CORS、bucket
server-side prevent-overwrite、callback body/hash 或 scanner 链路正确；发布后仍需完成真实合成图片的
upload intent → PutObject → callback → pending smoke，并清理测试 object/row。

Preflight 与后端必须生成同一种、符合阿里云 RAM grammar 的 policy：`Version=1`、单个 Allow statement、
仅 `oss:PutObject` 和单个 exact object ARN。不得加入 OSS 未声明支持的 `oss:ContentLength` condition；
否则 STS 会以 `InvalidParameter.PolicyGrammar` 拒绝 AssumeRole。对象大小的现行边界和 PostObject 迁移
状态以上述当前实现说明为准。Provider 规则以阿里云的
[OSS RAM action/condition reference](https://www.alibabacloud.com/help/en/ram/api-object-storage-service)
和 [PostObject policy reference](https://www.alibabacloud.com/help/en/oss/policies-for-setting-post-requests-in-oss)
为准。

新 binary 的 moderation deletion worker 和 upload-intent housekeeping 独立于
`MEDIA_RETENTION_GC_ENABLED`：前者继续处理已隔离 object，后者清理未 callback 的过期 exact key，并
删除消费满 30 天的 intent credential、expiry 超过 1 天的 preview grant、grace 已结束的 detached
binding、超过 48 小时的 credential-attempt fact，以及符合下述期限的 synthetic cleanup tombstone。
关闭通用 GC 不会关闭这些安全清理链路。
`MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED=false` 也只暂停 hold/retry/job/evidence 的 365 天 metadata
purge，不暂停 object 删除或 intent credential 清理。

## Bucket、RAM、CORS 与 CDN 决策

上线前必须批准并记录：

- 推荐 private bucket；若 public bucket，任何 direct URL 都可能绕过 database status，不能上线
  pending/blocked policy。
- `uploads/` prefix 对 upload RAM role 启用官方
  [OSS prevent-overwrite rule](https://www.alibabacloud.com/help/en/oss/user-guide/prevent-file-overwrite)，并保持 bucket versioning disabled；
  该规则是 callback hash/preview evidence 与 immutable object identity 成立的前提。上线验收必须用省略
  `x-oss-forbid-overwrite` header 的恶意客户端再次 PutObject，仍得到 `FileAlreadyExists`。
- Upload role 仅 `PutObject` 到 server-issued prefix/exact key，不含 list/get/delete/ACL。
- Backend delete role 只覆盖平台 upload prefix；管理和 rotation credential 分离。
- CORS 只允许正式 Web/preview 需要的 origin、method 和 header；不使用 wildcard credential policy。
- Callback 强制 HTTPS，gateway 保留原始 path/body/header 供验签，不做会改变 canonical body 的 rewrite。
- Public clean asset 使用 CDN origin protection/签名策略；private DM asset 使用短期 user-bound URL。
- CDN 不缓存 pending/blocked/private response，purge 与 asset state change 有可观察结果。

Private/public、CDN signing 和原图保留仍为 `Decision needed`；在决定前不要把 direct OSS URL 写入
头像、主题、课评或 DM 作为永久事实。

## Upload 与绑定流程

1. Client 请求 intent，声明 kind/content type；服务端授权 exact key。
2. Client 直传 OSS，展示进度和可重试失败，不自行认为业务发布成功。
3. OSS callback 创建 pending upload；profile/Forum client 按持久化 usage 恢复列表并轮询 owner-only
   status，因刷新或换设备不会把 pending 当失败，也不会自行获得公开绑定授权；quarantined 与 blocked
   都不能绑定或派生 URL。Forum draft 保存时同步 exact `draft_asset_references`，因此草稿中仍使用的
   pending/clean asset 不会被误当成无引用对象。
4. 当前 raster image 可由严格层级下的人工审核员通过受限代理核验 magic header/尺寸并批准；file/PDF
   必须等待 scanner 验证 MIME、病毒/恶意内容及 sandbox 结果。图片 EXIF/GPS stripping 和 variants 仍待实现。
5. Clean 后业务 mutation 用 `assetId` 绑定 avatar/thread/comment/review/DM；avatar/banner 和
   thread/comment Web/API 已执行 owner+image+usage+clean 约束，review/DM 仍待实现。
6. Forum 编辑替换会把旧 usage 标为 `content_edit` 并保留版本区间；软删除标为 `target_deleted`，30 天后
   才是 GC candidate。Profile/推广替换或清除写 media-owned detached binding 和 30 天 grace。恢复只在
   资产仍 clean 时重新绑定。启用后的通用 GC 只对 `cleaned_at` 已满 30 天且所有引用/grace 都已结束的
   clean object 做有界扫描；锁定候选后重新验证 active usage/binding/draft reference、future grace 和
   operational hold，再 quarantine 并复用 durable provider deletion job。Pending 不走这条年龄规则。

第 4 步 scanner/variants 仍未完整实现，状态为 `Partial/P1`。现有 callback 的 MIME/SHA 仍只是
metadata 形状检查，不是可信内容扫描；不得在没有 magic-byte/decoder/scanner 的情况下自动 clean。

Forum binding 使用显式、version-aware 的 `asset_usages(asset_id, target_type, target_id, position)` 事实表；
当前同一目标内禁止重复 asset，position 与 Markdown AST 顺序一致。跨目标复用仍要逐次经过 owner/usage/
visibility policy，refcount 只是可重建 cache。Private DM asset
不能被公共内容复用。GC 只处理没有 active reference、超过 approval-age/grace 门槛且不受 active
operational hold 的 clean asset，
不能依靠单个业务 row 的 nullable URL 猜引用。

### Operational retention hold、system job 与账号 purge

- 本域 hold 只有 `moderation` 和 `security` 两种 operational purpose。它不是法律保全机制，不提供
  external case authority、跨域冻结或经法律负责人批准的 release policy；真正 legal hold 仍为
  `Planned/Decision needed`。
- Hold 只面向具备 `operations.jobs` 的管理员，使用 recent-authenticated revocable session；输入包含
  3–500 字 reason、5 分钟至 365 天 expiry 和 `expectedHoldId`。创建要求显式“当前无 hold”，续期/替换
  和解除要求命中刚查看的 exact id，防止并发操作覆盖。敏感 hold inventory 按 expiry 分页，返回
  `private, no-store`，读取本身也要求 recent-auth 并审计。
- Hold 只暂停 durable object deletion，不恢复公开 URL 或业务 binding。普通 moderation queue 只返回
  通用 held/expiry，不披露 purpose/reason/actor；完整原因只在 operations inventory 内可见。
- Place/release 与 worker 都先锁 upload；worker 取 lease 后 provider I/O 在事务外执行，此时新 hold fail
  closed。Hold 先提交时 worker 在锁后的第二条语句重验并跳过，防止 snapshot race。
- `operations.jobs` 还能在 recent-auth 后读取 `private, no-store` 的 system deletion-job inventory；每一页
  读取都写 purpose-limited audit。非 moderation dead letter 可填写原因后重排；retry event 与
  append-only governance audit 同事务写入。
- Provider 删除成功后，upload row 清除/替换 object key、URL、hash、size、MIME、usage、dimensions 等
  storage locator/fingerprint，保留稳定 upload id 和 purpose-limited、伪名化 audit 引用；不能把
  `blocked` row 当作仍保存 object metadata。
- Account purge 先立即 detach profile avatar/banner、删除 owner draft reference、撤销 intent，再为 owner
  且无 live Forum/推广共享引用、无 future grace 的 object quarantine 并建立 durable job。Active
  operational hold 不阻止 quarantine/enqueue，只让 provider worker 暂停；因此公开派生立即停止，hold
  结束后同一 job 可继续。共享内容/运营引用和 future grace 仍不排队，属于显式 policy-retained asset。
- Media purge 只有在没有更多可排队对象、没有 queued/leased/dead-letter job、没有缺失 deletion job 时
  才能向账号生命周期报告删除工作完成。Held object 已有 durable job 时可作为 `retainedAssets` 随账号
  tombstone 暂留；held quarantined object 缺 job 仍由 `missingDeletionJobs` 阻断 terminal。共享引用/
  future grace 也可使 `retainedAssets` 非零，之后由引用解除、grace/hold 到期和 reconciliation 决定后续。
  System enqueue 与通用 GC 使用同一个 rollout flag；flag 关闭时不得把仍待排队的 media 工作当作终态。

### Intent 与 operations metadata retention

- 没有 callback 的 intent 不会生成普通 pending upload；过期后由独立 housekeeping 为该 exact key 建立
  cleanup tombstone 和 durable deletion job。账号 purge 会撤销本人未 callback intent，并在 rollout gate
  开启后为 exact key 排队；禁止按 prefix 猜测或批量删除。
- 已 callback/消费的 intent credential 在 `consumed_at` 满 30 天后由 housekeeping 删除。这条 bounded
  credential cleanup 当前不受 history-purge flag 控制。
- Synthetic exact-key cleanup 成功后只留下已 redacted tombstone，独立 housekeeping 至少保留 30 天再
  删除。存在任意 hold history 或 operator retry history 时，tombstone 会等相应 365 天 operations history
  被批准清理后再删；succeeded job history 可以先删除或随 tombstone cascade 删除，inventory 始终通过
  upload join，不能因此暴露孤立 job 或已 redacted provider metadata。
- Hold/release reason 与 staff id、system retry reason/actor、已成功 deletion job、已 redacted object 的
  moderation evidence 已实现 365 天后的有界 purge，但
  `MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED=false` 是默认值。Privacy/legal owner 批准这组目的和期限前
  不得启用或声称历史已清除；append-only governance actor audit 不在该 purge 中，其期限仍为
  `Decision needed`。

### Migration `0057` 与启用步骤

Migration `0057` 允许 deletion job 使用 system actor，并把 upload-intent callback credential 从 plaintext
column 切换为 digest-only schema。旧 API 仍读写 plaintext callback column，旧 deletion binary 只认识
moderator actor；因此这是 application-level breaking cutover。Trigger-before-backfill 保护 source-table
引用快照，但不提供旧/新 API schema 兼容，不能把它误写成 zero-downtime rolling safety。Digest backfill
使新版能够验证迁移前已签发 callback 中的原 token；但 callback/API maintenance gap 内已经写入 OSS、尚未
成功 callback 的 object 仍会成为 orphan，必须依赖 exact-key intent cleanup 和监控收敛。

按以下顺序启用：

1. 保持 `MEDIA_RETENTION_GC_ENABLED=false` 和
   `MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED=false`，先在 edge/API 停止签发新的 upload credential，但保持旧
   callback 可用。等待至少 15 分钟 STS/intent TTL 加 10 分钟 cleanup safety buffer；或者由数据库 active
   intent、gateway in-flight callback 和 provider callback 指标权威确认 outstanding intents/callbacks 为零。
2. 再 drain/停止全部旧 callback/API/writer image、旧 deletion worker 与旧 account-lifecycle worker；确认
   没有旧进程仍会访问 callback plaintext column、写业务引用或处理队列。
3. 以 migration owner 顺序执行 `0057`、`0058`，再部署 binding-aware 的全部 profile/promotion/Forum
   draft/published-content writer 与 lease-fenced 新版 deletion/lifecycle worker。`0058` 回收的旧 running
   lifecycle row 只有在确认旧进程已经停止后才能由新版重新 claim；新 worker 的 UUID token 防止旧 lease
   覆盖等待、dead-letter/missing-job 阻断或最终完成。新 moderation deletion worker 和
   upload-intent housekeeping 此时独立运行，但通用 GC/account-purge system enqueue 仍关闭。
4. 从仓库根目录用获批的数据库连接运行 DB preflight，并要求其所有 drift/anomaly 为零：

   ```bash
   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/check_media_retention_references.sql
   ```

5. SQL 只验证 profile/promotion/draft facts 与 deletion/redaction anomaly；它不检查已发布 Forum
   Markdown AST 与 `asset_usages`，也不访问 OSS。另行完成全部 retained thread/comment canonical Markdown
   reference reconciliation，以及 DB row/object exact-key 的 OSS inventory reconciliation；未知 ownership、
   missing object、orphan object 或 usage drift 必须为零或由 owner 明确处置，不能以单个 SQL 通过代替。
6. 只有三项硬门槛（DB preflight、published Markdown reconciliation、OSS reconciliation）都通过后，才在
   目标环境设置 `MEDIA_RETENTION_GC_ENABLED=true` 并重启/滚动新版进程。检查 startup log 确认 GC worker
   已调度，观察 clean candidate、queue/lease/succeeded/dead-letter、intent cleanup 和账号 purge progress；
   实际抽样验证 live binding/draft/grace/hold 均未被误删。
7. `MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED` 单独由 privacy/legal owner 批准后启用，并先验证 365 天
   边界和 append-only governance audit 不受影响。GC 启用不授权这个 history purge。

回滚先把 `MEDIA_RETENTION_GC_ENABLED` 设回 `false` 停止新的 general GC/account-purge system enqueue；
由新版 deletion worker 排空或保留已有 system job。队列中仍有 system actor 时不得恢复旧 deletion
binary，也不得通过伪造管理员 actor 换取兼容。Trigger、backfill 和 redacted tombstone 保持 forward-only。

## Preview 与测试

- PR preview 不注入生产 OSS key/bucket，也不写生产 object。
- Protocol tests 使用 fake STS/OSS HTTP 或 alternate object-store boundary，覆盖 policy、callback canonical
  signature、redirect rejection、intent replay、key/MIME/size mismatch、quarantine-before-delete、provider
  I/O 不持有数据库锁、retry/dead-letter/finalize ordering；preview provider I/O 同样不跨数据库锁。
- Handler→DB test 覆盖 profile image 对 pending、他人 clean、本人 clean asset 的拒绝/接受与解绑。
- Retention/GC integration test 覆盖 profile/推广/draft binding、clean approval age、pending 不按年龄回收、
  digest-only callback、credential quotas/48 小时 attempt retention、exact-key intent cleanup/30 天 tombstone、
  active usage、future grace、hold CAS/release、recent-auth/capability、rollout gate、account purge held enqueue/
  terminal progress、lease fence、redaction、system inventory every-read audit/dead-letter retry；全部使用 fake
  object store。
- Forum handler→DB test 覆盖 exact ordered set、duplicate/missing/extra id、无 alt、远程/data URL、
  cross-account、pending/blocked、stale edit、revision、作者/staff/举报 delete、archive、restore、申诉
  overturn、parent hide/delete 并发锁序与并发 restore；不调用真实 OSS。Revision page 需要验证多版本
  attachment 在一次 batch projection 后仍各自匹配正确 asset。
- Admin preview integration test 使用 fake object store，覆盖 capability、严格 role hierarchy/independent
  reviewer、分页前过滤、一次性 token、
  MIME/byte/dimension-bound same-origin response、replay rejection、`no-store`/`nosniff`、dimension persistence
  和不含 key/URL 的 audit；协议 unit test 覆盖四种允许图片 header 与 pixel limit；Web test 覆盖 reason、
  one-time proxy 调用、browser blob 展示和 DOM 不出现 provider metadata。
- Owner status test 覆盖 usage filter、pending/clean 恢复、他人 asset 不可枚举和响应不泄露 OSS metadata；
  Web component/axe test 覆盖 Forum pending/blocked 不可发布、clean 可发布、引用插入/移除和 object key
  不进入 DOM。
- End-to-end test bucket 若存在，使用独立 account/prefix、最短 lifecycle 和合成无 PII asset。
- Test 完成自动清理；cleanup failure 进入告警，不靠人工记住 object key。

## 监控与 reconciliation

至少监控：intent issued/expired/consumed、STS/callback latency/error、pending age、scan result、
approve/block、delete failure、binding/GC backlog、storage bytes/quota、CDN 4xx/5xx 和 unauthorized URL。

Reconciliation 分成三个独立硬门槛，不能用一个检查替代：

- `check_media_retention_references.sql` 只比较 DB 中 profile/promotion/draft references、deletion job 与
  redaction anomaly；
- published Forum reconciliation 解析所有 retained thread/comment canonical Markdown，并与 version-aware
  `asset_usages` exact match；
- OSS inventory 比较 row 有 object 无、object 有 row 无、blocked 仍存在和 exact key ownership；
- 默认 dry-run 和有界 batch，修复幂等并写 audit；
- 不自动删除无法确定 ownership、live reference 或 operational/legal preservation policy 的 object。

## 故障处理

- **STS unavailable**：新上传 fail closed，已有 clean media 读取继续；检查 RAM scope/expiry/network。
- **Callback failure**：object 可能已存在但 row 未创建；不要重复签发相同 key，靠 intent/object reconcile。
- **Scanner backlog**：file/PDF 保持 pending，不人工批量 approve 未扫描对象；已完成可信图片预览的 raster
  image 才能由同一审核员批准。
- **Delete failure**：保持 quarantined、停止公开派生，durable job 自动重试；moderation job 由审核面、
  system job 由 `operations.jobs` 工作台填写原因重新排队。不要回滚为 pending/clean，也不要先标
  blocked 假装 provider 删除已完成。
- **Credential exposure**：创建最小权限新 credential、更新 secret、验证，再撤销旧值并审计 object access。
- **Public leak**：先收紧 bucket/CDN access 与 purge，保留必要证据，再修 database/asset policy。

## 上线清单

- Bucket visibility、RAM least privilege、uploads prefix server-side prevent-overwrite、CORS、callback HTTPS
  和 CDN origin protection 已审查并做过绕过 header 的覆写回归。
- Production/preview secret 完全隔离，credential rotation 已演练。
- DB fail-closed credential/storage limits（10 active、rolling 24h 100、512 MiB、500 live、2,000 retained）
  已有实现与测试；上线仍要观察拒绝率和 housekeeping。当前可信 preview 只有 bounded raster header/pixel
  检查，scanner、EXIF stripping 与更完整的 content-abuse policy 仍为 `Partial`，不能因 quota 落地勾选。
- Asset binding、durable delete/replace、draft reference 与 GC 代码/测试已交付；通用 GC 只有完成上述
  reconciliation、显式启用 flag 并验证 worker/queue 后才算目标环境启用。
- Operational moderation/security hold 已有 no-store inventory、CAS/recent-auth 与测试；真正 legal hold
  仍为 `Planned/Decision needed`。Operations history purge 只有 owner 批准并启用独立 flag 后才生效。
- Private URL/CDN signing 仍为 `Decision needed`，不得因其他媒体链路完成而勾选。
- Admin 文案、pending preview、block delete 和 recovery 行为与后端一致。
- Metrics、alerts、reconciliation、backup/restore 与 incident owner 明确。

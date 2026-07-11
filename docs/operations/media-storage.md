# OSS 媒体存储

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Media maintainers、Platform maintainers、Security owner
>
> 最近核验：2026-07-11，`origin/main@33584db`

本 runbook 描述当前 Alibaba Cloud STS/OSS 代码边界和上线前配置要求。代码支持不代表 main/production
已经配置；当前部署的 bucket、RAM、CORS、CDN 与 scanner 状态必须由 operator 独立核验。

## 当前实现

- Authenticated client 请求一次 upload intent，服务端生成 account/kind/UUID-scoped exact object key。
- STS credential 约 15 分钟过期，policy 只允许该 key 的 `oss:PutObject`，最大 20 MiB。
- Web SDK 每次 PutObject 都发送 `x-oss-forbid-overwrite: true`；生产 bucket 还必须对 `uploads/` prefix 和
  upload RAM role 配置 server-side prevent-overwrite rule。客户端 header 只是纵深防御，不能替代 bucket
  规则；否则恶意客户端可在 callback/preview 后、STS 到期前覆盖同一 key，审核 evidence 不可信。
- 当前允许 JPEG、PNG、GIF、WebP 和 PDF；SVG、视频和其他文件拒绝。
- OSS callback public-key URL 只允许官方 host、禁 redirect、有 5 秒 timeout 和 16 KiB key document limit。
- Callback 锁定 intent，核对 key/MIME/bytes/SHA-256 shape，原子创建 `pending` upload 并消费 intent。
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
- 归档/隐藏不等于删除，仍保留 active usage 以支持长期恢复；作者、staff、举报 uphold 的软删除都会在
  同一事务 detach，staff restore 和申诉 overturn 都从 canonical Markdown 重新验证 clean owner asset
  后 rebind。恢复验证失败会整体回滚，不能出现正文已恢复但图片进入 GC 的半完成状态。
- Admin queue 同样不返回 object key、hash 或持久 URL，并可按 pending/clean/quarantined/blocked 浏览。
  服务端在分页前执行严格层级过滤：mod 只能审核 user，admin 可审核 user/mod，任何人不能自审或审核同级。
  持有 `moderation.content` 的合格审核员
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

管理 UI 必须如实说明 block 会先隔离再永久删除 object、失败会重试且 dead letter 可人工重新排队，不能把
`202 Accepted` 误写成已经完成 provider 删除。

## Runtime 配置

| Variable | 用途 |
|---|---|
| `OSS_REGION` | Bucket region |
| `OSS_BUCKET` | Object bucket name |
| `OSS_ACCESS_KEY_ID` / `OSS_ACCESS_KEY_SECRET` | Backend AssumeRole/delete credential |
| `OSS_ROLE_ARN` | Upload-only RAM role |
| `OSS_CALLBACK_BASE_URL` | OSS 可访问的 HTTPS callback base |

任一必要值缺失时 media route fail closed。Credential 只在 main/production secret store 中注入，不进入
`.env.example`、PR preview、workflow、日志或截图。优先使用可轮换的 workload/RAM identity，减少长期
AccessKey；当前环境变量模型迁移前需限制权限和 host access。

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
   都不能绑定或派生 URL。
4. 当前 raster image 可由严格层级下的人工审核员通过受限代理核验 magic header/尺寸并批准；file/PDF
   必须等待 scanner 验证 MIME、病毒/恶意内容及 sandbox 结果。图片 EXIF/GPS stripping 和 variants 仍待实现。
5. Clean 后业务 mutation 用 `assetId` 绑定 avatar/thread/comment/review/DM；avatar/banner 和
   thread/comment Web/API 已执行 owner+image+usage+clean 约束，review/DM 仍待实现。
6. Forum 编辑替换会把旧 usage 标为 `content_edit` 并保留版本区间；软删除标为 `target_deleted`，30 天后
   才是 GC candidate。恢复只在资产仍 clean 时重新绑定。当前没有执行 object purge 的 GC worker。

第 4 步和第 6 步 GC worker 当前未完整实现，状态为 `Partial/P1`。现有 callback 的 MIME/SHA 仍只是
metadata 形状检查，不是可信内容扫描；不得在没有 magic-byte/decoder/scanner 的情况下自动 clean。

Forum binding 使用显式、version-aware 的 `asset_usages(asset_id, target_type, target_id, position)` 事实表；
当前同一目标内禁止重复 asset，position 与 Markdown AST 顺序一致。跨目标复用仍要逐次经过 owner/usage/
visibility policy，refcount 只是可重建 cache。Private DM asset
不能被公共内容复用。GC 只处理没有 active usage、超过 grace period 且不受 legal hold 的 asset，
不能依靠单个业务 row 的 nullable URL 猜引用。

## Preview 与测试

- PR preview 不注入生产 OSS key/bucket，也不写生产 object。
- Protocol tests 使用 fake STS/OSS HTTP 或 alternate object-store boundary，覆盖 policy、callback canonical
  signature、redirect rejection、intent replay、key/MIME/size mismatch、quarantine-before-delete、provider
  I/O 不持有数据库锁、retry/dead-letter/finalize ordering；preview provider I/O 同样不跨数据库锁。
- Handler→DB test 覆盖 profile image 对 pending、他人 clean、本人 clean asset 的拒绝/接受与解绑。
- Forum handler→DB test 覆盖 exact ordered set、duplicate/missing/extra id、无 alt、远程/data URL、
  cross-account、pending/blocked、stale edit、revision、作者/staff/举报 delete、archive、restore、申诉
  overturn 与并发 restore；不调用真实 OSS。
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

Reconciliation 比较 database rows、bindings 和 OSS prefix：

- row 有 object 无、object 有 row 无、blocked 仍存在、clean 无 binding、binding 指向非-clean；
- 默认 dry-run 和有界 batch，修复幂等并写 audit；
- 不自动删除无法确定 ownership/legal hold 的 object。

## 故障处理

- **STS unavailable**：新上传 fail closed，已有 clean media 读取继续；检查 RAM scope/expiry/network。
- **Callback failure**：object 可能已存在但 row 未创建；不要重复签发相同 key，靠 intent/object reconcile。
- **Scanner backlog**：file/PDF 保持 pending，不人工批量 approve 未扫描对象；已完成可信图片预览的 raster
  image 才能由同一审核员批准。
- **Delete failure**：保持 quarantined、停止公开派生，durable job 自动重试；dead letter 告警并由审核面
  填写原因重新排队。不要回滚为 pending/clean，也不要先标 blocked 假装 provider 删除已完成。
- **Credential exposure**：创建最小权限新 credential、更新 secret、验证，再撤销旧值并审计 object access。
- **Public leak**：先收紧 bucket/CDN access 与 purge，保留必要证据，再修 database/asset policy。

## 上线清单

- Bucket visibility、RAM least privilege、uploads prefix server-side prevent-overwrite、CORS、callback HTTPS
  和 CDN origin protection 已审查并做过绕过 header 的覆写回归。
- Production/preview secret 完全隔离，credential rotation 已演练。
- Magic-byte/decoder/scanner、EXIF、pixel limit、quota 和 abusive upload rate-limit 已交付。
- Asset binding、private URL、delete/replace、orphan GC 和 legal hold 已交付并测试。
- Admin 文案、pending preview、block delete 和 recovery 行为与后端一致。
- Metrics、alerts、reconciliation、backup/restore 与 incident owner 明确。

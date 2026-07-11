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
- 当前允许 JPEG、PNG、GIF、WebP 和 PDF；SVG、视频和其他文件拒绝。
- OSS callback public-key URL 只允许官方 host、禁 redirect、有 5 秒 timeout 和 16 KiB key document limit。
- Callback 锁定 intent，核对 key/MIME/bytes/SHA-256 shape，原子创建 `pending` upload 并消费 intent。
- Authenticated URL endpoint 对 clean asset 允许任意登录用户，pending 只允许 owner/staff，blocked
  不返回；当前生成的是 direct OSS URL，不是 private signed/CDN URL。
- Staff approve 将 `pending -> clean`。Block 会先永久删除 OSS object，删除成功后才事务提交
  `pending -> blocked` 与 governance audit；删除失败时 row 保持 pending。

管理 UI 必须如实说明 block 会永久删除 object，不能继续显示“不会自动删除”。

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
3. OSS callback 创建 pending upload；client 轮询/查询 upload status。
4. Scanner 验证 magic bytes、MIME、尺寸/像素、病毒/恶意 PDF，图片移除 EXIF/GPS 并生成 variants。
5. Clean 后业务 mutation 用 `assetId` 绑定 avatar/thread/comment/review/DM；owner、状态和 target type
   在服务端验证。
6. 替换/删除解除 binding；无引用 asset 进入 grace period，GC worker 最终删除 object 和派生 variants。

第 4–6 步当前未完整实现，状态为 `Planned/P1`。现有 callback 的 MIME/SHA 是 metadata 形状检查，
不是可信内容扫描；不得在没有 magic-byte/decoder/scanner 的情况下自动 clean。

Binding 使用显式 `asset_usages(asset_id, target_type, target_id, slot/position)` 事实表；同一 clean asset
可以有多个经过 owner/visibility policy 允许的 usage，refcount 只是可重建 cache。Private DM asset
不能被公共内容复用。GC 只处理没有 active usage、超过 grace period 且不受 legal hold 的 asset，
不能依靠单个业务 row 的 nullable URL 猜引用。

## Preview 与测试

- PR preview 不注入生产 OSS key/bucket，也不写生产 object。
- Protocol tests 使用 fake STS/OSS HTTP 或 alternate object-store boundary，覆盖 policy、callback canonical
  signature、redirect rejection、intent replay、key/MIME/size mismatch 和 delete-before-block ordering。
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
- **Scanner backlog**：保持 pending，不人工批量 approve 未扫描对象，显示用户可理解的延迟状态。
- **Delete failure**：保持 pending，重试/告警；不要先标 blocked 留下可访问 object。
- **Credential exposure**：创建最小权限新 credential、更新 secret、验证，再撤销旧值并审计 object access。
- **Public leak**：先收紧 bucket/CDN access 与 purge，保留必要证据，再修 database/asset policy。

## 上线清单

- Bucket visibility、RAM least privilege、CORS、callback HTTPS 和 CDN origin protection 已审查。
- Production/preview secret 完全隔离，credential rotation 已演练。
- Magic-byte/decoder/scanner、EXIF、pixel limit、quota 和 abusive upload rate-limit 已交付。
- Asset binding、private URL、delete/replace、orphan GC 和 legal hold 已交付并测试。
- Admin 文案、pending preview、block delete 和 recovery 行为与后端一致。
- Metrics、alerts、reconciliation、backup/restore 与 incident owner 明确。

# OSS/CDN 媒体存储

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Media maintainers、Platform maintainers、Security owner
>
> 最近核验：2026-07-12，migration <code>0061</code>、阿里云 OSS/CDN 官方文档

本 runbook 负责 YourTJ 媒体的 Alibaba Cloud OSS/CDN 配置、部署、验收、轮换和故障处置。
代码支持不代表目标环境已经配置；operator 必须逐项记录控制台配置和 smoke 结果。Target production
仍是目标架构，不能把 main staging 的配置描述为已经上线生产。

## 交付状态和硬边界

以下后端链路为 <code>Current</code>：

- 私有 Ingest 与私有 Delivery 使用不同 bucket 和不同写凭证。
- 浏览器只获得 Ingest exact-key STS PutObject 权限。
- 应用 runtime 和部署合成 preflight 的所有 OSS Get/Head/Put/Delete 都使用 OSS Signature V4；STS
  AssumeRole 与 CDN OpenAPI 使用各自官方 RPC 签名协议。OSS V1 自 2025-09-01 起不再对新增 bucket
  开放，不存在 deploy 例外。
- Pending owner preview 和 staff moderation preview 都通过同源、鉴权、<code>no-store</code> 代理，
  不返回 provider URL、bucket 或 object key。
- 当前默认对 callback 已验签且 metadata 与 intent 匹配的 JPEG、PNG、WebP 使用 system actor 自动批准，
  并在同一事务写审计和 processing job；这项策略不执行内容安全识别。自动或人工批准只得到
  <code>clean</code> 状态；worker 完成全部可信变体后才原子进入
  <code>published</code>。Clean 但未 published 的资产不能绑定或签发 URL。
- Owner upload DTO 和 moderation queue 都返回独立的 <code>deliveryState</code>，其状态只允许
  unpublished、processing、published、failed、blocked；客户端不得用 moderation <code>status</code>
  猜测交付状态。只有 moderation DTO 额外返回受控的 nullable <code>deliveryErrorCode</code> 供管理员区分
  重试和重传提示；owner DTO 不暴露 provider/internal failure detail。
- 通用 <code>/media/{id}/url</code> 仅供当前资产 owner 拉取本人的 clean+published 结果；跨账号即使知道
  ID 也返回 404。Forum、profile、promotion 等公开展示必须先由 owning domain 完成可见性授权，再消费
  Media 的 typed delivery projection（assetId、五分钟 URL/expiry、MIME、变体尺寸与 variant），不能借
  通用 route 绕过业务可见性。包含 bearer URL 的 owner 响应为 <code>private, no-store</code>；owning
  domain 的响应也不得被缓存到 <code>expiresAt</code> 之后。
- Worker 只接受 JPEG、PNG、静态 WebP，限制源文件、尺寸、像素和 decoder allocation。对 decoder
  明确支持的 JPEG/WebP 应用 orientation 后重编码为不携带 EXIF/GPS 的确定性 lossless WebP；当前不承诺
  PNG EXIF orientation 或 ICC/wide-gamut 到 sRGB 的色彩转换，客户端应上传已归一到 sRGB 的静态图片。
- Delivery 变体为 <code>thumb_256</code>、<code>display_1280</code>、<code>full_2048</code>，
  object key 包含 policy version 和内容 SHA-256。
- 公开/校园可见资源使用 CDN Type A 五分钟 bearer URL。签名 URL 在到期前可转发，不是 viewer-bound；
  真正私密的 DM 媒体仍不得复用这条公开交付链路。
- Block 先在 PostgreSQL 停止签名，再持久排队 CDN force purge、全部 Delivery 删除和 Ingest 删除。
  Operational hold 只保留私有 Ingest 证据，不能保留 Delivery 变体或 CDN cache。
- 普通审核员和委派管理员禁止自审。只有最高角色 ADMIN 可审核本人媒体，并且必须有 recent-auth、
  reason、<code>selfReviewConfirmed=true</code> 和 <code>selfReview=true</code> audit；自批准还必须有同一可信
  preview evidence，自 block 属于 fail-closed 隐藏操作，不要求可能已不存在的 pending preview evidence。

以下仍为 <code>Partial</code>：

- PDF/file scanner、sandbox renderer 尚未接入，因此 file approval fail closed。
- Main staging 是否已经创建双 bucket、CDN domain、RAM identity 和 GitHub Environment secrets，必须由
  本文验收步骤确认。
- Provider inventory reconciliation 需要 operator 配置 OSS Inventory；代码和 migration 不能替代该配置。
- 通用 retention GC 仍受 <code>MEDIA_RETENTION_GC_ENABLED</code> rollout gate 保护。

新 GIF intent 已禁用。Migration <code>0061</code> 会把既有 clean GIF/其他不受支持图片保持为不可交付的
<code>failed</code> publication，并记录 <code>legacy_animated_format_requires_reupload</code>；不得默默取
首帧或丢弃动画语义。审核 preview 同样 fail closed，不再接受 legacy GIF。APNG 和 animated WebP 也由 worker fail closed，用户必须转成受支持
的静态格式后重传。

## 数据流

1. Client 请求 upload intent；服务端生成 account/kind/UUID-scoped exact key。
2. Backend 用 Ingest caller credential AssumeRole，session policy 只允许该 key 的
   <code>oss:PutObject</code>。
3. Browser 直传私有 Ingest；OSS callback 创建 <code>pending</code> row，数据库不保存可交付 URL。
4. 当 <code>MEDIA_IMAGE_AUTO_APPROVAL_ENABLED=true</code> 且上传是受支持 raster 时，同一 callback 事务
   将 row 改为 <code>clean</code>、publication 置为 <code>processing</code>、排队 worker 并写 system audit。
   Flag 关闭或上传不符合条件时仍保持 pending；owner 可走同源 preview，审核员走一次性审计 preview grant。
5. 自动或人工审核通过后，durable worker 从 Ingest V4 读取并验证实际
   MIME、长度、SHA-256、尺寸和解码限制。
6. Worker 重新编码并使用独立 Delivery credential V4 PutObject，带
   <code>x-oss-forbid-overwrite:true</code>；随后 V4 HEAD 校验每个变体。
7. 三个变体全部存在后，一个数据库事务把 variants 和 publication 设为 <code>published</code>。
8. Owner domain 先完成内容可见性授权，再由 Media resolver 返回不含 Ingest key、bucket/provider host、
   credential 或独立持久 locator 字段的 typed projection；短期 CDN URL 必然包含可见的 immutable Delivery path；
   generic URL route 只允许 asset owner 本人。
9. Block/quarantine 立即让签名查询失效；durable cleanup 按 purge → Delivery delete → Ingest delete
   顺序执行并可重试、dead-letter。

Processing retry 是受控 operations 动作：<code>POST
/api/v2/admin/media/uploads/{id}/processing/retry</code> 只接受 publication=failed 且 job=dead_letter 的
组合，要求 <code>RunOperations</code>、recent-auth 和 3–500 字符 reason，并在同一事务清空失败状态、重排
job 和写 governance audit。返回 202 只表示已重新入队，不表示已 published；重复请求返回 conflict。

PostgreSQL 是 moderation/publication/job 的权威来源；OSS object、CDN cache 和客户端 URL 都是可重建或
可撤销派生物。不得用 OSS object 是否存在反推业务发布状态。

## Runtime 配置

代码读取以下精确变量名：

| Variable | Secret | 用途 |
|---|---:|---|
| <code>OSS_REGION</code> | 否 | 两个 bucket 的通用 Region ID，例如 <code>cn-shanghai</code>；不得带 <code>oss-</code> |
| <code>OSS_BUCKET</code> | 否 | 私有 Ingest bucket；保留该名字是当前兼容约束 |
| <code>OSS_ACCESS_KEY_ID</code> / <code>OSS_ACCESS_KEY_SECRET</code> | 是 | Ingest backend caller：AssumeRole、Get/Head/Delete；不得写 Delivery |
| <code>OSS_ROLE_ARN</code> | 否 | Browser upload-only RAM role |
| <code>OSS_CALLBACK_BASE_URL</code> | 否 | OSS 可访问的 HTTPS API base |
| <code>MEDIA_DELIVERY_OSS_BUCKET</code> | 否 | 私有 Delivery bucket |
| <code>MEDIA_DELIVERY_OSS_ACCESS_KEY_ID</code> / <code>MEDIA_DELIVERY_OSS_ACCESS_KEY_SECRET</code> | 是 | Delivery worker 与 deploy smoke 的 Put/Get（含 HEAD）/Delete；不得读写 Ingest |
| <code>MEDIA_CDN_BASE_URL</code> | 否 | 加速域名，例如 <code>https://media-dev.yourtj.de</code> |
| <code>MEDIA_CDN_PRIMARY_KEY</code> | 是 | CDN Type A primary key |
| <code>MEDIA_CDN_SECONDARY_KEY</code> | 是 | CDN Type A secondary key，必须与 primary 不同 |
| <code>MEDIA_CDN_SIGNING_KEY_SLOT</code> | 否 | <code>primary</code> 或 <code>secondary</code> |
| <code>MEDIA_CDN_URL_TTL_SECONDS</code> | 否 | 必须为 <code>300</code>；代码拒绝其他值 |
| <code>CDN_ACCESS_KEY_ID</code> / <code>CDN_ACCESS_KEY_SECRET</code> | 是 | 仅调用 CDN force purge/status API 的独立最小权限身份 |
| <code>MEDIA_IMAGE_AUTO_APPROVAL_ENABLED</code> | 否 | 新 JPEG/PNG/WebP callback 是否自动进入 processing；当前默认 <code>true</code>，设为 <code>false</code> 恢复人工可信预览审批 |
| <code>MEDIA_RETENTION_GC_ENABLED</code> | 否 | 通用 clean-object GC/account purge enqueue；默认 <code>false</code> |
| <code>MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED</code> | 否 | 365 天 operations metadata purge；默认 <code>false</code> |

Delivery 任一必需变量缺失、双 key 相同、base URL 不是无 path 的 HTTPS URL、TTL 不是 300，都会 fail
closed。启动校验也拒绝 Ingest/Delivery 同 bucket，或 Ingest、Delivery、CDN purge 任两 AccessKey ID
相同。CDN signing key、OSS AccessKey 和 CDN purge AccessKey 是不同凭证，不得复用。

## Alibaba Cloud 控制台配置

### 1. 创建两个 private bucket

在 OSS 控制台创建同 Region 的 Ingest 和 Delivery bucket：

- ACL 均为 private。
- Versioning 关闭；不可变 key 依赖禁止覆盖，不依赖历史版本兜底。
- 开启 SSE-OSS。若改为 SSE-KMS，必须单独向 worker/CDN service role 授予最小 KMS decrypt/encrypt 权限。
- Ingest 只保存 <code>uploads/</code>；Delivery 只保存 <code>assets/</code>。
- 对两个 prefix 启用
  [禁止覆盖](https://www.alibabacloud.com/help/en/oss/user-guide/prevent-file-overwrite)。
  验收必须用不带 <code>x-oss-forbid-overwrite</code> 的第二次写入确认仍返回
  <code>FileAlreadyExists</code>。
- Delivery 不能 public-read；Ingest 永远不能成为 CDN origin。

不要把两个用途放进同一 bucket。否则 CDN service role、CORS、inventory 和误配置爆炸半径无法独立收敛。

### 2. RAM 最小权限

建立四个独立权限边界：

1. Upload role
   - Trust policy 只信任 Ingest backend caller。
   - Role policy 最多允许 Ingest <code>uploads/</code> 的 PutObject。
   - Backend 每次 AssumeRole 再用 session policy 缩小到一个 exact object ARN。
   - 禁止 List/Get/Delete、ACL、bucket configuration。
2. Ingest backend caller
   - 只允许对上述 role 的精确 <code>sts:AssumeRole</code>。
   - Worker/preview 需要 Ingest <code>GetObject</code>/<code>HeadObject</code>。
   - Cleanup 需要 Ingest <code>DeleteObject</code>。
   - 禁止 Delivery 写权限。
3. Delivery worker identity
   - 只允许 Delivery <code>assets/</code> 的 <code>oss:PutObject</code>、
     <code>oss:GetObject</code>（GET/HEAD）和 <code>oss:DeleteObject</code>。
   - 禁止 Ingest、List、ACL、bucket configuration。
4. CDN purge identity
   - 只允许 <code>cdn:RefreshObjectCaches</code> 与 <code>cdn:DescribeRefreshTaskById</code>。
   - 不授予任何 OSS 数据权限。

不要使用 <code>AliyunOSSFullAccess</code>。Caller/role 双向关系必须同时成立：caller 有
<code>sts:AssumeRole</code>，role trust policy 也信任 caller。参考
[RAM policy](https://www.alibabacloud.com/help/en/oss/user-guide/ram-policy/) 和
[AssumeRole](https://www.alibabacloud.com/help/en/ram/user-guide/assume-a-ram-role)。

### 3. Ingest CORS

Browser 只直连 Ingest，因此 CORS 只配置在 Ingest：

- Allowed origins 使用精确值，例如 main staging 的 <code>https://pf-dev.yourtj.de</code>，以及获批的
  localhost 开发 origin。不要使用 <code>*</code>。
- PR Preview 默认没有真实 provider credential，因此不应把所有 <code>/pr-N</code> 或任意 origin 加入
  main Ingest CORS。需要 provider-backed preview 时另建 preview bucket/domain/role。
- Allowed methods：PUT、POST、OPTIONS。
- Allowed headers：authorization、content-type、<code>x-oss-*</code>。
- Expose headers：ETag、<code>x-oss-request-id</code>。
- Max age 建议 600 秒，并确认响应带正确 <code>Vary: Origin</code>。
- Main frontend CSP 的 <code>connect-src</code> 必须只增加由 Ingest bucket 和 Region 推导的精确 OSS HTTPS
  origin（例如 <code>https://bucket.oss-cn-shanghai.aliyuncs.com</code>）；不能用通配符，也不能把 Delivery
  origin 或 CDN domain 当作上传目标。部署脚本从同一份已校验 provider 配置渲染该 origin，避免另设可漂移值。

Delivery OSS 不开放 browser CORS，因为 browser 不得直连 origin。普通 <code>img</code> 展示不要求 CDN
CORS；只有 canvas/download 等明确用例才在 CDN response header 配置精确 CORS。参考
[OSS CORS](https://www.alibabacloud.com/help/en/oss/user-guide/configure-cross-origin-resource-sharing)
和 [CDN CORS](https://www.alibabacloud.com/help/en/cdn/user-guide/configure-cors)。

### 4. CDN domain 与 private origin

在 Alibaba Cloud CDN：

1. 添加 <code>media-dev.yourtj.de</code>。没有中国内地 ICP 时选择“全球（不含中国内地）”，不要选择
   需要 ICP 的区域。
2. Origin 选择同账号的 Delivery OSS bucket，配置正确 origin host/SNI。
3. 开启 private bucket back-to-origin，并创建/授权
   <code>AliyunCDNAccessingPrivateOSSRole</code>，该 role 只能读 Delivery。
4. 开启 HTTPS acceleration，安装证书，强制 HTTPS；确认稳定后再启用 HSTS。
5. Access Control → URL Signing 选择 Type A：
   - primary/secondary key 与 GitHub Environment secrets 对应，均为 6–128 位字母或数字且互不相同；
   - validity period 固定 300 秒；
   - 用控制台 Signed URL Generator 和后端测试向量交叉验证。
6. 对不可变 <code>/assets/</code> 设置长 edge TTL；404/403 不缓存。不要把 origin
   <code>no-store</code> 配到公开 immutable variants。
7. 开启 access log、4xx/5xx、hit ratio、back-to-origin 和 refresh task 监控。

Type A URL 形如
<code>?auth_key=timestamp-random-0-md5(path-timestamp-random-0-key)</code>。Backend 不向 CDN URL 附加
OSS presign query；CDN 自己用 private-origin identity 回源。参考
[private OSS origin](https://www.alibabacloud.com/help/en/oss/allow-only-cdn-accelerated-domain-names-to-access-oss-resources)、
[CDN acceleration](https://www.alibabacloud.com/help/en/oss/user-guide/cdn-acceleration)、
[URL signing](https://www.alibabacloud.com/help/en/cdn/user-guide/configure-url-signing) 和
[Type A](https://www.alibabacloud.com/help/en/cdn/user-guide/type-a-signing)。

### 5. Cloudflare DNS

Alibaba CDN 创建 domain 后会给出 CNAME：

- Cloudflare DNS 新建 <code>media-dev</code> CNAME 指向 Alibaba CNAME。
- Proxy status 必须是 DNS only（灰云），不能再套 Cloudflare orange-cloud proxy/CDN。
- 等待 Alibaba 控制台显示 CNAME active，再验证 TLS 和 URL auth。

双层 proxy 会改变回源、缓存、签名和 purge 边界。参考
[Alibaba CNAME](https://www.alibabacloud.com/help/en/cdn/add-a-cname-record-for-a-domain-name) 和
[Cloudflare proxy compatibility](https://developers.cloudflare.com/dns/proxy-status/use-cases/)。

## GitHub Environment 与 PR 隔离

在 GitHub Repository → Settings → Environments 创建或使用 <code>main-staging</code>：

- 只允许 main 分支部署 job 使用该 Environment。
- 将所有 AccessKey、CDN signing key 放到 Environment secrets，不放 Repository secrets。
- Region、bucket、base URL、slot、TTL 可放 Environment variables；若现有 deploy 脚本统一从 secrets
  注入，也必须保持相同 environment protection。
- Workflow job 必须声明 <code>environment: main-staging</code>，并只把上表精确 allowlist 写入 run 专用
  <code>0600</code> env file。
- Preflight 和日志只输出“变量存在/验证通过”，不得输出 key、Authorization、signed URL 或完整 env file。
- PR Preview workflow 不引用这些 secrets。自动化测试使用进程内 fake provider。

若确需真实 PR Preview，创建独立 preview Ingest/Delivery/CDN domain、RAM identities、signing keys、
callback base、quota 和 lifecycle；绝不能复用 main staging credential。

## Migration 0061 部署顺序

<code>0061</code> 是 forward-only 媒体交付 cutover。旧 binary 只认识单对象删除，不能与新 multi-step
cleanup 长期混跑；completeness trigger 会阻止它错误标记完成，但旧 worker 会反复失败。因此使用维护窗口：

1. 保持 <code>MEDIA_RETENTION_GC_ENABLED=false</code>，暂停新 upload intent。
2. 等待至少 15 分钟 STS/intent TTL 加 10 分钟 callback safety buffer，或由数据库和 gateway 指标确认
   outstanding intent/callback 为零。
3. Drain/停止旧 API media deletion worker。
4. 完成双 bucket、RAM、CDN、DNS 和 GitHub Environment 配置。
5. 在 disposable/fresh database 先执行全部 migrations；再对 staging 执行 <code>0061</code>。
6. 部署只使用 V4 和 publication-aware resolver 的新版 binary。
7. Migration 会为既有 clean JPEG/PNG/WebP 排队处理；在其变体完成前资产不可交付，这是安全降级。
   Legacy GIF 保持 failed 并要求重传。
8. 观察 processing/cleanup backlog 和 dead letter；完成下面真实 smoke 后才恢复 upload intent。
9. 只有 DB reference、Forum Markdown、Ingest inventory、Delivery inventory 四项 reconciliation 均通过，
   才评估启用通用 GC。

`0061` 只能 forward-fix。新 backend container 一旦启动就可能已经执行 migration，部署失败时不得重启旧
backend；应停止新 processing/GC，保留新 container、publication/variant/job 现场和 migration ledger，
部署理解 multi-step cleanup 的修复 revision。不能回滚 schema、重新公开旧 URL，或让旧 worker 处理已经有
Delivery variants 的 job。已 quarantine 的 cleanup 由新版 worker 排空；frontend 可独立恢复旧静态 release。

## 部署 preflight 与真实验收

Main 部署脚本在停止旧 container 前运行 fail-closed 合成 preflight：

1. 严格解析 run 专用 `0600` env file，拒绝未知/缺失 key、同 bucket、重复 provider principal、非 HTTPS
   exact CDN origin、相同 Type-A 双 key、非 current slot 或非 300 秒 TTL。
2. 匿名 HEAD 两个 bucket 必须精确为 403；callback 必须经 HTTPS 可达并拒绝 unsigned body；Ingest caller
   必须成功取得仅允许随机 exact key PutObject 的 STS temporary credential。Preflight 用该 credential 首次
   写入 `uploads/deploy-smoke/` 后，再省略 `x-oss-forbid-overwrite` 尝试覆盖同一 key；只有 provider path rule
   返回 HTTP 409 `FileAlreadyExists` 才通过，最后由 Ingest caller 删除合成对象。客户端自己携带 header 不是
   防审核后换包的安全边界。
3. Delivery writer 用与 Rust runtime 同公式的 OSS V4 Authorization header PUT 一个固定微型静态 WebP
   到随机 `assets/deploy-smoke/<uuid>.webp`：HMAC-SHA256，scope 为
   `<date>/<region>/oss/aliyun_v4_request`，canonical URI 包含 bucket，query 为空，canonical headers 包含
   Content-Type、`x-oss-date`、`x-oss-content-sha256:UNSIGNED-PAYLOAD`、SHA-256 metadata 和
   `x-oss-forbid-overwrite:true`。同一 V4 principal 再以 HEAD/GET 核对长度、MIME、摘要和完整 bytes。
4. 对这个已由直连 GET 确认存在的 exact path，unsigned CDN GET 必须精确为 403；404 不能被当成 auth
   rejection。当前 `MEDIA_CDN_SIGNING_KEY_SLOT` 的 Type-A GET 必须为 200 且 bytes 完全相同。
5. CDN purge principal 以 POST 调用 `RefreshObjectCaches(ObjectType=File)`，再轮询
   `DescribeRefreshTaskById`；只有全部 task 为 `Complete` 才通过，`Refreshing` 有界重试，terminal/未知状态
   或十分钟 deadline 都失败。
6. 无论任一步成功或失败，`finally` cleanup 都尽力先提交/复用 purge，再 DELETE 随机 Delivery object；
   primary failure 不被覆盖，但任何 cleanup failure 都会显式让部署失败。

该脚本只有 Python 标准库、单请求八秒 timeout、64 KiB provider response 上限，并禁止 redirect。它不向
Actions 输出 provider body、Authorization、signed URL、purge task ID 或 object locator。V4 canonical
request 与固定时间签名同时由阿里云官方 canonical vector 和 Rust runtime delete vector 锁定；preflight
与 runtime 不允许出现签名版本漂移。Preflight 自动验证 Delivery writer、当前 Type-A 和 purge 的真实正向
权限，但仍不能证明两套 OSS credential 越权互斥、Browser CORS、篡改/过期 URL、Ingest object 不能经 CDN
读取或 publication processing 业务旅程；这些负向/业务项继续人工或 E2E 验收。

真实 staging smoke 使用不含 PII 的合成图片：

1. 请求 intent，检查 credential 只对应 Ingest exact key。
2. PutObject + callback 成功；默认策略下 upload 为 clean、deliveryState 为 processing，存在一次
   <code>media.upload.auto_approved</code> system audit，且 audit 不含 object key/URL/hash。
3. Processing 期间 URL endpoint 仍不可交付，Ingest 原图保持 private，Web 不把 clean 误报为 published。
4. 在隔离测试环境设置 <code>MEDIA_IMAGE_AUTO_APPROVAL_ENABLED=false</code> 并重启后，新 upload 保持 pending；
   owner preview 为 <code>private, no-store</code>，响应/DOM 无 aliyuncs host 和 object key。
5. 回退人工路径中，普通管理员不能自审；ADMIN 自审缺 recent-auth/confirmation 时拒绝。完成可信 preview
   + approve 后状态先 processing；file/PDF 即使 flag=true 仍不可批准。
6. 三个变体完成并原子 published；Type A URL 200。
7. 修改 auth_key、等待超过 300 秒分别得到 403；客户端重新拉 owner resource 获得新 URL。
8. Block 后立即不能签新 URL；一分钟内出现 purge task。
9. Delivery objects 被删除；有 hold 时 Ingest original 仍存在但不可公开。
10. 解除 hold 后 Ingest 删除，upload 进入 blocked/redacted。

Smoke object/row 使用显式测试标签并由同一 run cleanup；cleanup failure 必须告警。

## Key rotation

CDN Type A 双 key 轮换：

1. 在 CDN secondary slot 放入新 key。
2. 更新 <code>MEDIA_CDN_SECONDARY_KEY</code>，部署但仍用 primary，验证两边配置一致。
3. 设置 <code>MEDIA_CDN_SIGNING_KEY_SLOT=secondary</code> 并部署。
4. 验证新 URL；等待旧 URL TTL 300 秒和允许的 clock skew。
5. 将旧 primary 替换为下一把备用 key，再把 GitHub primary secret 更新为相同值。
6. 不得出现 CDN 与 backend 没有共同有效 key 的窗口。

OSS/CDN AccessKey 轮换采用“先加新、验证最小权限、切换 workload、撤销旧、审计访问”顺序。三套身份独立
轮换，不能因为其中一套泄露而整体复用同一新 key。

## Retention、GC 与 reconciliation

- Clean + published 不代表永久保存。通用 GC 只处理 approval/grace 已满、无 active
  binding/usage/draft reference 的 asset。
- GC 或 account purge 一旦选中已发布资产，必须在同一事务为每个 Delivery variant 排入 CDN purge 和
  Delivery delete，再排 Ingest delete；不能只删源对象而遗留可缓存派生物。
- Pending 不按年龄进入通用 clean GC；过期未 callback exact key 由 intent housekeeping 删除。
- Operational hold 只冻结 Ingest evidence。Block 时 CDN purge 和 Delivery delete 仍继续。
- Provider 成功后 upload 清除 locator/hash/size/MIME 等 storage metadata，只保留稳定 ID 和
  purpose-limited audit。
- CDN purge 使用 <code>RefreshObjectCaches</code> POST form 提交并持久化 provider task ID，再以
  <code>DescribeRefreshTaskById</code> 轮询；响应 task ID 集合必须与持久化请求集合完全一致，且只有全部
  <code>Complete</code> 才允许删除 Delivery object。
  <code>Refreshing</code> 每 30 秒重试且单个 task 最长等待 10 分钟，超时或 terminal failure 进入有界重试，
  最终 dead-letter；task ID 不进入 DTO 或日志。
- Account purge 先停止公开派生并排 durable cleanup；active hold 可以延迟 Ingest delete，但不能让
  account lifecycle 把缺失 job 当作完成。
- Processing worker 在 provider I/O 后发现资产已经离开 clean 状态时，必须重新武装该 immutable key 的
  purge/delete steps；即使旧 cleanup 已经 terminal，也不能让超时或过期 worker 的晚到 Delivery 写回变成
  orphan。Terminal blocked upload 可只重跑 Delivery cleanup，不得恢复 Ingest locator 或重新公开资产。

Reconciliation 必须分开运行、默认 dry-run、限速和幂等：

1. PostgreSQL publication 是否有完整、同 policy version 的三个 published variants。
2. Profile/promotion/draft binding facts 与 owner source row 是否一致。
3. 全部 retained Forum Markdown <code>yourtj-asset</code> 与 version-aware
   <code>asset_usages</code> 是否 exact match。
4. Ingest OSS Inventory 与 upload_intents/uploads 是否一致。
5. Delivery OSS Inventory 与 asset_variants 是否一致。
6. processing、cleanup、purge task 是否 lease-stuck/dead-letter。

使用
[OSS Inventory](https://www.alibabacloud.com/help/en/oss/user-guide/bucket-inventory) 和独立最小权限
inventory role；不要使用过宽的默认 role。Inventory 是快照，执行破坏性修复前必须 HEAD 当前 object。
无法确定 ownership、live reference 或 preservation policy 的 object 不自动删除。

## 监控与 SLO

至少监控：

- intent issued/expired/consumed、STS/callback latency/error；
- pending age、system auto-approval 数量/失败、preview、人工 approve/block/self-review；
- processing queue age、attempt、dead-letter、decode rejection code；
- publication incomplete/mismatch；
- CDN signed 2xx/403/5xx、hit ratio、back-to-origin、purge task；
- cleanup step queue/lease/dead-letter；
- Ingest/Delivery bytes、object count 和 inventory drift。

Block 后 signer 应立即停止；purge 应在 1 分钟内提交。Alibaba CDN purge 通常约 5–6 分钟传播，超过
10 分钟告警。五分钟签名 TTL 是 purge 失效传播期间的第二道边界，不替代 force purge。

## 故障处理

- **STS unavailable**：新上传 fail closed；检查 caller/role 双向授权、session policy grammar、时钟。
- **Callback 502**：若 gateway 无访问记录，检查 callback URL、DNS/TLS 和
  <code>callbackSNI=true</code>；若已有访问记录，检查签名、原始 path/body 和 intent metadata。
- **Pending preview 403/404**：browser 不应直读 OSS。检查调用的是 owner same-origin preview route，
  asset 是否仍 pending/owned/static raster；默认自动策略下正常 raster 已进入 processing，不再提供 pending preview。
- **Processing dead letter**：资产保持 clean 且 <code>deliveryState=failed</code>；不能手工写 published。
  检查 source digest/MIME、decoder limit、Delivery V4 permission、prevent-overwrite/HEAD，然后由
  recent-auth 的 RunOperations actor 使用 processing retry endpoint，填写具体 reason；确认 audit 后等待
  owner/queue 的 deliveryState 依次进入 processing、published。
- **SignatureDoesNotMatch**：runtime 或 deploy smoke 都应对照 OSS 错误中的
  CanonicalRequest/StringToSign，检查 V4 Region/service scope、包含 bucket 的 canonical URI、空 query、
  UTC timestamp、required x-oss headers 与 `UNSIGNED-PAYLOAD`；不得回退 V1。两者结果不一致时，以官方
  vector 和 Rust runtime vector 定位实现漂移。
- **CDN 403**：检查 Type A key slot、TTL、path、CDN CNAME 和 private-origin role。不要给 Delivery
  改 public-read。
- **Purge failure**：保持 quarantined，重试 durable step；不要先标 blocked 或删除 Ingest 以伪装完成。
- **Credential exposure**：创建新最小权限 credential、切换并验证、撤销旧值、审计 object/CDN access。
- **Public leak**：先停止签名、收紧 CDN/private origin、force purge；只在 Ingest hold 中保留必要证据。

## 官方依据

- [OSS V4 request signatures](https://www.alibabacloud.com/help/en/oss/developer-reference/recommend-to-use-signature-version-4)
- [OSS PutObject](https://www.alibabacloud.com/help/en/oss/developer-reference/putobject)
- [V1 to V4 upgrade](https://www.alibabacloud.com/help/en/oss/developer-reference/guidelines-for-upgrading-v1-signatures-to-v4-signatures)
- [Private OSS through CDN](https://www.alibabacloud.com/help/en/oss/allow-only-cdn-accelerated-domain-names-to-access-oss-resources)
- [CDN URL signing](https://www.alibabacloud.com/help/en/cdn/user-guide/configure-url-signing)
- [CDN Type A](https://www.alibabacloud.com/help/en/cdn/user-guide/type-a-signing)
- [CDN refresh/prefetch](https://www.alibabacloud.com/help/en/cdn/user-guide/refresh-and-prefetch-resources)
- [RefreshObjectCaches API](https://www.alibabacloud.com/help/en/cdn/developer-reference/api-cdn-2018-05-10-refreshobjectcaches)
- [DescribeRefreshTaskById API](https://www.alibabacloud.com/help/en/cdn/developer-reference/api-cdn-2018-05-10-describerefreshtaskbyid)
- [CDN HTTPS](https://www.alibabacloud.com/help/en/cdn/user-guide/configure-an-ssl-certificate)
- [CDN cache expiration](https://www.alibabacloud.com/help/en/cdn/user-guide/configure-the-cdn-cache-expiration-time)

## 上线签字清单

- [ ] Ingest/Delivery private bucket、Region、SSE、versioning、prevent-overwrite 已复核。
- [ ] Upload/Ingest/Delivery/CDN purge 四个权限边界通过正向和越权负向测试。
- [ ] Ingest 精确 CORS 已验证，Delivery direct OSS 仍为 403。
- [ ] Alibaba CDN private origin、HTTPS、Type A 双 key、300 秒、cache/purge 已验证。
- [ ] Cloudflare CNAME 为 DNS only，Alibaba 控制台显示 active。
- [ ] GitHub <code>main-staging</code> Environment 已保护；PR workflow 不引用 provider secrets。
- [ ] Main preflight 的 Delivery Put/Head/Get、同路径 unsigned 403、当前 Type-A exact body、purge Complete
      和 DELETE cleanup 已通过，Actions log 无 secret/provider body/task ID/object locator。
- [ ] Fresh migration、媒体 unit/integration/fake provider tests 通过。
- [ ] 默认 auto-approval 与关闭 flag 后的 preview/approve 回退路径都通过；两条路径随后均完成
      process → signed CDN → block → purge/delete smoke。
- [ ] DB/Forum/Ingest/Delivery reconciliation 无未处置 drift。
- [ ] Processing/cleanup/purge alerts、key rotation、incident owner 已演练。
- [ ] GC 仍为 false，或启用审批与证据已单独记录。

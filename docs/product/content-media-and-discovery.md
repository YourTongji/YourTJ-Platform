# 内容、媒体与发现

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Forum/Reviews/Media/Courses/Web maintainers、Product owner
>
> 最近核验：2026-07-14，`contract/openapi.yaml`、migrations `0061`、`0065` 与 owner-domain tests

本规范覆盖主题、评论、课评、Markdown、草稿、OSS 资产、互动状态、feed、聚合搜索和社区
推广位。核心原则是先定义内容与媒体边界，再选择编辑器和推荐算法。

## 当前状态

### Current

- 论坛已有板块、主题、评论、楼中楼字段、投票、收藏、订阅、标签、举报、修订、草稿、
  问答采纳、投票贴、mention/quote、敏感词和管理状态。
- 主题和评论的创建/编辑已经共享 plain-text 长度与结构校验、watched-word canonical policy；
  censor 后正文持久化，block 被拒绝，queue 内容不进入公开计数、activity、通知或搜索投影。
- 主题的 tags/poll/activity 与 canonical row 同事务创建；编辑 revision 与正文更新同事务并串行分配
  sequence；评论引用只允许同一主题下仍可用的评论。
- 主题列表返回 canonical tag slug，并支持板块内或全局 exact-tag filter；`subscriptions` feed
  使用 thread 订阅覆盖 board 订阅的明确优先级，muted 可覆盖 board watching/tracking。
- `following` feed 直接读取 `forum.user_follows`，按主题创建时间和 id 使用稳定 cursor；板块/tag
  过滤、active/suspension、block/mute 与公开内容状态在服务端生效，不再复用 subscription 语义。
- 所有主题列表返回同一 canonical Markdown 纯文本摘要及批量 viewer vote/bookmark state；首页与论坛页
  只展示这些服务端事实，不从数组位置或本地假数据推断摘要和互动状态。
- 主题列表、主题详情和评论返回作者当前 clean + published `thumb_256` avatar 的 typed Delivery
  projection；Forum 通过 Identity/Media owner API 批量投影，不跨域读表。Web 在首页、论坛页和详情页
  显示真实头像与资料链接，缺失时使用 handle 首字母；非首屏头像与内容图片默认 lazy-load。统一 API
  响应层按 `assetId + variant` 在内存复用仍距离过期超过 30 秒的 bearer URL，加载失败先淘汰精确旧 URL，
  再刷新对应 owning resource。
- 首页和论坛页以服务端 `nextCursor/hasMore` 继续当前 feed/filter，显式“加载更多”控件支持键盘和
  辅助技术；切换 feed、板块或标签会建立新的查询边界，不拼接旧条件的数据。
- 主题/评论详情返回当前用户 vote/bookmark 状态；主题同时返回有效 subscription、read position 和
  poll selections。vote、bookmark、subscription 和 poll vote 均有幂等撤销路径，计数与 activity
  在同一事务内校正。
- Profile 的主题/回复/媒体/喜欢和 owner 收藏列表返回统一的可见内容投影、viewer vote/bookmark state、
  回复数、赞数和授权后的 Forum attachment。媒体只接收 Media owner 确认的 current clean binding，
  喜欢按正向 vote 的更新时间稳定分页，收藏不会把已隐藏或已删除内容重新暴露。
- Comment `replyCount` 表示仍可见的直接子回复数；migration `0065` 回填现有数据并由 insert/delete 以及
  parent/visible-state 变更 trigger 增量校正，读路径不临时聚合整棵回复树。
- 阅读位置验证 comment 必须属于同一公开主题且只向前推进；传 null 会读到当前最后一条可见评论，
  unread count 从可见评论事实计算，不依赖可漂移的 reply counter。
- 课评有发布、编辑、点赞/取消、举报和审核基础。
- Media 后端已有 account-bound upload intent、短期 OSS STS、回调验签、MIME/大小限制和
  `pending/clean/quarantined/blocked` 审核状态；交付另有
  `unpublished/processing/published/failed/blocked` publication 状态。当前默认策略只对已验证 callback 的
  JPEG、PNG、WebP 以 system actor 自动批准并写审计，随后立即进入 fail-closed Delivery processing；这不等于
  内容安全扫描或公开原图。关闭 `MEDIA_IMAGE_AUTO_APPROVAL_ENABLED` 后恢复 pending + 可信 raster preview
  的人工队列；file/PDF 无论该开关如何都在 scanner 前 fail closed。
- Web 已使用 Alibaba 官方 Browser SDK 实现 direct-to-OSS 基础链路：先取 exact-key STS intent，
  本地计算 SHA-256，将服务端通用 Region ID 规范化为 SDK region，再由 OSS signed callback 取得
  canonical upload id；SDK 动态加载，不进入首屏包。
- 头像/封面设置已挂接上述链路，并按持久化 usage 查询 owner-safe 上传状态；默认自动策略下新 raster
  callback 直接显示 processing，关闭策略或不符合自动条件时 pending owner 预览仍走同源鉴权 blob。Web
  只在 `clean + published` 后允许绑定，并区分 pending、processing、failed、blocked；解除绑定不直接删除原始资产。
- 主题、评论和 revision 已显式持久化 `plain_v1/markdown_v1`；历史记录通过 migration 保持
  `plain_v1`，创建/编辑 API、详情 DTO、搜索与通知纯文本投影使用同一格式事实。
- Web 的发帖、回复和详情页已挂载 CodeMirror 6 编辑/预览与 react-markdown/unified + GFM + strict
  sanitizer renderer；Markdown 依赖随业务路由加载，不进入首屏主包。
- 发帖和回复使用 typed 云端 draft payload；900ms debounce 后按 `expectedVersion` compare-and-swap，
  UI 展示加载/保存/失败状态，跨设备冲突必须显式选择云端或当前版本，发布后幂等删除草稿。
- 已发布主题和评论从 `contentVersion=1` 开始；每次作者编辑携带 `expectedVersion`，canonical update、
  revision 和版本递增在同一事务完成。陈旧写入返回 `409 VERSION_CONFLICT` 及当前版本，Web 保留本地
  输入并让作者显式选择载入线上版本或基于最新版重试。
- Revision history 使用有界 cursor page。作者不因本人恰好是 mod/admin 而失去读取能力；staff 读取
  他人历史必须具备 `moderation.content`、目标非本人且作者角色严格更低，普通他人和同级/更高角色
  target 均拒绝。单页 attachment history 由 Media 一次 batch 解析，再逐 revision 与 AST exact-match。
- 主题列表/详情和评论 DTO 返回服务端计算的 `canEdit/canDelete/canModerate`。作者可用动作按当前
  内容/父主题状态计算，staff moderation 同时检查 capability、self target 和 `user < mod < admin`
  层级；Web 不再根据登录角色猜测内容按钮。
- 服务端使用 pulldown-cmark event stream 执行独立 profile：拒绝 raw HTML、非 HTTP(S)/站内链接和
  非平台或未绑定图片，限制节点、嵌套和链接数；mention 跳过代码节点，搜索/通知不再截取 raw Markdown。
- 新主题/评论的 mention handle 以有界集合交给 Identity 批量解析；Forum 按接收方
  `everyone/following/nobody` policy、recipient-follows-actor、双向 block、recipient mute 和通知偏好
  决定是否创建通知。拒绝或不存在的 `@handle` 不影响 canonical 正文，也不会变成语义链接或泄露
  策略；编辑历史内容不会重放 mention 通知。
- 主题最多 8 张、评论最多 4 张 clean + published 图片；canonical source 只接受带非空 alt 的
  `![alt](yourtj-asset:<assetId>)`。发布请求的 `attachmentAssetIds` 必须与 AST 中的引用按顺序完全一致，
  重复、遗漏、额外 id、远程 URL、data URI、pending/blocked 或他人资产均被服务端拒绝。
- Media 持有 `asset_usages`、owner/clean 校验和最小公开投影；Forum 在 canonical mutation 的同一事务调用
  owner API。公开 DTO 只返回匹配正文的 asset id/reference、alt、可选尺寸和派生 URL，不返回 Ingest key、
  bucket/provider host、独立持久 locator/hash 字段、owner 或 vendor destination URL；短期 CDN URL 自身
  包含可见的 immutable Delivery path。列表 DTO 最多带首张图供 feed card，详情才返回完整有界集合；
  投影与正文不一致时 fail closed，不披露多余 URL。
- 归档/隐藏保留 binding；作者删除、staff 删除和举报 uphold 会随内容 mutation 原子 detach 并进入 30 天
  grace。普通 restore 与申诉 overturn 都重新解析 canonical source、验证 owner/clean/version 后原子 rebind，
  避免治理撤销恢复正文却永久丢失图片。
- Comment 申诉恢复与所有 comment mutation 统一使用 thread→comment lock；parent visibility 只在 thread
  lock 后读取。并发 parent hide/delete 先完成时，恢复的 comment 继续保留 media binding，但不会错误
  恢复 activity/vote 投影或被旧 parent snapshot 当作公开内容。
- Thread 隐藏、软删除、归档、举报处置、排队编辑和自动归档统一在 source mutation 事务内同步整棵
  activity 子树：主题、全部评论及两类正向 vote 按稳定锁序停用；恢复只重启仍可公开的 canonical source。
- Web CodeMirror 已接 direct-to-OSS 图片上传、持久状态恢复、审核与 Delivery 状态、owner pending
  preview、引用插入和移除；pending 可保存在本人云端草稿，但发布按钮和服务端 binding 都要求
  clean + published。Processing 会轮询，failed 明确要求运维重试或用户重传。
- 课程/课评、论坛主题、用户与论坛 discovery object 各有最小化 Meilisearch 候选能力；独立
  `search` domain 返回 typed courses/reviews/threads/users/boards/tags，每类都由 owner domain 回
  PostgreSQL 重建并保持候选顺序，Web 综合页与全局搜索框可到达对应 canonical route。
- 课程管理、账号 handle/profile/privacy 和 board/tag mutation 会在提交后 reconcile 对应最小文档；
  后台 forum rebuild 同时重建 thread、identity user 与 forum discovery index。任务仍是进程内 202 job，
  没有 durable progress/retry，因此可靠投影闭环仍为 `Partial`。
- 首页左侧推广由 platform API 返回真实自营内容；支持两个 placement、状态/排期、受众、priority、
  optimistic version、独立 capability、reason/audit 和后台 UI。目标只接受安全站内相对路径，素材只
  接受当前操作者拥有的 clean image asset id，不保存图片 URL。公开列表为每次展示签发不含账号、IP
  或设备标识的两小时票据；卡片持续达到 50% 可见 500ms 才记曝光，点击会补齐可能丢失的曝光。同一
  票据的曝光/点击分别幂等；跨 UTC 零点的点击仍归入原 impression 的 UTC 日期，避免 click 被静默
  丢弃。后台提供 30 天汇总和最多 93 天 UTC 日趋势。公开 promotion DTO 由 Platform 先授权排期/受众，
  再通过 Media batch resolver 返回 nullable typed Delivery projection；匿名卡片不调用 owner URL route，
  响应带短期 bearer URL 时为 `private, no-store`。

### Partial

- Markdown 已在主题/评论上线、接通 versioned autosave 和 clean Forum 图片 binding；仍缺跨
  Web/iOS/Flutter conformance corpus，以及课评/公告各自更细的 syntax profile。
- queued 内容目前复用 hidden 状态，没有独立 pending 状态/审核任务；通知已经迁移到 durable outbox，
  搜索索引等事务外副作用仍没有统一可靠投递。
- drafts 的 OpenAPI、Rust response 和 Web 已统一为 bounded typed payload、完整对象响应、owner scope、
  50 条上限与 CAS version；已发布主题/评论也具备版本化编辑和服务端 viewer action 字段。
- 首页已移除固定摘要、按 index 伪造徽章和无行为的收藏/分享/筛选/频道按钮，并已接 canonical
  摘要、列表级 viewer state 与用户 following；`hot` 仍是热榜而不是个性化 recommended 模型。
- 个人主页和独立收藏页也不再保留占位动作：收藏/取消收藏调用幂等 Forum API，分享调用系统 share
  或复制 canonical URL，“更多”只展示可执行动作；请求失败保持错误态和重试入口，不显示伪造计数。
- Avatar/banner、主题/评论与推广素材已完成 owner 上传或 clean + published reference、业务事务内
  binding、Forum draft reference、私有 Delivery 变体、短期 signed CDN projection 和解绑 grace。
  Retention-aware GC 已有实现与测试但默认 rollout flag 关闭；课评/私信仍没有 binding，file/PDF scanner
  未完成，且目标环境双 bucket/CDN/provider reconciliation 仍需按 runbook 验收。
- Forum attachment、Forum content author avatar 与 Promotion 已保留 Delivery expiry/刷新语义；Web 对这些
  typed projection 按 asset/variant 有界复用 URL，并在临近过期、图片错误或登录主体变化时失效。
  Public profile/relationship/search/DM avatar 已统一投影 `thumb_256`，但与 account legacy field 一样，兼容
  DTO 仍只有 URL string，尚未统一 typed expiry 或全 surface 的定时刷新。该 gap 不允许通过延长签名 TTL、
  改 public-read 或恢复 direct OSS URL 规避。
- 聚合搜索已有六类 typed 结果、有效 type filter、独立 Web 综合结果页与全局搜索入口；高亮由 owner
  domain 回表授权后的 canonical text 计算 Unicode character ranges，Web 只拆分文本节点并用 `mark`
  渲染，不接受索引 HTML/snippet。拼写建议只从本页已授权 canonical fields 推导，候选歧义时不返回；
  仍缺拼音/别名，以及 transactional outbox 驱动的索引可靠更新。单类 cursor、all 页“查看更多”和
  局部失败状态已完成。
- 推广使用 media-owned 非版本化 binding，并在替换/归档时进入 30 天 grace；公开列表通过 owning-domain
  授权后的 typed Delivery projection 支持匿名 clean 图片交付，不再借用 owner-only URL endpoint。
  短期 URL 到期由列表定时刷新/错误重取；效果数据已按日聚合，但不等于商业广告归因或跨域画像。

## 内容类型与格式

历史内容不能突然被解释成 Markdown。所有支持格式化的内容增加显式 `contentFormat`：

| 内容 | 目标格式 | 能力边界 |
|---|---|---|
| 主题正文 | `plain_v1` 或 `markdown_v1` | 完整安全子集、代码、链接、列表、平台图片、预览和草稿 |
| 评论 | `plain_v1` 或受限 `markdown_v1` | 紧凑编辑器、引用/回复；限制大标题和复杂媒体 |
| 课评 | `plain_v1` 或受限 `markdown_v1` | 强调、列表、链接；图片是否开放需单独决策 |
| 私信 | `plain_v1` | 附件独立建模，不把 Markdown 当聊天协议 |
| 公告 | 受限 `markdown_v1` | 更严格链接/图片 policy，由 staff 发布并保留 revision |
| bio | `plain_v1` | 不允许 HTML、Markdown 图片或任意 embed |

旧记录保持 `plain_v1`。升级只能通过显式编辑/迁移生成新的格式版本，不能按正文字符猜测。

## Markdown 安全与编辑体验

- 存 canonical Markdown source；渲染 HTML 是可丢弃派生物。
- 禁 raw HTML；只允许明确标签/属性和 `http/https` URL。
- 外链使用 `noopener noreferrer nofollow ugc`，内部链接保留 canonical route。
- Markdown 图片只允许 clean 的本站 OSS/CDN asset，要求 alt text；拒绝 data URI 和任意远程图。
- mention、链接和引用从 AST 提取，不能对原始文本做会命中代码块的简单正则。
- 编辑预览与最终展示使用同一 renderer/profile；服务端仍独立执行输入验证和安全检查。
- 限制 AST 深度、节点数、链接数、图片数与最终输出尺寸，覆盖 XSS 和资源消耗攻击。
- Search document、notification excerpt、moderation preview 和 export 由同一 AST 生成有界纯文本，
  不直接截 raw Markdown、图片 URL 或链接目标。
- CSP、禁止第三方远程图片和最小化第三方脚本作为 defense-in-depth；sanitizer 仍是主边界。

编辑器选型以成熟维护、可访问性、移动端输入、paste/drop upload、插件隔离和可控输出为标准。
第一阶段使用 CodeMirror 6 源码编辑、react-markdown/unified + remark-gfm 渲染和 rehype-sanitize
二次约束；不启用 raw HTML。服务端用 pulldown-cmark 独立解析 canonical source；图片 destination
仅允许 `yourtj-asset:<正整数>`，并在同一事务验证引用集合、owner、usage 与 clean 状态。相关前端依赖
进入独立 `markdown-vendor` chunk；跨客户端
conformance corpus 仍是下一步，不能让任一客户端自行扩展语法。

Web、iOS 和 Flutter 可以使用不同 renderer，但 `markdown_v1` 的 allowed syntax、URL/image policy、
plain-text projection 与恶意样例必须由共享 protocol profile 和 conformance corpus 约束。任何客户端
未支持新 version 时按 plain-safe fallback 展示，不能自行扩展 raw HTML/embed。

## Link preview / Onebox

当前 Onebox 只接受标准端口 HTTPS，并在每次 redirect 前重新执行 domain allowlist、DNS 和
public-IP 检查；所有解析出的地址都必须为公网地址，请求固定到已验证地址并显式禁用系统代理，
避免代理侧重新解析 host 绕过 pin。body 以 512 KiB 流式上限读取；响应必须是精确的
`text/html` 或 `application/xhtml+xml`，缺失/非 HTML MIME 和非 UTF-8 charset 声明 fail closed，
声明 UTF-8 但含损坏字节时使用 replacement character 容错解码。HTML metadata 由 `html5ever`
上层的维护中 parser 解析，支持畸形 markup、属性乱序/大小写与 entity，字段在去控制字符、折叠
空白后有界截断。

输入 URL 通过标准 URL parser 规范化并移除 fragment，cache hash 包含 policy version。无 query 的
规范化 URL 可持久化：成功 cache 最长 7 天，错误仅 2 分钟；带 query 的 URL 不进入 PostgreSQL 或
成功 Redis cache，失败只保存不可逆 hash 的短期 Redis marker。Migration 会清除旧 query URL 和
历史远程图片字段。远程 `og:image` 始终不返回给客户端，日志只记录 URL hash、允许域名和错误类别。
响应契约固定返回 `type`（`plain` 或 `card`）、规范化 `url`、nullable `title`、`description`、
`imageUrl` 与 `siteName`；`imageUrl` 当前恒为 `null`，保留该字段只用于未来的平台 media proxy，客户端
不得把任意远程图片 URL 当作可信 preview asset。

受控本地 HTTPS fixture 已穿透同一生产状态机，覆盖证书 host/SNI、逐跳 allowlist/DNS/public-IP pin、
DNS rebinding、精确 MIME、Content-Length 与 chunked 流式超限、损坏 UTF-8/charset，以及单请求/总
deadline；测试只在 `cfg(test)` trust boundary 把已验证的合成公网 pin 映射到 loopback 并信任运行时
生成的测试证书，不调用公网，也不放宽生产策略。网络抓取边界为 `Current`；用户侧自动 link preview
仍为 `Partial`，因为 Web 尚未消费 `/onebox`。当前产品明确不展示远程预览图，因此 media proxy 不是
上线前置；若未来要显示图片，必须先接平台 asset/proxy。

目标规则：

- 只允许 `https`，每一跳 redirect 都重新验证 allowlist、解析后的 public IP 和 DNS rebinding。
- 拒绝 loopback、link-local、private、metadata 和非标准/危险端口；限制 redirect 次数和总 deadline。
- 使用流式 byte limit，在解析前拒绝超限 content length/body；charset/UTF-8 安全处理。
- 使用受维护 HTML parser 和有界 meta fields，不执行脚本、不加载页面子资源。
- `og:image` 不直接作为用户浏览器远程图片；经安全 media proxy/asset pipeline 或不展示。
- Cache key 包含规范化 URL/policy version，安全策略变化必须 bump version；缓存错误有短 TTL；所有
  SSRF/redirect/large-body 例子自动测试。
- 日志只记录 URL hash、允许的 host 和错误类别，不记录可能含 token/PII 的完整 URL 或页面正文。

## 创作与内容生命周期

创建和编辑必须调用同一 canonical pipeline：规范化 → 长度/结构验证 → asset ownership/clean
检查 → watched words/反滥用 → mention/quote 解析 → 事务写入 → outbox side effects。Mention 解析
只识别 visible text，最多处理 10 个去重 handle；策略拒绝是静默 side-effect 决策，不是内容校验失败。

当前主题/评论输入已经统一执行格式感知的长度/结构/Markdown AST、watched words 与引用约束，
canonical row/content format/revision/content version、主题附属数据和 Forum asset binding 具备事务边界；
显式内容 pending 状态和搜索/媒体等其余 side-effect outbox 仍是该目标管线的缺口；通知 outbox 已完成。

目标状态与行为：

- `draft`：仅作者可见，autosave 幂等，支持恢复和显式删除。
- `visible`：满足 board、账号和治理政策后公开。
- `pending`：等待自动或人工审核，不进入公开 feed/search。
- `hidden`：临时不可公开，证据和 revision 保留，可恢复。
- `soft_deleted`：普通读取排除，恢复窗内可恢复；不等于物理 purge。
- `archived`：不再接受新互动，但保留阅读上下文。
- `restored`：只有其他阻断状态都解除后才重新进入公开投影。

编辑使用 revision/version 防止两个设备静默覆盖。作者删除、管理员移除和 retention purge 是
不同动作，UI 文案和审计必须区分。

新增或改变 content format 时，必须同时核对 canonical row、revision/history、draft、public/admin
DTO、search/cache、notification/moderation excerpt、export 和 legacy client；不能只改详情页 renderer。

## 草稿和互动状态

- 编辑器以 900ms debounce 自动保存，显示检查/保存中/已保存/失败状态；关闭发帖面板时立即 flush。
- drafts list/get/save/delete 的 OpenAPI、Rust 响应和 Web 已统一；`expectedVersion=0` 只创建，正数
  只更新对应版本，冲突返回 409，不允许另一设备静默覆盖。
- 已发布内容的 legacy 编辑请求省略 `expectedVersion` 时只按版本 1 尝试；这允许尚未修改的历史内容
  安全迁移，但内容一旦改变就明确冲突，不提供隐式 last-write-wins。新客户端始终发送服务端返回版本。
- 当前断网输入会保留在已打开页面并允许重试，但尚无浏览器持久离线队列；产品不能把它描述为
  offline-first。
- 互动 DTO 返回 `viewerVote`、`isBookmarked`、`isLiked`、`mySubscriptionLevel`、
  `myVotes`、`canEdit`、`canDelete`、`canModerate`。
- vote、like、bookmark 和 subscription 支持幂等创建与删除；客户端可乐观更新但以服务端结果校正。
- 当前 forum 已实现 vote、bookmark、subscription、poll vote 的创建/删除和详情 viewer state；
  review like 继续使用独立 owner-domain 语义。
- Profile likes 是 Forum 正向 vote 的只读历史投影，不引入第二套 like 表；bookmarks 仍是 owner-private
  事实，profile media 是 authored content 与 Media current binding 的交集，不从正文字符串直接生成 URL。
- 楼中楼 UI 显示回复对象和引用上下文，不能把 materialized path 简单平铺成无关系列表。

## OSS 资产模型

业务表保存 `assetId` 或 attachment reference，不保存用户输入 URL。资产安全状态与业务绑定是两条
独立状态机，不能把“clean”误当成“已经用于某篇内容”：

```text
审核状态: intent -> uploaded/pending -> clean
                                   \-> quarantined -> blocked

交付状态: unpublished -> processing -> published
                             \-> failed     \-> blocked

绑定状态: unbound <-> bound -> gc_eligible -> garbage_collected
```

- 当前数据库有 intent、`pending/clean/quarantined/blocked`、moderation evidence、publication、variant
  processing job、ordered cleanup step、profile/推广非版本化 `asset_bindings`，以及 Forum 专用的
  version-aware `asset_usages` 和 `draft_asset_references`。Block 可从 pending 或 clean 立即停止签名并进入
  durable CDN purge → Delivery delete → Ingest delete；外部请求不持有 DB lock，失败可 retry/dead-letter。
  通用 GC 只选择 `cleaned_at` 已满 30 天、无 live usage/binding/draft reference、无 future grace、无 active
  operational hold 的 clean object，并在锁后重验；`pending` 永不因年龄自动删除。GC 与账号 purge system
  enqueue 默认 rollout-gated，代码合并或部署不等于目标环境已启用。File/PDF scan、课评/DM binding 与
  覆盖全部业务 surface 的 usage 仍为 `Planned`。
- Upload callback token 在 provider flow 外不保存明文；数据库只存 SHA-256 digest。Credential issuance
  在 account lock 下执行 10 active、rolling 24h 100 次、stored + reserved 512 MiB、live + active intent
  500、retained + active intent 2,000 的 PostgreSQL fail-closed limits，attempt fact 保留 48 小时。
- Upload object identity 必须不可变：Web PutObject 设置 forbid-overwrite header，生产 OSS 还必须按
  `uploads/` prefix + upload role 配置 server-side prevent-overwrite rule；仅信任客户端 header 不足以让
  callback 或人工 preview evidence 成为可靠事实。
- Profile 上传额外保存 intended usage，并提供 owner-only recent list/单项 poll；响应不暴露 object key、
  hash、owner id、provider error 或持久 object URL，刷新页面不会丢失审核/交付状态。
- Ingest 与 Delivery 是不同 private bucket 和不同最小权限 principal。Owner pending preview 与 staff
  moderation preview 都使用鉴权、`private, no-store` 的同源 bounded proxy；浏览器不能直接 GET Ingest。
- Raster approve 后 durable worker 通过 V4 读取 Ingest，校验 magic bytes/MIME/尺寸/像素/decoder allocation，
  归一 orientation，重新编码为移除 EXIF/GPS 的 WebP，并生成 `thumb_256`、`display_1280`、`full_2048`。
  三个变体全部 HEAD 验证后才原子 published。
- Public/campus resource 由 owner domain 先重验业务可见性，再取得 Media typed projection：asset id、
  variant、MIME、尺寸、五分钟 CDN bearer URL/expiry。通用 media URL route 仅限资产 owner，不能作为
  forum/profile/promotion 的跨账号授权后门；带 URL 响应不缓存到 expiry 之后。
- SVG/GIF/APNG/animated WebP 保持 fail closed；JPEG、PNG 和静态 WebP 是当前可处理输入。PDF、视频和
  一般文件必须先定义 scanner/sandbox、配额与保留政策。DM 资产未来使用更短、更严格的 viewer-bound
  private delivery，不能复用公开 CDN bearer 语义。
- Forum attachment 记录 owner、target type/id、position、alt 和绑定 content version；status 仍由 upload
  持有。创建/编辑与 binding 切换同事务，stale CAS 不留下 usage；revision 可按旧 content version 解析。
- Forum draft 保存本人 pending/clean asset 时同步 exact draft reference；发布时仍要重验 clean +
  published 并切换为 version-aware usage。Migration `0057` 在 backfill 前安装
  profile/promotion/draft source trigger，为旧
  source snapshot 提供引用完整性保护；同一 migration 删除 callback plaintext column，因此必须 drain
  全部旧 API/writer/worker。DB preflight、published Markdown usage 和 OSS object reconciliation 全部通过前
  不启用 GC。
- 软删除将 active usage 标记为 `target_deleted` 并设置 30 天后才可进入 GC 候选，不直接删除 OSS object；
  staff restore 在同一锁序重新解析 canonical source 并重验 owner/clean 后 rebind。启用后的 GC 仅在
  approval age 满 30 天、无 active reference、无未结束 grace、无 active operational hold 时排队；候选
  锁后再次验证，失败发布或并发 rebind 不会留下错误 usage，也不会删除刚恢复的 object。
- `operations.jobs` 管理员可在 recent-auth 后为 object 设置 5 分钟至 365 天的
  moderation/security operational hold；创建、续期/替换和解除使用 `expectedHoldId` compare-and-set。
  Hold 只暂停删除、不恢复公开访问；一旦 provider job 已 leased/succeeded 就拒绝新 hold。完整
  reason/kind/actor 只进入 recent-auth、`private, no-store` 的 operations inventory，普通审核 queue 只见
  通用状态。真正 legal hold 仍为 `Planned/Decision needed`。
- 只有当前 ADMIN 可对本人 raster upload 使用自审例外；每次 preview/approve/block 都要求 recent-auth、
  reason、`selfReviewConfirmed` 和 `selfReview` audit。Approve 还要求同一 reviewer 的可信 preview evidence；
  self-block 是收紧可见性的 fail-closed 操作，不要求可能已不存在的 pending preview。该例外不绕过
  decoder、variant completeness、cleanup、retention，也不扩展到 moderator/委派管理员或其他治理对象。
- 未 callback 的过期 intent 由独立 housekeeping 按 exact object key 排队，不把 pending age 当代理；已
  消费 credential 满 30 天删除。Provider 删除成功后 redaction 清除 object key/URL/hash/size/MIME/usage/
  dimensions，保留稳定 upload id 和 purpose-limited audit 引用。
- Independent housekeeping 还清理 expiry+1 天的 preview grant、grace 已结束的 detached binding、48 小时
  credential-attempt fact。Synthetic exact-key cleanup 的 redacted tombstone 至少保留 30 天；存在 hold/
  operator retry history 时等待相应 365 天记录清理，succeeded job 可先删或随 tombstone cascade。
- Account purge 对无共享引用/future grace 的 owner object 先 quarantine 并 durable enqueue，即使 active
  operational hold 也不例外；hold 只暂停 provider worker。共享引用/future grace 不排队。只有没有更多
  eligible work、queued/leased/dead-letter 或缺失 job 时才 terminal；held object 有 job 可暂留，held
  quarantined object 缺 job 必须阻断。Operations-history purge 默认关闭且不清 append-only governance audit。
- Raster 的受限解码、metadata stripping 与变体生成已交付；内容安全自动扫描仍未交付。当前风险接受是
  对新 JPEG/PNG/WebP callback 默认跳过人工内容审核，但原图保持 private，只有 worker 验证、解码并生成完整
  sanitized variants 后才可绑定或签发 URL；每次自动决定使用 system actor 审计。该策略可通过
  `MEDIA_IMAGE_AUTO_APPROVAL_ENABLED=false` 回退人工可信预览，file/PDF 不允许借该开关或人工操作绕过 scanner。

## Feed 语义

| Feed | 输入 | 目标 |
|---|---|---|
| latest | 当前可见内容的时间序 | 可预测的新内容浏览 |
| hot | 有时间衰减的互动/回复信号 | 发现正在发生的高质量讨论 |
| subscriptions | 用户订阅的 board/thread | 跟进已选择的话题 |
| following | 用户 follow 的作者 | 关系驱动，依赖 follow graph |
| recommended | 明确且可解释的多信号排序 | P2；必须经过隐私、block/mute 和治理过滤 |

Feed 卡片只显示真实作者、当前可交付头像、正文摘要、asset、viewer state 和计数。头像缺失或暂不可交付
时使用确定性的文字 fallback；无法取得的字段显示中性空态，不得按数组位置伪造等级、徽章或内容。

## 聚合搜索

统一搜索返回稳定 typed sections，例如 courses、reviews、threads、users、boards/tags；每类有
自己的 cursor/limit、canonical id、display fields 和 highlight。综合页支持类型 tab、查看更多、
过滤、拼音/别名/纠错、加载/无结果/局部失败状态。

- `type` 过滤在查询和响应两端生效，不能取全量后在 Web 隐藏。
- 高亮只能使用 owner-rehydrated canonical field 的半开 Unicode character ranges；不得向客户端传
  Meilisearch HTML/snippet。客户端必须渲染可信文本节点，越界、重叠或过量 ranges fail closed。
- 拼写建议必须保守且隐私安全：只使用本次响应中已通过 owner 授权的 canonical words，歧义时返回 null；
  不能从被过滤命中的索引文档、私有内容或搜索日志补全建议。拼音/别名尚未实现。
- courses/reviews/threads 的 DTO 与对应详情模型有明确映射，前端不把 minimal hit 当完整对象。
- user search 遵守 discoverability、block 和账号状态。
- 评论正文是否成为独立 hit 为 `Decision needed`；推荐以 comment hit + parent thread context +
  canonical deep link 建模，并同时验证 comment、thread 和 board 可见性，而不是只把评论文本塞进 thread。
- index 更新使用 transactional outbox/idempotent consumer；full reindex clear 完成后再 add。
- PostgreSQL 重验权限、status、deleted/hidden/archived 状态；索引文档不能成为隐私事实源。
- 查询日志最小化、限定保留；是否用于推荐属于 `Decision needed`。

当前 `/api/v2/search` 支持 `course | teacher | review | thread | user | board | tag | all`，其中
teacher 是课程文档的检索入口，不产生不完整的独立 teacher DTO；query 长度 2–100，单类 limit
1–30。非 `all` 查询返回与规范化 query/type 绑定的 opaque cursor，并在最多 240 条可见结果的窗口内
用 lookahead 准确声明 `hasMore`；错误 query/type、篡改 cursor 和越界窗口均返回 400。`all` 是每类
最多 6 条的概览，用 `hasMoreScopes` 驱动单类“查看更多”，不使用一个游标混合推进六个不同排名。
`failedScopes` 只声明局部不可用分类，不暴露内部错误，也不把失败伪装成“没有结果”。

User 结果只允许已验证、discoverable、当前 viewer 可见且未被 block/mute 的 active 账号；board/tag
返回当前公开对象和实时计数。索引内部
`course-<id>` / `review-<id>` 前缀不得出现在 HTTP 结果，隐藏课评以及 hidden/deleted/archived/pending
主题即使仍有陈旧 hit 也必须在回表阶段丢弃。

## 社区推广位

推广位已经改为后台模型和 API 驱动，包含 placement、title、description、CTA、target URL、
image asset、starts/ends at、priority、audience、status、created/updated by 和 audit reason。

- 初始 placement 明确为 `home-left-primary`、`home-left-secondary`；新增 placement 先定义尺寸、
  fallback、响应式行为和无推广空态。
- 状态机为 `draft -> scheduled -> published -> paused -> archived`，排期结束自动不再返回，但保留审计。
- 同一 placement 可有多个候选，但返回顺序固定为 priority desc、startsAt desc、id asc；priority
  相同也不能依赖数据库自然顺序。Web 每个 placement 只渲染排序后的首个候选；排期重叠预警仍为
  `Partial`，运营发布前需要检查列表中的 placement、有效期和 priority。
- 第一阶段 URL 只允许 `/` 开头、非 `//`、非 `/api` 的站内应用路径；商业外链未开放。图片必须是
  当前操作者拥有的 clean image asset，业务只保存 asset id。
- 排期和优先级由后端决定，Web 只渲染当前 placement 的已发布结果。
- 第一阶段仅允许自营校园/社区信息；商业广告需新的合规、审核与计费决策。
- 只记录必要的按日聚合 impression/click，不采集跨站追踪标识。

## Decision needed

- 课评和公告各自开放的 Markdown 子集与图片数量。
- DM/private attachment 的 viewer-bound delivery、原始 Ingest 保留期和 legal-hold release policy；
  公共资源使用 private Delivery origin + 五分钟 signed CDN URL 已确定。
- 默认搜索排序、用户搜索可见性和查询日志保留。
- 是否索引评论正文、结果展示的上下文长度和已删除/折叠回复 deep-link 行为。
- 是否在未来增加独立 repost/quote 内容类型；第一阶段系统 share/复制 canonical URL 已确定并实现，
  不以“分享”按钮暗中创建内容事实。

## 验收基线

- create/edit 使用同一 policy，无法通过编辑绕过长度、watched words、asset 或 mention 规则。
- Disallowed、未知、inactive、suspended、self 或 blocked mention 保持 literal text，不生成通知；
  `following` 只在接收方关注作者时成立，解析与授权无逐 handle 跨域查询。
- 两个设备以同一版本编辑时至多一个提交成功；失败请求不留下 revision、版本或 canonical 半状态，
  冲突恢复期间本地输入可访问且不会被 refetch 静默替换。
- Revision endpoint 对 author/user/mod/admin 层级、self/peer/higher target、非法 cursor 和 0/101 limit
  有 handler→PostgreSQL 负向测试；两页拼接不重复、不跳项，historical attachment batch 保持版本正确。
- 旧纯文本显示不变；Markdown preview 与最终输出一致且通过 XSS/资源限制测试。
- 所有 UGC 图片来自 clean + published asset；owner pending preview 不泄露 provider locator，公开 URL
  由 owner domain 授权后短期签发，删除/替换/处理失败路径无永久孤儿或越权 URL。
- Profile media 对正文 reference 与 current binding 做 exact-match；likes/bookmarks 重新应用可见性和
  relationship policy，回复数/赞数来自写入维护的 canonical projection，不在 Web 伪造或读时全表聚合。
- Promotion 的匿名图片由 Platform 授权后批量解析 typed Delivery，不调用 owner-only generic URL；短期
  URL 到期或首次加载失败可重新获取且 response 不被共享缓存。
- viewer state 支撑所有互动的 active/cancel 行为，计数和缓存被正确校正。
- 聚合搜索的 OpenAPI、Rust、index、Web shape 一致，隐藏内容和隐私对象无法泄露。
- Feed 与推广卡不包含伪造字段或装饰性不可用按钮。

# 内容、媒体与发现

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Forum/Reviews/Media/Courses/Web maintainers、Product owner
>
> 最近核验：2026-07-12，`contract/openapi.yaml` 与 owner-domain tests

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
- 主题/评论详情返回当前用户 vote/bookmark 状态；主题同时返回有效 subscription、read position 和
  poll selections。vote、bookmark、subscription 和 poll vote 均有幂等撤销路径，计数与 activity
  在同一事务内校正。
- 阅读位置验证 comment 必须属于同一公开主题且只向前推进；传 null 会读到当前最后一条可见评论，
  unread count 从可见评论事实计算，不依赖可漂移的 reply counter。
- 课评有发布、编辑、点赞/取消、举报和审核基础。
- Media 后端已有 account-bound upload intent、短期 OSS STS、回调验签、MIME/大小限制和
  `pending/clean/blocked` 审核状态。
- Web 已使用 Alibaba 官方 Browser SDK 实现 direct-to-OSS 基础链路：先取 exact-key STS intent，
  本地计算 SHA-256，再由 OSS signed callback 取得 canonical upload id；SDK 动态加载，不进入首屏包。
- 头像/封面设置已挂接上述链路，并按持久化 usage 查询 owner-safe 上传状态；Web 只在 `clean` 后允许
  绑定，`pending/blocked` 分别展示待审核/未通过，解除绑定不删除原始资产。
- 主题、评论和 revision 已显式持久化 `plain_v1/markdown_v1`；历史记录通过 migration 保持
  `plain_v1`，创建/编辑 API、详情 DTO、搜索与通知纯文本投影使用同一格式事实。
- Web 的发帖、回复和详情页已挂载 CodeMirror 6 编辑/预览与 react-markdown/unified + GFM + strict
  sanitizer renderer；Markdown 依赖随业务路由加载，不进入首屏主包。
- 发帖和回复使用 typed 云端 draft payload；900ms debounce 后按 `expectedVersion` compare-and-swap，
  UI 展示加载/保存/失败状态，跨设备冲突必须显式选择云端或当前版本，发布后幂等删除草稿。
- 已发布主题和评论从 `contentVersion=1` 开始；每次作者编辑携带 `expectedVersion`，canonical update、
  revision 和版本递增在同一事务完成。陈旧写入返回 `409 VERSION_CONFLICT` 及当前版本，Web 保留本地
  输入并让作者显式选择载入线上版本或基于最新版重试。
- 主题列表/详情和评论 DTO 返回服务端计算的 `canEdit/canDelete/canModerate`。作者可用动作按当前
  内容/父主题状态计算，staff moderation 同时检查 capability、self target 和 `user < mod < admin`
  层级；Web 不再根据登录角色猜测内容按钮。
- 服务端使用 pulldown-cmark event stream 执行独立 profile：拒绝 raw HTML、非 HTTP(S)/站内链接和
  未绑定图片，限制节点、嵌套和链接数；mention 跳过代码节点，搜索/通知不再截取 raw Markdown。
- 课程/课评、论坛主题、用户与论坛 discovery object 各有最小化 Meilisearch 候选能力；独立
  `search` domain 返回 typed courses/reviews/threads/users/boards/tags，每类都由 owner domain 回
  PostgreSQL 重建并保持候选顺序，Web 综合页与全局搜索框可到达对应 canonical route。
- 课程管理、账号 handle/profile/privacy 和 board/tag mutation 会在提交后 reconcile 对应最小文档；
  后台 forum rebuild 同时重建 thread、identity user 与 forum discovery index。任务仍是进程内 202 job，
  没有 durable progress/retry，因此可靠投影闭环仍为 `Partial`。
- 首页左侧推广由 platform API 返回真实自营内容；支持两个 placement、状态/排期、受众、priority、
  optimistic version、独立 capability、reason/audit 和后台 UI。目标只接受安全站内相对路径，素材只
  接受当前操作者拥有的 clean image asset id，不保存图片 URL。

### Partial

- Markdown 第一阶段已在主题/评论上线并接通 versioned autosave，但仍缺 clean asset 图片插入/绑定、
  跨 Web/iOS/Flutter conformance corpus，以及评论/课评/公告各自更细的 syntax profile。
- queued 内容目前复用 hidden 状态，没有独立 pending 状态/审核任务；搜索/通知等事务外副作用仍没有
  durable outbox。
- drafts 的 OpenAPI、Rust response 和 Web 已统一为 bounded typed payload、完整对象响应、owner scope、
  50 条上限与 CAS version；已发布主题/评论也具备版本化编辑和服务端 viewer action 字段。
- 首页已移除固定摘要、按 index 伪造徽章和无行为的收藏/分享/筛选/频道按钮，并已接 canonical
  摘要、列表级 viewer state 与用户 following；`hot` 仍是热榜而不是个性化 recommended 模型。
- Avatar/banner 已完成 owner 上传、审核状态恢复、clean binding 和解除绑定；主题/评论等 UGC 仍没有
  asset binding，且 scanner/变体/EXIF/GC 尚未完成，因此 profile 子链路完成不等于媒体产品闭环。
- 聚合搜索已有六类 typed 结果、有效 type filter、独立 Web 综合结果页与全局搜索入口；仍缺每类
  cursor、highlight/纠错、局部失败，以及 transactional outbox 驱动的索引可靠更新。
- 推广尚无通用 `asset_usages` binding/GC、匿名 clean 图片交付和按日聚合 impression/click；这些缺口
  不影响无图片卡片和登录用户通过 media 授权 URL 显示 clean asset。

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
二次约束；不启用 raw HTML。服务端用 pulldown-cmark 独立解析 canonical source，当前图片一律拒绝，
直到 clean asset binding 可在同一事务验证。相关前端依赖进入独立 `markdown-vendor` chunk；跨客户端
conformance corpus 仍是下一步，不能让任一客户端自行扩展语法。

Web、iOS 和 Flutter 可以使用不同 renderer，但 `markdown_v1` 的 allowed syntax、URL/image policy、
plain-text projection 与恶意样例必须由共享 protocol profile 和 conformance corpus 约束。任何客户端
未支持新 version 时按 plain-safe fallback 展示，不能自行扩展 raw HTML/embed。

## Link preview / Onebox

当前 Onebox 已只接受标准端口 HTTPS，并在每次 redirect 前重新执行 domain allowlist、DNS 和
public-IP 检查；所有解析出的地址都必须为公网地址，请求固定到已验证地址，body 以 512 KiB
流式上限读取并用容错 UTF-8 解码。远程 `og:image` 不返回给客户端，cache hash 包含 policy version，
日志只记录 URL hash、允许域名和错误类别。

该能力仍为 `Partial`：OG metadata 仍使用有界正则提取而不是维护中的 HTML parser；尚无安全
media proxy、规范化 URL/短期错误缓存和完整的网络 fixture 测试。Markdown 自动 link preview
必须等这些剩余边界完成后才可默认开启。

目标规则：

- 只允许 `https`，每一跳 redirect 都重新验证 allowlist、解析后的 public IP 和 DNS rebinding。
- 拒绝 loopback、link-local、private、metadata 和非标准/危险端口；限制 redirect 次数和总 deadline。
- 使用流式 byte limit，在解析前拒绝超限 content length/body；charset/UTF-8 安全处理。
- 使用受维护 HTML parser 和有界 meta fields，不执行脚本、不加载页面子资源。
- `og:image` 不直接作为用户浏览器远程图片；经安全 media proxy/asset pipeline 或不展示。
- Cache key 包含规范化 URL/policy version，缓存错误有短 TTL；所有 SSRF/redirect/large-body 例子自动测试。
- 日志只记录 URL hash、允许的 host 和错误类别，不记录可能含 token/PII 的完整 URL 或页面正文。

## 创作与内容生命周期

创建和编辑必须调用同一 canonical pipeline：规范化 → 长度/结构验证 → asset ownership/clean
检查 → watched words/反滥用 → mention/quote 解析 → 事务写入 → outbox side effects。

当前主题/评论输入已经统一执行格式感知的长度/结构/Markdown AST、watched words 与引用约束，
canonical row/content format/revision/content version 和主题附属数据也具备事务边界；asset binding、
显式 pending 状态和 durable outbox 仍是该目标管线的缺口。

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
- 楼中楼 UI 显示回复对象和引用上下文，不能把 materialized path 简单平铺成无关系列表。

## OSS 资产模型

业务表保存 `assetId` 或 attachment reference，不保存用户输入 URL。资产安全状态与业务绑定是两条
独立状态机，不能把“clean”误当成“已经用于某篇内容”：

```text
安全状态: intent -> uploaded/pending -> clean
                                  \-> blocked -> object removed

绑定状态: unbound <-> bound -> gc_eligible -> garbage_collected
```

- 当前数据库有 intent、`pending/clean/blocked` 和 profile reference binding，但没有自动 scan、通用
  `asset_usages` 或 GC；后者仍为 `Planned`，不能因 profile 子链路存在声称完整 OSS 产品链路。
- Profile 上传额外保存 intended usage，并提供 owner-only recent list/单项 poll；响应不暴露 object key、
  hash、owner id 或持久 object URL，刷新页面不会丢失审核状态。
- 推荐 private bucket；owner/staff 获得短期 pending preview，公开内容只引用 clean asset。最终
  bucket/CDN/signing 策略仍是 `Decision needed`，上线前按
  [媒体存储 runbook](../operations/media-storage.md) 验证。
- clean asset 通过受保护 CDN/origin 或签名策略访问；DM 资产使用更严格的短期授权 URL。
- 上传后校验 magic bytes、MIME、尺寸/像素、文件大小；图片移除 EXIF/GPS 并生成响应式变体。
- SVG 保持禁用；GIF、PDF、视频和一般文件分别制定格式、扫描、配额和保留政策。
- attachment 记录 owner、target type/id、position、alt、status；绑定与业务发布在可恢复事务流程中。
- 替换头像、删除内容和失败发布解除引用；后台任务清理过期 intent、孤儿和超出保留期对象。
- 自动扫描处理正常图片，人工队列只处理命中、失败或举报，不要求逐图人工批准。

## Feed 语义

| Feed | 输入 | 目标 |
|---|---|---|
| latest | 当前可见内容的时间序 | 可预测的新内容浏览 |
| hot | 有时间衰减的互动/回复信号 | 发现正在发生的高质量讨论 |
| subscriptions | 用户订阅的 board/thread | 跟进已选择的话题 |
| following | 用户 follow 的作者 | 关系驱动，依赖 follow graph |
| recommended | 明确且可解释的多信号排序 | P2；必须经过隐私、block/mute 和治理过滤 |

Feed 卡片只显示真实作者、正文摘要、asset、viewer state 和计数。无法取得的字段显示中性空态，
不得按数组位置伪造等级、徽章或内容。

## 聚合搜索

统一搜索返回稳定 typed sections，例如 courses、reviews、threads、users、boards/tags；每类有
自己的 cursor/limit、canonical id、display fields 和 highlight。综合页支持类型 tab、查看更多、
过滤、拼音/别名/纠错、加载/无结果/局部失败状态。

- `type` 过滤在查询和响应两端生效，不能取全量后在 Web 隐藏。
- courses/reviews/threads 的 DTO 与对应详情模型有明确映射，前端不把 minimal hit 当完整对象。
- user search 遵守 discoverability、block 和账号状态。
- 评论正文是否成为独立 hit 为 `Decision needed`；推荐以 comment hit + parent thread context +
  canonical deep link 建模，并同时验证 comment、thread 和 board 可见性，而不是只把评论文本塞进 thread。
- index 更新使用 transactional outbox/idempotent consumer；full reindex clear 完成后再 add。
- PostgreSQL 重验权限、status、deleted/hidden/archived 状态；索引文档不能成为隐私事实源。
- 查询日志最小化、限定保留；是否用于推荐属于 `Decision needed`。

当前 `/api/v2/search` 支持 `course | teacher | review | thread | user | board | tag | all`，其中
teacher 是课程文档的检索入口，不产生不完整的独立 teacher DTO；query 长度 2–100，单类 limit
1–30。user 结果只允许已验证、discoverable、当前 viewer 可见且未被 block/mute 的 active 账号；
board/tag 返回当前公开对象和实时计数。索引内部
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

- 评论、课评和公告各自开放的 Markdown 子集与图片数量。
- private bucket、CDN origin protection、签名 URL 和原图保留期。
- pending asset 是先发布占位还是 clean 后发布；建议 clean 后绑定公开内容。
- 默认搜索排序、用户搜索可见性和查询日志保留。
- 是否索引评论正文、结果展示的上下文长度和已删除/折叠回复 deep-link 行为。
- share 第一阶段只复制 canonical URL，还是建立独立 repost/quote 内容类型；建议先做前者。

## 验收基线

- create/edit 使用同一 policy，无法通过编辑绕过长度、watched words、asset 或 mention 规则。
- 两个设备以同一版本编辑时至多一个提交成功；失败请求不留下 revision、版本或 canonical 半状态，
  冲突恢复期间本地输入可访问且不会被 refetch 静默替换。
- 旧纯文本显示不变；Markdown preview 与最终输出一致且通过 XSS/资源限制测试。
- 所有 UGC 图片来自 clean asset，删除/替换/失败路径无永久孤儿或越权 URL。
- viewer state 支撑所有互动的 active/cancel 行为，计数和缓存被正确校正。
- 聚合搜索的 OpenAPI、Rust、index、Web shape 一致，隐藏内容和隐私对象无法泄露。
- Feed 与推广卡不包含伪造字段或装饰性不可用按钮。

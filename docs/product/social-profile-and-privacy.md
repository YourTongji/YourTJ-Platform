# 个人资料、社交图与隐私

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Forum/Identity/Web maintainers、Privacy owner、Product owner
>
> 最近核验：2026-07-13，migrations `0034`、`0044`、`0045`、`0050`、`0054`、`0061`、`0064`、`0065` 与 owner-domain tests

本规范定义公开资料、用户关注、板块/主题订阅、block、mute、资料隐私和徽章语义。目标是
建立可解释的社交关系，而不是把所有关系都命名为 “following” 或 “屏蔽”。

## 当前状态

### Current

- Identity 持有 display name、院校、bio、HTTPS website、clean + published avatar/banner asset reference 和独立
  profile/list/discoverability/DM policy；任意头像 URL 已停止写入并在 migration 中清除。
- 院校是 owner 可编辑的公开资料字段，现有和新建资料默认“同济大学”，只在 profile visibility 允许后
  返回；设置页限制为去除首尾空白后的 1–100 字符，owner export 包含原值，账号 purge 删除资料行。
- Forum 的主题、评论、私信会话/消息及治理举报投影同时返回可选 display name 与不可变 canonical
  handle；主题列表、主题详情和评论还通过 Identity public account batch 与 Media batch resolver 返回
  当前 clean + published 的 typed avatar Delivery，Web 同时展示头像、名称与 `@handle`，并在 URL 临近
  到期或首次加载失败时刷新 owning resource。关注流的候选过滤继续复用 Identity public account
  projection，并完整传递其 display name。
- Web 资料设置已接 profile-specific OSS 上传、owner-only 状态恢复与轮询；默认自动策略下新 raster
  callback 直接进入 processing，策略关闭或不符合自动条件时 pending owner preview 仍走同源鉴权 blob。
  只有 clean + published 才出现绑定操作，当前头像/封面可解除绑定。
- 资料响应包含 owner-domain 授权后的五分钟 signed Delivery URL、角色、信任等级、徽章、主题/回复/获赞
  与准确 followers/following 计数；动态 API 默认 `private, no-store`，URL 不是持久字段。
- 公开资料把成就徽章、实时角色标识与人工身份/特殊认证分开；人工认证只返回仍有效、类型允许公开且
  grant 明确 `displayOnProfile` 的 label/说明/受控图标/有效期，不返回签发人、原因或证据引用。
- Platform 持有 versioned 成就定义、可撤销/可重新授予的账号成就和 append-only 事件历史；图标使用
  受控 token。自动贡献规则首次授予可幂等排队 mint，人工授予永不 mint，撤销也不改写历史积分。
- Forum 持有公开单向 follow、私密单向 mute 和双向安全边界 block；follow/unfollow/mute/block 都幂等，
  relationship API 一次返回页面操作所需状态。
- 资料 owner 可以从自己的关注者列表幂等移除一条 incoming follow；该操作不会创建 block，对方以后
  仍可重新关注，界面必须明确说明这一后果差异。
- followers/following 使用稳定游标，读取时过滤 suspended/deleted、viewer block 和第三方
  `discoverable=false` 账号。
- 创建 block 与同一用户对的 follow 串行化，在同一事务移除双方 follow；数据库 trigger 维护计数，
  解除 block 不恢复关系。
- Profile、资料内容列表、feed、通知、新 DM、评论/引用回复和投票使用统一 block/mute 规则；mute
  不改变资料访问、follow 或直接互动权限。
- DM `everyone` 只允许未被接收方关注的新联系人进入单消息请求箱，不等于陌生消息直接送达；
  `following` 只允许接收方已关注用户直接送达，`nobody` 阻止新会话和 pending request 接受。
- board/thread 支持 watching、tracking、muted subscription；thread direct subscription 覆盖 board
  fallback，删除 direct override 后恢复 board 语义，列表和 feed 使用稳定 cursor。
- 用户搜索以 `discoverable` 和已验证校园身份作为候选门槛，响应时重新应用 active/suspension、
  profile visibility、block/mute 与 clean avatar policy；匿名只看 public profile，校园登录用户可看
  public/campus，`only_me` 和 `discoverable=false` 不进入第三方搜索结果。
- Activity visibility 已独立于 profile visibility：本人始终可看；`public` 允许匿名、`campus` 要求
  登录校园账号、`only_me` 只允许本人。Profile DTO 返回 viewer-specific `canViewActivity`，Web 在
  无权限时展示明确私密状态而不重复请求逐条列表；主题、回复、媒体和喜欢 tabs 复用同一 gate。
- Profile media 只列出本人创作、当前可见且正文 exact binding 仍解析为 clean + published 图片的 Forum
  内容；likes 只列出本人仍有效的正向 Forum vote，并在读取时重新应用内容状态、账号生命周期和
  block/mute。收藏是 owner-only tab，返回仍可见内容的最小完整投影，不向第三方开放。
- Profile 主题、回复、媒体、喜欢和收藏卡片统一使用真实回复数、赞数、viewer bookmark state 与短期
  Media projection；收藏/取消收藏幂等写回后端，分享优先调用系统 share、否则复制 canonical URL，
  “更多”只提供真实的打开内容和复制链接动作。请求失败显示可重试错误，不以 `0` 冒充钱包余额。
- Profile 上的主题、回复和获赞 aggregate 继续表示公共内容贡献总量，只要 profile 本身可见就展示；
  activity policy 控制逐条列表，不反向隐藏 canonical 公共主题，也不把公开计数伪装成私密事实。
- Mention policy 已支持 `everyone/following/nobody`；`following` 表示接收方关注发起人。Identity 批量
  解析 active、未 suspended 的 handle，Forum 在单次有界写路径应用 policy、follow、block、mute 与
  通知偏好。无权限、未知或生命周期关闭的 `@handle` 仍作为普通公开文字保存，不报错、不生成语义
  通知，也不向发帖人暴露账号存在性或策略。

### Partial

- 暂无 handle history/cooldown/redirect；公开 profile 的 tabs 已形成 owner-domain read model，但仍缺
  浏览器级跨 viewport/登录态旅程验收。
- Public profile 是否允许外部搜索引擎索引仍需独立政策。
- 第一阶段不做私密账号与 follow pending request；DM message request 使用独立状态机，二者不得混用。
- Avatar/banner 已有 owner + clean/published binding、上传/处理状态 UI、sanitized variants、解绑 grace
  与 retention-aware GC 代码/测试，但 GC 默认 rollout flag 关闭，且目标环境双 bucket/CDN/provider smoke
  仍需签字，不能声称 staging/production 已完成运营验收。Raster 当前默认由有审计的 system policy 自动批准
  后进入安全变体处理，可按环境回退 staff 人工审核；这不是内容安全扫描。File/PDF scanner 仍缺，Web
  不再提供任意 URL 输入。
- Public profile/account/relationship/search/DM avatar 的兼容 DTO 目前仍把 typed Media projection 降为
  nullable URL string，不携带 `expiresAt`；Profile Web 也未统一在四分钟刷新或首次 image error 时回源。
  已加载图片通常不受 URL 到期影响，但长驻 React Query cache/lazy image 可能持有过期 bearer URL；在
  统一 typed DTO 或受控 refetch 前，该跨 surface 刷新闭环保留为 `Partial`。
- 旧 `/me/ignores` 作为 block-by-id 兼容 alias 保留，新客户端只使用 handle-based block API。

## 四种关系不得混用

| 关系 | 方向 | 主要作用 | 是否通知对方 |
|---|---|---|---|
| follow | 单向、第一阶段无 pending | 用户关系、关系计数、DM policy；不等于内容订阅 | 建立时按偏好通知 |
| subscription | 用户到 board/thread | 内容更新通知和订阅 feed | 不通知内容作者 |
| mute | 单向私密 | 降低 feed、通知或会话可见性 | 否 |
| block | 双向安全边界 | 阻止 follow、私信和直接互动，并统一可见规则 | 不主动通知，但操作结果不可伪装 |

Forum 订阅在用户界面统一显示为“订阅”；内部保留的 `following` wire value 仅用于旧客户端兼容，
不得再解释成用户 follow。

## Follow 状态机

公开资料第一阶段状态为 `not_following -> following -> not_following`，不创建 pending 或隐式请求。
如果未来支持需要审批的资料，再增加 `pending -> accepted | rejected | cancelled`。当前规则包括：

- 不能关注自己；block 任意方向存在时不能关注或批准请求。
- 并发重复 follow/unfollow 幂等，计数在同一事务中更新或从关系事实可靠投影。
- 列表使用稳定游标并在读取时应用 block、账号状态和可见性规则。
- suspended/deleted 账号不出现在 profile、relationship 或列表，也不能成为新 follow target；用户仍可
  对 suspended 账号建立 block/mute，并可清理自己对生命周期关闭账号的既有关系。
- remove follower 只删除对方→本人的关系，使用与 follow/block 相同的账号对事务锁并由 trigger 校正
  计数；它不自动 block、不通知对方，也不阻止对方未来重新关注。

relationship 一次返回 `following`、`followedBy`、`blockedByMe`、`blockedMe`、`muted`、
`canFollow`、`canStartConversation`、`canMention`，Web 不再下载整张 block 列表推断单个用户状态。
`canMention` 只是当前关系和接收方策略的 viewer read model；内容写入仍重新批量检查，不能把按钮
状态当授权。第一阶段没有 follow `requestState`；DM request 只存在于私信 API。

## Block 与 mute

目标语义：

- mute 是当前用户私有过滤，不阻止对方继续访问公共板块内容或互动。
- block 在任意方向存在时阻止 follow、DM、回复到对方内容、直接 mention 和对对方内容的反应。
- 创建 block 在同一 transaction 删除双方 accepted follow、关闭 pending DM request 并校正关系计数；
  解除 block 不恢复这些关系或请求。
- 公共板块原讨论仍由板块可见性决定；block 会从双方个性化 feed 隐藏内容、阻止 profile 内容聚合
  与直接互动，但不删除或改写历史公共讨论，不破坏其他参与者的回复上下文。
- profile、搜索、通知、feed、DM 和互动端点共享同一 relationship policy，不各自解释。
- 解除 block 不自动恢复 follow 或订阅关系。

最终的公开资料可见规则仍需产品负责人确认；在确认前，UI 不得承诺“双方完全不可见”。

## 资料模型

目标公开资料字段：

- 不可变 account id、handle；owner-editable display name、院校、avatar asset、banner asset、bio。
- 有协议/域名白名单和数量限制的外部链接。
- join time、公开角色/认证、信任等级、成就徽章。
- followers、following、主题、回复、获赞等准确计数。
- tabs：posts、replies、likes、media；bookmarks 仅本人可见，其他逐条列表由 activity visibility 决定。

校园邮箱、制裁证据、设备、内部风险分和私信绝不属于公开资料。Staff console 按 capability
获取必要运营字段，也不因为“管理员”而默认显示邮箱或任意 DM。

## 隐私矩阵

目标设置至少包含：

| 设置 | 建议选项 | 推荐默认值 |
|---|---|---|
| profile visibility | public / campus / only_me | campus，待最终确认 |
| activity visibility | public / campus / only_me | only_me |
| follower list visibility | public / campus / followers / only_me | followers |
| following list visibility | public / campus / followers / only_me | followers |
| DM policy | everyone / following / nobody | following |
| mention policy | everyone / following / nobody | everyone，受 block 限制 |
| discoverability | on / off | on for campus |

公共论坛主题的可见性由 board policy 控制，不由作者 profile visibility 反向改变。若未来要做
followers-only 内容，应建立独立内容类型和授权模型。

上述默认值已经落库：activity=`only_me`、mention=`everyone`。Activity 的逐条列表权限在 profile
visibility 和双向 block 之后应用；本人例外只绕过 activity policy，不绕过账号生命周期。Mention
自己的 handle 不创建通知，block 任意方向都禁止语义 mention；mute 不阻止对方写普通公开文字，但
会抑制接收方通知。

## 徽章、认证与角色标识

三者必须分开：

- **成就徽章**：由贡献或规则授予，例如首帖、优质作者，可自动获得。
- **身份认证**：证明组织、官方账号或经核实身份，有签发、到期、撤销、证据和审计。
- **角色标识**：说明 moderator/admin 的当前平台职责，不作为成就或身份认证。

人工认证由 `platform` owner 管理 typed definition 和 grant history。Definition 区分 `identity` 与
`special`，图标和 Badge variant 只能使用受控枚举；不接受 SVG、HTML、CSS 或外链素材。Grant 默认
私密，记录 issuer、issued/expiry/revoked time、签发/撤销 reason 与可选 opaque evidence reference。
同一账号同一类型同时最多一个有效 grant；到期或撤销后可以重新签发，历史不覆盖。

公开投影同时要求 definition 允许公开、grant 明确 `displayOnProfile`、未撤销且未到期。公开 DTO 只含
类型、label、普通文本说明、受控图标/样式和 issued/expiry time；issuer、reason、evidence 与内部 grant id
不进入公开 profile 或公共列表。后台按独立 `verifications.manage` capability 创建类型、授予、查看历史和
撤销，所有 mutation 要求 reason 并与 governance audit 同事务提交。Self、同级和更高角色目标被拒绝。

成就由 Platform owner 管理 definition、grant/revoke/regrant 和事件历史，Forum 只拥有首帖、首评、
精选作者等贡献资格判断。Definition 的 `slug` 不可变，名称、说明、受控图标、状态和自动规则积分使用
`expectedVersion` 乐观并发更新；停用阻止新授予但不抹掉历史。人工授予/撤销只接受 lower-role target，
要求理由并与 governance audit 同事务提交。只有 `automatic` 来源的首次贡献授予可以写入幂等 pending
mint；人工操作不能发积分，撤销不会冲销已经由真实贡献获得的 ledger 历史。

## API 与数据所有权

Forum 拥有 follow/block/mute 与计数投影；Identity 拥有账号、owner-editable profile 和 privacy policy；
Media 验证 owner + clean/published image 后调用 Identity 的受限 asset binding API；Platform 拥有成就和人工认证
的定义与授予历史。Forum 只通过 Platform public API 获取公开成就与最小公开认证投影，不跨 schema
SQL。HTTP shape 以 OpenAPI 为准。

Identity 同时维护最小化的用户搜索候选文档，只包含 account id、handle 和可选 display name；Forum
通过 Identity public account API 取回当前可见资料，再叠加自己的 follow/block/mute、计数和 Media
派生 URL。搜索聚合层不得跨域读取账号表，也不得把索引 hit 原样返回。

头像和 banner 只保存 clean + published media asset reference。资料页先授权 profile viewer；公开论坛内容
则先按 board/content policy 选择 canonical row，再把作者归属作为内容署名，通过 Identity public account
batch 和 Media batch resolver 返回 typed、到期的 avatar Delivery。资料页隐藏或 activity list 私密不会
抹去作者已经发布在公共讨论中的 handle、display name 或当前公开头像；账号生命周期关闭、内容不可见或
头像不再 clean/published 时投影立即为空。当前 profile/account/relationship/search/DM 兼容 HTTP DTO 仍只取
Delivery URL，不能让客户端提交任意第三方 URL 或把 bearer URL 当权威字段。新/修改的公开 surface 应保留
`expiresAt`，既有兼容 surface 需要通过 additive contract 或有界 refetch 收敛。
上传 intent 会持久化可选的 `profile_avatar/profile_banner` usage，使待审/处理状态可在刷新后恢复；
usage 不是放宽授权的凭证，最终绑定仍重新验证 owner、image kind、clean 和完整 publication。

Mention handle 解析属于 Identity 的最小 public batch API，只返回 account id、canonical handle 和
mention policy，不返回邮箱、资料正文或生命周期内部原因；Forum 使用自己的 follow/block/mute 事实
在内容事务中写候选 outbox，并在最终 delivery transaction 重读当前 mention policy、偏好、关系、
账号生命周期和 canonical 内容可见性。Profile activity/media/likes list 与 owner bookmarks 由 Forum
持有，但只消费 Identity 返回的 activity visibility/public account projection，不跨 schema 读取私有
字段；媒体 URL 由 Media 对 exact binding 批量授权后返回，Forum 不直接读取 Media 表。

## 已决策与后续决策

- 第一阶段 profile 默认 `campus`，followers/following 默认 `followers`，DM 默认 `following`，
  discoverability 默认 on；公开 profile 是否允许搜索引擎索引仍待决定。
- 第一阶段不做私密账号请求；若未来需要，不复用 follow boolean 偷渡 pending 状态。
- Block 对精确资料直链：block 发起方保留最小资料以便解除，对方 block 当前 viewer 时返回 not found；
  公共讨论本身不删除。更细的历史串折叠体验仍需浏览器 E2E 验收。
- Handle 释放期、改名冷却和认证账号改名流程仍待决定。

## 验收基线

- follow/unfollow 与 block 并发安全，计数、列表和 relationship 结果一致。
- 只有资料 owner 能通过 `/me` surface 移除自己的 incoming follower；重复移除幂等、不能借此删除
  其他账号的关系，且移除后双方未被隐式 block。
- follow、subscription、mute、block 的文案、API 与行为不混用。
- block/mute 在 profile、feed、search、notification、DM 和互动中使用同一 policy。
- 隐私设置对匿名、校园成员、关系用户、本人和 staff 有矩阵化授权测试。
- Profile 的 `canViewActivity` 与主题/回复/media/likes endpoint 使用同一 policy；私密状态不触发 Web 重试风暴，
  aggregate 公共内容计数仍保持可解释。
- 收藏 endpoint 只允许 owner，按收藏顺序返回当前仍可见内容；媒体列表不披露 stale binding，喜欢列表
  不返回已撤销 vote 或当前 viewer 已被关系策略排除的内容。
- Mention 覆盖 everyone/following/nobody/self/block/inactive/suspended；被拒绝的文字仍原样公开且没有
  通知，relationship `canMention` 与最终写入授权一致；privacy/mute/block/content hide 与排队 delivery
  的竞态由同一事务 advisory/row lock 重验覆盖。
- 所有公开 profile 响应不含邮箱或内部治理字段，媒体来自平台 asset。
- profile 上传设置页刷新后仍能恢复 pending/processing/published/failed/blocked；非 clean + published
  资产没有可用绑定按钮，服务端也拒绝绑定。Pending preview 不泄露 OSS locator。公共 profile/list 的
  signed URL 到期刷新仍需完成上文 `Partial` 闭环。
- 成就、认证和角色标识有独立 DTO、视觉语义和权限来源；成就/认证授予与撤销留下同事务审计。
- 成就定义 stale version 返回 conflict；人工授予不产生 pending mint，重复自动授予最多产生一个 grant、
  一个事件、一个幂等 mint 和一个通知；撤销后公开资料不展示但历史仍可追溯并产生变更通知。
- 私密、已撤销、已到期或 definition 禁止公开的认证不会出现在任何公开资料响应。

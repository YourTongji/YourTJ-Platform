# 通知、公告与私信

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Forum/Web/Platform maintainers、Community operations、Privacy owner
>
> 最近核验：2026-07-12，migrations `0044_dm_message_requests.sql`、`0047_governance_appeals.sql`

通知告诉用户“发生了什么”，公告传达平台级信息，私信承载参与者之间的非公开交流。三者都
涉及未读、实时、偏好和保留，但必须保持各自的权限和证据边界。

## 当前状态

### Current

- 站内通知有类型、payload、`read_at`、unread count 和 aggregation key，多种论坛/互动/DM
  写入点会创建通知；后端有 SSE 基础。
- 通知列表支持有界 cursor pagination、数据库 unread filter、逐条/批量/全部已读；Web 有全局未读
  角标、全部/未读筛选、加载更多，以及内容/私信等已知事件的安全站内 target。
- 治理通知使用独立的 private store、unread count 和 mark-read API；账号制裁、内容处置及申诉每次
  transition 在业务事务内幂等创建。Web 将其置于通知页独立区域并计入全局角标，普通互动偏好不能关闭。
- 治理通知仅返回安全摘要、subject 和申诉入口。它不返回 staff/reporter identity、内部 evidence 或
  原始请求体；处置通知使用治理事件编号预填申诉中心，申诉更新指向对应 appeal history。
- Web 使用带 Authorization header 的 fetch stream 消费 SSE，收到 typed event 后只失效通知/私信
  query 并回源 API；断线指数退避重连，不把瞬时 payload 当 durable 事实。
- `/me/notification-prefs` 使用严格的 typed event×channel 契约；Web 可分别控制回复、提及、引用、
  赞同、徽章、订阅和私信站内提醒，以及每周摘要邮件。站内事件写入点在落库前执行同一偏好映射，
  未识别的安全/治理事件默认保留。
- 后端已有每 7 天调度的 digest worker；它是发送骨架，不代表偏好、投递与运营闭环完成。
- 公告有 `draft/scheduled/published/archived` 状态、时间窗口、受众、严重度、presentation、稳定排序、
  optimistic version、receipt revision 和 append-only mutation snapshot；管理写需要 capability、reason
  和同事务审计。
- 登录用户按 `(account, announcement, revision)` 保存 seen/dismiss/ack，未看 current revision 会进入
  可访问的全局弹窗队列；公告页显示有效期、revision 和本人确认状态，后台显示 receipt summary/history。
- 匿名访客只收到 `all` 受众公告，Web 用 revision-scoped 本地 seen 避免同版本重复；登录后服务端
  receipt 重新成为唯一事实。
- 私信有 canonical 1:1 conversation、分页 inbox/messages、单调 read pointer、准确未读、
  block/sanction/trust 检查、单条举报和受限 staff evidence。
- 私信支持 participant-local archive/unarchive、可恢复删除、会话搜索和 mute；新消息会让双方的归档/
  隐藏会话重新进入收件箱，mute 只抑制通知而不篡改未读事实。
- 陌生联系使用显式 `pending -> accepted | declined` 消息请求：接收方有独立请求箱，发送方有已发送
  请求箱；pending 只允许创建时的一条、最多 1000 字附言，接受前双方都不能追加消息。
- 请求接受会创建普通 conversation 并通知发送方；删除/拒绝或发送方撤回不会通知、不会隐式 block，
  未举报附言随终态立即删除。举报会在同一事务创建最小 evidence 并移出请求箱。
- 全局私信角标分别投影 accepted conversation 未读数与 incoming request 数；Web 的 SSE 只把
  `dm/dm_request/dm_request_accepted` 当回源刷新信号，不把瞬时事件当消息事实。

### Partial

- 旧 badge 和 comment vote 通知仍缺可到达 target；已接入的新治理通知都有稳定申诉 target。
- SSE 只适合单实例，没有 Redis bridge 或 durable event delivery。
- Digest 已读取 `email.weeklyDigest` 并兼容历史 `email_digest` 数据，但仍缺 delivery
  status/retry 与运营验证。
- 公告哪些 audience/正文变化必须 ack、receipt/revision 的具体保留期仍需运营与隐私负责人批准；
  当前实现提供机制，不替代政策决定。
- `presentation` 已进入 contract、数据库和后台，但 Web 目前在公告页统一以 card 展示；全站 persistent
  banner 及其占位、关闭和无障碍行为仍需单独完成。
- 私信仍缺附件、typing/presence、消息撤回、request expiry、retention/legal-hold worker 和多实例广播。

## 通知模型

通知事件必须有稳定 schema：

- event type 和 schema version；
- actor 的最小公开身份；
- subject type/id 与可理解摘要；
- target URL；
- aggregation key；
- created/read time；
- 是否属于不可关闭的安全/治理类别。

业务事务写 durable outbox，消费者幂等写入站内通知并触发允许的外部渠道。不能依赖请求中的
无监督 task；SSE 只是更新提示，不是通知事实源。

治理通知是当前例外的同步关键事实：处置/申诉 mutation 与 notice 在同一数据库事务提交，以免用户
看到已生效限制却没有申诉入口。未来 outbox 可用于 email/push 或实时提示，但不得替代这份站内记录。
其 dedupe key 绑定原治理事件或 appeal transition，重试不会生成重复通知。

## 已读、聚合与实时

- 支持 mark one、mark selected 和 mark all before cursor/time，均幂等且只影响当前账号。
- 普通通知和治理通知分别维护已读事实，Web 的“全部已读”同时调用两套 owner-scoped API；任一存储
  不得通过另一类偏好或删除动作清空。
- unread filter 在数据库查询生效；角标、列表和 mark 操作使用同一事实。
- 聚合只合并同类型/同 subject 的兼容事件，保留最近 actor 和准确数量。
- Header 和消息导航显示 unread badge；点击 target 后按明确规则标记已读。
- Web 使用 SSE/EventSource 接收“有新数据”信号，再通过 API 拉取；多实例通过 Redis pub/sub
  或 durable stream 广播并支持断线恢复。

当前 `/notifications/read` 新客户端明确发送 `ids` 或 `all=true`；服务端暂时接受历史空对象作为
mark-all，以支持滚动升级。列表 cursor 表示上一页最后返回的通知 id，下一页使用严格 `< cursor`，
lookahead 只判断 `hasMore`，不能把未返回的 row 当 cursor 导致漏项。

## 偏好与发送渠道

偏好是 `event category × channel` 矩阵，channel 至少有 in-app、email、web push。安全事件、
制裁、内容处置结果和重要公告不能被普通营销/互动开关关闭。

- 当前 v2 契约要求完整提交 `inApp` 和 `email`；未知字段或缺少事件 key 会被拒绝，避免旧客户端把
  任意 JSON 静默保存为无效设置。新账号默认开启七类站内互动，默认关闭每周摘要。
- `reply/mention/quote/vote/badge/watching/dm` 分别映射到稳定的偏好 category；新增可关闭事件时必须
  同时更新 OpenAPI、映射、Web 设置和 handler→DB 行为测试。
- in-app 是平台事实层，允许关闭声音/聚合但不能丢失必要治理记录。
- email 初期用于登录/重置和可选 digest；互动即时邮件是否开放为 `Decision needed`。
- web push 在真实 service worker、permission 和 subscription 后才能在 UI 中标为可用。
- provider 失败有重试/告警，不回滚已经完成的社区互动；身份验证码除外，邮件接受前不得成功。

## 公告状态与全局弹窗

公告字段包含：status、presentation、severity、priority、starts/ends at、audience、`requiresAck`、
optimistic `version`、receipt `revision`、published/archived time 和 staff audit。

```text
draft -> scheduled -> published -> archived
            |            |
            +------------+ 可取消/提前归档
```

用户 receipt 按 `(account, announcement, revision)` 记录 `firstSeenAt`、`dismissedAt` 和
`acknowledgedAt`：

- 每个对当前用户生效、current revision 尚未 seen 的 published announcement 都进入全局弹窗队列，
  不能只在首页卡片展示；多条按 priority、published time 和 id 稳定排序，逐条呈现。
- `presentation` 用于声明弹窗后的 persistent card/banner 形态，不取消首次未读全局弹窗；当前 Web
  已实现公告页 card，persistent banner 尚未落地。
- 弹窗完成可见渲染后记录 seen；普通公告允许 dismiss，不强制阻断继续使用。
- `requiresAck=true` 的公告需要明确确认；普通弹窗关闭不等于确认。
- 每次管理 mutation 增加 version 并追加不可变 snapshot；重大正文变化由管理员明确提升 receipt
  revision 并重新展示，拼写修正可只提升 version。
- 未登录用户可临时在本地记 seen，但登录后以服务端 receipt 为准。
- 规则/条款类公告归档而非硬删除，保留 revision 与审计。

## 私信权限与请求

`dmPolicy` 为 `everyone | following | nobody`：接收方已关注的发送者可以直接送达；`everyone`
允许其他 active、未 block 用户先发一条请求；`following` 拒绝陌生请求；`nobody` 拒绝所有新会话和
pending request 的接受。已接受会话不因 policy 后续收紧而静默关闭，block 和 suspension 仍立即阻断。

- 不能给自己、无效/暂停账号或任意方向 block 的账号发消息。
- 当前 TL0 不能发起新会话；active silence 阻止创建和发送，发送时重新检查账号、参与者与 block。
- 创建请求支持 account-scoped `Idempotency-Key`，相同 key/内容返回同一 conversation，不同内容冲突；
  Redis 每账号每天 10 次请求限制之外，数据库还以最小 attempt 元数据对最近 24 小时请求做相同上界，
  并发 attempt 按 sender 串行，缓存缺失时不失守。
- 接受动作可安全重放；并发 pair mutation 使用同一 advisory lock，并与账号 suspension 的 account lock
  串行，canonical unordered account pair 始终只有一个 conversation。
- 接收方拒绝进入 30 天 sender/recipient pair 冷却，发送方撤回进入 5 分钟防抖；二者不等价于 block，
  也不创建拒绝通知。Block 会原子关闭 pending request 并进入 30 天冷却。冷却结束后的新请求仍重新
  检查当时 policy、账号状态和 block。
- 消息当前为不可编辑纯文本，普通消息上限 16000 字，请求附言上限 1000 字；编辑/撤回若增加必须显式建模。

## 私信生命周期与隐私

- archive/unarchive 只改变参与者自己的 inbox 组织。
- delete/recover 是参与者自己的可恢复隐藏，不立即删除对方副本。
- pending request 不复用 archive/delete：incoming 只有 accept、delete/decline、report，outgoing 只有等待
  或 withdraw。declined conversation 的 pair/status/cooldown 作为最小反滥用元数据保留，未举报正文立即删除。
- 双方删除且无举报/legal hold 后，retention worker 在恢复窗结束后清除正文和未绑定 asset。
- 消息默认不可编辑；如支持撤回，保留“消息已撤回”事件和治理证据，而不是静默消失。
- 附件使用 private clean asset 和短期授权 URL，不能进入公共 CDN。
- staff 没有任意 DM 浏览能力；只有 participant 报告的具体消息和最小上下文进入证据队列，
  每次访问和决定都审计。
- 产品不声称端到端加密，隐私说明必须覆盖数据库、备份、举报与运营访问。

## Decision needed

- 哪些公告必须 ack、哪些正文修订要求重新确认、哪些 audience 可定向；未读 current revision 的
  首次全局弹窗是既定基线，不属于可关闭的 presentation preference。
- 除已经固定不可关闭的治理通知外，其他安全事件分类以及 email/push 的第一阶段范围。
- 消息撤回语义和 pending request 自动过期时间；既有 accepted conversation 持续有效已确定。
- 未举报消息、举报证据、附件和备份的保留期；group DM 建议 P2。

## 验收基线

- 通知 list/unread/mark 在分页和并发下保持一致，target URL 可到达授权内容。
- 偏好按事件与渠道生效，安全/治理通知不能被错误关闭。
- 治理通知与处置/申诉事务同成败，重复写幂等；列表只暴露当事人可见摘要，target 可到达本人申诉历史。
- 断开 SSE 不丢 durable 通知，多实例任一节点写入都能刷新客户端。
- 未看过且对当前用户生效的公告在全局按 current revision 正确展示，seen/dismiss/ack 不混淆。
- 私信 policy、request、block、sanction、participant 权限和 read pointer 有数据库集成负向测试。
- staff 只能访问被举报的最小证据，删除与保留 worker 幂等、可观察并尊重 legal hold。

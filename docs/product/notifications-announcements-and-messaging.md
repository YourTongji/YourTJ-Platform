# 通知、公告与私信

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Forum/Web/Platform maintainers、Community operations、Privacy owner
>
> 最近核验：2026-07-11，`origin/main@ed8a06c`

通知告诉用户“发生了什么”，公告传达平台级信息，私信承载参与者之间的非公开交流。三者都
涉及未读、实时、偏好和保留，但必须保持各自的权限和证据边界。

## 当前状态

### Current

- 站内通知有类型、payload、`read_at`、unread count 和 aggregation key，多种论坛/互动/DM
  写入点会创建通知；后端有 SSE 基础。
- 通知列表支持有界 cursor pagination、数据库 unread filter、逐条/批量/全部已读；Web 有全局未读
  角标、全部/未读筛选、加载更多，以及内容/私信等已知事件的安全站内 target。
- Web 使用带 Authorization header 的 fetch stream 消费 SSE，收到 typed event 后只失效通知/私信
  query 并回源 API；断线指数退避重连，不把瞬时 payload 当 durable 事实。
- `/me/notification-prefs` 使用严格的 typed event×channel 契约；Web 可分别控制回复、提及、引用、
  赞同、徽章、订阅和私信站内提醒，以及每周摘要邮件。站内事件写入点在落库前执行同一偏好映射，
  未识别的安全/治理事件默认保留。
- 后端已有每 7 天调度的 digest worker；它是发送骨架，不代表偏好、投递与运营闭环完成。
- 公告有管理员 CRUD、理由和审计，首页显示最近标题。
- 私信有 canonical 1:1 conversation、分页 inbox/messages、单调 read pointer、准确未读、
  block/sanction/trust 检查、单条举报和受限 staff evidence。
- 私信支持 participant-local archive/unarchive、可恢复删除、会话搜索和 mute；新消息会让双方的归档/
  隐藏会话重新进入收件箱，mute 只抑制通知而不篡改未读事实。Web 已接通三类收件箱和全局私信角标。

### Partial

- 旧 badge、部分治理和 comment vote 通知仍缺可到达 target。
- SSE 只适合单实例，没有 Redis bridge 或 durable event delivery。
- Digest 已读取 `email.weeklyDigest` 并兼容历史 `email_digest` 数据，但仍缺 delivery
  status/retry 与运营验证。
- 公告缺 publish 状态、排期、严重度、展示方式、受众、revision 和用户 receipt。
- 私信仍缺 DM policy、消息请求、附件、实时、retention/legal-hold worker 和多实例广播。

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

## 已读、聚合与实时

- 支持 mark one、mark selected 和 mark all before cursor/time，均幂等且只影响当前账号。
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

目标公告字段包含：status、presentation、severity、priority、starts/ends at、audience、
`requiresAck`、revision、published/archived time 和 staff audit。

```text
draft -> scheduled -> published -> archived
            |            |
            +------------+ 可取消/提前归档
```

用户 receipt 按 `(account, announcement, revision)` 记录 `firstSeenAt`、`dismissedAt` 和
`acknowledgedAt`：

- 每个对当前用户生效、current revision 尚未 seen 的 published announcement 都进入全局弹窗队列，
  不能只在首页卡片展示；多条按 priority、published time 和 id 稳定排序，逐条呈现。
- `presentation` 控制弹窗后的 persistent card/banner 和视觉样式，不取消首次未读全局弹窗。
- 弹窗完成可见渲染后记录 seen；普通公告允许 dismiss，不强制阻断继续使用。
- `requiresAck=true` 的公告需要明确确认；普通弹窗关闭不等于确认。
- 重大正文变化提升 revision，可按政策重新展示；拼写修正不应无意义打扰全部用户。
- 未登录用户可临时在本地记 seen，但登录后以服务端 receipt 为准。
- 规则/条款类公告归档而非硬删除，保留 revision 与审计。

## 私信权限与请求

目标 `dmPolicy`：`everyone | following | nobody`。不满足直接送达条件的新联系人进入 message
requests；接受后进入 canonical conversation，拒绝/举报不会暴露额外资料。

- 不能给自己、无效/暂停账号或任意方向 block 的账号发消息。
- 当前 TL0 不能发起新会话；active silence 阻止创建和发送，发送时重新检查账号、参与者与 block。
- 消息当前为不可编辑纯文本，长度边界以 OpenAPI 为准；编辑/撤回若增加必须显式建模。
- 已存在会话在 policy 改变后能否继续属于 `Decision needed`；安全 block 总是立即生效。
- 发送、read pointer 和 request accept 幂等；并发创建仍只有一个 unordered account pair。

## 私信生命周期与隐私

- archive/unarchive 只改变参与者自己的 inbox 组织。
- delete/recover 是参与者自己的可恢复隐藏，不立即删除对方副本。
- 双方删除且无举报/legal hold 后，retention worker 在恢复窗结束后清除正文和未绑定 asset。
- 消息默认不可编辑；如支持撤回，保留“消息已撤回”事件和治理证据，而不是静默消失。
- 附件使用 private clean asset 和短期授权 URL，不能进入公共 CDN。
- staff 没有任意 DM 浏览能力；只有 participant 报告的具体消息和最小上下文进入证据队列，
  每次访问和决定都审计。
- 产品不声称端到端加密，隐私说明必须覆盖数据库、备份、举报与运营访问。

## Decision needed

- 哪些公告必须 ack、哪些正文修订要求重新确认、哪些 audience 可定向；未读 current revision 的
  首次全局弹窗是既定基线，不属于可关闭的 presentation preference。
- 不可关闭的通知类别以及 email/push 的第一阶段范围。
- DM policy 变化对既有会话的影响、消息撤回语义和 request 过期时间。
- 未举报消息、举报证据、附件和备份的保留期；group DM 建议 P2。

## 验收基线

- 通知 list/unread/mark 在分页和并发下保持一致，target URL 可到达授权内容。
- 偏好按事件与渠道生效，安全/治理通知不能被错误关闭。
- 断开 SSE 不丢 durable 通知，多实例任一节点写入都能刷新客户端。
- 未看过且对当前用户生效的公告在全局按 current revision 正确展示，seen/dismiss/ack 不混淆。
- 私信 policy、request、block、sanction、participant 权限和 read pointer 有数据库集成负向测试。
- staff 只能访问被举报的最小证据，删除与保留 worker 幂等、可观察并尊重 legal hold。

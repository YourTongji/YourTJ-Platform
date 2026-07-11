# 当前能力、缺口与路线图

> 文档类型：实现盘点与产品路线图
>
> 状态：Active
>
> 负责人：Product owner、Platform maintainers
>
> 最近核验：2026-07-12，`codex/x-community-complete`

本盘点以当前源码、OpenAPI、migration 和 Web 为基线。它说明已经存在什么、哪里只有骨架、
哪些界面承诺与实际行为不一致。后续 PR 改变这些结论时，必须在同一 PR 同步更新本文件的
跨域摘要和对应领域规范。

## 已有的坚实基础

- Rust/Axum 单库多域后端、PostgreSQL、Redis、Meilisearch 与 OSS 上传信任边界。
- 校园邮箱验证码、JWT/refresh、后端密码登录与找回、角色与制裁检查。
- 课程、选课镜像、课评、论坛板块/主题/评论/投票/收藏/订阅/标签/草稿/修订。
- 活跃度事件、每日计数和管理员可版本化权重；首页热力图使用真实接口。
- canonical 1:1 私信、未读指针、双向发送阻断、单条举报和最小化后台证据。
- capability RBAC、用户邀请、角色/禁言/封禁、会话撤销、多域审核、审计和管理 UI。
- 论坛搜索候选由 PostgreSQL 重新验证可见性，索引可以全量重建。
- 积分 ledger 有 signing intent、hash chain、数据库 append-only 防护；tip 与 escrow 状态机有
  target/party 校验、事务锁和 CAS，Web 使用服务端返回的 exact signing bytes。

这些基础降低了下一阶段成本，但不能掩盖用户主流程和语义仍未闭环。

## 当前关键差距

| 领域 | 状态 | 已验证问题 |
|---|---|---|
| 登录注册 | `Current` | purpose-bound 一次性 code、密码防枚举与 reset/change 撤销语义已完成；Web 已拆分密码、验证码、注册和找回流程，注册要求显式公开 handle |
| 账号与会话 | `Partial` | session-bound access、refresh replay 防护与 Web 设备中心已完成；仍缺 onboarding、recent-auth、自助导出/停用/删除，refresh token 仍存 localStorage |
| 社交图 | `Current` | 已有公开单向 follow、幂等接口、followers/following、relationship API 与 trigger 维护的准确计数；第一阶段明确不做私密账号审批 |
| block/mute | `Current` | mute 为单向私密 feed/通知过滤，block 为双向安全边界并原子移除双方 follow；profile、feed、通知、DM、回复与投票已接统一规则 |
| 个人资料 | `Partial` | display name、bio、HTTPS website、clean OSS avatar/banner reference、owner 上传/状态恢复/绑定 UI、关系数和 profile/list/DM/discoverability 隐私已落地；仍缺 handle history、activity/mention privacy 和公开 media/likes tabs |
| 内容正确性 | `Partial` | 主题/评论 canonical policy 和事务边界已完成；tags/exact filter、有效 subscription、poll/vote/bookmark viewer state、read tracking 与撤销互动已接齐并有 handler→DB 验证；typed draft 契约、CAS version 和 owner scope 已对齐，仍缺显式 pending、已发布内容的乐观 version 和 durable outbox |
| Trust/板块权限 | `Partial` | promotion 已检查 active-days，单次扫描至多升降一级；create-thread 在预检和写事务内执行 board lock/min-trust，`moderation.content` 只豁免这两个 gate；仍需确定 staff 被普通举报自动隐藏政策并把 trust policy 版本化 |
| 创作体验 | `Partial` | 主题/评论已持久化显式 content format，Web 已接 CodeMirror 编辑/预览、安全 renderer、debounced 云端草稿和跨设备冲突选择；仍缺 clean asset 图片、跨客户端 conformance 和已发布内容编辑冲突处理 |
| Link preview/Onebox | `Partial` | 已实现 HTTPS、逐跳 allowlist/DNS/public-IP pin、流式 body 上限、安全 UTF-8、无远程预览图和最小化日志；仍缺维护中的 HTML parser、media proxy、错误缓存与完整网络 fixture |
| 媒体 | `Partial` | 后端 STS、回调、审核、owner-only 状态查询及 clean avatar/banner binding 已存在，Web 可恢复待审状态并只绑定 clean 资产，任意头像 URL 已停写；帖子、评论、课评、私信仍无 binding，且缺 scanner/缩略图/EXIF/配额/孤儿回收/CDN 策略 |
| Feed | `Partial` | latest/hot/subscriptions 使用真实后端语义，首页已移除固定摘要、伪造徽章和无行为按钮；仍缺用户 following、canonical 列表摘要/viewer state 和明确 recommended 规则 |
| 聚合搜索 | `Partial` | courses/reviews/threads 已由独立 search domain 返回 typed、可跳转且回表重验的结果，type/limit 和 Web 综合结果页生效；仍缺每类 cursor、users/boards/tags、highlight/纠错和可靠 outbox 更新 |
| 通知 | `Partial` | bounded cursor/unread/逐条与全部已读、安全站内 target、Web 角标/筛选/SSE 回源刷新已接通；typed event×channel 偏好及 weekly digest 开关已由 OpenAPI、写入点和设置页对齐，仍有部分旧事件无 target，也没有 multi-instance delivery/outbox |
| 公告 | `Current` | 有状态、排期、受众、严重度、presentation、version/revision、seen/dismiss/ack receipt、全局未看弹窗、公告页和后台 revision history；匿名访客用 revision-scoped 本地 seen，登录用户以服务端 receipt 为准 |
| 私信 | `Partial` | canonical 1:1、新会话 DM policy、举报证据、archive/delete/recover、搜索、会话 mute 和全局角标已接通；仍缺消息请求、附件、实时及 retention/legal-hold worker |
| 推广位 | `Partial` | 左侧已由 API 返回自营站内推广，具备 clean owned asset id、状态、排期、受众、位置、优先级、独立 capability、审计和后台 UI；匿名素材图、asset usage/GC、曝光/点击日聚合尚未完成 |
| 徽章与认证 | `Partial` | 成就徽章、人工身份/特殊认证和实时角色标识已经拆分；成就具备独立 capability、versioned 受控定义、自动幂等授予/mint、人工非 mint 授予、撤销/重新授予、append-only history、同事务审计、后台 UI 与公开投影。人工认证具备 typed definition、可到期/撤销 grant、私有 evidence reference、后台 UI 与安全公开投影；仍缺成就/认证通知、自动授予 durable outbox 和认证证据存储/复核政策 |
| 治理 | `Partial` | 审核和制裁基础较强；缺当事人通知、申诉、冲突回避、账号生命周期、保留 worker 和高风险 recent-auth |
| 积分运营 | `Partial` | 用户侧 verify、内容打赏和 escrow 完整性已加固；持久化只读 reconcile、单并发/幂等执行、逐钱包漂移指标、独立 capability、审计和管理视图已接通；仍缺告警/SLO 与受审批 projection 重建，历史 constraint anomaly 需单独兼容策略 |
| 运维 | `Partial` | 设置仍为 string key/value；任务只确认提交，无持久状态、进度、失败日志和重试；缺 SLO/恢复演练 |
| 测试 | `Partial` | 后端 CI 有 lint/集成，Web 有 lint/type/build 与最小 Vitest/Testing Library/axe harness；仍无浏览器 E2E、完整前端覆盖，许多契约与 UI 行为差异无法被 CI 捕获 |

Web shell 已采用路由级 lazy loading、可朗读 loading state、受控页面/操作反馈动画和
`prefers-reduced-motion` 降级；这只建立了体验基础，不代表各业务页面已完成视觉与旅程验收。

## P0：先恢复正确性、安全与产品真实性

1. 主题/评论 canonical policy、评论 SQL、引用约束、事务边界和 versioned typed draft 已完成；下一步补
   显式 pending、已发布内容的 `contentVersion/expectedVersion` 和 durable side-effect outbox。
2. 为 Onebox 补维护中的 HTML parser、规范化/错误缓存与完整网络 fixture；现有逐跳
   scheme/allowlist/DNS/public-IP pin、流式 body 限制和禁用远程预览图继续作为安全基线。
3. Trust active-days/单步升降、board lock/min-trust、tags/filter/subscription/viewer/read/cancel 和
   伪 UI 清理已完成；下一步明确 staff 内容达到普通举报阈值时自动隐藏还是升级复核，并将 trust
   policy 版本化。
4. typed 通知偏好和 Web SSE 已对齐；下一步补齐所有 target URL、durable outbox 和 multi-instance delivery。
5. 公告 revision/seen/ack 和全局未看弹窗已完成；继续为公告受众、强制确认和保留期限形成运营政策。
6. 任意头像 URL 已停写，avatar/banner 已完成上传、状态恢复和 clean binding UI；下一步把 asset
   binding 接到 UGC，完成 scanner/variants/GC/CDN，并保持后台如实说明 block 会永久删除对象。
7. 在已对齐的 typed 聚合搜索上补可靠 outbox/reconciliation 与局部失败；新增
    users/boards/tags 前先落实 discoverability、block 和账号状态规则。
8. 为上述身份、通知、内容、搜索和公告行为补 handler→DB 与前端关键旅程验证。
9. credit 只读 reconcile job、指标和管理视图已完成；下一步接告警/SLO，并为确需重建的 wallet
   projection 设计独立审批流程。历史 constraint anomaly 走单独兼容决策，不在 migration 中改写
   ledger，也不提供 balance editor 或任意 ledger append。

## P1：形成完整社区闭环

- 已完成用户 follow graph、relationship API、粉丝/关注列表、准确计数与公开账号第一阶段状态机；
  下一步补 remove follower 与未来私密账号独立决策。
- Follow/subscription/mute/block 已拆分，profile/list/new-DM/discoverability 隐私已实现；下一步补
  activity/mention policy、账号搜索和全站浏览器验证。
- Display name、bio、banner、受控链接和 OSS 头像 binding 已落地；下一步补 handle history/cooldown、
  scanner/variants 和 orphan GC。
- `plain_v1/markdown_v1` 契约、安全 renderer、编辑器、预览和 CAS autosave 已完成；下一步接 OSS 图片。
- latest/hot/subscription/following feed 和 typed 聚合搜索。
- DM policy、消息请求、archive/delete/recover、附件与保留 worker。
- 推广 asset usage/GC 与效果聚合、成就/认证通知和证据复核、typed settings、durable job center。
- 当事人治理通知、申诉流程、账号导出/停用/删除/恢复。
- notification/search/media/activity outbox 与 reconciliation，多实例实时广播。
- 对齐并验证现有 weekly digest 的 preference、投递状态、retry 和运营指标。

## P2：增长和高级社交

- 有透明输入和安全过滤的 recommended feed、趋势与关注建议。
- 独立短动态、repost/quote-post；不复用公共论坛隐私语义硬凑。
- 私密账号审批、搜索个性化、digest 个性化、typing/presence 和 group DM。
- 推广位效果分析、复杂 audience targeting 和更丰富的身份认证流程。

## 依赖顺序

```mermaid
flowchart LR
    accTitle: YourTJ 社区能力实施依赖
    accDescr: 先修正确性与身份安全，再建设社交和隐私基础，然后完成内容媒体与发现，最后进入增长功能。

    P0["正确性与真实产品承诺"] --> I["身份、会话与账号生命周期"]
    I --> S["社交图、block/mute 与隐私"]
    S --> C["Markdown、OSS 与内容生命周期"]
    C --> N["通知、公告、私信与聚合搜索"]
    N --> O["后台运营、申诉与可靠任务"]
    O --> G["推荐、趋势和高级社交"]
```

推荐、转发和群聊不是当前最短路径。没有社交图、隐私、通知和治理基础时，它们只会放大
错误数据、骚扰和运营成本。

## 决策门

在开始相应 schema/API 前，产品负责人必须确认：

- 匿名与校园成员的默认可见范围。
- follow 是否仅影响 feed/DM，block 与 mute 的全站语义。
- 密码登录标识、毕业账号恢复和首次注册是否强制设置密码。
- Markdown 支持的内容类型和历史纯文本兼容方式。
- OSS private/public、签名 URL、审核前发布和媒体保留策略。
- 推广位是否仅自营、各认证类型的具体证据/复核/保留政策、管理员恢复机制。
- 数据导出、删除恢复窗、举报证据/审计/备份保留期。

## 核验入口

本盘点主要核对 `backend/crates/identity`、`forum`、`reviews`、`media`、`activity`、
`governance`、`platform`、`contract/openapi.yaml`、`web/src/pages`、
`web/src/components`、`web/src/lib/api` 和 `.github/workflows`。不要把本文件当作 API 或 schema
字段清单；对应细节仍以契约和 migration 为准。

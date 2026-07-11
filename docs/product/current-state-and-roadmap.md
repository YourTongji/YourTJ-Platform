# 当前能力、缺口与路线图

> 文档类型：实现盘点与产品路线图
>
> 状态：Active
>
> 负责人：Product owner、Platform maintainers
>
> 最近核验：2026-07-11，`origin/main@ed8a06c`

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
| 社交图 | `Planned` | 无用户 follow 表、接口、计数或列表；现有 `following` 只是 board/thread subscription |
| block/mute | `Partial` | `user_ignores` 同时承担单向隐藏与双向私信阻断，UI 却称完整屏蔽；资料、搜索和直接互动规则不一致 |
| 个人资料 | `Partial` | 有公开 profile、内容列表和徽章；缺 display name、bio、banner、受控链接、关系数、隐私和 handle history；头像接受任意 URL |
| 内容正确性 | `Partial` | 主题/评论创建与编辑已共享长度/结构/watched-word canonical policy，queue 不进入公开副作用，revision/正文和主题附属数据具备事务边界；仍缺显式 pending、乐观 content version、durable outbox，tags/filter、subscription、poll/vote viewer state 和 read tracking 未接齐 |
| Trust/板块权限 | `Partial` | TL2/TL3 promotion 缺 active-days；TL3 demotion 可一次降两级；board `is_locked/min_trust_to_post` 字段存在但发帖路径未执行 |
| 创作体验 | `Partial` | Web 仍为 textarea + 纯文本；草稿 API/实现/前端不一致；无 Markdown、预览、autosave、附件和编辑冲突处理 |
| Link preview/Onebox | `Partial` | 已实现 HTTPS、逐跳 allowlist/DNS/public-IP pin、流式 body 上限、安全 UTF-8、无远程预览图和最小化日志；仍缺维护中的 HTML parser、media proxy、错误缓存与完整网络 fixture |
| 媒体 | `Partial` | 后端 STS、回调和审核已存在；头像、帖子、评论、课评、私信没有 asset binding；缺缩略图、EXIF 清理、配额、孤儿回收和 CDN 访问策略；管理 UI 对 block 是否删除 OSS 的文案与后端相反 |
| Feed | `Partial` | 首页有固定摘要、按列表位置伪造徽章和无行为的收藏/分享/筛选；缺真实 following feed 和明确 recommended 规则 |
| 聚合搜索 | `Partial` | courses/reviews/threads 已由独立 search domain 返回 typed、可跳转且回表重验的结果，type/limit 和 Web 综合结果页生效；仍缺每类 cursor、users/boards/tags、highlight/纠错和可靠 outbox 更新 |
| 通知 | `Partial` | bounded cursor/unread/逐条与全部已读、安全站内 target、Web 全局角标和筛选已接通；偏好 key 与事件类型仍不一致，部分旧事件无 target，也没有 Web SSE/multi-instance delivery |
| 公告 | `Partial` | 只有 CRUD 和首页标题；无状态、排期、受众、revision、seen/ack receipt 和未读全局弹窗 |
| 私信 | `Partial` | 核心 1:1 可用；缺 DM policy、消息请求、archive/delete/recover、附件、搜索、会话 mute、实时与全局角标 |
| 推广位 | `Planned` | 左侧推广卡硬编码；无后台模型、排期、素材、受众、审计和聚合效果指标 |
| 徽章与认证 | `Partial` | 成就徽章后端存在，缺授予/撤销 UI；身份认证、特殊认证和角色标识尚未拆分 |
| 治理 | `Partial` | 审核和制裁基础较强；缺当事人通知、申诉、冲突回避、账号生命周期、保留 worker 和高风险 recent-auth |
| 积分运营 | `Partial` | 用户侧 verify、内容打赏和 escrow 完整性已加固；缺持久化 reconcile job、漂移指标/告警和只读管理视图；含历史异常的 constraint 会保持 NOT VALID，需运营审计与单独兼容策略 |
| 运维 | `Partial` | 设置仍为 string key/value；任务只确认提交，无持久状态、进度、失败日志和重试；缺 SLO/恢复演练 |
| 测试 | `Partial` | 后端 CI 有 lint/集成，Web 有 lint/type/build 与最小 Vitest/Testing Library/axe harness；仍无浏览器 E2E、完整前端覆盖，许多契约与 UI 行为差异无法被 CI 捕获 |

Web shell 已采用路由级 lazy loading、可朗读 loading state、受控页面/操作反馈动画和
`prefers-reduced-motion` 降级；这只建立了体验基础，不代表各业务页面已完成视觉与旅程验收。

## P0：先恢复正确性、安全与产品真实性

1. 主题/评论 canonical policy、评论 SQL、引用约束和事务边界已完成；下一步补草稿契约、显式
   pending、`contentVersion/expectedVersion` 和 durable side-effect outbox。
2. 为 Onebox 补维护中的 HTML parser、规范化/错误缓存与完整网络 fixture；现有逐跳
   scheme/allowlist/DNS/public-IP pin、流式 body 限制和禁用远程预览图继续作为安全基线。
3. 修正 trust promotion/demotion，并在发主题时执行 board lock/min-trust；明确 staff 内容达到普通
   举报阈值时自动隐藏还是升级复核，并为阈值与角色边界补测试。
4. 接通 tags、tag filter、subscription、poll/vote/viewer state、read tracking 与撤销互动。
5. 删除伪造等级/徽章/摘要和无行为按钮；界面只能展示后端真实能力。
6. 对齐通知 event preference、补齐所有 target URL、Web 实时消费和 multi-instance delivery。
7. 定义公告 revision/seen/ack，完成“该用户未看过公告”的全局提示/弹窗决策与实现。
8. 停止持久化任意头像/UGC 外链，确定 OSS asset/clean/binding；修正 block 会永久删除对象的后台文案。
9. 在已对齐的 typed 聚合搜索上补可靠 outbox/reconciliation 与局部失败；新增
    users/boards/tags 前先落实 discoverability、block 和账号状态规则。
10. 为上述身份、通知、内容、搜索和公告行为补 handler→DB 与前端关键旅程验证。
11. 为 credit 增加只读 reconcile job/指标/告警；历史 constraint anomaly 走单独兼容决策，不在
    migration 中改写 ledger，也不提供 balance editor 或任意 ledger append。

## P1：形成完整社区闭环

- 用户 follow graph、relationship API、粉丝/关注列表与准确计数。
- 明确并拆分 follow、subscription、mute、block；实现 profile/activity/DM/discoverability 隐私。
- display name、bio、banner、受控链接、handle history/cooldown 和 OSS 头像。
- `plain_v1/markdown_v1` 契约、安全 renderer、编辑器、预览、autosave 与 OSS 图片。
- latest/hot/subscription/following feed 和 typed 聚合搜索。
- DM policy、消息请求、archive/delete/recover、附件与保留 worker。
- 管理员推广位、认证/特殊徽章、typed settings、durable job center。
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
- 推广位是否仅自营、身份认证代表什么、管理员恢复机制。
- 数据导出、删除恢复窗、举报证据/审计/备份保留期。

## 核验入口

本盘点主要核对 `backend/crates/identity`、`forum`、`reviews`、`media`、`activity`、
`governance`、`api/src/platform.rs`、`contract/openapi.yaml`、`web/src/pages`、
`web/src/components`、`web/src/lib/api` 和 `.github/workflows`。不要把本文件当作 API 或 schema
字段清单；对应细节仍以契约和 migration 为准。

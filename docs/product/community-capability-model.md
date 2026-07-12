# 成熟社区能力模型

> 文档类型：产品参考模型
>
> 状态：Active
>
> 负责人：Product owner、Community operations、Platform maintainers
>
> 最近核验：2026-07-12，`origin/main@0492746` 与 ADMIN/普通管理员产品决策

本模型定义 YourTJ 作为成熟校园社区需要覆盖的业务面和完成标准。它不是要求一次性复制所有
社交产品功能，而是用来识别依赖、避免“有按钮但没有闭环”，并为路线图提供共同语言。

## 什么叫一项能力完成

一项能力只有同时满足以下相关层，才可标记为 `Current`：

- 产品语义：参与者、权限、状态机、失败和恢复行为明确。
- 数据与契约：schema、HTTP 契约、幂等和兼容策略明确。
- 用户体验：主流程、空态、错误态、加载态和无权限态可用。
- 安全治理：滥用、隐私、审计、保留与删除路径明确。
- 运营能力：必要的后台、指标、告警和人工恢复路径存在。
- 验证：核心正向旅程、权限负向用例和关键边界有自动化或明确人工验收。

只有表、端点或静态页面不能单独证明产品能力完成。

## 能力总览

| 领域 | 成熟基线 | 详细规范 |
|---|---|---|
| 身份与访问 | 校园资格、密码/验证码登录、找回、会话设备、recent-auth、账号生命周期 | [身份与访问](identity-and-access.md) |
| Onboarding | 条款与规则确认、handle、资料、兴趣板块、首个关系和可恢复引导 | [身份与访问](identity-and-access.md) |
| 个人资料 | display name、bio、头像/封面、受控链接、资料页分栏、handle 生命周期 | [社交与隐私](social-profile-and-privacy.md) |
| 社交图 | follow/request/unfollow、计数与列表、relationship、block、mute | [社交与隐私](social-profile-and-privacy.md) |
| 隐私 | profile/activity/关系列表/DM/discoverability 可见性与默认值 | [隐私生命周期](../security/privacy-and-data-lifecycle.md) |
| 内容创作 | 板块、主题、评论、课评、Markdown、草稿、修订、附件、删除/恢复 | [内容与媒体](content-media-and-discovery.md) |
| 互动 | vote/like/bookmark/share/subscription、viewer state、幂等和撤销 | [内容与媒体](content-media-and-discovery.md) |
| Feed 与发现 | latest/hot/subscription/following/recommended、标签、趋势与推广位 | [内容与媒体](content-media-and-discovery.md) |
| 聚合搜索 | 论坛、课程、课评、用户和结构化对象的 typed results、过滤与高亮 | [内容与媒体](content-media-and-discovery.md) |
| 通知与公告 | typed event、渠道偏好、已读、聚合、实时、公告版本和确认 | [通知、公告与私信](notifications-announcements-and-messaging.md) |
| 私信 | 1:1、请求箱、权限、未读、归档/删除、附件、举报和保留 | [通知、公告与私信](notifications-announcements-and-messaging.md) |
| 信誉与徽章 | 信任等级、成就徽章、身份认证、特殊标识的独立语义和后台 | [信任安全与后台](trust-safety-and-administration.md) |
| 信任与安全 | 反滥用、举报、可逆审核、处罚、通知、申诉和利益冲突控制 | [信任安全与后台](trust-safety-and-administration.md) |
| 管理与运营 | 用户、内容、板块、推广、公告、政策、任务、审计、健康指标 | [信任安全与后台](trust-safety-and-administration.md) |
| 闭环积分 | 贡献 mint、内容打赏、悬赏/商品托管、可验证账本与 projection | [积分与托管](credit-and-escrow.md) |
| 数据权利 | 导出、停用、删除、恢复、匿名化、保留、legal hold | [隐私生命周期](../security/privacy-and-data-lifecycle.md) |
| 可靠性 | outbox、durable jobs、reconciliation、SLO、备份恢复、降级 | [架构规范](../architecture/contracts-and-data.md) |
| 无障碍与质量 | 键盘、读屏、响应式、性能预算、兼容、前端和旅程测试 | [设计与无障碍](design-system-and-accessibility.md) |

详细的当前证据和优先级见[当前能力、缺口与路线图](current-state-and-roadmap.md)。

## 身份、资格与账号

成熟基线包括：

- 校园资格验证与公开身份隔离，邮箱绝不进入公共 DTO。
- 登录与注册分流；支持密码和邮箱验证码，找回流程不泄露账号存在性。
- 验证码按 purpose 隔离、原子一次性消费、有限尝试和限流。
- access/refresh 生命周期、rotation、设备列表、单设备/全设备撤销和安全事件通知。
- 高风险操作 recent-auth，管理员 bootstrap 与最终管理员恢复有独立规则。
- `active -> deactivated -> deleted -> purged` 的可恢复账号生命周期，以及数据导出。
- 毕业、邮箱失效、改 handle、受保护名称和旧链接跳转规则。

## 资料、社交关系与隐私

成熟基线包括：

- 公开 handle 与不可变 account id；display name、bio、头像、封面和受控外链。
- follow、pending request、unfollow 的并发安全状态机和准确计数。
- 一次 relationship 查询返回双方关注、请求、block、mute 和 `canDm`。
- 用户 follow 与板块/主题 subscription 分开命名、分开存储、分开影响 feed。
- mute 单向降低可见性；block 双向阻止直接互动，其可见性规则全站一致。
- profile、activity、followers/following、DM、mention 和搜索可发现性的隐私矩阵。
- 成就徽章、身份认证、角色标识三类视觉与治理含义分离。

## 内容、媒体与互动

成熟基线包括：

- 主题、评论、课评等类型分别定义格式、长度、链接、图片、编辑和删除规则。
- `plain_v1` 与 `markdown_v1` 显式版本；历史纯文本不被静默重新解释。
- 创建和编辑共享同一验证、敏感词、mention、限流与治理管线。
- Markdown 禁 raw HTML，安全 URL 协议、外链属性、图片 alt 和统一 sanitizer。
- 草稿自动保存、恢复、离开保护、版本冲突和跨设备行为明确。
- 内容状态包含 visible、pending、hidden、soft-deleted、archived、restored；修订可追溯。
- 所有图片和附件引用平台 OSS asset id；上传、扫描、clean、绑定、解除和延迟 GC 可恢复。
- 互动响应包含 viewer state，可执行取消赞、取消票、取消收藏和取消订阅。
- 举报、隐藏、恢复会一致更新搜索、计数、热榜、通知和活跃度投影。

## Feed、搜索与运营发现

成熟基线包括：

- latest、hot、板块/主题订阅、用户 following 和 recommended 的名称及输入信号明确。
- 推荐只能使用允许的数据，必须经过内容可见性、block/mute 和治理过滤。
- 聚合搜索返回类型化 section，不用一个不稳定的混合数组伪造统一搜索。
- 搜索覆盖课程、教师、课评、论坛、用户与板块/标签，并支持过滤、高亮和无结果建议。
- PostgreSQL 是可见性事实源；索引通过 outbox 更新，可全量重建并监控漂移。
- 社区推广位有 placement、素材、目标、受众、排期、优先级、状态和审计；不硬编码在 Web。
- 推广曝光/点击只保留必要聚合，不构建不透明的跨域用户画像。

## 通知、公告与私信

成熟基线包括：

- 稳定通知类型和 payload，包含 actor、subject、target URL 与聚合键。
- 事件、渠道和偏好分层；安全及治理通知不可被普通偏好关闭。
- 单条、选中和截止游标全部已读有明确幂等语义，导航显示准确角标。
- 实时更新支持多实例，持久通知写入与业务事务通过 outbox 保证不丢。
- 公告支持 draft、scheduled、published、archived，带 audience、priority、presentation 和 revision。
- `seen`、`dismissed`、`acknowledged` 分开；重大正文修订可以要求重新确认。
- 私信有 DM policy、陌生人请求、1:1 canonical conversation、未读、mute、举报和最小证据访问。
- 会话归档、参与者删除、恢复、双方删除后的延迟 purge 与 legal hold 明确。

## 治理、管理与运营

成熟基线包括：

- capability-based RBAC、目标角色层级、自操作限制、理由和最近认证。ADMIN 是唯一超级管理角色；
  普通管理员由 ADMIN 按审核域、目标上限和期限显式授权，不可继续转授权。
- 内容、用户、媒体和私信举报队列各自保持证据边界。
- 利益冲突默认禁止自审；仅 ADMIN 可在最近认证、强制理由、明确告警和专用审计下自审本人
  媒体上传，不扩展到申诉、角色授权、账号处置或审计。
- 处置优先可逆；当事人收到类别、时长、政策依据和申诉入口。
- 申诉关联原事件，复核人不能是原处置人，决定保留历史而不覆盖证据。
- 管理后台覆盖用户、内容、板块/标签、敏感词、公告、推广、徽章、政策和任务。
- 设置是 typed、versioned、validated；任务是 durable、可观察、可重试、可去重。
- 审计是 append-only，记录 actor/action/target/reason/result/request correlation，避免 PII 和正文。
- 管理指标包含积压、处理时长、误判恢复、重复施害和任务失败，而不只显示用户总数。

## 隐私、安全与可靠性

成熟基线包括：

- 数据分类、用途、访问主体、保存期限、导出和删除矩阵。
- 账号删除跨 identity、forum、reviews、DM、media、search、cache 和 backup 编排。
- 积分账本不改写，删除后使用不可反查的账号 tombstone。
- refresh secret、验证码、签名凭据和供应商密钥不出现在日志、源码或 PR preview。
- 关键写入 fail-open/fail-closed 规则明确；邮件、OSS、Redis、Meili 故障有降级和恢复。
- outbox、幂等 consumer、dead-letter、reconciliation、RPO/RTO 和恢复演练。
- Web 具备单元、组件、无障碍和关键旅程测试；视觉改动验证桌面与移动端。

## 刻意后置的高级能力

算法推荐、转发/引用动态、私密账号审批、typing/presence、群聊、商业广告定向和复杂实验平台
属于 P2。它们依赖社交图、隐私、通知、媒体和治理基础，不应挤占 P0 正确性与 P1 核心闭环。

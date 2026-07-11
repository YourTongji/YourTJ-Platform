# 授权与审计

> 文档类型：安全规范
>
> 状态：Active
>
> 负责人：Security owner、Identity/Governance maintainers
>
> 最近核验：2026-07-12，`codex/x-credit-reconciliation`

后端授权每一个 staff 操作。Web 按 capability 隐藏导航只是可用性和数据最小化措施，绝不是
安全边界。

## 当前状态

### Current

- 持久角色为 `user < mod < admin`，服务端映射成 named capabilities 并返回给 Web。
- identity 用户管理拒绝 self/equal/higher-role target；角色变化、管理员强制注销与 suspension 同时
  撤销 refresh session 和已签发 access JWT。
- forum/review moderation 通过 identity 的最小角色边界执行作者层级检查。
- `governance.audit_events` 是 append-only account/system/service actor 记录；多数管理 mutation
  在业务事务中写入，reported-DM evidence list 也会审计。
- 人工认证使用独立 `verifications.manage`，拒绝 self/equal/higher-role target；类型创建、授予和撤销
  都要求 reason，并把业务状态与成功 audit 放在同一事务。

### Partial

- capability 仍按角色静态映射，没有 per-account delegation。
- 缺 recent-auth、标准 request id/source/result、失败/拒绝 attempt audit 和受控 export。
- 缺双人审批、利益冲突 assignment/recusal、appeal independent review 和明确 retention。
- 仍有 admin/platform 业务 SQL 位于 api crate，owner/audit 一致性需要持续收敛。

## 当前 capability 基线

| Capability | mod | admin | 主要用途 |
|---|:---:|:---:|---|
| `moderation.content` | yes | yes | forum/review/media/reported-DM 审核与恢复 |
| `users.search` | yes | yes | 隐私安全的用户目录与制裁历史 |
| `users.silence` | yes | yes | 对 lower-role 限时禁言与撤销 |
| `audit.read` | yes | yes | 中央审计查询 |
| `users.invite` | — | yes | 到期校园邀请 |
| `users.roles` | — | yes | lower-role 的 user/mod 变更 |
| `users.suspend` | — | yes | suspension、撤销和会话撤销 |
| `community.manage` | — | yes | boards、tags、watched words、现有 badge backend |
| `courses.manage` | — | yes | 课程目录管理 |
| `platform.settings` | — | yes | 当前 generic settings |
| `activity.policy` | — | yes | 活跃权重和历史 |
| `announcements.manage` | — | yes | 当前公告管理 |
| `promotions.manage` | — | yes | 自营推广、排期、站内目标和 clean asset reference |
| `verifications.manage` | — | yes | typed 身份/特殊认证定义、低角色账号授予历史与撤销 |
| `operations.jobs` | — | yes | selection sync/reindex triggers |
| `credit.integrity` | — | yes | 运行和读取只读 ledger/wallet reconciliation |

用户角色没有 staff capability。没有 capability 可以查看任意校园邮箱/DM、编辑 wallet balance 或
append 任意 ledger。推广、人工认证与积分完整性分别使用 `promotions.manage`、`verifications.manage`
和只读 `credit.integrity`；PII reveal 若上线也必须使用独立 capability，不能塞进过宽的
`community.manage`。

## 授权规则

- Handler 在读取敏感列表或锁定目标前检查 capability；普通 public id 解析也不应扩大可见性。
- `moderation.content` 在 create-thread 中只允许绕过 board 的 `is_locked/min_trust_to_post` gate，
  不绕过账号状态、sanction、内容 policy 或 rate limit；`community.manage` 才能修改 board policy。
- User mutation 锁定目标，拒绝 self/equal/higher-role，验证 reason、duration 和当前状态。
- Admin 不能处置另一个 admin；最终管理员管理走 out-of-band policy。
- Moderator silence 有明确最长时长，suspend 和角色授予仅 admin。
- PII reveal、永久/高影响制裁、角色改变、账号删除和敏感 export 需要 recent-auth；部分操作还
  应双人审批。
- Reported-DM 只开放 participant 报告的最小 evidence，读取动作本身写 audit。
- 内容打赏的 recipient 必须由内容 owner domain 解析为当前可见 target 的 author，再由 identity
  目的限定接口确认账号 active 且无有效 suspend；拒绝 self-tip 和客户端伪造 recipient。
- 商品 `deliveryInfo` 是订单双方信息，不进入公开 Product DTO、搜索、日志或第三方订单列表。
- SSE/实时请求通过 Authorization header 认证，不把 access token 放入 URL/query；事件只作为刷新提示，
  客户端仍从受授权 API 读取 durable 通知与私信状态。
- Credit reconcile 的 request/resume/start/succeeded/failed 使用同一 run id 关联 audit；reason、账本是否通过和
  bounded drift counts 可入 metadata，idempotency key、签名、邮箱、数据库错误和完整 ledger payload 不入。
- `credit.integrity` 只允许验证和读取持久结果，不授予 wallet update、ledger append 或历史 ledger
  mutation；即使直接构造请求，普通用户和 moderator 也必须被拒绝。
- 后端 denial 使用统一错误 envelope；客户端不能靠 capability 推断隐藏数据存在。
- `verifications.manage` 只允许管理员处理 lower-role account。Definition 只接受受控 category/icon/style；
  grant 默认私密，公开开关不能绕过 definition policy，重复有效 grant 与重复/过期撤销返回 conflict。

## Audit event

最低字段：

- immutable id、created time；
- actor kind (`account/system/service`)、account id/role/capability snapshot；
- action、target type/id；
- reason；
- request/correlation id、source surface；
- result (`succeeded/rejected/failed`)；
- purpose-limited metadata 或 before/after hash。

Account actor 必须有 account id；system/service 不使用虚构 id `0`。Secrets、校园邮箱、code、token、
signature-as-credential、raw request body、完整内容或任意 DM 不得进入 metadata。
认证 audit 可以记录 account id、type slug、是否公开、到期时间和是否存在 evidence，但不能记录 evidence
reference 或实际证据内容。

## 原子性与异步操作

- 业务状态和成功 audit 在同一 transaction 提交；audit 失败则敏感 mutation 不提交。
- 撤销/修正追加新事件，不更新或删除旧 audit。
- Durable job 的 requested/started/succeeded/failed 使用同一 correlation id，不能只审计“按钮被点”。
- 有界的 rejected/failed privileged attempts 需要安全事件策略，避免既无审计又被攻击者刷爆。
- Audit export 加 watermark、purpose、rate limit、expiry 和下载审计。

## Staff safety

- 管理页面展示 effect、scope、target、reason、duration 和 recovery path。
- 高风险动作防重复提交，destructive copy 使用清晰中文术语而不是模糊“删除”。
- 定期复核 role/capability、异常 evidence access、长期 sanction 和失效 staff 账号。
- Staff 使用独立个人账号，不共享管理员凭据；service credential 只用于自动化。

## 验收基线

- 每个 staff route 有缺 capability、self/equal/higher target、无 reason 和 stale state 的负向测试。
- Web 不显示无 capability 操作，手工请求仍被后端拒绝。
- 敏感 mutation 与成功 audit 原子，失败不留下半状态。
- Evidence/PII read 目的限定、最小化并可追踪。
- Audit 中不存在 secret、邮箱、完整 DM 或无界 request payload。
- Recent-auth、双人审批和 export 上线前有独立 threat review 与恢复测试。

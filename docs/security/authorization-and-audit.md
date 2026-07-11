# 授权与审计

> 文档类型：安全规范
>
> 状态：Active
>
> 负责人：Security owner、Identity/Governance maintainers
>
> 最近核验：2026-07-12，migrations `0047`、`0048`、`0055` 与 governance/identity integration tests

后端授权每一个 staff 操作。Web 按 capability 隐藏导航只是可用性和数据最小化措施，绝不是
安全边界。

## 当前状态

### Current

- 持久角色为 `user < mod < admin`，服务端映射成 named capabilities 并返回给 Web。
- identity 用户管理拒绝 self/equal/higher-role target；角色变化、管理员强制注销与 suspension 同时
  撤销 refresh session 和已签发 access JWT。
- forum/review moderation 通过 identity 的最小角色边界执行作者层级检查。
- Forum 主题/评论列表和详情返回服务端计算的 author/moderation viewer actions；`canModerate` 同时要求
  `moderation.content`、非自身目标和 lower-role author。Web 不再仅凭本地 capability 显示治理按钮，
  但这些字段仍不是写操作授权边界。
- `governance.audit_events` 与 `governance.appeal_events` 是数据库 trigger 保护的 append-only 记录；
  row-level `UPDATE/DELETE` 和 statement-level `TRUNCATE`（包括 cascade 到达）均拒绝。多数管理
  mutation 在业务事务中写入，reported-DM evidence list 也会审计。
- 人工认证使用独立 `verifications.manage`，拒绝 self/equal/higher-role target；类型创建、授予和撤销
  都要求 reason，并把业务状态与成功 audit 放在同一事务。
- 成就运营使用独立 `badges.manage`；定义 mutation 使用版本并发控制，人工授予/撤销拒绝 self/equal/
  higher-role target、要求 reason，并把状态与成功 audit 放在同一事务。
- 角色变更、suspend/解除 suspend 和管理员强制 session revoke 要求 10 分钟内的
  server-side session recent-auth；密码和 `recent_auth` purpose 邮箱 code 均可验证。
- 申诉复核使用独立 `appeals.review` capability。moderator/admin 均可读取 lower-role 队列，但原处置
  actor 不能领取，领取后只有同一 reviewer 可通过 optimistic version 提交决定。当前角色层级、self
  和 recusal 在 SQL cursor/limit 之前生效，不通过取页后的内存过滤隐藏空页。
- Identity 可签发 `scope=appeal` 的短期 access JWT；普通 auth middleware 明确拒绝任何 scoped token，
  只有本人申诉/治理通知路由使用专门认证函数。受 suspension 影响的账号仍可申诉，deleted 账号拒绝。

### Partial

- capability 仍按角色静态映射，没有 per-account delegation。
- 缺标准 request id/source/result、失败/拒绝 attempt audit 和受控 export。
- 缺双人审批、自动 assignment/recusal workflow、SLA escalation 和明确 retention；申诉的原处置人
  回避、reviewer 绑定与 lower-role 检查已经在服务端生效。
- 仍有 admin/platform 业务 SQL 位于 api crate，owner/audit 一致性需要持续收敛。

## 当前 capability 基线

| Capability | mod | admin | 主要用途 |
|---|:---:|:---:|---|
| `moderation.content` | yes | yes | forum/review/media/reported-DM 审核与恢复；所有用户目标仍受 strict lower-role 与 no-self 约束 |
| `users.search` | yes | yes | 隐私安全的用户目录与制裁历史 |
| `users.silence` | yes | yes | 对 lower-role 限时禁言与撤销 |
| `audit.read` | yes | yes | 中央审计查询 |
| `appeals.review` | yes | yes | lower-role 申诉的独立领取与决定 |
| `users.invite` | — | yes | 到期校园邀请 |
| `users.roles` | — | yes | lower-role 的 user/mod 变更 |
| `users.suspend` | — | yes | suspension、撤销和会话撤销 |
| `community.manage` | — | yes | boards、tags、watched words |
| `courses.manage` | — | yes | 课程目录管理 |
| `platform.settings` | — | yes | 当前 generic settings |
| `activity.policy` | — | yes | 活跃权重和历史 |
| `announcements.manage` | — | yes | 当前公告管理 |
| `promotions.manage` | — | yes | 自营推广、排期、站内目标和 clean asset reference |
| `badges.manage` | — | yes | versioned 成就定义、lower-role 人工授予/撤销与事件历史 |
| `verifications.manage` | — | yes | typed 身份/特殊认证定义、低角色账号授予历史与撤销 |
| `operations.jobs` | — | yes | selection sync/reindex triggers |
| `credit.integrity` | — | yes | 运行和读取只读 ledger/wallet reconciliation |

用户角色没有 staff capability。没有 capability 可以查看任意校园邮箱/DM、编辑 wallet balance 或
append 任意 ledger。推广、成就、人工认证与积分完整性分别使用 `promotions.manage`、
`badges.manage`、`verifications.manage` 和只读 `credit.integrity`；PII reveal 若上线也必须使用独立
capability，不能塞进过宽的 `community.manage`。

## 授权规则

- Handler 在读取敏感列表或锁定目标前检查 capability；普通 public id 解析也不应扩大可见性。
- `moderation.content` 在 create-thread 中只允许绕过 board 的 `is_locked/min_trust_to_post` gate，
  不绕过账号状态、sanction、内容 policy 或 rate limit；`community.manage` 才能修改 board policy。
- User mutation 锁定目标，拒绝 self/equal/higher-role，验证 reason、duration 和当前状态。
- Admin 不能处置另一个 admin；最终管理员管理走 out-of-band policy。
- Moderator silence 有明确最长时长，suspend 和角色授予仅 admin。
- 当前角色改变、suspend/解除 suspend 和强制注销已要求 recent-auth；普通 silence 发放/撤销与
  内容审核不滥用 step-up。未来 PII reveal、账号删除和敏感 export 上线时同样必须要求；
  部分操作还应双人审批。
- recent-auth 必须绑定当前未撤销 session，只信任数据库时间，不信任 JWT `iat` 或 Web
  状态。高风险 mutation 在自身业务事务内锁定并重验 session，与并发 session revoke 形成
  明确先后顺序；不允许在事务外检查后带着 TOCTOU 窗口写入。refresh rotation 可携带原
  freshness 但不延长时间；legacy JWT fail closed。
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
- Profile 的 `canViewActivity` 和 relationship `canMention` 是 viewer-specific 可用性事实，不是客户端
  授权凭证。逐条 activity endpoint 与内容写入 side effect 必须重新检查 Identity policy、账号状态和
  Forum block/follow 事实；mention 被拒绝时仍成功保存普通文字，不能通过状态码、延迟或 payload 暴露
  target 是否存在、被 suspend 或选择了哪项 policy。
- Author edit 以 canonical `contentVersion` 做 compare-and-swap；409 只返回当前版本，不回显新的正文
  或内部状态。revision 与 canonical mutation 原子，陈旧请求不能留下审计/历史半状态。
- Revision history 是敏感内容面：任意角色的作者可读取本人历史；读取他人历史必须有
  `moderation.content`、目标非本人且作者角色严格更低。普通他人、moderator→moderator/admin 和
  admin→其他 admin 均拒绝；Web 是否展示按钮不改变该规则。
- `verifications.manage` 只允许管理员处理 lower-role account。Definition 只接受受控 category/icon/style；
  grant 默认私密，公开开关不能绕过 definition policy，重复有效 grant 与重复/过期撤销返回 conflict。
- `badges.manage` 只允许管理员处理 lower-role account。Definition 只接受受控 icon/plain text，stale
  version 返回 conflict；人工授予不能触发 mint，撤销不能反转历史积分，事件与中央审计都只追加。
- 申诉提交先通过 owner domain 验证原事件可申诉且属于当前账号；失败统一不泄露事件是否存在。复核人
  必须有 `appeals.review`、高于 appellant 且不同于原处置 actor，不能借 Web 隐藏按钮绕过。
- Appeal token 只接受 purpose-bound 密码/邮箱证明，不创建 session/refresh；普通 `authenticate` 对其
  返回 forbidden。该 scope 不得被扩展为“受限通用登录”，新增可访问 route 需要单独 threat review。

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
- 申诉终态和 owner-domain reversal 位于同一 PostgreSQL transaction；论坛/课评只恢复被原事件精确
  改变的状态，账号 amendment 只允许缩短制裁。后续治理事件冲突、投影修复或 audit/notice 失败时回滚。
- Durable job 的 requested/started/succeeded/failed 使用同一 correlation id，不能只审计“按钮被点”。
- 有界的 rejected/failed privileged attempts 需要安全事件策略，避免既无审计又被攻击者刷爆。
- Audit export 加 watermark、purpose、rate limit、expiry 和下载审计。

## 数据库角色边界

- Production migration/table owner 只用于受控 schema rollout，不作为应用连接账号；runtime login
  不拥有 governance schema/table，也没有 `ALTER`、`DROP`、`TRUNCATE` 或 disable-trigger 权限。
- Runtime 对 `audit_events`/`appeal_events` 只需要有界 `SELECT` 与 `INSERT`；`UPDATE`、`DELETE`、
  `TRUNCATE` 必须显式撤销。Migration `0055` 撤销 `PUBLIC` 的相关 grant 并补 statement trigger，
  但 table owner/superuser 仍可能人为 disable trigger，因此 role separation 是独立安全边界。
- Live maintenance 不通过 `session_replication_role` 或 disable trigger 清理历史。恢复、legal hold 和
  retention 由批准的 append/归档流程处理；确需灾备恢复时在隔离环境执行并记录 operator 审计。

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
- Recent-auth 不把 password、code 或 email 写入 reason/audit/log，并覆盖过期、重放、并发、
  provider 未接受和 session 撤销恢复。双人审批与 export 仍需独立 threat review。
- Staff recent-auth 只证明同一账号在当前 session 内再次提供单因素；它不是 phishing-resistant
  MFA，WebAuthn/passkey、recovery 和 break-glass 仍为 `Partial`。
- Appeal token 不能访问普通 authenticated surface；他人事件、同级/自身目标、原处置人复核、重复 key
  变更 payload 和 stale version 有 handler→PostgreSQL 负向测试。
- Governance append-only 测试必须执行真实 `UPDATE`、`DELETE`、`TRUNCATE`，并验证失败来自
  append-only trigger；只检查 trigger metadata 或 FK 拒绝不足以证明保护有效。

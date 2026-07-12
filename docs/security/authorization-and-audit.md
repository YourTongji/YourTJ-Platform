# 授权与审计

> 文档类型：安全规范
>
> 状态：Active
>
> 负责人：Security owner、Identity/Governance maintainers
>
> 最近核验：2026-07-12，migrations `0061`–`0062`、governance/identity/media tests、Cloudflare official CIDR

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
- Media operations inventory 与 moderation queue 分权：`moderation.content` 处理 lower-role 内容；
  `operations.jobs` 只在 recent-auth 后读取 no-store operational hold/system deletion job；每次分页读取
  都审计。CAS 续期/解除 hold 或理由化重排 system dead letter 同样 recent-auth，审计不包含 provider
  key、URL 或 hash。
- 当前持久角色 `admin` 具有 ADMIN 媒体自审专用例外。本人 preview/approve/block 必须在业务事务中
  重验角色与当前 session recent-auth，接受 `selfReviewConfirmed=true` 和强制 reason；approve 还必须
  使用同一 reviewer 的可信 preview evidence，fail-closed 的 self-block 不依赖预览。`selfReview=true`
  写入对应 evidence、cleanup job 与 governance audit。Moderator、未来委派管理员和 ADMIN 的其他治理
  对象仍执行 no-self。
- Media Delivery processing retry 使用 `operations.jobs`，每次要求 recent-auth、reason、failed/dead-letter
  状态重验和同事务 audit；不能通过 retry capability 直接写 publication=published。

### Partial

- 目标模型已确定为 ADMIN 拥有所有平台定义的 staff capability，并且只有 ADMIN 可以任免
  普通管理员、逐账号授予审核 capability。当前 capability 仍按角色静态映射，没有
  per-account delegation、grant expiry/revocation/history；ADMIN 媒体自审是已交付的唯一利益冲突例外，
  不能被当成委派模型已经上线。
- 缺标准 request id/source/result、失败/拒绝 attempt audit 和受控 export。
- 缺双人审批、自动 assignment/recusal workflow、SLA escalation 和明确 retention；申诉的原处置人
  回避、reviewer 绑定与 lower-role 检查已经在服务端生效。
- 仍有 admin/platform 业务 SQL 位于 api crate，owner/audit 一致性需要持续收敛。

## 当前 capability 基线

| Capability | mod | admin | 主要用途 |
|---|:---:|:---:|---|
| `moderation.content` | yes | yes | forum/review/media/reported-DM 审核与恢复；strict lower-role 与 no-self 生效，只有 admin 本人媒体走专用自审条件 |
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
| `promotions.manage` | — | yes | 自营推广、排期、站内目标和 clean + published asset reference |
| `badges.manage` | — | yes | versioned 成就定义、lower-role 人工授予/撤销与事件历史 |
| `verifications.manage` | — | yes | typed 身份/特殊认证定义、低角色账号授予历史与撤销 |
| `operations.jobs` | — | yes | selection sync/reindex；敏感 no-store media operations inventory、operational hold CAS、Delivery processing 与 system cleanup dead-letter retry |
| `credit.integrity` | — | yes | 运行和读取只读 ledger/wallet reconciliation |

用户角色没有 staff capability。没有 capability 可以查看任意校园邮箱/DM、编辑 wallet balance 或
append 任意 ledger。推广、成就、人工认证与积分完整性分别使用 `promotions.manage`、
`badges.manage`、`verifications.manage` 和只读 `credit.integrity`；PII reveal 若上线也必须使用独立
capability，不能塞进过宽的 `community.manage`。

## 目标 ADMIN 与普通管理员模型

普通管理员 assignment/grant 部分为 `Planned`，不改写上述 Current capability 表。未来实现必须以新
migration、OpenAPI、Web 类型和 handler→PostgreSQL 测试同步交付。ADMIN 媒体自审小节明确标为
`Current`，是可单独成立的严格例外，不依赖未来 delegated grant schema。

### 角色与授权来源

- ADMIN 是唯一超级管理角色。目标迁移中，现有持久化 `admin` 账号按 ADMIN 语义处理，不得把它们
  静默降为普通管理员。
- 普通管理员是可撤销的 account-scoped staff assignment，不是第二种全权 admin。有效权限是“账号未关闭 +
  assignment 有效 + capability grant 未撤销/未过期 + target ceiling 允许”的交集。
- 仅 ADMIN 拥有 `administrators.manage`；普通管理员不能新增、复制、延长、撤销或查看超出必要范围的
  他人 grant，也不能修改本人 assignment。
- ADMIN 的全 capability 集是应用层权限，不授予 table-owner/superuser 权限，不能越过积分合规、
  append-only audit、DM/PII 最小化、retention/legal-hold 或 owner-domain 不变量。

### 可委派 capability 粒度

当前 `moderation.content` 同时包含 forum/review/media/reported-DM 与恢复，对普通管理员过宽。委派上线前
至少拆成以下独立能力；最终标识在 OpenAPI/实现中只能有一份 canonical 命名：

| 能力边界 | 允许的工作 | 不自动包含 |
|---|---|---|
| forum moderation | 论坛举报、主题/评论可逆处置 | 课评、媒体、DM evidence、用户制裁 |
| review moderation | 课评举报与可逆状态 | 论坛、媒体、课程目录管理 |
| media moderation | 媒体队列、可信预览、approve/block | operations hold/job、provider credential、其他内容域 |
| reported-DM review | 只读参与者举报的最小证据与决定 | 通用私信浏览、完整 conversation export |
| content recovery | 精确 id 的 retained content 恢复 | 举报决定、用户制裁、物理 purge |

ADMIN 任命时可组合上述审核权限，并为每项 grant 设定 target ceiling 与可选 expiry。
`appeals.review`、`users.silence`、`operations.jobs`、`audit.read` 等仍是独立权限，不因“普通管理员”标签自动获得。

### Grant 生命周期与会话

- create/update/revoke 必须要求 ADMIN recent-auth、强制 reason、`expectedVersion` 和与 grant 变更同事务的 audit。
- 历史是 append-only event，不覆盖原 grant；事件保存 grantor、grantee、capability、target ceiling、expiry、reason
  与 before/after hash，不保存邮箱、token 或审核正文。
- 授权变更必须递增服务端 authorization version；每个 handler 重验当前有效 grant，并拒绝旧 JWT/
  refresh snapshot。如当前 session 模型不能保证即时失效，撤销 assignment/grant 时必须同步撤销相关 session。
- 到期在读/写授权时 fail closed；后台空态明确区分“无 grant”、“已撤销”、“已过期”和“目标超出上限”。

### ADMIN 媒体自审例外（Current）

利益冲突默认仍是 no-self。唯一例外是 ADMIN 对本人媒体上传执行预览、approve 或 block，用于在
没有第二位 staff 时恢复头像/素材上线路径。该例外必须同时满足：

- actor 在 mutation 事务中重验为 ADMIN，不依赖 stale JWT role/capability；
- 当前 session 完成 recent-auth，填写强制 reason，Web 展示“正在审核本人上传”的明确二次确认；
- approve 仍先通过可信 raster preview/decoder 边界；self-block 不要求读取待审内容；PDF/file 在
  scanner/sandbox evidence 完成前不因 ADMIN 而可批准；
- audit 显式记录 `selfReview=true`、upload id、action、reason、request id 与 result，不记 object key/URL/hash；
- 普通管理员和 moderator 仍不得自审；ADMIN 也不得自审申诉、本人角色/grant、账号制裁、认证/成就授予、
  audit export 或积分完整性处置。

可信 preview grant 与 evidence 都绑定 upload、actor 和 `selfReview`，preview token 一次消费且响应
`private, no-store`。Approve 必须有该 evidence，仍只把 publication 置为 processing；完整的 sanitized variants 发布前，
ADMIN 也不能建立业务 binding 或签发 CDN URL。Block/cleanup 将同一 `selfReview` provenance 带入 job
与 audit，不能在异步边界丢失利益冲突记录。

## 授权规则

- Handler 在读取敏感列表或锁定目标前检查 capability；普通 public id 解析也不应扩大可见性。
- `moderation.content` 在 create-thread 中只允许绕过 board 的 `is_locked/min_trust_to_post` gate，
  不绕过账号状态、sanction、内容 policy 或 rate limit；`community.manage` 才能修改 board policy。
- User mutation 锁定目标，拒绝 self/equal/higher-role，验证 reason、duration 和当前状态。
- 当前 admin 不能处置另一个 admin。目标模型中，ADMIN 可通过 `administrators.manage` 任免普通管理员，
  但另一 ADMIN 的 bootstrap/recovery/revocation 仍走 out-of-band policy。
- 当前 moderator silence 有明确最长时长，suspend 和角色授予仅 admin。目标模型中，ADMIN 仍是
  普通管理员任免和 staff 角色变更的唯一人类 actor；委派的 `users.silence` 不自动授予 suspend/role 权限。
- 当前角色改变、suspend/解除 suspend 和强制注销已要求 recent-auth；普通 silence 发放/撤销与
  内容审核不滥用 step-up。未来 PII reveal、账号删除和敏感 export 上线时同样必须要求；
  部分操作还应双人审批。
- recent-auth 必须绑定当前未撤销 session，只信任数据库时间，不信任 JWT `iat` 或 Web
  状态。高风险 mutation 在自身业务事务内锁定并重验 session，与并发 session revoke 形成
  明确先后顺序；不允许在事务外检查后带着 TOCTOU 窗口写入。refresh rotation 可携带原
  freshness 但不延长时间；legacy JWT fail closed。
- Media operational hold 只接受 `moderation/security` purpose。Hold inventory、创建、续期/替换和解除
  要求 `operations.jobs` 与当前 session recent-auth；`expectedHoldId` 必须表达 create-if-none 或刚查看的
  exact hold，陈旧操作返回 conflict，不能先解除再创建留下删除窗口。Inventory 响应为
  `private, no-store`，读取动作也写 audit；普通 moderation queue 不披露 reason/kind/actor。
- Media system deletion-job inventory 同样是 `private, no-store`；每页读取要求 `operations.jobs`、当前
  session recent-auth，并在读取事务写 result count audit。只有 non-moderation dead letter 可在 reason 和
  当前 job/upload 状态重验后重排。Retry reason 写独立 event，不改写原始 system purpose，并与
  append-only audit 同事务。
- Media processing retry 与 deletion retry 是不同状态机：前者只重排 clean asset 的 failed publication/
  dead-letter variant job，后者只重排允许的 quarantined system cleanup。两者均重验 current row、
  recent-auth 和 reason，返回 queued 不代表 published/deleted；不得复用一个“重试任务”按钮跳过 owner
  state、moderation provenance 或 cleanup 顺序。
- Reported-DM 只开放 participant 报告的最小 evidence，读取动作本身写 audit。
- 内容打赏的 recipient 必须由内容 owner domain 解析为当前可见 target 的 author，再由 identity
  目的限定接口确认账号 active 且无有效 suspend；拒绝 self-tip 和客户端伪造 recipient。
- 商品 `deliveryInfo` 是订单双方信息，不进入公开 Product DTO、搜索、日志或第三方订单列表。
- SSE/实时请求通过 Authorization header 认证，不把 access token 放入 URL/query；事件只作为刷新提示，
  客户端仍从受授权 API 读取 durable 通知与私信状态。
- `operations.jobs` 可读取通知 outbox 的非 payload 运维元数据，并只对 `dead` 事件执行理由化人工重试；
  source key、payload、正文和邮箱不返回，重试次数与 reason 写入不可变 audit。
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
- ADMIN 媒体自审例外不扩大 Forum revision 可见性；ADMIN 读取本人 revision 仍是作者权限，不是 staff 审核例外。
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
- actor kind (`account/system/service`)、account id/role/effective capability 与相关 grant/version snapshot；
- action、target type/id；
- reason；
- request/correlation id、source surface；
- result (`succeeded/rejected/failed`)；
- purpose-limited metadata 或 before/after hash。

Account actor 必须有 account id；system/service 不使用虚构 id `0`。Secrets、校园邮箱、code、token、
signature-as-credential、raw request body、完整内容或任意 DM 不得进入 metadata。
认证 audit 可以记录 account id、type slug、是否公开、到期时间和是否存在 evidence，但不能记录 evidence
reference 或实际证据内容。
媒体自审 audit 额外记录布尔 `selfReview=true` 和稳定 upload id；不得记录 Ingest/Delivery object key、
preview token、signed CDN URL、content hash 或 provider response。

## 原子性与异步操作

- 业务状态和成功 audit 在同一 transaction 提交；audit 失败则敏感 mutation 不提交。
- 撤销/修正追加新事件，不更新或删除旧 audit。
- 申诉终态和 owner-domain reversal 位于同一 PostgreSQL transaction；论坛/课评只恢复被原事件精确
  改变的状态，账号 amendment 只允许缩短制裁。后续治理事件冲突、投影修复或 audit/notice 失败时回滚。
- Durable job 的 requested/started/succeeded/failed 使用同一 correlation id，不能只审计“按钮被点”。
- Provider 删除成功后，Media redacts object key/URL/hash/size/MIME/usage/dimensions；稳定 upload id 与
  purpose-limited audit 继续用于关联。Media operations 表中的 hold/retry/succeeded-job/redacted-evidence
  365 天 purge 默认关闭，不能宣称 staff id 已清；governance actor audit 不属于该 purge，其保留期仍需批准。
- 有界的 rejected/failed privileged attempts 需要安全事件策略，避免既无审计又被攻击者刷爆。
- Audit export 加 watermark、purpose、rate limit、expiry 和下载审计。

## Edge proxy 与客户端 IP 信任

IP 只是一项易共享、可变化的反滥用信号，不是身份或授权凭据。部分 email/onebox rate limit 当前读取
`X-Forwarded-For` 首项，因此其可信性依赖 edge proxy 覆盖输入 header，而不是让应用自行信任任意 client
header。

当前 versioned host Nginx 只对 Cloudflare 官方 IPv4/IPv6 CIDR 使用 `set_real_ip_from`，从
`CF-Connecting-IP` 恢复 `$remote_addr`，再把上游 `X-Forwarded-For` 与 `X-Real-IP` 都设置为该单值。
直接 origin 请求来自非 trusted CIDR，即使携带伪造 CF/XFF header，也不会改变 `$remote_addr`。网段最近于
2026-07-12 对照 [Cloudflare IPv4](https://www.cloudflare.com/ips-v4/) 与
[IPv6](https://www.cloudflare.com/ips-v6/) 核验；官方说明要求定期更新 trusted prefix，完整更新、
`nginx -t`、回滚和正/负 smoke 流程见[部署 runbook](../operations/deployment-and-previews.md)。

同日服务器 `ss`/Docker 实测证明 shared staging 旧 frontend/backend 直连端口绑定所有 interface，host
iptables 默认接受；是否另被 cloud NSG 阻断不能成为可信边界。本 revision 为它启动的 app 增加 loopback
bind 和运行时核验，但 live 旧容器要部署后才收敛。PostgreSQL/Redis/Meili 已精确绑定 `127.0.0.1`；此前
本机 TCP 探针不代表公网暴露，不应误报。App 外部负向复测与 cloud firewall review 前，本边界不满足
release 条件；详见部署 runbook 的 blocker。

硬边界：

- 不信任 `0.0.0.0/0`、`::/0`、任意 `X-Forwarded-For` chain 或客户端自报 `X-Real-IP`。
- 公网只能经受控 edge/proxy 到达应用。Main/PR backend host-network port、PostgreSQL、Redis 与 Meili
  必须由 security group/host firewall 限制；若 backend port 可被公网直连，攻击者可完全绕过 Nginx
  header normalization，这不是应用代码能补救的情况。
- 网段更新只通过 reviewed versioned config 发布，不在 deployment runtime 下载远程列表后自动信任；
  provider endpoint/TLS 失败时 fail closed。
- 安全日志和 rate-limit key 只保留必要、有界、受控的 IP 信息；不把完整 IP 列表、用户 IP 或合成 smoke
  地址写入公开 PR artifact。NAT/校园出口共享会产生误伤，账号级限制和申诉/恢复路径仍必须存在。
- Cloudflare Pseudo IPv4/header transform 是独立配置变更；改变后先确认 backend 应使用真实 IPv6 还是
  pseudo IPv4，并更新 privacy、限流测试和 incident playbook。

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

- 当前每个 staff route 有缺 capability、self/equal/higher target、无 reason 和 stale state 的适用负向测试；
  ADMIN 媒体自审用专用正向测试覆盖，不删除其他 route 的 no-self 负向矩阵。
- 普通管理员委派落地时，再增加 expired/revoked grant、越 target ceiling、越权转授权、自改权限、stale
  version、到期竞态与 authorization/session 失效的 handler→PostgreSQL 矩阵；只有 ADMIN 可创建/修改/
  撤销 grant。在这些测试与实现出现前，该能力保持 `Planned`。
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
- Cloudflare CIDR 与官方 IPv4/IPv6 集合 exact-match；trusted proxy 正向恢复 source IP，direct-origin
  伪造 CF/XFF header 的负向 smoke 保持实际 source。Backend/DB/cache/search port 不得从公网绕过 edge。

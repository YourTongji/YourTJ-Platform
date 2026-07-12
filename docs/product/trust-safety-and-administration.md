# 信任安全、治理与管理后台

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Community operations、Security owner、Identity/Forum/Reviews/Media/Web maintainers
>
> 最近核验：2026-07-12，migrations `0047`、`0048`、`0054`、`0055` 与 governance/identity/achievement/outbox integration tests

管理后台是社区政策的执行界面，不是数据库编辑器。所有操作必须先有产品语义、capability、
目标层级、理由、审计、恢复和通知，再决定按钮放在哪里。

## 当前状态

### Current

- 静态 role→capability RBAC，Web 按服务端 capabilities 构建导航，后端再次授权。
- 管理员可邀请校园邮箱用户，用户仍需证明邮箱所有权；可改 lower-role、禁言/封禁和撤销会话。
- forum/review/media/reported-DM 有独立审核面；主题/评论支持多种可逆状态动作。
- board、tag、watched word、course、activity policy、announcement、settings 和 job trigger 有后台入口。
- 推广有独立 `promotions.manage`、站内安全链接、clean owned asset、状态/排期/受众/排序后台与审计；
  公告后台覆盖 revision、receipt summary 和不可变 mutation history。
- board 后台可配置说明、排序、锁定、最低发帖信任等级和问答属性；公开 board DTO 返回当前用户
  是否可发帖及稳定 restriction code。
- governance audit 记录多域 staff/system 事件；论坛自动隐藏与 staff 隐藏有不同 provenance。
- 治理 audit 与申诉 transition history 在数据库层拒绝 `UPDATE`、`DELETE` 和直接或级联
  `TRUNCATE`；正常 append 不受影响。Production 仍必须把应用 runtime 与 migration/table owner
  分离，trigger 不是给 owner/superuser 绕过权限的替代品。
- 账号制裁、论坛主题/评论处置和课评隐藏会在同一业务事务中创建当事人专属治理通知；通知只含
  有界摘要、目标和治理事件编号，不暴露举报人、审核人或私密证据。
- 申诉中心支持受限制账号通过密码或申诉用途邮箱验证码获取一小时、无 refresh 的 purpose-bound
  凭据；该凭据只可访问本人的申诉和治理通知，不能访问资料、内容、私信或积分接口。
- 申诉按原治理事件精确绑定，30 天内一次提交且要求 `Idempotency-Key`；状态历史 append-only，
  独立复核 capability、原处置人回避、lower-role 层级和 optimistic version 均由服务端执行。
- 申诉队列在 SQL `LIMIT/cursor` 前按复核人当前角色、非本人和原处置人回避过滤；不可领取的
  同级/更高角色案件不会先占满一页再被 gateway 丢弃。
- 撤销决定在同一 PostgreSQL 事务调用 identity/forum/reviews owner adapter 恢复精确制裁或内容状态；
  若存在不兼容的后续治理动作或无法安全修复投影，整项决定 fail closed 并回滚。
- 人工身份/特殊认证由 platform typed definition 与可过期/可撤销 grant 独立建模；后台使用专属
  `verifications.manage` 创建、授予、查看历史和撤销，reason/audit 与 mutation 同事务。
- 积分完整性区使用独立 `credit.integrity` capability，持久展示只读 ledger verification、逐钱包
  projection comparison、漂移汇总、请求原因与任务状态，不提供余额编辑或任意流水写入。
- 角色变更、suspend/解除 suspend 和强制 session revoke 由后端要求当前 session recent-auth；
  Web 收到 428 后使用密码或账号绑定邮箱 code 恢复原操作。
- Media operations workspace 以 `operations.jobs` 提供 `private, no-store` 的 operational hold 和 system
  deletion-job inventory；两类 inventory 每页读取、hold 创建/CAS 续期/解除及 dead-letter retry 都使用
  recent-auth，并写 purpose-limited audit。

### Partial

- “手动注册”只有安全的邀请流程，没有管理员设置明文密码或跳过邮箱验证；这是有意边界。
- 缺账号停用/删除/恢复/purge；申诉仍缺自动 assignment/reassignment、SLA escalation、证据访问工作台
  和最终 retention worker。
- generic settings 只有 string key/value；多数 job trigger 无 durable 状态、进度、失败日志或重试。
  Media system deletion 是已持久化、可重试的局部例外，通用 GC 与历史 purge 仍分别受默认关闭的 rollout/
  policy flag 保护，不能扩写成平台统一任务中心已完成。
- 成就徽章使用独立 `badges.manage` capability，具备 versioned 定义、人工授予/撤销/重新授予、事件历史、
  运营 UI 与同事务审计；它不复用身份/特殊认证权限。自动贡献事实通过 durable outbox 投递，授予、
  mint 和 outbox completion 同事务幂等；人工/自动授予及撤销都有可到达的 durable 通知。
- 推广的曝光/点击聚合、公告 receipt 保留策略、批量审核和综合服务健康视图缺失；积分完整性已有
  只读视图，但告警/SLO 和受审批 projection 重建仍缺。
- Staff WebAuthn/MFA、高风险双人确认和尚未交付的 PII 操作仍缺完整安全流程。

## Trust level — 统一信任等级 1–6

信任等级现在是 Lv.1–6 的统一尺度，由 `activity` domain 基于 lifetime 有效活动分数自动评定。
Lv.0 仅供未注册访客 UI，注册账号最低 Lv.1。详细阈值、策略版本化、自动升/降级逻辑
与审计见 `activity-scoring.md` 的「统一信任等级与活动分数」一节。

### 发帖与举报权重

| 等级 | 举报权重 |
|---|---:|
| Lv.1（绿茶）| 0.5 |
| Lv.2（白茶）| 1.0 |
| Lv.3（黄茶）| 1.5 |
| Lv.4–6（青/红/黑茶）| 2.0 |

同一内容 open report 总权重达到 `3.0` 自动隐藏。作者在 24 小时内两次内容
自动隐藏时，系统尝试追加 24 小时 silence。identity 跳过 mod/admin silence，
但普通用户举报仍可达到阈值并自动隐藏 staff 作者内容。Staff 不作为普通 reporter，
而应走理由化管理动作。自动措施保留 provenance 和通知。

### Board 权限

Board create-thread 在限流消费前检查 `is_locked/min_trust_to_post`，并在写事务锁定
board 后再次检查，避免策略并发修改时穿透。`moderation.content` 是唯一 staff
exception：只绕过 board lock/min-trust 两个 gate，仍必须通过 active account、sanction、
内容 policy 和 rate limit。该例外不是通用发帖免检，也不授予 `community.manage`。

迁移 0059 将旧 `min_trust_to_post` 值 +1 以适配统一量表（旧 TL1 → 新 Lv.2）。
信任阈值现在由 `activity.trust_level_policies` 版本化管理，不在代码中硬编码；
管理员可通过后台 `PUT /api/v2/admin/trust-policy` 修改阈值、like 日上限和 reason，
并查看版本历史。

## 治理原则

1. Staff 不静默改写用户表达；常规动作是隐藏、软移除、恢复、关闭或归档。
2. 可逆动作优先于物理清除；retention purge 不是 moderator 日常按钮。
3. 风险操作要求明确理由、作用范围、时长、目标和预期结果。
4. 同级/更高角色、自身对象和利益冲突案件受到额外限制。
5. 被处置者应获得类别、时长、政策依据、影响与申诉入口，安全例外需记录。
6. 公共身份是 handle；后台默认也不显示校园邮箱或任意私信。
7. 积分账本不可重写，管理后台不提供余额编辑或任意 ledger append。

## 角色、能力与目标层级

`user < mod < admin` 只描述默认角色层级；具体授权使用 capability。核心能力见
[授权与审计](../security/authorization-and-audit.md)。

- moderator 处理 lower-role 内容和有限时长 silence，不能授予角色或永久 suspend。
- admin 可管理 lower-role 角色、suspend、社区结构和平台策略，但不能处置同级 admin。
- system/service 是审计 actor kind，不是绕过权限的人类账号。
- 最终管理员 bootstrap、恢复和撤销需要独立 out-of-band 流程，不能通过普通 Web endpoint 完成。

## 用户管理

后台用户区至少覆盖：

- 按 handle 或精确 account id 搜索，分页筛选角色/状态；默认不暴露邮箱。
- 邀请校园邮箱用户：当前固定 7 天过期、一次接受、要求理由，接受者仍走邮箱验证；若要改为
  可配置期限，先定义上下限和 versioned policy。
- 查看公开资料、当前角色/状态、制裁与撤销历史、有限会话信息。
- role change、silence、suspend、revoke sanction、revoke sessions 当前执行目标层级；其中角色、
  suspend/解除 suspend 和强制注销有 server-side recent-auth，普通 silence 不滥用 step-up。
- 当前 moderator silence 必须有未来结束时间且最长 30 天；admin 可使用更长或 indefinite sanction，
  仍需 lower-role target、理由和审计。
- 未来的 deactivate/delete/recover/purge 使用独立账号生命周期 workflow，不复用封禁。

管理员不得创建或查看用户密码。确需协助注册时，发送可到期邀请；确需恢复账号时，走验证和
审计流程，而不是直接修改数据库。

## 内容与媒体管理

后台保持独立工作区：

1. forum reports：uphold/reject/ignore，查看必要上下文和自动隐藏 provenance。
2. review reports/status：决定举报、隐藏、恢复或软移除，不改写作者评分/正文。
3. reported DM：仅具体举报消息和有限上下文，无通用私信浏览器。
4. media review：pending/clean/quarantined/dead-letter asset；mod 只可审核 user，admin 只可审核 user/mod，
   reviewer 不能自审或审核同级。Image approve 必须由同一 reviewer 先完成可信预览，file/PDF 在 scanner
   证据接入前不可批准；block 先停止公开派生，再由 durable job 删除 provider object。普通 queue 只
   显示通用 hold state，不披露 operations reason/actor。
5. media operations：限时 `moderation/security` operational hold 与 system dead-letter 独立于 moderator
   日常审核，只向 `operations.jobs` 暴露。Hold inventory 需 recent-auth，续期/解除以 `expectedHoldId`
   compare-and-set；system deletion inventory 每页读取同样要求 recent-auth 并审计 result count，retry
   另外要求 reason。所有 inventory 都是 `private, no-store`。该机制只暂停物理删除，不恢复
   quarantined 内容，也不构成 legal hold。
6. content recovery：通过精确 id 查找 retained hidden/deleted 内容并执行理由化恢复。

主题支持 pin/unpin、close/reopen、archive/unarchive、hide/unhide、soft-delete/restore、move；
评论至少支持 hide/unhide、soft-delete/restore。所有动作按 target type/state 只展示当前合法转换，
后端仍验证。恢复时同步 search、feed、counters、activity 和 notifications 的必要投影。

Profile 上的 staff action 只提供 capability-aware 用户目录 deep link，不把制裁/PII 表单嵌入公共
markup；thread/comment 上的 inline control 根据当前 state 只展示合法转换。课程删除在存在任意
retained review row 时必须拒绝，不论其当前 visibility/moderation status；数据库 restrictive foreign
key 是最终不变量，不能依赖可能过期的 visible-review counter。

## 举报、处罚与申诉

- 举报有固定 category、可选有界说明、reporter/target、状态与证据引用。
- 同一 reporter/target 最多一个 open case；终态历史保留，后续新事件可创建新 case。
- 自动阈值隐藏与人工决定分离；reject/ignore 只撤销对应自动 transition。
- 处罚分 silence 与 suspend；期限、范围和撤销作为追加事件记录。

当前申诉状态：

```text
submitted -> in_review -> upheld | overturned | amended
          \-> withdrawn
```

appeal 关联原治理事件，不覆盖原决定。当前约束如下：

- 用户只能对属于自己的、owner domain 明确认可的账号制裁、主题/评论隐藏或删除、课评隐藏发起
  申诉；不支持的 action/target 统一按 not found 处理，避免泄露治理事件。
- 同一原事件和账号最多一项 appeal；提交窗口是原事件后 30 天，重复同一幂等请求返回既有结果，
  相同 key 携带不同请求返回 conflict。
- `appeals.review` 独立于一般内容审核；复核人必须高于当事人、不能是原处置 actor，并通过 version CAS
  领取案件，避免并发复核覆盖。队列查询与领取 mutation 使用同一 hierarchy/recusal 语义，读取分页
  不能产生“有可处理案件却返回空页”的后过滤假象。
- `overturned` 恢复对应 sanction/content 并同步 counters、activity、vote/feed/search cache 等必要投影；
  owner 检测到后续状态变化时拒绝决定。`amended` 当前仅支持缩短账号制裁，不能扩大范围或延长期限。
- 用户可在领取前撤回。每次 submitted/in-review/terminal transition 均追加不可变 history、中央 audit
  和 purpose-limited notice；用户可看公开理由，不可看 staff id、举报人或内部 evidence。

## 反滥用与 Captcha

当前 email code、课评发布和课评举报接入 YourTJCaptcha；浏览器只把 opaque single-use pass
token 发送给平台，平台按 operation purpose 在 Redis 原子消费。论坛写入目前主要依赖 auth、
sanction、trust-level rate limit 和 watched words，尚未接入同一 captcha 流程。

目标防线分层：

- IP/account/device 维度有界 rate limit，不记录不必要的长期 fingerprint。
- 注册、账号恢复、高风险发布/举报使用 purpose-bound captcha，provider 不接收邮箱或正文。
- watched words、重复内容、链接/mention 速度和举报信号进入可解释的 pending/review 流程。
- 自动措施有 provenance、阈值、过期、人工复核和误判恢复，不做不可审计的永久 shadow ban。
- 同一业务 retry 先命中 idempotency，再消费 captcha，避免网络重试要求用户重复挑战。
- Provider 不可用时，受保护写入按产品定义 fail closed；已有内容读取不应被无关 provider 阻断。

新增 anti-abuse signal 前必须说明数据用途、保留、误判、申诉和 staff 可见性。

## 管理后台信息架构

| 区域 | 核心工作 | 主要 capability |
|---|---|---|
| Overview | 队列、任务失败、安全/可靠性指标 | 按卡片能力组合 |
| Users | 邀请、搜索、角色、制裁、会话、生命周期 | `users.*` |
| Moderation | forum/review/DM reports、内容恢复 | `moderation.content` |
| Appeals | 独立领取、维持/撤销/缩短、状态历史 | `appeals.review` |
| Media | scan/flag queue、approve/block、asset lookup | `moderation.content` |
| Community | boards、tags、watched words | `community.manage` |
| Promotions | placement、clean asset、站内目标、排期、受众、状态 | `promotions.manage` |
| Achievements | 受控定义、自动规则、人工授予/撤销与事件历史 | `badges.manage`；不可复用认证权限 |
| Verifications | typed 身份/特殊认证定义、授予历史、到期与撤销 | `verifications.manage` |
| Announcements | draft、排期、发布、revision、receipt summary | `announcements.manage` |
| Policies | 社区规则、隐私政策、条款的 draft/review/publish/version/acceptance | 独立 `policies.manage` |
| Activity | 当前权重、预览、版本历史、发布 | `activity.policy` |
| Courses | catalogue 管理与受保护删除 | `courses.manage` |
| Settings | typed/versioned setting | `platform.settings` |
| Jobs | sync/reindex/reconcile；media operational hold inventory、system deletion dead-letter/retry | `operations.jobs` |
| Audit | actor/action/target/result/request filters、受控 export | `audit.read` |
| Credit integrity | 只读 ledger verify/reconcile、逐 wallet 漂移证据 | `credit.integrity` |

高风险按钮使用明确的动词和后果文本，要求 reason，防止重复提交；颜色不是唯一状态信号。列表
包含 loading、empty、error 和分页。批量操作先 preview，限制数量，返回每项结果并共享 correlation id。

## 设置、任务和运营健康

- 设置必须有 type、schema、描述、默认值、owner、version 和 domain validation；不能无限扩展
  generic string key/value。
- 操作任务持久化 `requested/queued/running/succeeded/failed/cancelled`，含 requestor、reason、
  progress、bounded log、dedupe key、retry count 和 timestamps。
- 重试幂等；同类互斥任务有锁，UI 的“已提交”不能显示为“已完成”。
- Media system deletion 已实现 durable retry/dead-letter；operator 只能重排非 moderation system job，
  reason 进入独立 retry event，不能覆盖原 job purpose。Provider 成功后 storage locator/fingerprint redacted，
  inventory 不返回 key/URL/hash。通用 GC 是否运行仍以环境 flag、startup log 和 queue 事实为准。
- 运营健康至少显示审核积压/时长、任务失败、索引/投影漂移、邮件/OSS 故障和恢复状态。
- credit 只提供 verify/reconcile 结果，不提供任意 balance editor。每次 run 需要 reason 与幂等 key，
  同类任务单并发；中断 run 通过理由化 resume 恢复且不新建快照；账本验证失败时停止 projection
  comparison，所有异常只留证不自动修复。

社区规则、隐私政策和服务条款不是普通 announcement/string setting。它们需要 immutable version、
draft/review/published/retired、effective time、owner/approver、diff、适用受众和用户 acceptance receipt。
重大版本在 onboarding/登录后要求重新确认；历史版本继续支持治理案件和导出解释。

## 推广、公告和徽章治理

- 推广只使用 clean asset、受控 URL、排期和 audience；第一阶段限定自营信息。
- 公告修改保留 revision；删除改为 archive，强制确认由 requires-ack policy 控制。
- 成就定义只接受受控图标 token，使用 version CAS；停用不删除历史。自动授予写入 achievement event
  和幂等 pending mint，人工授予明确不 mint；人工撤销/重新授予追加事件且不反转历史积分。
- 身份/特殊认证默认私密；只有 definition 允许且 grant 明确公开的有效认证进入 profile。公开不含
  issuer、reason、evidence；图标/样式来自受控 enum，不接受任意素材或 CSS。
- 认证 evidence 字段只保存 opaque internal reference，后台列表只显示是否存在引用，不回显原始引用；
  实际证据仍需独立 purpose-limited store、访问审计和 retention policy。
- 角色标识来自实时权限，不通过徽章永久复制。

## Decision needed

- 最终管理员 bootstrap/recovery 与双人审批机制。
- 哪些更高风险操作在 recent-auth 之上还必须双人审批。
- Staff MFA/WebAuthn、recovery code、break-glass 与最终管理员恢复流程。
- 推广位是否允许商业内容，以及后续广告合规/计费边界。
- 各具体认证类型的证据标准、默认有效期、复核周期和证据存储/保留政策；通用 typed grant 与默认私密
  展示边界已经确定，不再通过 string setting 临时配置。
- appeal reviewer 自动分配/改派、SLA escalation、举报证据和治理审计保留期；独立复核、冲突回避与
  owner-domain 原子恢复已经是实现基线。
- 真正 legal hold 的授权主体、case id、跨域范围、通知/release policy 与审计保留；Media 现有
  moderation/security operational hold 不满足该语义。Media hold/retry/job/evidence 的 365 天 purge
  虽有代码但默认关闭，启用前仍需 privacy/legal owner 批准；append-only governance actor audit 期限
  另行决定。

## 验收基线

- Web capabilities 与后端授权独立生效，直接构造请求仍被拒绝。
- 自操作、同级/更高角色、缺理由、过期 recent-auth 和利益冲突案件有负向测试。
- 内容处置、恢复、处罚、申诉和策略变更在业务事务中追加审计。
- audit/appeal history 的 row mutation 与 table truncate 均被真实 PostgreSQL 负向测试拒绝，拒绝后
  正常 append 和申诉状态流仍可继续。
- 受限制账号能使用 purpose-bound appeal token 查看/提交自己的申诉，但同一 token 访问普通 `/me`、
  内容、私信和积分接口均被拒绝；他人事件、原处置人复核、同级目标和 stale version 有负向测试。
- overturned/amended 必须与精确 owner-domain reversal 在同一事务提交；任何不支持的 action、后续
  冲突状态或投影修复失败都不能留下半完成的终态历史。
- Staff 无任意 DM/PII 浏览能力，敏感 evidence read 本身被审计。
- 管理 UI 不伪造任务完成、媒体状态、公告确认或 credit 完整性。
- 用户、内容、媒体、推广、公告、徽章和任务的核心状态机都有可恢复路径和验收旅程。
- 认证后台缺 capability、self/equal/higher target、非法展示、重复有效 grant、过期/重复撤销与非法
  evidence reference 均有 handler→PostgreSQL 负向测试。
- 成就后台缺 capability、self/equal/higher target、非法 icon/text、stale version、重复授予、撤销/
  重新授予和人工 mint 禁止均有 handler→PostgreSQL 测试；Web 操作具备 reason、后果说明和 axe 验证。

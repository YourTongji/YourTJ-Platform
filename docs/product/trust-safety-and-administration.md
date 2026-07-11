# 信任安全、治理与管理后台

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Community operations、Security owner、Identity/Forum/Reviews/Media/Web maintainers
>
> 最近核验：2026-07-12，`codex/x-credit-reconciliation`

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
- 人工身份/特殊认证由 platform typed definition 与可过期/可撤销 grant 独立建模；后台使用专属
  `verifications.manage` 创建、授予、查看历史和撤销，reason/audit 与 mutation 同事务。
- 积分完整性区使用独立 `credit.integrity` capability，持久展示只读 ledger verification、逐钱包
  projection comparison、漂移汇总、请求原因与任务状态，不提供余额编辑或任意流水写入。

### Partial

- “手动注册”只有安全的邀请流程，没有管理员设置明文密码或跳过邮箱验证；这是有意边界。
- 缺账号停用/删除/恢复/purge、当事人通知、申诉和独立复核。
- generic settings 只有 string key/value；job trigger 无 durable 状态、进度、失败日志或重试。
- 成就徽章后端存在但定义、人工授予与撤销 UI 仍不完整；它不复用已经闭环的身份/特殊认证后台。
- 推广的曝光/点击聚合、公告 receipt 保留策略、批量审核和综合服务健康视图缺失；积分完整性已有
  只读视图，但告警/SLO 和受审批 projection 重建仍缺。
- 高风险角色/永久封禁/PII 操作没有 recent-auth 或双人确认。

## Trust level 与当前自动规则

Trust level 是反滥用/权限信号，不是每日活跃度或积分。当前写入限制为：

| 行为 | TL0 | TL1+ |
|---|---:|---:|
| 发主题 | 2/day | 5/minute |
| 发评论 | 5/day | 20/minute |
| 投票 | 30/minute | 60/minute |
| 举报 | 5/day | 15/day |

举报权重当前为 TL0 `0.5`、TL1 `1.0`、TL2 `1.5`、TL3 `2.0`；同一内容 open report 权重达到
`3.0` 自动隐藏。作者在 24 小时内出现两次内容自动隐藏时，系统尝试追加 24 小时 silence；
identity 会跳过 mod/admin silence，但普通用户举报仍能达到阈值并自动隐藏 staff 作者内容。
Staff 不能作为普通 reporter，而应走理由化管理动作。自动措施保留 provenance 和通知。

当前自动迁移每次扫描对每个 active account 最多改变一级，降级优先于升级：

- TL0→TL1：注册至少 2 天，主题+评论至少 3，读过至少 10 个主题。
- TL1→TL2：注册至少 15 天，累计论坛 active day 至少 8，获赞至少 10，且没有被裁定成立的举报。
- TL2→TL3：注册至少 60 天，最近 60 天 active day 至少 20，获赞至少 50，成功举报至少 3。
- TL2/TL3 有被裁定成立的举报时，本次扫描只降一级。

Active day 由主题、评论、vote 或 thread read 的 UTC 日期事实计算。Board create-thread 在限流消费前
检查 `is_locked/min_trust_to_post`，并在写事务锁定 board 后再次检查，避免策略并发修改时穿透。
`moderation.content` 是唯一 staff exception：只绕过 board lock/min-trust 两个 gate，仍必须通过 active
account、sanction、内容 policy 和 rate limit。该例外不是通用发帖免检，也不授予 `community.manage`。

剩余 `Partial/P0` 是 staff 内容达到普通举报阈值时自动隐藏还是升级独立复核队列。Trust 阈值目前
仍是代码 policy；若开放后台配置，必须先版本化、预览影响并审计，不能复用 generic string setting。

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
- role change、silence、suspend、revoke sanction、revoke sessions 当前执行目标层级；高风险动作补
  recent-auth 是 `Planned/P0`，不能因 UI confirmation 当作已经完成。
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
4. media review：pending/scan failure/flagged asset；reviewer 不能审核自己的上传。
5. content recovery：通过精确 id 查找 retained hidden/deleted 内容并执行理由化恢复。

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

目标申诉状态：

```text
submitted -> in_review -> upheld | overturned | amended
          \-> withdrawn
```

appeal 关联原治理事件，不覆盖原决定；复核人不能是原处置人，证据访问最小化。overturned 时
恢复合法内容/账号状态并触发相关投影修复；amended 记录新的范围或期限。

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
| Media | scan/flag queue、approve/block、asset lookup | `moderation.content` |
| Community | boards、tags、watched words | `community.manage` |
| Promotions | placement、clean asset、站内目标、排期、受众、状态 | `promotions.manage` |
| Badges | 成就定义、规则与人工例外 | 后续独立 achievement capability；不可复用认证权限 |
| Verifications | typed 身份/特殊认证定义、授予历史、到期与撤销 | `verifications.manage` |
| Announcements | draft、排期、发布、revision、receipt summary | `announcements.manage` |
| Policies | 社区规则、隐私政策、条款的 draft/review/publish/version/acceptance | 独立 `policies.manage` |
| Activity | 当前权重、预览、版本历史、发布 | `activity.policy` |
| Courses | catalogue 管理与受保护删除 | `courses.manage` |
| Settings | typed/versioned setting | `platform.settings` |
| Jobs | sync/reindex/reconcile 状态、日志、重试 | `operations.jobs` |
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
- 成就徽章的自动规则、人工授予和撤销都审计。
- 身份/特殊认证默认私密；只有 definition 允许且 grant 明确公开的有效认证进入 profile。公开不含
  issuer、reason、evidence；图标/样式来自受控 enum，不接受任意素材或 CSS。
- 认证 evidence 字段只保存 opaque internal reference，后台列表只显示是否存在引用，不回显原始引用；
  实际证据仍需独立 purpose-limited store、访问审计和 retention policy。
- 角色标识来自实时权限，不通过徽章永久复制。

## Decision needed

- 最终管理员 bootstrap/recovery 与双人审批机制。
- 高风险操作使用密码重验、recent-auth challenge 还是双重审批。
- Staff MFA/WebAuthn、recovery code、break-glass 与最终管理员恢复流程。
- 推广位是否允许商业内容，以及后续广告合规/计费边界。
- 各具体认证类型的证据标准、默认有效期、复核周期和证据存储/保留政策；通用 typed grant 与默认私密
  展示边界已经确定，不再通过 string setting 临时配置。
- appeal reviewer 分配、SLA、举报证据和治理审计保留期。

## 验收基线

- Web capabilities 与后端授权独立生效，直接构造请求仍被拒绝。
- 自操作、同级/更高角色、缺理由、过期 recent-auth 和利益冲突案件有负向测试。
- 内容处置、恢复、处罚、申诉和策略变更在业务事务中追加审计。
- Staff 无任意 DM/PII 浏览能力，敏感 evidence read 本身被审计。
- 管理 UI 不伪造任务完成、媒体状态、公告确认或 credit 完整性。
- 用户、内容、媒体、推广、公告、徽章和任务的核心状态机都有可恢复路径和验收旅程。
- 认证后台缺 capability、self/equal/higher target、非法展示、重复有效 grant、过期/重复撤销与非法
  evidence reference 均有 handler→PostgreSQL 负向测试。

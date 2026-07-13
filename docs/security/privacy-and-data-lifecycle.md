# 隐私与数据生命周期

> 文档类型：安全与隐私规范
>
> 状态：Active
>
> 负责人：Privacy owner、Security owner、Domain maintainers
>
> 最近核验：2026-07-13，migrations `0064`–`0065`、隐私/生命周期实现与 ADMIN 媒体自审边界

本规范将数据最小化、可见性、导出、删除和保留作为产品前置条件。它不是法律意见；涉及 PIPL、
未成年人、广告或跨境处理的最终政策需要合格法律与隐私负责人确认。

## 当前状态

### Current

- 校园邮箱不出现在公开 profile、论坛或现有 staff directory DTO。
- Identity 支持 email-at-rest encryption/blind index 配置。
- Main staging/production deployment 要求独立的 32-byte AEAD/blind-index key 与 strict mode；应用在启动
  时完成兼容行 backfill，并在仍有 plaintext email 时 fail closed。PR preview 只允许合成邮箱且不复用
  main key。
- 设备 session 只向账号本人展示 bounded user-agent label 和必要时间；新认证流程不持久化精确 IP。
- recent-auth 只在当前 session 保存服务端验证时间和受控方法标签，不保存密码、code、
  email 或第二份账号级凭据；session 撤销/保留即是其撤销/保留边界。
- Password recent-auth 额外保存当时的账号 credential version；密码改变后旧验证即使并发返回也不能
  写回 fresh。Recovery credential 只存 SHA-256、proof method、lifecycle version、15 分钟 expiry 与
  consume time，不是普通认证 token，浏览器只放 sessionStorage。
- Identity 密码 set/change/reset 和 refresh replay 只写 account-scoped 安全事实类型、可选 session id 与时间，
  不写邮箱、IP、user-agent、password/code/token 或请求正文；普通登录失败不作 PII 事件流。Owner export
  只返回尚在保留期的 `eventType/createdAt`，不返回 subject session id。
- 密码安全通知和管理员邀请的 durable job 只持久 account id、固定 template kind、lease/retry 和有界错误码；
  收件邮箱和正文在 worker 内存中解密/渲染，不进入 job、owner export 或日志。
- Staff 无通用 DM 浏览接口，只能访问 participant 报告的最小证据。
- 陌生私信请求只保存一条最多 1000 字附言；decline/withdraw/block 会立即删除未举报正文，report
  只保留被举报附言作为治理证据。请求状态、pair、时间、撤回 5 分钟防抖和拒绝/block 30 天冷却
  属于最小反骚扰元数据。
- Governance audit 和制裁保留 actor/reason 历史，credit ledger append-only。
- Profile 默认仅校园登录用户可见；followers/following 默认仅关注者可见，新 DM 默认只允许接收方
  已关注的人发起。匿名只有在 owner 显式选择 `public` 后才能访问资料。
- Follow、mute、block 是独立事实；mute/block 不向对方提供列表接口。Block 删除双方 follow，
  suspended/deleted 账号不进入公开资料与关系列表。
- Display name、院校、bio、HTTPS website 与 privacy setting 可由 owner 替换；院校默认“同济大学”且
  与其他公开资料一起受 profile visibility 控制；avatar/banner 只保存本人 clean OSS asset id，公开
  URL 是状态校验后的派生值。
- 公共 board 中仍可见的主题/评论把 canonical handle、可选 display name 与当前 clean + published avatar
  作为内容署名，而不是资料页披露；因此 profile/activity visibility 不会匿名化已经发布的公共讨论。
  Forum 只对已授权内容行批量请求 active public account 与短期 typed Media Delivery，账号生命周期关闭、
  内容隐藏或头像失去 clean/published 状态后不再返回头像；DTO 不包含邮箱、object key 或持久 vendor URL。
- 人工认证默认私密；公开 profile 只返回 definition 允许且 grant 明确公开的有效 label/说明/有效期，
  不返回 issuer、reason、evidence reference 或内部 grant id。
- 账号搜索只索引已验证、discoverable 且非 `only_me` 账号的 id、handle 和可选 display name；响应
  回表重验 active/suspension、profile visibility、block/mute 与 clean avatar，不索引或返回邮箱、bio、
  relationship 私密名单和内部治理字段。
- 治理通知是 account-private 最小投影，只保存有界摘要、opaque subject/event/appeal id 与已读时间；
  不复制举报人、staff identity、evidence 或完整内容。申诉记录保存原事件引用、本人理由、公开决定理由
  和 append-only transition history，内部 reviewer id 只在 capability-gated admin DTO 出现。
- Appeal access JWT 只有一小时有效期、无 refresh/session 持久化，浏览器仅放 sessionStorage；普通 API
  拒绝该 scope，减少受限账号为申诉而重新开放其他个人数据的风险。每次 restricted credential 另生成
  不含 token/账号信息的随机 cache partition，退出时同步清除该 partition 的申诉与治理通知缓存，避免
  同一浏览器切换受限账号时短暂展示前一账号数据。
- Identity、Forum、Media、Platform、Reviews 与 Credit 的普通认证入口都必须显式拒绝带 scope 的受限
  credential；不能把没有 `sid/ver` 的 appeal token 当作 legacy access token。钱包、账本、订单、履约和
  credit 管理端点均有负向矩阵测试。
- Activity visibility 默认 `only_me`，本人始终可读，`public` 允许匿名、`campus` 只允许已登录校园
  账号；它在 profile visibility 和双向 block 之后控制逐条主题/回复/likes/media
  tabs。Profile 可见时，主题/回复/获赞 aggregate 仍是公共内容贡献计数，不因列表私密而置零。
- Likes tab 只披露仍有效的正向 Forum vote 所指向的当前可见内容；media tab 只披露 owner-authored、
  exact binding 仍 clean/published 的内容。Bookmarks 只向 owner 返回并在读取时过滤已不可见目标；三者
  都应用 lifecycle、block/mute 和 content policy，不向客户端返回存储 key、hash 或他人私密字段。
- Mention policy 默认 `everyone`；`following` 表示接收方关注作者。Identity 的 batch projection 只
  返回 active、未 suspended 账号的 id/handle/policy，Forum 再应用 follow/block/mute 和通知偏好。
  不满足策略、未知或生命周期关闭的 handle 仍保留为公开普通文字，不产生通知或存在性信号。
- Forum 主题/评论图片只接受本人 clean platform asset。公开内容 DTO 返回正文精确引用所需的 asset id、
  alt、position、可选尺寸和状态校验后的派生 URL；不返回 Ingest key、bucket/provider host、独立 key/hash
  字段、上传 owner 或原始回调信息。短期 CDN URL 自身包含可见的 immutable Delivery path。
  pending/blocked upload id 仅可留在 owner draft/status surface，不构成公开授权。
- Forum revision body 与 historical attachment 不是公共 profile 数据：作者本人可读；staff 只可凭内容
  审核能力读取另一位严格 lower-role 作者的有界 page。普通他人、self-targeted staff 权限以及同级/
  higher-role target 不能扩大历史内容或旧 asset URL 的可见性。
- 自助 owner export 由 Identity 保存 recent-auth 保护的 durable job；八个 owner domain 仅返回本人可见
  projection。完成 artifact 在 PostgreSQL JSONB 最多保留 24 小时，下载用 account-bound、SHA-256 at-rest、
  5 分钟且一次性消费的 header token，并记录 `downloaded_at`。原始校园邮箱不复制进 JSONB artifact；
  只在授权下载响应组装时从 Identity encrypted-at-rest 字段解密并注入。
- Deactivate/delete 立即撤销 session。30 天恢复窗后 durable worker 删除 Identity 邮箱/密码/session/
  profile/privacy/onboarding/recovery/export artifact，并清理 Forum/Reviews/Activity/Platform/Media 的
  owner-private projection；ledger verification 所需的 Ed25519 public key 只做 revoke 并随 tombstone
  保留，公共内容、治理事实和 credit ledger 因而仍可验证。跨域清理开始前，worker 在账号行锁中重验
  deadline 并原子写不可逆 purge marker；running/failed/marker 任一成立都禁止恢复，避免部分清理后重新
  激活。耗尽 20 次的 job 可由 `operations.jobs` 管理员 recent-auth 后审计 requeue，但不能移除 marker。
- 账号 purge 后 Identity email worker 不再解密或投递，未完 job 进入无收件人终态并按 90 天清理。
  安全事实可在 365 天保留期内继续关联不可反查邮箱的 tombstone account id，到期后删除；不阻止 PII purge。
- Upload callback bearer 只在 provider flow 短暂流转，PostgreSQL 仅保存 SHA-256 digest；migration `0057`
  backfill digest 后删除 plaintext column。Credential issuance 的 account-scoped quota/attempt facts 不含
  callback token、object body、IP 或 device fingerprint。

### Partial

- Public profile 的外部搜索引擎索引政策尚未完成；精确 handle 直达仍由 profile visibility 决定，
  不因站内 discoverability 关闭而伪装成账号不存在。
- Lifecycle worker 已覆盖数据库 owner-private projection、用户搜索 reconciliation，以及 lifecycle
  dead-letter 的 capability-gated list/requeue API；Media 子步骤会协调 rollout-gated durable OSS deletion，
  但仍不存在通用 legal-hold registry、统一 operator UI 或跨域逐项 reconciliation report。
- Media 已有局部 intent housekeeping、rollout-gated GC、账号 purge 协调和 operations-history purge 代码；
  未完成 breaking cutover、DB/published Markdown/OSS reconciliation 和逐环境启用前，不能声称 OSS 删除闭环。
- Scanner、variants、CDN cache、日志和 backup expiry 仍未形成完整、可演练的删除编排。
- 公共内容、治理证据、audit、DM 中他人副本和备份的具体保留期限仍待 privacy/legal owner 决策；因此
  `purged` 不等价于“所有历史物理字节立即消失”。
- 当前 export worker 在内存中组装完整 JSONB，适合现阶段规模但不是大账号终态；上线大规模历史前需
  改为有界分页、流式加密 archive/object storage、完整性摘要与同等短期授权，不能靠提高进程内存硬撑。

## 数据分类

| 类别 | 示例 | 默认访问 | 处理原则 |
|---|---|---|---|
| 资格 PII | 校园邮箱、邮箱验证状态 | identity purpose only | 加密/盲索引、绝不公开、限制保留 |
| 安全凭据 | password hash、code hash、refresh hash、keys/tokens | security code only | 不记录明文、最短保留、可撤销 |
| 会话元数据 | bounded user-agent、创建/最近使用/到期时间、recent-auth 时间/方法 | 账号本人、安全代码 | 不收集精确 IP，不存 credential，随 session retention 删除 |
| 公开身份 | handle、公开头像、display name、院校、bio | 资料按 profile visibility；公共内容保留最小作者署名 | 用户可控、handle history 防冒用；内容署名不携带资料正文或 PII |
| 公共内容 | thread、comment、review、reaction | 按 board/content policy | revision、治理、导出/删除规则 |
| 社交关系 | follow、block、mute、subscription | 本人及 policy 允许对象 | block/mute 默认私密、最小暴露 |
| 私密通信 | DM body、单条 request 附言、private attachment | participants | staff 仅举报证据；未举报 declined request 正文立即删除，其他内容独立 retention |
| 治理证据 | reports、sanctions、appeals、appeal history、audit | 本人最小披露；staff capability + purpose | 防篡改、访问审计、期限/hold；不向本人泄露 reporter/staff/evidence |
| 治理通知 | 处置/申诉安全摘要、subject/event/appeal id、read time | 仅受影响账号 | 不受互动偏好关闭、无 evidence/PII、随治理 retention 协调 |
| 通知 outbox/receipt | account/actor id、event type、有界站内 payload、状态/error code、delivery outcome | consumer；operators 只见无 payload 元数据 | 成功/取消 30 天，dead/receipt 90 天；不含邮箱、secret、stack trace |
| 认证凭证 | type/grant、签发/撤销原因、opaque evidence reference | `verifications.manage`；允许时为最小公开投影 | 默认私密、可到期/撤销、公开不含证据/操作者 |
| 运营数据 | job log、metrics、aggregated promo events | operators | 聚合、去标识、有限保留 |

邮箱维度的 Redis abuse-control key 只能使用 Identity 生成的 opaque blind index/HMAC subject；不得把规范化
邮箱直接写入 Redis key 或限流错误日志。
| Identity 安全事实 | password set/change/reset、refresh replay 类型与时间 | owner export、security code | 不含 PII/credential；365 天后由 retention worker 删除 |
| Identity email job | account id、template kind、attempt/lease/error code | worker；无用户/staff payload API | 不含收件人/正文；succeeded 30 天、dead 90 天 |
| Owner export artifact | 八域本人数据 JSON、job/下载时间 | 仅 owner + worker | recent-auth 创建、24 小时清除、5 分钟一次性下载 grant，不写日志 |
| 媒体操作元数据 | operational hold/release、system deletion/retry、redacted-object evidence | `operations.jobs` 或限定的 moderation purpose | no-store inventory、理由化访问、有限保留；不包含 provider key/URL/hash |
| Staff 授权元数据 | 普通管理员 assignment、capability、target ceiling、expiry、grant/revoke event | ADMIN/security owner；被授权人只见本人有效范围 | 不公开；变更即时失效；理由/历史随安全审计保留，不进入普通 owner export |
| 外链预览缓存 | 无 query 的规范化 allowlisted HTTPS URL、公开 metadata、失败类别 | 服务端与页面请求者 | ready 最长 7 天、error 2 分钟；query URL/远程图片不持久化，日志只记 hash |
| 公告 receipt | announcement/revision、seen/dismiss/ack time | 本人、汇总后的公告管理员 | 账号删除级联清除，不记录设备/IP；后台只返回聚合计数 |
| 积分记录 | ledger、wallet projection | owner/verification policy | ledger 不改写，删除后 tombstone |
| 积分完整性证据 | run reason、operator id、wallet account id、派生/缓存差额 | `credit.integrity` staff | 随 ledger/audit 完整性证据保留；不含邮箱、签名、key 或原始错误 |
| 交易履约信息 | escrow product `deliveryInfo` | purchase buyer/seller | 不进入公开 listing/search/log；随订单保留与删除策略最小化 |

新 column/event/index 前必须在对应产品文档说明 data category、purpose、controller/processor、
可见者、retention、export 和 deletion。没有答案时不得先“留着以后分析”。

## 可见性与默认值

- Board 声明 `public/campus/staff` 等访问级别；公共讨论不由作者 follow 关系临时改变。
- Profile、activity、follower/following、new-DM、mention 和 discoverability 使用独立设置；隐藏
  activity list 不会改变 board/content policy，拒绝 mention 不会改写公开正文。
- Block/mute 是关系 policy，不通过前端隐藏代替服务端授权。
- 人工认证只有在 definition 允许公开、grant 明确 `displayOnProfile`、未撤销且未到期时可见；过期和
  撤销实时从公开投影消失，不依赖前端缓存隐藏。
- 搜索、feed、cache、CDN 与 notification 在输出时应用同一可见性规则。
- 第一阶段默认：profile=`campus`、activity=`only_me`、followers/following=`followers`、
  DM=`following`、mention=`everyone`、discoverable=on；owner 可按 OpenAPI 的有界枚举调整。

## Profile 与社交关系数据生命周期

`identity.profiles` 的目的仅为用户选择公开展示的校园社区身份；`identity.profile_privacy` 仅用于
服务端授权。`forum.user_follows` 用于用户关系和准确计数，`forum.user_mutes` 与保留旧物理名但
语义已固定为 block 的 `forum.user_ignores` 用于本人过滤与安全边界。可见者和生命周期如下：

- display name/院校/bio/website/asset id：owner 可编辑，viewer 仅在 profile policy 允许时读取；院校
  用于公开校园归属展示，默认“同济大学”，不作为邮箱资格或认证证明；账号导出包含原值，账号 purge
  worker 显式删除 profile，公共内容不因此改写。
- privacy policy：仅 owner 写，服务端读取；导出包含，purge worker 显式删除，不进入公开 DTO 或日志。
- `activity_visibility` 与 `mention_policy` 是非 PII 授权偏好，和其他 privacy policy 一起仅 owner 写、
  导出包含、账号 purge 时删除。公开 DTO 只输出 viewer-specific `canViewActivity/canMention`，不输出
  owner 原始 policy；普通日志、metrics 和通知 payload 不记录 policy 值。
- follow：关系列表按 owner policy 和 discoverability 输出；账号导出包含自己的 incoming/outgoing
  关系，账号删除时 cascade，计数是可重建 projection。
- 站内用户搜索文档是可全量重建的最小公开身份投影；owner 关闭 discoverability/only_me 后触发删除，
  stale hit 仍会在 PostgreSQL 回表时丢弃。账号删除编排必须清除索引和相关 cache。
- mute/block：仅发起者的安全设置与服务端 policy 可读；不得在通知、分析或公开 profile 暗示具体
  名单。账号导出可包含自己创建的关系，删除时 cascade。
- OSS asset：profile 只持有引用；blocked/pending asset 不派生 URL。Profile/推广解绑进入 30 天 grace；
  Forum draft、已发布 usage 和非版本化 binding 都是 Media GC 的 live-reference 事实。通用 GC 只选择
  `cleaned_at` 已满 30 天、所有 reference/grace 已结束且没有 active operational hold 的 clean object；
  pending 永不因年龄自动删除。该 worker 与账号 purge system enqueue 默认 rollout-gated，代码合并/
  部署不等于环境已启用。
- Profile upload usage 只表达 owner 选择的头像/封面槽位，用于刷新后恢复审核状态；owner status API 不
  返回 object key、hash、account id 或持久 URL，账号 purge 时与 upload/intent 一起进入 media 清理编排。
- Forum upload usage 只表达 thread/comment intended surface；draft export 应包含本人 source 与 upload id，
  公共 export 只包含 canonical `yourtj-asset` reference 和允许公开的派生 attachment metadata。软删除将
  active usage detach 并设置 30 天 GC grace，保留 revision/恢复所需事实；restore 重新验证 clean。Draft
  save 同步 exact reference，发布再建立 clean usage。实际 object purge 仍须无 active reference、无 active
  operational hold、approval age/grace 已满且由已完成 rollout 的可审计 GC worker 执行。
- Revision attachment projection 按单页 content version batch 读取，只披露仍为 clean 且当时有效的
  binding；投影与历史 AST 不一致时返回空 attachment，而不是回退到 vendor URL。Cursor/limit 上限同时
  约束历史正文与媒体 metadata 的单次披露量。
- Moderation preview grant 只保存 token SHA-256、upload/moderator、reason、60 秒 expiry 与消费时间；独立
  housekeeping 在 expiry 超过 1 天后清理 grant row。长期治理证据保留独立 audit（不含 token、URL/key），
  不把短期 grant 当作永久访问日志。
- 未 callback 的过期 upload intent 由独立 housekeeping 按服务端签发的 exact key 排队删除；账号 purge
  撤销的未 callback intent 也只能删除 exact key，不能扫描 owner prefix。已消费 intent credential 满
  30 天删除。Provider 删除成功后 upload row redacts key、URL、hash、size、MIME、usage 与 dimensions，
  只保留稳定 id、redaction 状态和 purpose-limited audit 引用。
- PostgreSQL 在 account lock 下限制 10 active intent、rolling 24 小时 100 次 credential attempt、stored +
  reserved 512 MiB、live object + active intent 500 条、retained record + active intent 2,000 条；attempt fact
  只保存 account、reserved bytes、time，48 小时后由独立 housekeeping 删除。Detached binding fact 在
  30 天 grace 结束后由同一 housekeeping 删除。

Profile 字段与社交关系不进入普通请求日志、metrics label 或 governance audit body。未来推荐/广告若要
使用关系数据，必须另行说明目的、opt-out、保留和公平性，不能因字段已存在而默认获权。

## 账号删除编排

当前数据库编排：

1. **Deactivate**：停止公开展示和新互动，允许恢复，保留登录恢复所需最小信息。
2. **Delete requested**：记录请求与恢复 deadline，撤销 sessions、停止通知和新关系。
3. **Deleted**：durable worker 标记账号不可用，但 30 天 deadline 前仍可恢复。
4. **Purge started**：worker claim 在同一 transaction 锁账号、重验恢复 deadline、写不可逆
   `purge_started_at`，然后才允许调用 owner-domain API；任何一步失败都保留 durable job 并退避重试，
   但不得再恢复账号。
5. **Purged**：owner-domain mutable private projection 均幂等清理后，再清除 Identity PII/security
   credential 并写随机 tombstone。
6. **Tombstoned**：保留无法合法改写的最小 ledger/audit/foreign-key identity，不可反查原邮箱。

当前 worker 覆盖 identity、forum/DM private projection、reviews reactions/idempotency、media unfinished
authority、activity、platform receipt/verification evidence 与 user search reconciliation；credit ledger、公共
content、governance/audit 按保留政策不改写。Lifecycle job 有 queued/running/succeeded/failed、claim-unique
UUID lease token、lease expiry、attempt/backoff 与 bounded error code；所有终态/等待写入都按 token CAS，
并在账号 mutation 前按 job→account 顺序锁定。HTTP 返回 lifecycle state、deadline 与 recovery credential，不声称
同步完成 purge；`/admin/account-lifecycle/jobs` 提供 capability-gated dead-letter 观察面，requeue 还要求
recent-auth、理由和 append-only audit。该修复只恢复 worker 执行，不恢复账号或被清理的数据。
恢复后再次请求删除会在账号事务内把上一轮 succeeded job 重置为全新的 queued 周期；若旧 job 不是可复用
终态，整次请求冲突回滚，不能先关闭账号却没有后续 worker。

OSS object GC/账号 purge coordination 的代码已存在但默认关闭，只有完成 DB、published Markdown 与 OSS
三项 reconciliation 后才可逐环境启用；CDN/cache、日志、backup expiry、通用 legal hold 和逐域报告仍未
闭环。这些缺口必须在 UI/政策中如实披露，不能把 tombstone 解释为所有副本即时物理删除。
普通通知 outbox 以 recipient account 外键级联删除，actor 删除只置空 actor id；不含账号 id/payload 的
delivery receipt 最长继续保留到 90 天幂等窗口后清理。已投递通知按账号通知导出/删除政策处理。

Media account-purge 子步骤先 detach profile、删除 owner draft reference 并撤销 intent；无共享 Forum/
推广引用、无 future grace 的 owner object 都先 quarantine 并 durable enqueue，即使存在 active operational
hold。Hold 只暂停 provider worker，不保留公开状态；共享引用/future grace 仍不排队。子步骤只有在没有
更多 eligible work、没有 queued/leased/dead-letter、没有缺失 job 时才 terminal。Held object 有 durable
job 可作为 `retainedAssets` 随账号 tombstone 暂留；held quarantined object 缺 job 必须阻断 terminal。
System enqueue 与通用 GC 使用同一默认关闭 flag，关闭时仍有 eligible work 必须保持 job 非终态。
正常 pending/分批工作返回 queued 且不消耗 20 次失败预算；dead-letter 或 quarantined object 缺 job 会把
lifecycle job 置为 exhausted failed，只有修复 Media 队列后再经 recent-auth、理由化审计 requeue 才能继续。

## 数据导出

- 用户可导出自己的 identity/profile/privacy/onboarding/lifecycle/session metadata、作者内容、draft、关系、
  偏好、通知、本人发出的 DM、课评、activity、公告 receipt、成就、认证安全投影、media metadata 和积分记录。
- 内部 outbox source key、lease/error 和 receipt 不作为用户内容导出；用户导出的是已投递通知事实。
- 治理通知与本人申诉的提交/状态/公开理由可进入本人导出；reviewer identity、举报人、内部 metadata 和
  evidence 不默认导出，额外披露需目的限定政策与访问审计。
- 普通管理员可在安全设置中查看本人当前 effective capability、target ceiling 和 expiry，但不把 grantor、内部 reason、
  其他 staff assignment 或 append-only 授权历史加入普通 owner export。账号删除会立即撤销有效 assignment；
  历史以伪名 account id 与 audit 目的保留，具体期限随 governance audit retention 决策。
- 账号导出应包含本人认证的类型、当前状态、签发/到期/撤销时间；staff reason、issuer 与 evidence
  reference 属于治理记录，不默认进入用户导出，具体申诉披露按政策处理。
- 导出生成与 ready artifact 的 download-grant 签发都需要 recent-auth；创建还要求 8–128 位 printable
  idempotency key。Job 最多保留 24 小时；artifact 只能以 5 分钟、account-bound、一次性
  `X-Export-Token` header 下载，成功消费记录下载时间。
- Owner 可列出最近 20 个 job 并在刷新/换页后恢复状态查看；创建和轮询不要求请求长时间挂起。
- 不包含他人私密资料、内部风险分、举报人身份或治理证据；共享对话要最小化第三方信息。
- Inbound DM body、reporter/reviewer/staff identity、内部 evidence/metadata、verification reason/reference、
  OSS object key/hash/provider credential 和 credit signature/metadata 明确排除。
- 导出格式为 versioned machine-readable JSON，带生成时间与 included sections。Artifact 是短期 PII 副本，
  worker 到期将 status 改为 expired 并清空 JSON；purge 也会删除该账号所有 export job/grant。

## 保留与 legal hold

除 recovery/export/media 的当前实现窗口外，平台整体保留表仍为 `Decision needed`；不能把这些硬编码窗口
扩写为跨域政策。需要分别定义：

- expired email codes、revoked sessions、security logs；
- soft-deleted public content 和 revision；
- unreported DM、reported evidence、private attachments；
- DM request pair/cooldown metadata、request idempotency、其他 job records；通知 outbox 的成功/取消记录
  固定 30 天、dead-letter 和 delivery receipt 固定 90 天，purge worker 幂等执行；
- sanctions、appeals、audit 与 access logs；
- account-private governance notices 与 appeal idempotency records；notice 清理不能先于其申诉窗口，
  appeal/audit 清理不能破坏仍有效的 legal hold 或原决定可解释性；
- verification grant history、签发/撤销 reason 与证据对象/reference；
- search query logs、promotion aggregates、activity fine-grained events；
- backups、OSS versions 和 CDN cache。

已确定的实现期限：删除恢复窗 30 天；recovery credential 15 分钟；owner export artifact 24 小时；
download grant 5 分钟且一次性；worker lease 10 分钟。它们不是治理证据、backup 或 public-content 的
通用保留政策。

Media 当前代码中的 bounded 窗口与启用状态如下：

- clean orphan 从 approval `cleaned_at` 满 30 天后才可能进入通用 GC；profile/promotion/Forum detach
  grace 为 30 天。Pending 不使用年龄规则，只能由明确 moderation、account purge 或未 callback intent 的
  exact-key cleanup 处理。
- 已消费 upload-intent credential 满 30 天由独立 housekeeping 删除；该最小 credential cleanup 已启用，
  不受 operations-history flag 控制。
- Preview grant 在 expiry+1 天、detached binding 在 grace 结束、credential attempt 在 48 小时后由独立
  housekeeping 有界删除；这些清理不受 general-GC 或 operations-history flag 控制。
- Synthetic no-callback cleanup 成功后的 redacted tombstone 至少保留 30 天。任意 hold history 或 operator
  retry history 会把 tombstone 延长到相应 365 天 history 已清；succeeded deletion job 可以先删或随
  tombstone cascade，no-store inventory 不会因此留下孤立可枚举记录。
- Operational hold/release row、system retry event、succeeded deletion job 和已 redacted object 的 moderation
  evidence 已实现 365 天有界 purge，但 `MEDIA_OPERATIONS_HISTORY_PURGE_ENABLED=false` 为默认。Privacy/
  legal owner 批准 purpose/期限前不得启用或宣称 staff id 已清；append-only governance actor audit 不在
  该 purge 范围，其保留期仍为 `Decision needed`。

Media hold 仅支持 `moderation/security` operational purpose、单 asset、5 分钟至 365 天 expiry、CAS
续期/解除和 recent-auth operations inventory。它不会恢复公开访问，也不能在 provider deletion lease
开始后追认，但它不是 legal hold。真正 legal hold 仍需定义合法授权者、case scope、跨域冻结、通知、
release 和 audit retention；其他 domain（包括 DM）的 Planned legal-hold worker 描述继续有效。任何 hold
都不得成为无限期保留的默认借口。

## 供应商与外部请求

- Cloudflare Email、Alibaba OSS/CDN、captcha、Meilisearch/Redis 运维都需要数据流和 secret 边界。
- 任意第三方头像/Markdown 图片会泄露访问者 IP，因此持久媒体只允许平台 asset。
- Onebox 只服务 allowlisted 公共 HTTPS 页面；逐跳重新解析且全部地址必须为公网，请求 pin 到已验证地址
  并禁用系统代理，TLS 继续校验原 host。fragment 被移除，含 query 的 URL 不进入持久 cache，metadata
  有界且不返回远程图片。受控 HTTPS 回归只连接 loopback fixture，不调用公网；测试证书信任和地址映射
  仅在 test build 存在。Migration 清除历史 query URL/remote-image cache；访问日志不记录 URL。
- Web renderer 只把 `yourtj-asset` 映射到同一响应中匹配的服务端派生 URL；remote/data destination 与
  DTO 中多余/损坏 binding 都 fail closed。管理审核 DTO 同样不披露 object key、hash 或持久 URL；待审
  证据只通过 capability-gated、60 秒一次性 token 的同源 bounded proxy 读取，读取 purpose/reason 以 upload id
  审计，token 仅存 hash 且不进入 URL、日志或 audit。普通管理员/moderator 仍要求独立审核且禁止自审；
  ADMIN 的本人媒体自审是唯一例外，仍要求 recent-auth、强制 reason、显式确认和 `selfReview` audit；
  approve 必须有可信预览证据，fail-closed 的 block 不要求预览。该例外已交付且不扩展到
  DM/申诉/角色/认证/积分证据。
- 推广保存平台 clean asset id 和站内目标路径，不保存远程图片 URL。曝光/点击只使用两小时有效的
  随机签名展示票据去重，票据不含账号、IP、设备或 audience 身份；原始 receipt 48 小时后由 worker
  删除；worker 启动时立即执行并按小时复查，点击归因到同一票据 impression 的 UTC day。长期只保留
  promotion × UTC day 的曝光/点击总数。该数据不能用于个人级 attribution、跨域画像或重建访问者身份。
- Captcha 只收到完成验证必要的信息，不发送邮箱、正文或私信；其 metadata 保留需进入隐私说明。
- PR preview 不注入生产邮件/OSS/PII 凭据，不使用生产数据快照。

## 日志、指标与分析

- 日志使用 opaque id 和结构化错误，不记录邮箱、code、token、raw body 或完整 DM。
- 搜索 query、关系和安全指标先聚合/去标识；明细访问 purpose-limited。
- 任何推荐或广告分析在上线前说明输入信号、保留、opt-out、公平与安全过滤。
- 指标的 cardinality 和 metadata 有界，避免通过 observability 复制业务数据库。

## Decision needed

- Public profile 的搜索引擎索引政策。
- 匿名化显示名、handle 释放/防冒用期，以及 30 天恢复窗是否需在法律/政策审查后调整。
- 各类治理证据、DM、query log、audit 与 backup 的具体保留期。
- Media operations 365 天 purge 是否批准，以及 governance actor audit 的独立期限；在决定和显式启用
  flag 前，不能声称 hold/retry/job/evidence 或 staff identity 已按期清除。
- 毕业账号的校园资格、恢复和邮箱换绑。
- 是否允许商业推广及其 consent/measurement 边界。

## 验收基线

- 新 PII schema/事件在 PR 中有 purpose、visibility、retention、export 和 delete 说明。
- 公共、本人、关系用户、staff、system 的可见性有矩阵化授权测试。
- Export/delete workflow recent-auth、幂等、可观察，跨域失败可重试且不会静默漏删。
- 搜索、cache、OSS/CDN 和 backup 的 deletion/expiry 有 reconciliation 或演练证据。启用 Media GC 前，
  DB preflight、全部 published Markdown/`asset_usages` exact reconciliation 与 DB/OSS object reconciliation
  三项都必须通过；单个 SQL 零异常不等于完整证据。
- Credit ledger 在删除后仍可验证，但 tombstone 不能反查邮箱或公开身份。
- PR preview、日志、audit 和 metrics 不包含生产 secret 或不必要 PII。
- Governance notice/User appeal DTO 只返回 owner 可见字段；普通 token、appeal token、staff capability
  和他人账号之间有矩阵化授权测试。普通通知 outbox/receipt retention 已固定；治理 notice/申诉的
  独立 retention 在其 worker 上线前仍明确标为待决。

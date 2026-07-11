# 隐私与数据生命周期

> 文档类型：安全与隐私规范
>
> 状态：Active
>
> 负责人：Privacy owner、Security owner、Domain maintainers
>
> 最近核验：2026-07-12，migrations `0034`、`0037`、`0038`、`0040`

本规范将数据最小化、可见性、导出、删除和保留作为产品前置条件。它不是法律意见；涉及 PIPL、
未成年人、广告或跨境处理的最终政策需要合格法律与隐私负责人确认。

## 当前状态

### Current

- 校园邮箱不出现在公开 profile、论坛或现有 staff directory DTO。
- Identity 支持 email-at-rest encryption/blind index 配置。
- 设备 session 只向账号本人展示 bounded user-agent label 和必要时间；新认证流程不持久化精确 IP。
- Staff 无通用 DM 浏览接口，只能访问 participant 报告的最小证据。
- Governance audit 和制裁保留 actor/reason 历史，credit ledger append-only。
- Profile 默认仅校园登录用户可见；followers/following 默认仅关注者可见，新 DM 默认只允许接收方
  已关注的人发起。匿名只有在 owner 显式选择 `public` 后才能访问资料。
- Follow、mute、block 是独立事实；mute/block 不向对方提供列表接口。Block 删除双方 follow，
  suspended/deleted 账号不进入公开资料与关系列表。
- Display name、bio、HTTPS website 与 privacy setting 可由 owner 替换；avatar/banner 只保存本人
  clean OSS asset id，公开 URL 是状态校验后的派生值。
- 人工认证默认私密；公开 profile 只返回 definition 允许且 grant 明确公开的有效 label/说明/有效期，
  不返回 issuer、reason、evidence reference 或内部 grant id。
- 账号搜索只索引已验证、discoverable 且非 `only_me` 账号的 id、handle 和可选 display name；响应
  回表重验 active/suspension、profile visibility、block/mute 与 clean avatar，不索引或返回邮箱、bio、
  relationship 私密名单和内部治理字段。

### Partial

- Activity/mention privacy 和 public profile 的外部搜索引擎索引政策尚未完成；精确 handle 直达仍由
  profile visibility 决定，不因站内 discoverability 关闭而伪装成账号不存在。
- `deleted` 数据库状态不等于跨域匿名化、purge 或备份过期。
- 无自助 export、deactivate、delete、recover 或 retention worker。
- OSS、搜索索引、缓存、日志、备份、审计与积分的删除语义没有完整编排。

## 数据分类

| 类别 | 示例 | 默认访问 | 处理原则 |
|---|---|---|---|
| 资格 PII | 校园邮箱、邮箱验证状态 | identity purpose only | 加密/盲索引、绝不公开、限制保留 |
| 安全凭据 | password hash、code hash、refresh hash、keys/tokens | security code only | 不记录明文、最短保留、可撤销 |
| 会话元数据 | bounded user-agent、创建/最近使用/到期时间 | 账号本人、安全代码 | 不收集精确 IP，随 session retention 删除 |
| 公开身份 | handle、公开头像、display name、bio | 按 profile visibility | 用户可控、handle history 防冒用 |
| 公共内容 | thread、comment、review、reaction | 按 board/content policy | revision、治理、导出/删除规则 |
| 社交关系 | follow、block、mute、subscription | 本人及 policy 允许对象 | block/mute 默认私密、最小暴露 |
| 私密通信 | DM body、private attachment | participants | staff 仅举报证据、独立 retention |
| 治理证据 | reports、sanctions、appeals、audit | capability + purpose | 防篡改、访问审计、期限/hold |
| 认证凭证 | type/grant、签发/撤销原因、opaque evidence reference | `verifications.manage`；允许时为最小公开投影 | 默认私密、可到期/撤销、公开不含证据/操作者 |
| 运营数据 | job log、metrics、aggregated promo events | operators | 聚合、去标识、有限保留 |
| 公告 receipt | announcement/revision、seen/dismiss/ack time | 本人、汇总后的公告管理员 | 账号删除级联清除，不记录设备/IP；后台只返回聚合计数 |
| 积分记录 | ledger、wallet projection | owner/verification policy | ledger 不改写，删除后 tombstone |
| 积分完整性证据 | run reason、operator id、wallet account id、派生/缓存差额 | `credit.integrity` staff | 随 ledger/audit 完整性证据保留；不含邮箱、签名、key 或原始错误 |
| 交易履约信息 | escrow product `deliveryInfo` | purchase buyer/seller | 不进入公开 listing/search/log；随订单保留与删除策略最小化 |

新 column/event/index 前必须在对应产品文档说明 data category、purpose、controller/processor、
可见者、retention、export 和 deletion。没有答案时不得先“留着以后分析”。

## 可见性与默认值

- Board 声明 `public/campus/staff` 等访问级别；公共讨论不由作者 follow 关系临时改变。
- Profile、follower/following、new-DM 和 discoverability 已使用独立设置；activity/mention 仍待实现。
- Block/mute 是关系 policy，不通过前端隐藏代替服务端授权。
- 人工认证只有在 definition 允许公开、grant 明确 `displayOnProfile`、未撤销且未到期时可见；过期和
  撤销实时从公开投影消失，不依赖前端缓存隐藏。
- 搜索、feed、cache、CDN 与 notification 在输出时应用同一可见性规则。
- 第一阶段默认：profile=`campus`、followers/following=`followers`、DM=`following`、discoverable=on；
  owner 可将 profile/list 改为 public/campus/only_me 等 OpenAPI 声明的值。

## Profile 与社交关系数据生命周期

`identity.profiles` 的目的仅为用户选择公开展示的校园社区身份；`identity.profile_privacy` 仅用于
服务端授权。`forum.user_follows` 用于用户关系和准确计数，`forum.user_mutes` 与保留旧物理名但
语义已固定为 block 的 `forum.user_ignores` 用于本人过滤与安全边界。可见者和生命周期如下：

- display name/bio/website/asset id：owner 可编辑，viewer 仅在 profile policy 允许时读取；账号导出
  应包含原值，账号 purge 时随 account cascade 删除，公共内容不因此改写。
- privacy policy：仅 owner 写，服务端读取；导出应包含，删除时 cascade，不进入公开 DTO 或日志。
- follow：关系列表按 owner policy 和 discoverability 输出；账号导出包含自己的 incoming/outgoing
  关系，账号删除时 cascade，计数是可重建 projection。
- 站内用户搜索文档是可全量重建的最小公开身份投影；owner 关闭 discoverability/only_me 后触发删除，
  stale hit 仍会在 PostgreSQL 回表时丢弃。账号删除编排必须清除索引和相关 cache。
- mute/block：仅发起者的安全设置与服务端 policy 可读；不得在通知、分析或公开 profile 暗示具体
  名单。账号导出可包含自己创建的关系，删除时 cascade。
- OSS asset：profile 只持有引用；blocked/pending asset 不派生 URL。解绑后的 object retention、
  scanner、orphan GC 和 legal hold 仍按 OSS runbook 的后续阶段执行。
- Profile upload usage 只表达 owner 选择的头像/封面槽位，用于刷新后恢复审核状态；owner status API 不
  返回 object key、hash、account id 或持久 URL，账号 purge 时与 upload/intent 一起进入 media 清理编排。

Profile 字段与社交关系不进入普通请求日志、metrics label 或 governance audit body。未来推荐/广告若要
使用关系数据，必须另行说明目的、opt-out、保留和公平性，不能因字段已存在而默认获权。

## 账号删除编排

目标流程：

1. **Deactivate**：停止公开展示和新互动，允许恢复，保留登录恢复所需最小信息。
2. **Delete requested**：记录请求与恢复 deadline，撤销 sessions、停止通知和新关系。
3. **Deleted**：对 public profile/content 应用政策化匿名化，启动跨域 cleanup。
4. **Purged**：恢复窗结束后删除可变 PII、未保留私密数据和无引用 media。
5. **Tombstoned**：保留无法合法改写的最小 ledger/audit/foreign-key identity，不可反查原邮箱。

编排覆盖 identity、forum、reviews、DM、media、activity、platform verification、search、cache、notifications、audit、credit
和 backups。每个 step 幂等、有状态、有重试和人工恢复；删除 API 返回 job/status 而非假装立即完成。

## 数据导出

- 用户可导出自己的 profile、内容、关系、偏好、通知、允许的 DM 和积分记录。
- 账号导出应包含本人认证的类型、当前状态、签发/到期/撤销时间；staff reason、issuer 与 evidence
  reference 属于治理记录，不默认进入用户导出，具体申诉披露按政策处理。
- 导出生成需要 recent-auth、短期下载 URL、过期和下载审计。
- 不包含他人私密资料、内部风险分、举报人身份或治理证据；共享对话要最小化第三方信息。
- 导出格式 machine-readable 并带生成时间、范围和字段说明。

## 保留与 legal hold

具体天数仍为 `Decision needed`，但必须分别定义：

- expired email codes、revoked sessions、security logs；
- soft-deleted public content 和 revision；
- unreported DM、reported evidence、private attachments；
- idempotency/outbox/job records；
- sanctions、appeals、audit 与 access logs；
- verification grant history、签发/撤销 reason 与证据对象/reference；
- search query logs、promotion aggregates、activity fine-grained events；
- backups、OSS versions 和 CDN cache。

Legal hold 有合法目的、授权者、范围、到期和审计，不得成为无限期保留的默认借口。

## 供应商与外部请求

- Cloudflare Email、Alibaba OSS/CDN、captcha、Meilisearch/Redis 运维都需要数据流和 secret 边界。
- 任意第三方头像/Markdown 图片会泄露访问者 IP，因此持久媒体只允许平台 asset。
- 推广保存平台 clean asset id 和站内目标路径，不保存远程图片 URL；当前没有个人级 impression/click
  tracking，未来效果指标只允许必要的按日聚合。
- Captcha 只收到完成验证必要的信息，不发送邮箱、正文或私信；其 metadata 保留需进入隐私说明。
- PR preview 不注入生产邮件/OSS/PII 凭据，不使用生产数据快照。

## 日志、指标与分析

- 日志使用 opaque id 和结构化错误，不记录邮箱、code、token、raw body 或完整 DM。
- 搜索 query、关系和安全指标先聚合/去标识；明细访问 purpose-limited。
- 任何推荐或广告分析在上线前说明输入信号、保留、opt-out、公平与安全过滤。
- 指标的 cardinality 和 metadata 有界，避免通过 observability 复制业务数据库。

## Decision needed

- Public profile 的搜索引擎索引政策。
- 删除恢复窗、匿名化显示名、handle 释放期。
- 各类治理证据、DM、query log、audit 与 backup 的具体保留期。
- 毕业账号的校园资格、恢复和邮箱换绑。
- 是否允许商业推广及其 consent/measurement 边界。

## 验收基线

- 新 PII schema/事件在 PR 中有 purpose、visibility、retention、export 和 delete 说明。
- 公共、本人、关系用户、staff、system 的可见性有矩阵化授权测试。
- Export/delete workflow recent-auth、幂等、可观察，跨域失败可重试且不会静默漏删。
- 搜索、cache、OSS/CDN 和 backup 的 deletion/expiry 有 reconciliation 或演练证据。
- Credit ledger 在删除后仍可验证，但 tombstone 不能反查邮箱或公开身份。
- PR preview、日志、audit 和 metrics 不包含生产 secret 或不必要 PII。

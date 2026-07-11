# 隐私与数据生命周期

> 文档类型：安全与隐私规范
>
> 状态：Active
>
> 负责人：Privacy owner、Security owner、Domain maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

本规范将数据最小化、可见性、导出、删除和保留作为产品前置条件。它不是法律意见；涉及 PIPL、
未成年人、广告或跨境处理的最终政策需要合格法律与隐私负责人确认。

## 当前状态

### Current

- 校园邮箱不出现在公开 profile、论坛或现有 staff directory DTO。
- Identity 支持 email-at-rest encryption/blind index 配置。
- Staff 无通用 DM 浏览接口，只能访问 participant 报告的最小证据。
- Governance audit 和制裁保留 actor/reason 历史，credit ledger append-only。

### Partial

- profile、内容和列表对匿名访客的默认公开范围尚未形成统一政策。
- 无 profile/activity/follow-list/DM/discoverability privacy settings。
- `deleted` 数据库状态不等于跨域匿名化、purge 或备份过期。
- 无自助 export、deactivate、delete、recover 或 retention worker。
- OSS、搜索索引、缓存、日志、备份、审计与积分的删除语义没有完整编排。

## 数据分类

| 类别 | 示例 | 默认访问 | 处理原则 |
|---|---|---|---|
| 资格 PII | 校园邮箱、邮箱验证状态 | identity purpose only | 加密/盲索引、绝不公开、限制保留 |
| 安全凭据 | password hash、code hash、refresh hash、keys/tokens | security code only | 不记录明文、最短保留、可撤销 |
| 公开身份 | handle、公开头像、display name、bio | 按 profile visibility | 用户可控、handle history 防冒用 |
| 公共内容 | thread、comment、review、reaction | 按 board/content policy | revision、治理、导出/删除规则 |
| 社交关系 | follow、block、mute、subscription | 本人及 policy 允许对象 | block/mute 默认私密、最小暴露 |
| 私密通信 | DM body、private attachment | participants | staff 仅举报证据、独立 retention |
| 治理证据 | reports、sanctions、appeals、audit | capability + purpose | 防篡改、访问审计、期限/hold |
| 运营数据 | job log、metrics、aggregated promo events | operators | 聚合、去标识、有限保留 |
| 积分记录 | ledger、wallet projection | owner/verification policy | ledger 不改写，删除后 tombstone |

新 column/event/index 前必须在对应产品文档说明 data category、purpose、controller/processor、
可见者、retention、export 和 deletion。没有答案时不得先“留着以后分析”。

## 可见性与默认值

- Board 声明 `public/campus/staff` 等访问级别；公共讨论不由作者 follow 关系临时改变。
- Profile、activity、follower/following、DM、mention 和 discoverability 使用独立设置。
- Block/mute 是关系 policy，不通过前端隐藏代替服务端授权。
- 搜索、feed、cache、CDN 与 notification 在输出时应用同一可见性规则。
- 匿名与 campus 默认范围是 `Decision needed`；确认前不得宣称“校园内可见”或“全网公开”。

## 账号删除编排

目标流程：

1. **Deactivate**：停止公开展示和新互动，允许恢复，保留登录恢复所需最小信息。
2. **Delete requested**：记录请求与恢复 deadline，撤销 sessions、停止通知和新关系。
3. **Deleted**：对 public profile/content 应用政策化匿名化，启动跨域 cleanup。
4. **Purged**：恢复窗结束后删除可变 PII、未保留私密数据和无引用 media。
5. **Tombstoned**：保留无法合法改写的最小 ledger/audit/foreign-key identity，不可反查原邮箱。

编排覆盖 identity、forum、reviews、DM、media、activity、search、cache、notifications、audit、credit
和 backups。每个 step 幂等、有状态、有重试和人工恢复；删除 API 返回 job/status 而非假装立即完成。

## 数据导出

- 用户可导出自己的 profile、内容、关系、偏好、通知、允许的 DM 和积分记录。
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
- search query logs、promotion aggregates、activity fine-grained events；
- backups、OSS versions 和 CDN cache。

Legal hold 有合法目的、授权者、范围、到期和审计，不得成为无限期保留的默认借口。

## 供应商与外部请求

- Cloudflare Email、Alibaba OSS/CDN、captcha、Meilisearch/Redis 运维都需要数据流和 secret 边界。
- 任意第三方头像/Markdown 图片会泄露访问者 IP，因此持久媒体只允许平台 asset。
- Captcha 只收到完成验证必要的信息，不发送邮箱、正文或私信；其 metadata 保留需进入隐私说明。
- PR preview 不注入生产邮件/OSS/PII 凭据，不使用生产数据快照。

## 日志、指标与分析

- 日志使用 opaque id 和结构化错误，不记录邮箱、code、token、raw body 或完整 DM。
- 搜索 query、关系和安全指标先聚合/去标识；明细访问 purpose-limited。
- 任何推荐或广告分析在上线前说明输入信号、保留、opt-out、公平与安全过滤。
- 指标的 cardinality 和 metadata 有界，避免通过 observability 复制业务数据库。

## Decision needed

- 匿名/public/campus 默认范围和搜索引擎索引政策。
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

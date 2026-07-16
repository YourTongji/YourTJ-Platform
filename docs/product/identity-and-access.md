# 身份、登录与账号生命周期

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Identity maintainers、Security owner、Product owner
>
> 最近核验：2026-07-17，Identity wallet key projection、Credit owner lifecycle 与 Web wallet recovery

身份域证明“谁在使用平台”和“是否具备校园资格”，但不把校园邮箱变成公开身份。本规范定义
登录、注册、密码、会话、onboarding、handle 与账号生命周期的目标语义。

## 当前状态

### Current

- 校园邮箱验证码按 `login`、`registration`、`password_reset` 绑定用途；只有 provider accepted
  且未使用的 code 可在锁行事务中原子消费，迁移前 code 全部失效。
- 后端已有密码登录、忘记密码、重置密码和修改密码；Argon2id 由进程级四槽 semaphore
  限制内存并发，容量耗尽稳定返回 503。未认证 credential 路径同时使用 opaque client-network/global
  Redis bucket；验证码设置/重置密码先做不消费的 proof preflight，再在最终凭据事务重验并一次性消费，
  随机 code 不会触发 Argon2。不存在账号、无密码、密码错误和普通登录不可用的账号状态使用相同
  登录响应，并且 missing/invalid hash 分支仍执行一次受同一 semaphore 约束的 dummy Argon2。
- 新 access JWT 绑定 server-side session 与账号 auth version；refresh rotation 只创建一个 successor，
  consumed token 重放会撤销整个 token family。
- Web 完整登录携带同源随机 installation UUID；Identity 只保存按 account 做域隔离的 SHA-256 摘要，
  不保存原值，也不把 User-Agent 当设备身份。同一 installation 再登录会撤销其旧 session 并签发新 session，
  其他 installation 不受影响；未携带该值的旧客户端保持兼容，账号同时有效 session 最多 30 个，超出时
  撤销最久未使用项。显式撤销/替换后的旧 refresh 只返回未认证，不误报为 consumed-token replay 并撤销新会话。
- 后端支持当前设备、其他设备和全部设备撤销，以及本人设备 session 列表。首次设密、密码
  change 和 reset 都在凭据事务中推进 credential/auth version，撤销所有旧 refresh family，
  建立唯一替代 session 并向客户端返回新 token pair；旧 access/refresh 不能再操作新 session。
- 首次设密要求当前 session 的 recent-auth；已有密码时条件更新拒绝覆盖。reset code 绑定签发时
  `credential_version`，密码变更会使旧 reset code 失效，因此并发 set/reset/change 不能以旧证明覆盖新凭据。
- 密码 set/change/reset 成功与管理员邀请会在同一业务事务写入 Identity 邮件 job；worker 使用
  lease、有界退避和 dead letter 投递安全通知。job 不保存邮箱、subject、正文、code 或 provider response。
- Identity 只持久化密码 set/change/reset 和 refresh replay 等高价值安全事实；不保存带邮箱/IP 的普通登录失败流。
- 账号状态、角色、禁言/封禁会参与受保护请求判断。
- recent-auth 只读取当前可撤销 session 的服务端时间和方法，10 分钟后过期；有密码账号
  可验证当前密码，所有当前 session 可请求 `recent_auth` purpose 的校园邮箱 code。
- Password recent-auth 同时绑定账号 `credential_version`；修改、设置或重置密码都会推进版本。已验证旧
  密码的并发请求不能在新密码提交后把当前 session 重新标为 fresh。
- recent-auth 邮件路径不接受客户端 email；code 仍保持 hash、到期、尝试上限、provider-accepted
  和锁行一次消费。成功消费与当前 session 标记在同一事务提交。
- 校园邮箱不进入公开 profile DTO；支持加密和 blind index 配置。
- Main staging/production 发布脚本强制使用独立 AEAD/blind-index key 和
  `EMAIL_ENCRYPTION_STRICT=true`，启动 backfill 后仍有明文邮箱即拒绝 readiness；仅合成数据的本地/PR
  环境可以不配置 provider key。
- 邮箱维度的 Redis 限流 subject 不使用邮箱明文：配置邮箱加密时复用 active blind index，未配置时使用
  JWT secret 派生的 domain-separated HMAC-SHA256；限流 key 和错误日志均不得包含邮箱。
- Identity 已持有 owner-editable profile text、受控 avatar/banner asset reference 和 profile/list/new-DM
  privacy policy；任意头像 URL 不再可写。
- 新账号有 resumable onboarding row。普通社区 API 在当前条款、handle、资料和基础 privacy 选择原子
  完成前 fail closed；`/me`、onboarding、设备/密码、导出和关闭账号等必要安全路径仍可访问。
- 自助 lifecycle 已实现 `active -> deactivated` 与 `active -> deletion_requested -> deleted -> purged`；
  停用/删除立即撤销全部 session，删除有固定 30 天恢复窗，purge 以 durable job 重试并写不可反查
  原邮箱的 tombstone。Purge worker 在任何 owner-domain 清理前先锁行重验 deadline，并在 claim transaction
  写入不可逆 `purge_started_at`；此后即使部分领域已清理而后续领域失败，账号也不能恢复为 active。
- 账号恢复只接受 password、`recovery` purpose 邮箱 code 或关闭响应中的 15 分钟 recovery credential。
  recovery credential 不创建 access/refresh/session；成功恢复后旧会话仍失效，用户必须正常登录。
- Owner data export 使用 durable 24 小时 job；创建 job 和签发 5 分钟一次性 download grant 都要求
  recent-auth，并记录下载时间。Artifact 由各领域的公开 owner projection 组成，不由 gateway 跨域读取
  私有表。
- Identity 拥有账号至多一把 active Ed25519 public key，并通过 typed owner API 让 Credit 组合到认证账号
  本人的 `/wallet`；首次登记前返回 null。该 projection 不进入公开 profile/account surface，也不提供
  rotation、恢复或换绑 authority。
- `/wallet/bind` 在登记事务内独占锁定 actor account，并在等待 lifecycle/sanction 写后重新验证
  active/no-effective-suspend。`/wallet/claim-challenge` 以每账号 10 次/10 分钟的 Redis bucket 限流，随后
  `FOR UPDATE` 锁 actor，并在同一事务用新 challenge 替换旧 challenge；数据库 unique index 兜底保证每账号
  最多一条。`/wallet/claim` 另有 account 10 次/10 分钟与 opaque network/global bucket，把排序后的 actor
  `FOR SHARE` eligibility barrier 作为事务第一组数据库锁。UUID、lowercase SHA-256 hash 和 64-byte Ed25519
  signature 在 claim 查询前按 canonical 格式拒绝；一条有效 actor challenge 第一次进入 proof 校验即永久
  消费，即使 link 缺失/已认领/无 key 或签名错误，外部都只返回同一 generic proof error，客户端失败后必须
  重新获取并签名。缺 link/key 仍执行 dummy Ed25519 verify，避免明显的 crypto/no-crypto timing 分支。
  Lifecycle 先取得账号锁时，这些路径不写 key、challenge、legacy link/review owner 或 claim mint；wallet
  writer 先取得锁时，lifecycle `FOR UPDATE` 等待其提交后再清理。成功 claim 会在赋予 review owner 的同时
  清空旧 `wallet_user_hash` 与 `edit_token`，不把迁移凭据继续当作已认领内容的第二套 authority。
- Web wallet private key 是按规范 API environment + account 隔离的 non-extractable WebCrypto key；每次
  签名前都与 `/wallet.activePublicKey` 精确匹配。旧 `localStorage` seed 只有在派生公钥匹配、且 IndexedDB
  durable commit 成功后才删除；环境/账号不匹配、record 损坏或存储不可用都 fail closed。
- Web 在任何钱包网络调用前以 IndexedDB 原子认领 environment+account scoped operation digest，只有同一
  claim id 能转为 submitted/committed；未知响应只接受 owner intent 的 lock-aware committed/expired outcome。
  记录不保存 raw request、access/idempotency token、signature 或 signing bytes。

### Partial

- Web 已拆分密码登录、验证码登录、注册、忘记/重置密码和账号恢复，明确发送用途、重发倒计时、
  密码可见性、中性找回提示和登录后安全返回路径；注册 UI 强制用户主动选择公开 handle。
- Web 已提供设备中心、单设备/其他设备/全部设备撤销和修改密码；验证码登录允许无密码账号首次
  设置密码，已有密码不会被验证码覆盖。
- Web 已提供 focused onboarding、明确条款勾选、profile/activity privacy 默认值，以及 recent-auth
  保护的数据导出、停用和删除确认。关闭响应中的 recovery credential 只放 sessionStorage。
- access 与 refresh token 都保存在 localStorage；富文本上线前必须重新评估 XSS 后果。
- non-extractable WebCrypto key 不能被标准 API 导出，但成功执行的同源 XSS 仍可调用它签名；账号 purge
  也不能远程擦除离线浏览器 profile。本机清钥、legacy mismatch、密钥丢失恢复和 unresolved pending 的
  最终 UX 仍为 `Partial`，不能把本机存储等同于服务端 owner lifecycle。
- Web 尚无完整的跨标签页 refresh single-flight 协调，但旧 refresh 失败不会清除另一标签页刚写入的新
  token；条款内容仍由应用常量版本驱动，尚无 policy publish/历史阅读界面，onboarding 也还没有兴趣
  板块、头像上传和通知偏好的可恢复分步体验。

## 登录与注册体验

Web 必须把以下旅程分开，不能让用户猜测同一个表单当前处于什么状态：

1. **密码登录**：校园邮箱 + 密码，支持显示密码、错误恢复和登录后返回原页面。
2. **验证码登录**：校园邮箱 + 单次 code，适合无密码或临时登录。
3. **注册**：校园邮箱验证后选择不泄露邮箱信息的 handle，确认规则/隐私版本，再创建账号。
4. **忘记密码**：始终返回中性结果；只有合法且已设置密码的账号在后台收到 reset code。
5. **重置密码**：reset-purpose code + 新密码；成功后用新 token pair 继续当前旅程，旧会话全部失效，
   安全通知由 durable job 投递。

登录页面至少有加载、发送失败、倒计时、可重发、code 过期、限流、账号被暂停和网络重试
状态。密码规则在输入时可理解，不能只在提交后返回模糊错误。

## 验证码状态机

每个验证码必须绑定：

- 规范化邮箱 blind index；
- purpose，例如 `login`、`registration`、`password_reset`；
- code hash、到期时间、尝试次数、发送请求 id；
- nullable `used_at`，以及必要的 provider acceptance 状态。
- `password_reset` 还绑定签发时的 account credential version；任何密码更新后均不再接受旧版本 code。

验证在单个数据库事务中锁定最新有效记录，检查 purpose/到期/尝试次数，constant-time 验证，
然后写入 `used_at`。成功、并发重放和跨 purpose 使用都只能有一个成功结果。provider 未接受邮件时，
code 不可使用。

## 密码与会话

- 密码登录、忘记密码和 reset 的无效证明对“账号是否存在/是否具备密码”提供中性外部响应。
  普通密码登录在密码正确但账号停用或被 suspend 时也保持同一 401 响应；需要申诉的账号使用独立的
  appeal-purpose password/email credential，不通过普通登录错误暴露账号状态。
  忘记密码即使 provider 失败也返回相同的 204；reset 对 missing/ineligible/expired/invalid code 返回
  相同 400。失败 code 保持不可用并由不含 PII 的运维指标告警。
- 修改密码当前要求当前密码；重置密码依赖 reset-purpose code。
- 已认证 password change、首次 set 和 reset 都替换当前 refresh，而不是让旧 refresh 继续有效；
  它们同时撤销其他设备和旧 access token。角色变化和 suspend 仍撤销全部设备且不签发替代 session。
- refresh token rotation 必须识别重放；设备中心显示有限的设备/时间信息，不暴露精确历史 IP。
- User-Agent 仅是本人设备中心的可读 label。Web installation UUID 是用户清除站点数据即可重置的同源
  随机值，不得用于跨账号画像、推荐、广告或第三方追踪；服务端只用账号隔离摘要完成 session replacement。
- 支持撤销单设备、撤销其他设备和全设备注销。
- Web refresh credential 的长期目标是 `HttpOnly + Secure + SameSite` cookie；在迁移决策完成前，
  任何富文本和第三方脚本变更都必须把 localStorage token 风险列入安全评审。
- 角色、永久制裁、PII reveal、账号删除等高风险操作需要最近认证或独立 challenge。
- Moderator/admin 目标要求 phishing-resistant MFA（优先 WebAuthn/passkey）与受审计 recovery；在
  交付前 staff 账号安全仍为 `Partial`，不能把普通 JWT recent-auth 当完整二次验证。

### Recent-auth 状态机

- 新登录 session 默认不 fresh；密码或 purpose-bound 邮箱 code 验证成功后写入
  `recent_authenticated_at/method`，客户端 JWT `iat` 不参与判定。
- refresh rotation 在同一 session family 中传递原时间和方法，不延长 10 分钟窗口。
- session/family 撤销、密码重置、角色变化或 suspend 撤销对应 session/access，recent-auth 随之
  立即失效。只更新当前活跃 session，不写账号级“已验证”开关。
- 兼容期不含 session id 的 legacy JWT 可继续普通读写，但 recent-auth status 返回
  `sessionBound=false`，所有受保护高风险 mutation fail closed，用户需重新登录。
- Web dialog 可恢复过期/428 失败并在验证成功后重试原操作；服务端仍是唯一安全边界。

## Onboarding

当前首次注册后进入 server-enforced、可恢复的 focused 页面，并原子完成：

1. 显式确认公开 handle，可选 display name 与 bio。
2. 分别选择 profile/activity visibility 和 discoverability。
3. 阅读当前社区规则、隐私说明与积分边界摘要，以明确 checkbox 接受服务端要求的精确版本。

除 `/me`、onboarding、设备/密码、导出、停用、删除和 logout 外，普通业务鉴权在 onboarding 未完成时
拒绝请求。Web route guard 只开放 onboarding 与“账号与安全”设置，不能把必要的安全/数据权利动作绑到
条款接受上。已存在账号由 migration 以 `legacy-v1` 完成状态回填；rolling window 中旧 writer 的 trigger
也先标记为 legacy complete，新 registration/invitation writer 再显式重置为 incomplete。

后续增强按可跳过、可恢复的步骤增加：

1. 上传平台 OSS 头像；允许跳过，不能要求填写外部 URL。
2. 选择兴趣板块和通知基础偏好。
3. 解释 follow 与板块 subscription 的差异并提供透明的建议关注。
4. 条款/隐私/规则从 versioned policy publish surface 读取完整文本，而不是长期依赖应用常量摘要。

Onboarding 进度不应阻止必要的账号安全操作，也不能把营销同意与服务必要条款捆绑。

## Handle 生命周期

- account id 永久稳定；handle 是可变公开标识。
- 新 handle 不得从邮箱前缀、学号或姓名自动派生。
- 修改需要冷却期、保留名称检查和审计；旧 handle 在保护期内跳转到当前资料页。
- 认证账号、管理员和高曝光账号改名时保留可理解的历史。
- 删除账号后旧 handle 的释放与防冒用期限由隐私和安全负责人共同确定。

## 账号生命周期

当前状态机：

```text
active -> deactivated -----------------------> active
   |
   +-> deletion_requested -> deleted -> purged/tombstone
              |                |
              +------ 30 天内 -+-----------> active

active + silence = 可认证和读取，但受保护社区写入被拒绝
active + suspend = 登录、refresh 和受保护请求被拒绝
```

- `deactivated` 是用户可恢复停用，不等同于处罚。
- `deletion_requested` 立即撤销所有 session、停止公开展示和新互动，同时排入 `mark_deleted` 与
  deadline-based `purge` durable job；`deleted` 仍处于同一 30 天恢复窗。
- `purged` 删除可变 Identity PII、password/session/recovery/export artifact、wallet claim challenge 与已归属
  legacy-wallet link，并兜底清空已归属 legacy review 的 `wallet_user_hash`/`edit_token`，清理各域 owner-private
  projection，但保留并撤销验证历史 ledger 所必需的 Ed25519
  public key，以及政策要求的公共内容、治理历史和不可改写积分账本。账号行只剩随机 tombstone handle/id
  与必要外键锚点，不能反查原邮箱。
- Credit owner cleanup 幂等清空该账号创建的 task contact、售卖 product delivery instructions 和未消费
  signing intent；不删除交易对手拥有的私密字段，也不改写 task/product/purchase 状态、已消费 intent、
  ledger 或 escrow 事实。删除前 owner export 覆盖本人创建/接受的 task、创建的 product 与本人参与的 purchase。
  已开始 escrow 在账号失去认证能力前的阻断/结清策略尚未决策；本轮只防止 inactive/suspended
  creator/seller 的新 accept/purchase，无法代替 `SYS-AUDIT-11` 的跨域 lifecycle settlement。
- Lifecycle/export worker 使用 `FOR UPDATE SKIP LOCKED`、lease expiry、attempt/backoff 和 bounded error
  code。恢复 transaction 锁定 purge job；job 为 running/failed 或账号已有 `purge_started_at` 时一律
  fail closed。达到 20 次上限的 lifecycle dead letter 可由具备 `operations.jobs` capability 且完成
  recent-auth 的管理员通过审计 requeue API 重置 worker attempts；该动作不清除 purge marker，也不重新
  开放账号恢复。
- 毕业或校园邮箱失效不应自动抹除社区历史；恢复政策仍为 `Decision needed`。

## 数据与接口所有权

Identity crate 拥有 accounts、email codes、password hash、sessions、安全事实、邮件投递 jobs、account keys、onboarding、recovery
credentials、lifecycle/export jobs、profile/privacy 和未来 handle history。Forum、Reviews、Governance、
Credit、Activity、Platform 与 Media 各自公开 owner export/purge API；gateway 只组合这些 typed API，
不跨域编写私有 SQL。Identity 的 account-key owner API 提供当前 active public key，以及 full-ledger
verification 所需的 retained historical keys；Credit 只通过注入的 resolver 读取，intent consume 在业务
transaction 的同一连接上复核 active key，不能跨 schema 选择、登记或撤销 key。
其他域通过 identity 的公开 API 获取最小身份/角色视图，不直接查询或返回校园邮箱。HTTP 结构以
`contract/openapi.yaml` 为准，本文不复制字段清单。

## Decision needed

- 密码登录是否只接受校园邮箱，还是也接受 handle；推荐只接受邮箱以降低改名和枚举复杂度。
- 新注册是否必须设置密码；推荐允许验证码账号稍后设置，但 onboarding 明确提示恢复能力。
- 毕业用户的资格、恢复和邮箱换绑政策。
- refresh cookie 的跨端迁移、CSRF 防护和旧 token 撤销计划。
- Staff WebAuthn/passkey、recovery code 和 break-glass 的注册、丢失与撤销政策。
- 从未认领的 legacy wallet link 与 review 迁移凭据何时停止提供 claim、何时删除；在产品给出迁移截止日和
  用户通知前保留其最小 claim authority，但不得公开、写日志或复用于画像。

## 验收基线

- 密码与验证码两种登录都能从 Web 完成，注册和找回路径互不混淆。
- 注册 UI 不从邮箱、学号或姓名自动生成公开 handle；用户必须显式选择满足规则的 handle。
- 后端持久化 exactly the selected handle；已占用时在消费 registration code 前返回冲突，不能自动追加
  随机后缀或静默替换公开身份。
- 设备中心区分当前设备，支持撤销单个其他设备、其他全部设备和包括当前设备在内的全部会话。
- 同一 Web installation 重复密码/验证码登录后只有最新 session 有效，旧 access/refresh 均失败且不会
  撤销新 session；不同 installation 可并存，legacy 客户端也不能让 active session 无界增长。
- code 跨 purpose、并发重放、过期、超尝试次数均失败且无状态竞争。
- 外部响应不能可靠区分不存在账号、无密码和错误密码。
- 密码重置、角色变化、session revoke 和 suspend 后对应旧 access/refresh token 都不可继续使用。
- 首次 set 和 reset 的锁行/条件更新只能有一个并发请求成功；较旧 credential version 的密码证明或 reset code
  不能覆盖已提交的新密码。业务状态、替代 session、security fact 和 email job 任一失败都整体回滚。
- 安全邮件 provider 失败会在不重放凭据 mutation 的情况下退避重试；邮箱解密/数据库短暂失败不得误判为永久无收件人。
- Owner export 只包含尚在保留期的 security `eventType/createdAt`，不包含 session id 和内部邮件 job。
- `/wallet.activePublicKey` 只向认证账号本人返回，公开身份 surface 不携带；Web 环境/账号错配、legacy
  seed 不匹配、non-extractable key durable-write 失败和签名前 public-key mismatch 都有 fail-closed 回归。
- 高风险 identity staff mutation 必须用当前 server-side session 的未过期 recent-auth；legacy JWT、
  过期时间、撤销 session、错误/跨 purpose/replay code 和并发验证都有负向覆盖。
- 公共 API、日志、通知和审计不泄露邮箱、code、password hash 或 refresh secret。
- handler→repo→PostgreSQL 集成测试覆盖上述正向与负向旅程。
- Onboarding 未完成时普通社区 API fail closed，但 owner 安全、导出、关闭和 logout 仍可用；条款版本
  mismatch 不能静默接受。
- Deactivate/delete 必须 recent-auth、幂等并立即撤销全部会话；recovery credential 不能访问普通 route
  或创建 session，purge 后 password/email/recovery 都不能恢复。
- Purge claim 必须在账号行锁内重验 recovery deadline 并原子写不可逆 marker 后才可调用 owner cleanup；
  partial cleanup + failed/dead-letter 状态不能恢复账号，exhausted job 对 operator 可见且 requeue 有
  capability、recent-auth、append-only audit 和重试到 tombstone 的数据库回归。每次 claim 使用唯一 UUID
  lease token；过期 worker 的 complete/fail/defer/block 全部 CAS 失败，不能覆盖接管者的 Media 阻断或终态。
- 删除恢复会把当轮 lifecycle job 收口为 succeeded；账号以后再次请求删除时，服务端必须原子重置同一组
  unique job 的状态、attempt、lease、error 和新 deadline，不能因 `ON CONFLICT` 把第二轮 worker 静默丢失。
- Owner export 跨域 projection 不泄露 inbound DM、举报人、reviewer、staff/evidence 或 provider secret；
  job 可从过期 worker lease 恢复，download grant account-bound、短期且只消费一次。
- Credit owner export 包含本人创建或接受的 task、本人创建的 product 和本人参与的 purchase 私密投影；
  task contact 仍属于 creator。重复 purge 后只清空被删除账号拥有的 contact/delivery 字段与未消费
  intent，交易对手字段、ledger 和 escrow 事实保持不变。

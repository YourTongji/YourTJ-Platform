# 契约、数据与派生投影

> 文档类型：架构规范
>
> 状态：Active
>
> 负责人：Platform maintainers、Domain maintainers
>
> 最近核验：2026-07-17，migrations `0067`–`0071`、Courses/Selection projection readiness、Credit wallet key projection 与 owner lifecycle

本规范说明产品规则如何落实为 HTTP 契约、migration、domain API、事务和可重建投影。它不复制
完整 OpenAPI 或 DDL。

## 从需求到实现的顺序

1. 在对应产品文档确定参与者、状态、权限、失败/恢复、隐私和验收。
2. HTTP surface 先改 `contract/openapi.yaml`，包括 request/response、错误、分页和安全要求。
3. Schema 只新增下一个编号 migration，说明现有数据、并发、回滚意图和部署兼容性。
4. 在 owner crate 实现 repository/service/handler；`api` 只 merge routes 和 wiring。
5. 重新生成 Web 类型，接入所有需要的客户端与管理 UI。
6. 从 focused tests 开始，再跑对应 CI-parity checks。
7. 同步 current-state、产品、安全或运维文档，并在 PR 写明影响。

## HTTP 契约

- `contract/openapi.yaml` 是 wire contract 唯一事实源；prose 解释业务语义但不复制完整 schema。
- 路由版本为 `/api/v2`；Axum 0.8 path param 使用 `{id}`。
- DTO 显式命名并使用 camelCase；timestamp 传 Unix seconds，日期类字段按产品规范。
- 错误统一 `{ "error": { "code", "message" } }`；客户端依赖稳定 code，而不是数据库消息。
- 列表使用有界 cursor/page envelope；不能返回无界数组。
- Viewer-specific state 必须显式建模，不能让 Web 根据计数或缺失字段猜测。
- API router 默认为动态 JSON 返回 `Cache-Control: private, no-store` 与 `Pragma: no-cache`；handler 可设置
  更严格或 purpose-specific header，但不得放宽包含身份状态、viewer-specific capability 或五分钟媒体
  bearer URL 的响应。公开静态 frontend/CDN asset 使用独立缓存策略，不继承 API 默认值。
- 兼容变更优先 additive；破坏性变化需要版本/双写/迁移计划，不能静默改变历史字段语义。
- OpenAPI 变化后从 `web/` 运行 `pnpm run generate:api` 并提交生成的 schema。

## Migration 与数据所有权

- migration append-only；已在任何环境运行的文件不得修改。
- 每张表有一个 owner domain；外域不通过手写 SQL 触及其私有表。
- DB row 与 API DTO 分离；PII/内部状态不能因 `Serialize` 直接暴露。
- 多行 invariant、money、状态转换、counter 和 audit 在事务中提交。
- SQL 全部使用 bound parameters；输入先做长度、范围、枚举和 ownership 校验。
- 新 nullable/默认值必须说明 backfill、读旧写新和最终收紧策略。
- 删除语义区分软删除、匿名化、retention purge 与法律保留。
- `governance` 拥有跨域 audit、appeal、append-only appeal transition 和 account-private governance
  notice；identity/forum/reviews 继续拥有原处置状态及 reversal 规则。API composition 只能调用这些
  owner public functions，不能把制裁/内容恢复 SQL 搬进 gateway。
- Account export/lifecycle composition 遵循同一边界：Identity 拥有 job、recovery credential 和最终
  tombstone；Forum、Reviews、Governance、Credit、Activity、Platform、Media 各自实现 typed
  `snapshot`/private purge API。Gateway 可以并行组合 owner API，但不得复制任何 owner-private SQL。
- Credit owner snapshot 包含本人的 balance/ledger、本人创建的 task/product 和本人作为买卖任一方的
  purchase，并可在保留期内返回这些 owner-visible 私密字段。不可逆 purge 只幂等清空被删除账号拥有的
  task contact、product delivery instructions 和未消费 signing intent；交易对手拥有的字段、purchase/task/
  product 状态、已消费 intent、append-only ledger 与验证历史所需公钥继续保留。Gateway 不自行决定
  Credit 字段的导出或清理口径。

Migration `0053` 的 rolling 顺序是：先在维护窗口执行 migration，再部署同时理解 onboarding/lifecycle
状态与第六类 `recovery` email purpose 的应用，最后开放 Web route。它有三项特殊兼容约束：

- `identity.account_status` 通过 rename/create/cast 替换以便同一 transaction 可写新值；这会取得表锁，
  上线前必须确认账号表规模与 lock timeout。旧应用仍能处理原 `active/suspended/deleted` 文本，但不得在
  migration 期间并发修改 enum/constraint。
- 既有账号回填 `legacy-v1` completed onboarding；AFTER INSERT trigger 也让 rolling window 的旧 writer
  先完成。新 registration/invitation writer 在自己的 transaction 中明确重置为 incomplete，避免旧 writer
  创建永久绕过 onboarding 的账号。
- Migration 前已有的 legacy `deleted` row 不会被立即 purge：统一回填 migration-time request/deleted
  timestamp、30 天 deadline 和 queued purge job，给旧账号一次同等安全恢复窗。Upgrade fixture 必须验证
  constraint、deadline 和 job 一起成立。
- 新表/列对旧读路径保持可忽略；回退时关闭 onboarding/lifecycle/export/recovery routes 并保留 schema、
  job 与 append-only event，不通过反向 enum cast、drop table 或改写历史伪造恢复。

Fresh database 必须只通过 sqlx migration ledger 建立。普通启动、CI 和运维不能同时用裸 psql
重复执行同一组文件；开发流程见[本地环境](../development/local-development.md)。

Migration `0067` 收紧 Identity 拥有的 wallet key invariant。变更前 Credit 按 `created_at DESC` 读取现行
active key，因此 backfill 保留最新 key，并用 `public_key DESC` 确定解决同 timestamp 的历史歧义，把其他
active row 标记 revoked；这些 row 不删除，完整 ledger verification 仍可使用历史公钥。随后建立每账号
最多一把 active key 的 partial unique index。部署前暂停 wallet bind/signing-intent/value-moving 写入，
排空旧进程，并从最后一次签发等待五分钟 TTL 加一分钟 buffer；migration 后旧 binary 虽会被 index拒绝
第二把 key，却仍能无 recent-auth 首绑，所以 backend 必须 maintenance/batch 切换到新版本。失败时保留
收紧后的 schema 并 forward-fix，不能删除 index、重新激活旧 key 或回滚到允许 bearer-only 换绑的 binary。
Identity 通过 owner-domain API 返回至多一把 active public key，并按 signer account 批量返回 ledger
验证所需的 retained historical keys；API wiring 以 resolver 注入 Credit，Credit 不直接查询
`identity.account_keys`。Intent consume 在业务 transaction 的同一连接上复核并锁定 active key；full-ledger
verify 通过同一 owner boundary 读取历史 keys，reconcile 则在其 repeatable-read snapshot 内完成该读取。认证后的 `/wallet` 把 active key 组合为
仅账号本人可见的 `activePublicKey`，只用于客户端核对本机密钥，不是私钥、公开 profile 字段、key rotation
proof 或换绑 authority。
Credit 的 authenticated owner write 还通过 Identity owner boundary 在同一 business transaction 的第一组
锁中取得账号 `FOR SHARE` lifecycle barrier，并重验 active/no-effective-suspend。涉及双方的新 tip/accept/
purchase 先读取不可变 owner id，再按 account id 排序一次锁定 actor/counterparty，之后才锁 Credit/content
entity 并重验；lifecycle/sanction 的账号 `FOR UPDATE` 因此既不会越过在途私密写入，也不会与 reciprocal
account→entity→account 顺序形成环。
Identity 自己的 wallet writer 也不依赖认证前的旧 snapshot：bind 在事务内以 actor account `FOR UPDATE`
重新验证 active/no-effective-suspend；claim-challenge 经每账号 Redis bucket 后，以 actor `FOR UPDATE`
eligibility barrier 在同一事务替换旧 challenge，数据库 unique index 兜底保证每账号最多一条；legacy claim
以 actor `FOR SHARE` barrier 作为第一组锁，且在任何 claim lookup 前验证 canonical v4 UUID、64-char lowercase
SHA-256 hash 与 canonical 64-byte Ed25519 signature。Account 10 次/10 分钟及 opaque network/global bucket
限制 proof 尝试；有效 actor challenge 第一次进入 proof 校验即提交 used 状态，link/proof 失败都返回同一
generic error，并用 dummy key 补齐缺 link/key 的 Ed25519 verify。成功才把 link/review/mint 写入同一事务，
并在赋予 review owner 时清空旧 wallet hash/edit token。最终 purge 删除该 actor 未完成的 challenge 与已归属 legacy link，兜底清理已归属
review 的迁移凭据，撤销但保留 ledger verification 所需 key row。
该 contract 按 backend-first rolling 顺序发布：先确保所有 backend response 始终包含 required-but-nullable
字段，再发布严格客户端。旧客户端会忽略新增字段；新 Web 对 omission 显式 fail closed，生成的 Flutter
decoder 也把 omission 当不完整响应，不能在混部窗口把“字段缺失”解释为“尚未绑定”。

Migration `0070` 收紧 legacy wallet claim 的隐私与存储边界。它先清空已有 `account_id` 的 review 上残留的
`wallet_user_hash`/`edit_token`，再用 validated check constraint 阻止任何 writer 把迁移凭据留在已认领
review；未认领 review 保持可迁移。Challenge 表先删除 expired/used row，再按 `created_at DESC, id DESC`
每账号只保留最新一条，最后建立 account unique index。运行时每次失败 proof 都会消费有效 challenge，客户端
不得重放旧签名，必须重新签发 challenge；多维 Redis bucket 在已配置环境故障时 fail closed。升级 fixture 必须覆盖 populated backfill、重复
challenge 决定性收敛和旧数据保留；fresh fixture 必须覆盖 constraint/index。发布时停止 challenge/claim
writer、先执行 migration 再切换新版 backend；混部旧 writer 可能被新约束安全拒绝，不能当作长期兼容。
回退只能 forward-fix，不得删除 constraint/index、恢复已清理凭据或重新开放同账号多 challenge。

### Selection teaching-class identity 与兼容窗口

Migration `0069` 将选课公开身份从课程代码收紧为具体教学班。`offeringId` 是
`selection.courses.id` 的字符串形式，直接继承上游 `teachingClassId`；它是课表、本机存储、详情和时段
路由唯一可用的身份。`id` 只作为同值 deprecated alias 保留，`teachingClassCode` 只是可空展示字段，
`code` 则是 catalogue course code，二者都不能替代 `offeringId`。

- `SelectionOffering` 必须携带 calendar、教师数组、起止周、`weeksUnknown`、`scheduleUnknown`、状态、
  可空 catalogue 关联和历史 `reviewCount/reviewAvg/reviewScope`。零样本的 average 必须为 null；teacher
  精确匹配与 course fallback 是不同的公开事实，客户端不得合并口径。当前上游没有可靠停开状态，所以
  `status=unknown` 是事实，不得猜成 active。
- `selection.courses.calendar_id` 是非空分区事实。升级只从相同 teaching-class id 的可信 raw row 回填；
  无法证明的历史 NULL 让 migration fail closed，不能用“当前学期”或最大 calendar 猜测。
- `TimeSlot.weekNumbers` 是冲突计算的规范周集合。解析失败用 `weeksUnknown=true` 和空数组表达；地点缺失
  独立使用 `locationUnknown=true`。同一完整排课文本被多个教师 raw row 重复时按排课事实折叠，只有恰好
  一位教师拥有该事实才填 `teacherName`。
- Migration `0071` 将同一 D1 快照中的旧 teachers/courses/course-aliases 保存到 Courses-owned raw tables，
  并以绑定同一 snapshot hash 的第二份 import audit 校验三表 source/target/live counts、非空门禁和下降审批。
  Catalogue/selection 全量物化先对 16 张 raw 表取共享锁，并分别验证 selection 与 legacy-course audit；任何
  不一致都发生在首个 projection 写入之前，不能由 admin job、手工 SQL 或“结果看起来合理”绕过。
  Catalogue 投影只 upsert，snapshot 缺失不能触发跨域读取 Reviews 私表或自动删除长期课程/alias。
- 历史公开 aggregate 只接纳能唯一映射到 catalogue owner 的 legacy course；它和物化后新增的 community
  count/points 分开记账，重复物化替换 legacy 部分而不重复累加 community 部分。无法唯一映射的旧样本留在
  raw 层，旧正文、作者和互动也不由 Courses 读取或迁移。
- `/selection/offerings` 使用有界、query-fingerprint 绑定的 opaque cursor；weekday、startSlot、endSlot
  必须同时出现且范围有序，`week` 与 `includeUnknownSchedule=false` 也只能随该完整三元组出现。时间筛选
  默认保留 `scheduleUnknown` 或 `weeksUnknown`；显式传 false 时，两类未知事实都必须在 PostgreSQL 与
  Meilisearch 路径排除，避免两条查询路径给出不同的“无冲突”结论。
- 旧的 by-major/by-nature/search 路由在至少一个完整 consumer 迁移窗口内保持 deprecated 且硬上限返回；
  `/selection/courses/{teachingClassId}` 及其 timeslots 子路由是按教学班 id 精确读取的兼容别名。按 course
  code 任取一个教学班的 selection 兼容语义已移除，因为同课号平行班没有安全代表。删除剩余兼容路由必须
  另立 contract change，并先证明 Web 与 Flutter 已不再消费。

## 状态机与幂等

- 业务状态使用受约束 enum/check 和显式转换，不以多个模糊 boolean 代替。
- 外部可重试写入使用 idempotency key 或稳定 source key；同 key 不同 payload 返回 conflict。
- “已经是目标状态”是幂等成功还是冲突，由产品规范明确，不交给每个 handler 自由决定。
- 反向动作追加 reversal/history，不覆盖需要审计的原事件。
- 并发转换使用 row/advisory lock、unique constraint 或 compare-and-set，不只依赖前端禁用按钮。
- Appeal 以 `(appellant_account_id, governance_event_id)` 唯一约束原处置，以 idempotency key + request
  hash 区分安全重试和冲突 payload；review/decision 使用 version CAS。历史表由数据库 trigger 禁止
  update/delete/truncate，终态 constraint 要求 reviewer/decision 字段与状态一致。管理队列把当前
  appellant role、self 与 original-actor recusal 条件放在 SQL cursor/limit 之前，避免后过滤空页；
  领取和决定仍在 transaction 内重新锁定并验证同一授权事实。

## Outbox 与后台任务

业务事务需要可靠触发搜索、通知、媒体或其他跨域副作用时，写入 transactional outbox。consumer：

- 通过 event id/source key 幂等；
- 对可撤销 source 锁定 exact generation（follow/vote timestamp、DM request cycle/message id）或当前
  effective subscription；仅凭候选 payload/target URL 不构成可投递事实；
- 有 queued/running/succeeded/dead/cancelled 状态、lease、重试上限和 dead-letter；
- 记录不含 PII/secret 的 bounded error；
- 支持 reconciliation 比较事实源和 projection；
- 失败不伪装成成功，也不让 API 请求无限等待外部供应商。

通知 side effect 已通过 migration `0054` 统一为 PostgreSQL outbox：稳定 source key、30 秒 lease、
`SKIP LOCKED` claim、最多 8 次有结果失败的有界指数退避（过期 final lease 仍可按同一 attempt 恢复）、
副作用前 row-lock lease fencing、delivery receipt 和理由化人工 dead-letter retry。普通成功/
取消事件保留 30 天，dead-letter 和 delivery receipt 保留 90 天。Redis/SSE 只承载 refresh hint。
Identity 的密码变更/重置与管理员邀请通知使用本域 durable email job，而不是复制通用通知 payload：
业务事务只写 account id + bounded template kind；发送时再从 Identity 取当前收件地址并渲染模板。验证码
邮件仍保持同步 provider-acceptance 语义，不能排队后先向客户端谎报验证码已发送。
Account lifecycle/export 是数据库持久 job，但 consumer 仍由 API 进程内 supervised loop 启动：claim 使用
`FOR UPDATE SKIP LOCKED`，running lease 10 分钟后可回收，失败有 attempt/backoff/bounded error。Lifecycle
每次 claim 生成唯一 UUID lease token；complete/fail/defer/block 均按 token CAS，任何 account mutation 前
先锁定并验证 job lease，统一 job→account 锁序。正常等待 Media provider/system enqueue 会退回 queued 且
撤销本次 attempt，dead-letter/missing job 则耗尽预算等待审计 requeue。它比裸
fire-and-forget 可恢复。Lifecycle purge claim 还必须在同一 transaction 锁账号、重验 recovery deadline
并写不可逆 `purge_started_at`，owner cleanup 不得早于该 commit；running/failed/marker 后 recovery fail
closed。耗尽 20 次的 lifecycle job 可从 capability-gated admin API 观察，并由 recent-auth + reason + audit
的 requeue 恢复 worker 执行，但不会清除 marker。当前仍没有独立 worker deployment、统一 operator UI、
SLO 或逐域 reconciliation，因此不能作为所有 future job 的最终模板。
Selection catalogue/materialize/search/cache 同步由 `selection.sync_jobs` 持久化：同一时刻只允许一个
queued/running job，operator request 以 idempotency hash 去重，claim 使用 lease token，四步进度逐步
fence，失败最多八次退避后进入 dead 状态；dead retry 的理由写 account audit。长耗时 SQL 与 Meilisearch
阶段运行期间每 60 秒续租；catalogue/selection 物化以及 Meilisearch settings/clear/add 的每个破坏性动作
前后都按 job id、lease token 和未过期时间重验，失租立即取消旧 pipeline，旧 worker 不得继续推进、完成或
覆盖接管者结果。Meilisearch 仍采用 clear 后等待成功、再 add 的非原子重建，二者之间存在可见空窗，不是
versioned index 的原子切换。worker 仍随 API 进程启动，所以独立 worker deployment 与跨实例 SLO 仍是后续
平台工作。其他搜索索引路径仍有裸 `tokio::spawn`，属于迁移目标，不是推荐的新模式。

## 搜索、缓存与计数

- PostgreSQL 是权限、内容和当前状态事实源；Meilisearch 文档可全部删除重建。
- 索引只包含搜索所需最小字段；返回前应用 status、visibility、privacy、block/mute policy。
- 联邦搜索由 `search` crate 编排 typed section；owner domain 从索引取得 ranked candidate id 后，
  用自己的 public API 批量回表并保持候选顺序。聚合层不读取外域表，也不序列化 Meilisearch hit。
- `/api/v2/search` 的 `type` 在后端决定实际查询域；course/review/thread/user/board/tag 每类独立
  有界，ID 必须可直接用于 canonical route，不带内部 index prefix。可选 bearer 用于应用校园资料与
  viewer relationship policy；匿名响应始终使用更窄的 public 可见范围。
- Search cursor 只用于非 `all` scope，绑定规范化 query/type 和可见结果 offset，最大窗口 240；服务端
  从 ranked candidates 起点重新回表后切片，因此隐藏/stale candidate 不会造成客户端越权或跳过可见
  结果。`all` 通过 `hasMoreScopes` 切到单类续页；`failedScopes` 仅表达局部失败，不携带内部错误。
- Meilisearch document primary key 只能使用其允许的字母数字、`-`、`_` 字符；当前内部前缀为
  `course-<id>` / `review-<id>` / `board-<id>` / `tag-<id>`；用户索引使用公开 account id。HTTP DTO
  始终去掉内部前缀。改变前缀必须配套 full reindex。
- Full reindex 等待 clear task 成功后再 add，并观察 add 结果。
- Courses/Selection 的 search projection 在 PostgreSQL 记录 source/indexed generation、row count 与
  `stale|rebuilding|ready`。数据物化先推进 source generation 并置 stale；全量 index 只有在 generation 与
  source row count 都未变化时才能 CAS 为 ready。应用启动和每分钟 reconciliation 还比较 Meilisearch live
  document count，能恢复全新/外部清空的索引；未 ready 的课程或选课关键词查询返回 503，不能返回误导性的
  空数组。该 readiness fence 不把 clear→add 变成原子切换。
- Hot/search counter 使用增量/投影，读路径避免全表聚合；定期 reconciliation 纠偏。
- `forum.comments.reply_count` 是仍可见直接子回复的写入维护投影：migration `0065` 先回填，再由 child
  insert/delete、parent 变化以及 hidden/deleted 状态变化触发校正。公开 Profile/read DTO 只读取该列；
  改变“回复数”口径必须新 migration 回填并同步写路径，不能在热点读路径扫描评论树。
- Forum activity projection 与 canonical thread/comment mutation 共用事务。父状态转换先锁 thread、
  再按 id 锁 comments，随后按 thread source、comment id、vote target/account 的固定顺序取得 activity
  source lock；整棵子树由 canonical 可见性重算激活状态，恢复沿用原 content/reaction timestamp，
  不在读路径聚合修补。
- 每日签到以 `(account, Asia/Shanghai date)` 为唯一事实；首次 insert、不可逆 contribution event、daily
  score delta 与 account score 在同一事务提交，重复请求返回既有状态且不重复计分。签到 source 不接受
  通用 deactivate/reversal。
- Activity policy 发布在 exclusive projection lock 下把全部 daily/account score 重投影到同一 policy
  version；普通贡献使用 shared lock，不能产生混合版本。Trust evaluator 的每日 run 持久化 cursor，
  每 50 个账号续租，账号 mutation 在同一事务校验 lease token；单账号失败进入 bounded failure inventory
  而不阻塞后续账号，run 以退避重试并在 8 次后 dead-letter。每次账号评估至多升级一级；治理降级使用
  独立 event id 幂等并设置冷却与 score floor。
- `forum.user_follows` 是关系事实；`forum.user_social_stats` 是 trigger 同事务维护的可重建计数投影。
  Follow 与 block 对同一账号对使用相同 transaction advisory lock，防止 block 与并发 follow 双写穿透。
- Following feed 不复用 board/thread subscription：Forum 先按 follow/content/block/mute 事实取得有界
  候选，再通过 Identity public account API 批量过滤 lifecycle/suspension 并取得 handle；cursor 使用
  `(created_at, thread_id)`，不得依赖已删除的 cursor row。
- 用户搜索索引由 Identity 维护，只存 id/handle/display name；Forum 通过 Identity public account API、
  自己的 relationship/count projection 和 Media public API 组装最终 hit。聚合 `search` crate 不跨域 SQL。
- Profile media/likes/bookmark projections 由 Forum 选择 canonical 可见 thread/comment，并通过 Identity
  public account batch 取得作者公开身份、通过 Media batch resolver 取得 current clean attachment；
  Forum 不跨 schema 读取 Identity/Media owner table。媒体候选经过 AST/reference exact-match 后才披露 URL，
  sparse cursor page 可以为空但仍携带 next cursor，客户端必须允许继续分页。
- Redis cache key 版本化或短 TTL，mutation 精确 bump 相关 version；缓存故障不改变业务写入事实。
- Platform 为每个实际返回的 promotion 签发 purpose/audience/issuer 绑定、两小时有效且含随机 UUID 的
  HS256 presentation token；签名使用与 access JWT 分域的 key material，claims 不含 viewer 标识。
  `promotion_event_receipts(token_id,event_type)` 只负责一次性幂等，点击事务先补齐同票据曝光再累计点击，
  保证日表 `clicks <= impressions`。Receipt 48 小时后删除，`promotion_daily_metrics` 保留无身份日聚合；
  管理查询最多生成 93 天零填充序列，列表只批量读取 30 天汇总，不做 N+1。
- Onebox cache key 使用规范化 query-free URL 与 policy version；ready row 最长 7 天，failure row 仅
  2 分钟。抓取策略收紧时 bump policy version，避免旧 ready cache 绕过新边界。每个 HTTPS redirect
  重新执行 allowlist、DNS 和“全部地址均为公网”检查，再把 reqwest 固定到所选地址；transport 禁用
  系统代理，TLS 仍用原 host 完成 SNI/证书校验。含 query 的外部 URL 不持久化，旧 query/remote-image
  cache 由 additive migration 清除；过期 cache 不是外部内容事实源。
- 不使用 `LIKE %q%` 作为热点中文聚合搜索降级，除非产品/性能测试定义了严格有界范围。

## 内容与媒体契约

- 内容携带 `contentFormat`；legacy `plain_v1` 不自动解释为 `markdown_v1`。
- Forum 主题/评论的 canonical row 与 revision 同时保存 source format；create/update 省略格式的 legacy
  请求按 `plain_v1`，不能根据正文猜测。格式和正文只能一起修改。
- Forum draft 使用 `thread`/`comment` discriminated payload 和稳定 account-owned key；服务端限制 key、
  target 与内容大小。`version=1` 起步，保存以 `expectedVersion` compare-and-swap，账号行锁把 50 条上限
  检查与创建串行化；409 由用户显式解决，不能用 last-write-wins 静默覆盖另一设备。
- Forum 已发布主题/评论的 canonical row 使用正整数 `contentVersion`。PATCH 以 `expectedVersion` 做
  compare-and-swap；锁定后的旧 source、revision insert、canonical update 与版本递增位于同一事务，
  stale write 返回 `409 VERSION_CONFLICT` 和当前版本。历史写入依靠数据库默认版本 1；migration 的
  source-column trigger 会为未显式写版本的旧 backend 单步递增，新 backend 显式 `+1` 时不重复递增，
  因而滚动窗口也不会绕过版本线。legacy 客户端省略 expectedVersion 只按 1 尝试，已修改内容会安全
  冲突而非静默覆盖。滚动发布先执行 additive migration，再部署读取新列的应用版本。
- Thread/comment revision list 只向作者本人或有 `moderation.content` 且严格高于另一作者角色的 staff
  开放，使用 1–100 的 cursor page，不返回无界数组。单页所有 historical content versions 以一次
  Media owner batch projection 解析 attachment；每项仍与对应 canonical AST exact-match，损坏投影
  fail closed，不能因分页引入 per-revision N+1 或泄漏 asset URL。
- Public list/detail DTO 的 `canEdit/canDelete/canModerate` 是 viewer-specific read model。Forum 结合
  canonical 状态、作者关系和 Identity 的 role-only batch projection 计算；Web 只能用它改善可用性，
  mutation handler 仍独立执行 owner/capability/role hierarchy 授权。
- 服务端通过 pulldown-cmark event stream 验证 canonical source，限制结构/链接，拒绝 raw HTML、
  非安全 URL 和非平台图片；mention/search/notification projection 从解析事件生成。客户端 preview
  不构成安全边界。
- Forum 图片只用标准 Markdown image node + `yourtj-asset:<正整数>` vendor destination。Thread 最多 8 张、
  comment 最多 4 张，alt 必填且同一 asset 不得重复。HTTP 的 ordered `attachmentAssetIds` 与 AST 引用
  必须完全相等；`plain_v1` 不解析该语法。vendor destination 永远不直接成为浏览器请求 URL。
- Media credential 只允许 account-bound exact object key；callback 原子消费 intent。Callback bearer 只在
  provider flow 短暂出现，PostgreSQL 只保存 SHA-256 digest 并 constant-time 验证，不保留 plaintext。
- Credential issuance 在 account lock 下由 PostgreSQL 执行 10 active intent、rolling 24h 100 attempt、
  stored + reserved 512 MiB、live + active intent 500、retained record + active intent 2,000 的 fail-closed
  limits；attempt fact 保留 48 小时。Redis/cache 不能放宽该事实源约束。
- Web 只把服务端返回的短期 STS 凭证交给官方 OSS Browser SDK，不自行扩展 prefix/object key；客户端
  SHA-256 作为 callback custom value，业务后续只保存 signed callback 返回的 upload id。
- 业务保存 asset/reference，不保存任意 URL；访问 URL 必须是授权派生值。Ingest 与 Delivery 是不同
  private bucket/principal；owner pending preview 和 staff preview 都经同源鉴权 `no-store` proxy，不返回
  vendor locator。Clean 只触发 processing，三个 sanitized WebP variant 完整发布后才能签发五分钟 CDN
  bearer URL。
- 通用 media URL route 只允许 asset owner；Forum、Profile、Promotion 等 owning domain 必须先完成内容/
  受众/viewer 授权，再调用 Media batch resolver 获取 typed projection（asset、variant、MIME、尺寸、URL、
  expiry）。Platform promotion 可因此向匿名合法受众返回图片，而不扩大 generic route。带 projection 的
  owning-domain 响应沿用 API `private, no-store`，客户端按 expiry 或首次加载错误重新取 owner resource。
- Avatar projection 固定使用 `thumb_256`，banner/content/promotion 默认使用 `display_1280`；owner-only
  compatibility route 只允许选择已有的 server-owned variant，不能接受任意尺寸或 object path。
- 当前 Forum attachment、Forum content author avatar 与 Promotion DTO 保留 expiry。Web 可在单一登录主体的
  进程内按 `assetId + variant` 有界复用同一 URL，但只复用到 expiry 前 30 秒；图片错误精确淘汰失败 URL，
  登录主体变化清空缓存。该 presentation cache 不持久化、不延长授权，也不替代 PostgreSQL/owning-domain
  授权。Public profile/account/relationship/search/DM avatar 兼容字段仍只返回 URL string，是需要 additive
  contract 或统一有界 refetch 收敛的 `Partial` 例外，不能被复制到新 surface，也不能用更长 TTL/public
  origin 掩盖。
- Reviews 的历史 `reviewer_avatar` 不是 asset reference，也没有 owner/moderation/publication 事实；任何
  HTTPS 值都可能成为第三方追踪请求。因此 `Review.authorAvatar` 当前 fail closed 为 null。后续只能通过
  Identity/Media owner API 增加 clean `thumb_256` projection，不能从 legacy column 或任意 URL 恢复回退。
- asset moderation/publication、binding、owner、target、alt、variants 与 retention 由 media/domain API 协作维护。
  Profile/推广使用 media-owned 单槽 `asset_bindings`；Forum draft 使用 `draft_asset_references`，已发布
  内容使用 version-aware `asset_usages`。业务事务先锁并重验 upload 再建立 live reference；启用后的
  GC 锁候选后重新查询引用/grace/operational hold，避免 snapshot race。
- Media owner API 接受 Forum 已锁定的 transaction，按固定顺序锁 upload/publication/active usage，重验
  owner、kind、intended usage 和 clean + published，再以新 `contentVersion` 原子切换 binding。Forum
  不读写 media 私有 SQL；stale content CAS 会整体回滚 canonical row、revision 和 usage。
- `media.asset_usages` 保留 `boundContentVersion` 和 content-edit detach version，使 revision 只取得当时
  生效且当前仍 clean 的最小投影。target soft delete 使用独立 `target_deleted` detach，30 天 grace 后
  才可成为 GC candidate；restore 重新解析 canonical source 并重验 clean，不复用旧授权。
- 公开 Forum DTO 仅由 Media owner 返回 asset id/reference、position、alt、variant 尺寸和到期派生 URL。Forum
  再与 canonical AST 做 exact match；corrupt/extra projection fail closed，不向客户端泄漏 URL。
- Profile upload 的 intended usage 是 media-owned 恢复提示，不是授权事实；owner status DTO 使用最小字段，
  业务绑定仍在同一事务重新验证 owner、kind、exact intended slot 与 clean + published 状态，避免 pending-to-public
  race。替换/清除写 detached binding 和 30 天 grace；账号 irreversible purge 可立即 detach profile slot，
  共享 Forum/推广引用或 future grace 不排队；active operational hold 不阻止 quarantine/durable enqueue，
  只暂停 provider worker。只有没有 eligible work、queued/leased/dead-letter 或缺失 deletion job 时 media
  purge 才报告 terminal；held object 有 job 时可暂留，held quarantined object 缺 job 必须阻断 terminal。
- Admin pending evidence 不进入 queue DTO。Media owner 以 database-backed one-time grant 绑定 upload、
  moderator、reason 和 60 秒期限；GET 在同一事务消费 token hash 并写治理 audit，随后由 provider abstraction
  以 callback MIME/bytes、20 MiB、20,000 px 单边和 40 MP hard limit 代理同源 stream；图片 header 在首个
  response byte 前解析，可信 dimensions 回写 Media。Web 只创建短期 browser object URL，不获得 vendor
  URL/key/hash。当前 ADMIN 自审要求显式确认和 session recent-auth；preview/approve 的 grant/evidence
  带 `selfReview`，approve 必须有可信 evidence，而 fail-closed block 不依赖 preview；moderator/委派
  管理员仍 fail closed。
- Forum draft 可保存本人 `pending/clean/blocked` upload id 以跨设备恢复状态，但 draft usage 也必须匹配
  target，并同步 exact draft reference；只有发布 mutation 的 clean + published revalidation 才建立公开授权。
- Migration `0057` 先在 identity profile、platform promotion 和 forum draft source table 安装同步 trigger，
  再对既有行 backfill Media 事实；同时 backfill callback digest 并删除 plaintext column。后者使 migration
  与旧 API 不兼容，必须先 drain 全部旧 API/writer/worker，再 migration + 新版部署。DB preflight、全部
  published Markdown/`asset_usages` reconciliation 与 DB/OSS object reconciliation 均通过并显式启用环境
  flag 前，通用 GC 与账号 purge system enqueue 仍为 `Partial` 的 rollout-gated 能力。
- Migration `0058` 给 account lifecycle job 增加 claim-unique lease token 与 running-state 约束。它与
  `0057` 共用 drain-old-workers 的 breaking cutover；旧 worker 不得在新 schema 上继续完成已持有的 job。
- Migration `0061` 增加 publication/variant/processing/ordered cleanup，并把 non-versioned binding 收紧为
  clean + published。它为既有受支持 clean image 排队 backfill processing，legacy animated/unsupported
  format 保持 failed 并要求重传；旧单对象 deletion worker 不能与新 multi-step cleanup 长期混跑。
  Provider object/CDN 是可重建派生物，publication completeness trigger 阻止缺少任一 variant 的 published。
- 通用 GC 不处理 pending age，只选择 approval `cleaned_at` 已满 30 天且没有 live reference、future grace
  或 active operational hold 的 clean asset。未 callback intent 由独立 exact-key housekeeping 清理；不能
  以 owner prefix 或 upload creation time 替代这些事实。
- Provider 删除成功后 upload row redacts storage key/URL/hash/size/MIME/usage/dimensions，并保留稳定 id
  与 purpose-limited audit。Operations-only hold/system-job inventory 返回 `private, no-store`；hold inventory
  和 system-job inventory 每次读取、CAS 续期/解除及 system dead-letter retry 都要求 recent-auth 并审计，
  不复用 moderation hierarchy。Synthetic cleanup tombstone 至少保留 30 天；hold/retry history 存在时等其
  365 天记录清理，succeeded job 可先清或随 tombstone cascade，不产生孤立可见数据。
- Forum attachment migration 是 additive rollout：先扩展 upload usage、增加 nullable image dimensions、
  空 `asset_usages`、短期 moderation preview grant 和 revision source version，再部署读写代码。历史
  Markdown 原本拒绝所有图片，因此不
  猜测/backfill 旧 source；旧 `plain_v1` 不变。回退应用时停止新图片写入并保留 usage/history 供恢复，
  不通过 drop 表或改写 source 伪造回滚。
- Profile text/reference 由 Identity 持有；Media 在事务内验证本人 clean + published image，再调用 Identity
  受限 binding API。Forum 取得已授权 profile projection 后才批量解析 typed、到期 Delivery projection，
  不跨域直查 upload 表。

## 身份、隐私与审计

- 公开 handle 与内部 account id 可跨域使用，校园邮箱只在 identity 的目的限定接口中处理。
- Email code 在 issuance 时写入具体 purpose；兼容客户端省略 purpose 时只能消费记录中已持久化的
  login/registration purpose，绝不根据验证时的账号状态重新推断，也不能触及 password-reset code。
- Access JWT 的 session id/auth version 是 revocation binding，不是客户端授权事实；每次受保护请求仍
  查询账号状态和 session。滚动窗口内的 legacy JWT 受账号级 revoked-before timestamp 约束。
- Migration `0066` 给 session 增加 nullable、固定 32-byte 的 account-scoped installation digest；既有客户端
  无需 backfill。完整登录在 account row lock 下撤销同摘要旧 session、插入新 session 并把 active 数量
  收敛到 30；refresh successor 继承摘要。Rolling window 的数据库 trigger 让旧 writer 旋转新 backend
  创建的 session 时仍继承摘要；回退应用可忽略 nullable column，但同 installation replacement 需新版 writer。
  User-Agent 只用于展示，不能参与唯一性或撤销决策。
- Recent-auth 是 Identity 所有的 session 投影：只记录 server-written timestamp 和受控 method，不从
  JWT `iat` 推导。密码验证只能更新当前 active session；`recent_auth` email code 的一次消费和
  session 更新在同一事务中，客户端不提交 email。refresh successor 继承原 timestamp/method，但
  freshness deadline 不重置。高风险 identity mutation 在业务 transaction 内对 actor session 取 share lock
  并查验 freshness，使并发 revoke 先提交时 mutation 必须失败，而不留事务外 check/use 窗口。
- Password method 还绑定 `recent_auth_credential_version`。验证先取得 password hash/version，写 fresh 时
  以 account credential version 做 compare-and-set；password set/change/reset 先推进版本并清空不再有效的
  freshness。真实 PostgreSQL barrier test 覆盖“旧密码已验证但新密码先提交”的并发顺序。
- Migration `0062` 为 reset code 绑定 credential version，记录有界 append-only password/replay security
  event，并增加只持久 account id + template kind 的 email delivery job。首次设置、修改、重置密码在同一
  transaction 推进 credential/auth version、替换 session、写 security event 和 enqueue 通知；worker 再
  通过 lease/backoff/dead-letter 调 provider，不持久收件地址、验证码、主题、正文或 provider response。
- Migration `0048` 是 additive session column 和 email-purpose constraint 扩展，旧应用会忽略新列并写入
  nullable 默认；滚动发布先跑 migration，再部署读取新列的应用。不带 session id 的 legacy JWT
  status 明确返回 unbound，高风险 mutation 一律 428 fail closed。紧急回退应保留 schema 并在边缘
  关闭高风险 route 后回退应用，不能以恢复无 step-up 的 mutation 作为“可用性降级”。
- `0047` 的 appeal 与 `0048` 的 recent-auth 曾分别扩展同一个 email purpose constraint；`0052` 先固定
  五类 purpose 的并集，`0053` 再增加 recovery，避免后执行 migration 覆盖前一能力。Fresh/rolling 部署
  必须在应用接受对应请求前跑到 `0053`，集成测试覆盖 purpose isolation。
- Appeal-only JWT 使用显式 `scope=appeal`、短 TTL、无 refresh/session。普通 identity/forum/reviews/
  credit middleware 必须拒绝 scoped token；只有治理申诉/通知 composition 调用 restricted authenticator，
  且 deleted 账号仍不可访问。
- Identity 的 public account API 只返回 active、未 suspended 且无邮箱的 profile/privacy projection；
  新增 profile/list/new-DM target 解析通过该 API 与 Forum 的 follow/mute/block policy 组合。旧 Forum
  projection 中仍有直接 identity join，需按 owner public API 逐步迁移，不能作为新代码模式复制。
- Activity 与 mention policy 由 Identity 的 `profile_privacy` 持有。Forum profile 读取 owner public
  projection 后叠加 viewer auth/block，显式返回 `canViewActivity`；逐条 authored-content endpoints 重验
  同一事实，公共 thread/feed route 不受 profile activity policy 反向影响。Profile aggregate 仍是公开
  内容计数，不从 activity list 的拒绝推断为零。
- Mention 创建先从 canonical visible text 得到最多 10 个去重 handle，再通过 Identity public batch API
  一次解析 active、未 suspended 的 account id/canonical handle，并在内容事务中追加候选 outbox。
  Consumer 与 privacy/relationship writer 使用相同 advisory lock，在最终通知事务中重验当前
  recipient-follows-actor、双向 block、recipient mute、mention policy、通知偏好和 source content
  可见性；拒绝只省略语义通知，不修改 canonical 正文、不返回 target existence/policy，也不形成逐
  handle 跨 schema SQL。
- Migration `0050` 只增加非 PII policy columns，已有账号回填 activity=`only_me`、mention=`everyone`。
  旧应用 writer 不触碰新列；新应用的 PUT 对旧客户端缺少这两个字段时保留当前值，避免 rolling window
  把已设置的 policy 重置为默认。新 Web 遇到旧 backend 未返回字段时使用 only-me activity 与
  everyone mention 的既有默认，不提交 undefined。发布顺序仍是先 additive migration，再部署读取新列的应用。
- 第一阶段 follow 只有 `not_following/following`，没有 private-account pending；mute 不授权也不阻断，
  block 在任意方向阻止 follow/DM/回复/投票并删除双方 follow，解除时不恢复。
- 新 PII migration 同时更新[隐私与数据生命周期](../security/privacy-and-data-lifecycle.md)。
- Staff write 记录 actor kind/id/role、action、target、reason、result 和 correlation；metadata 最小化。
- 人工认证由 platform 持有 typed definition 与可到期/撤销 grant；forum 公开 profile 只调用其 public
  projection API。公开条件在 PostgreSQL 查询边界同时检查 type policy、grant opt-in、expiry 与 revoke，
  不把 issuer、reason、evidence reference 或 internal grant id 复制到 profile DTO。
- 成就定义、账号授予/撤销和 append-only event 由 platform 持有；forum 只计算贡献资格并调用 owner
  API。Definition 以 version CAS 更新，grant/revoke 与 governance audit 同事务；自动首次授予用
  `(account_id, badge_id)` 和稳定 mint idempotency key 去重，人工授予不进入 mint queue，撤销不改 ledger。
- Secrets、code、token、signature-as-credential、raw email、完整请求 body 和任意 DM 不进入日志/审计。
- Evidence read 本身是敏感动作，需要 capability、purpose 和 audit。
- Governance audit event 是申诉的不可变原始引用。提交时 gateway 让 action 的 owner domain 验证
  ownership/appealability；决定时治理状态转换与 identity/forum/reviews 精确 reversal 共用同一 connection/
  transaction。owner 发现后续 audit 或不兼容当前状态时 fail closed，commit 后才失效 cache/search。
- Forum comment reversal 采用固定 thread→comment row-lock 顺序，并在取得 parent thread lock 后读取
  parent visibility；随后才检查 later governance event 和恢复 comment/media。并发 parent
  hide/delete 若先提交，comment 可恢复为 retained 状态但 activity/vote 不会错误重新激活。
- Governance notice 与处置/appeal transition 同事务写入，使用稳定 dedupe key。notice 是当事人安全摘要
  而非 evidence 副本；通用通知 preference、SSE 或未来 outbox 均不能删除这项 durable 事实。
- DM archive、mute 和 delete 是 `dm_participants` 上的 participant-local 状态；不能改写另一参与者的
  副本。新消息可恢复双方 inbox 可见性，但 mute 保持独立，并且只影响通知投影，不影响未读计数。
- DM request 是 canonical pair conversation 上的显式 `pending -> accepted | declined` 状态，不复用
  participant archive/delete 或 follow boolean。Pending 只含创建时单条附言；accepted unread 与
  incoming request count 分开投影，decline/withdraw 不创建通知或 block，block 会原子关闭 pending。
  `dm_messages` trigger 在数据库边界拒绝 pending 的第二条/接收方消息和 declined delivery，handler
  检查不是唯一安全边界。
- Request creation 使用 account-scoped `Idempotency-Key` 和 request hash；同 key 不同 payload 冲突。
  `request_sender_id/request_recipient_id` 只用于参与者授权、反骚扰冷却与本人导出，不进入日志、公开
  profile、搜索或 staff 通用浏览面。

## 积分不变量

- `credit.ledger` 是 append-only 权威；wallet balance 是可重建 projection。
- Append 序列化，验证 prev hash、canonical payload、signature、nonce 和 signing intent。所有同时需要
  ledger advisory lock 与 wallet row lock 的 debit/release 事务必须统一按 advisory → wallet 顺序获取；
  禁止先锁 wallet 再等待全局 append lock，否则并发 release 会形成锁环。tip/task debit 必须在该锁内
  重读签名余额 snapshot；锁等待期间余额变化使旧 intent fail closed。
- System mint 和用户受控操作使用明确 signer；私钥从 runtime secret 注入。
- Identity 每账号最多一把 active wallet key；首次登记要求当前 session recent-auth，同 key 幂等，不同
  key rotation 必须由未来 old-key proof/审计恢复协议拥有，Credit 不自行选择或修改 key。
- `/wallet/bind` 首次登记 body 必须携带与 recent-auth bearer 同一时刻捕获的 canonical `accountId`；
  服务端在写入前精确比较二者。客户端还必须固定该账号的 token/session generation 到实际 request
  dispatch，不能让拦截器在切号竞态中改用另一个账号的 bearer。迁移期无 `accountId` 的 legacy body
  只允许确认 bearer 账号已存在的 exact active key；无 active key 或不同 key 都 fail closed。
- Web 只使用按规范 API environment + account 隔离、`extractable=false` 且仅允许 sign 的 Ed25519
  `CryptoKey`。本机 public key 必须与 owner-visible `activePublicKey` 精确匹配；IndexedDB/WebCrypto
  不可用、record 损坏、环境/账号错配或 key 不一致都 fail closed。旧 `localStorage` raw seed 只有在派生
  公钥匹配且新 IndexedDB transaction durable commit 后才可删除，不能作为长期 fallback。
- 每个 Web value-moving mutation 在任何网络调用前，先以 IndexedDB 原子 `add` 创建 environment/account
  scoped operation digest claim；后续阶段只按 claim id 条件转换为含 intent id/expiry 的 `submitted`，成功后
  保留短期 committed tombstone。记录不保存 raw request、access/idempotency token、signature 或 signing
  bytes。签名前必须校验 canonical envelope 精确字段、规范请求 SHA-256、owner wallet/surface
  snapshot 与 action-specific ledger 金额/收方/metadata，不对 opaque proof 盲签。未知结果只接受固定 URL
  POST body 的 owner intent outcome：其 `FOR SHARE` 与消费的 `FOR UPDATE` 互斥，`committed` 证明
  业务提交、`expired` 证明锁等待结束后仍未消费，`pending`/错误继续 fail closed。
- 新 product purchase intent 不再在 snapshot/ledger proof 复制 title；consumer 仅为部署前已签发且仍在
  TTL 内的旧 intent 接受一个严格的 `title: string` legacy snapshot 扩展，并继续按原 exact bytes 验签。
  其他额外/缺失字段仍拒绝，因此 stop/start 后的短时 drain 不会把旧 intent 误判为篡改。
- 新 ledger row 只允许 `mint`、`tip`、`escrow_hold`、`escrow_release`；数据库拒绝 update/delete。
- Task/purchase 状态转换在事务内 `FOR UPDATE`，用 expected status CAS 并检查 affected rows；release、
  终态和 hold 清理必须同事务提交。
- Task `cancel` 仅 creator、`reject` 仅 acceptor；open-task delete 因退款必须签名，cancelled-task delete
  不移动价值且不得创建无法消费的 signing intent。
- Tip target 由 forum/reviews owner public API 解析，API composition 通过 identity public API 验证
  recipient eligibility；credit 不跨域直查内容或账号私有表。
- Public Product 不包含 delivery instructions；只有 buyer/seller 可访问的 Purchase surface 返回。
- Purchase action 与 purchase signing-intent snapshot 必须在返回任何订单状态、party 或金额前验证
  buyer/seller scope；非 party 与不存在记录统一 `NOT_FOUND`。签名 intent 还必须验证 actor/action/current
  state，不能成为绕过 owner-scoped purchase list 的枚举或预签名入口。
- 不新增 recharge、withdraw、fiat conversion 或 free transfer；冲突需求必须停止并升级确认。
- Credit reconciliation run/result 由 credit domain 持久化：请求 reason 和 idempotency key hash 去重，
  active run 由 partial unique index 与数据库 advisory lock 双重互斥。每次扫描使用 repeatable-read
  snapshot，先验证 ledger，再用 full outer comparison 生成只读 wallet projection evidence；run/result
  写入和 governance audit 不得触碰 wallet cache 或 append-only ledger。Resume 只重新获取同一 run 的
  lock 并追加 attempt audit，terminal run 是目标状态幂等，不会重放扫描。
- Reconciliation schema 是 additive 空表，不回填历史 run，旧应用版本会忽略；滚动部署先执行 migration
  再开放新 route。异常回退时停用新 route 并保留 evidence 表，不能通过删除表或改 ledger 伪造恢复。

## Change impact matrix

| 变更 | 同一 PR 的必需产物 |
|---|---|
| HTTP | 产品语义、OpenAPI、实现、生成类型、客户端、contract/handler tests |
| Schema | 新 migration、owner code、fresh-up 验证、兼容/回填说明、相关架构/产品文档 |
| 权限/治理 | capability、负向测试、reason/audit/notification、产品与安全规范 |
| PII/保留 | data purpose、visibility、retention/export/delete、privacy review |
| Search/cache/counter | 事实源、投影写入、失效、reindex/reconcile 和隐私过滤测试 |
| Media | asset state/binding、OSS policy、URL authorization、cleanup 和安全测试 |
| Credit | 合规确认、签名/重放/边界测试、ledger verification |

精确验证命令见[测试策略与命令](../development/testing.md)。

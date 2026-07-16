# 每日活跃度计分

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Activity/Forum/Reviews maintainers、Community operations
>
> 最近核验：2026-07-15，migrations 0059–0060、签到/score projection、durable evaluator 与 Web cache/share tests

首页右侧方格图表示当前用户每天的社区活跃度，类似 contribution heatmap。
活动分数本身不直接铸造或转移积分；但 lifetime 有效活动分数会驱动统一信任等级
（1–6 茶等级），详见下文「统一信任等级与活动分数」。

## 当前状态

### Current

- 独立 `activity` domain 拥有幂等 activation/reversal events、每日计数、版本化计分策略和统一信任等级。
- 主题、评论、正向论坛 vote、课评 like 和每日签到计入；可撤销内容互动随源状态反向投影，
  签到作为用户确认过的自然日事实不可撤销。
- 管理员可查看/修改 thread/comment/like/check-in 权重、点赞每日计分上限、信任阈值、治理降级
  冷却期，并查看版本历史。
- 首页使用真实 `/me/activity`、`/me/check-in` 与 `/me/trust-progress` 数据；桌面和窄屏均可签到、
  查看热力图与等级进度。
- Web 在首次签到成功后打开可再次访问的分享结果卡，可复制文案或保存 SVG 图片；导出模型只允许
  上海日期、连续/累计天数、茶等级名称和进度，不接收 account、email、设备标识或媒体 URL。
- 每日 trust evaluator 使用上海自然日、PostgreSQL durable run/lease 与账号当日去重；以 50 个账号
  为一批持久推进 cursor 并续租，每个账号 mutation 重新校验 lease token。单账号错误写入 failure
  inventory 后继续后续账号，失败 run 有指数退避和 8 次上限，进程重启或多实例不会让同一账号在
  同一天重复升级。
- migration 会幂等 backfill 既有可见贡献，并初始化信任等级进度。

### Partial

- durable run/failure inventory 与结构化日志已提供最小 worker reconciliation 入口；尚无自动修复
  score projection 漂移的任务或管理 UI。
- 公开资料热力图尚未开放；必须先实现 activity visibility。
- 新贡献类型需要显式产品规则、migration、契约和策略版本，不能直接复用 `like`。

## 计分定义

初始策略为：

```text
score = threads * 10 + comments * 3 + likes * 1 + check_ins * 1
```

权重是数据库中的 versioned policy，不是应用常量。`likes` 指用户主动给出的正向反应：

- forum thread/comment up-vote；
- course review like。

`check_ins` 每个账号每个上海自然日只能是 0 或 1；默认权重同样可由管理员修改。签到不是登录次数，
重复请求只返回同一事实，不重复加分，也不能通过通用 contribution reversal 撤回。

不计入：收到的赞、down-vote、浏览、编辑、普通登录、DM、举报、管理动作或积分变化。一个 source
relationship 同时最多只有一个 active positive event，自赞/自投在源写路径拒绝。

## 状态转换

| Source transition | 每日投影 |
|---|---:|
| visible thread 创建 | 原始日期 `thread +1` |
| visible comment 创建 | 原始日期 `comment +1` |
| 正向 vote/review like 创建 | 反应日期 `like +1` |
| up-vote 改为 down-vote，或 unlike | 原始反应日期对应 `-1` |
| 举报阈值自动隐藏内容 | 内容及其不可用子树的原始日期对应 `-1` |
| 自动隐藏被 reject/ignore 且内容可见 | 原始日期对应 `+1` |
| uphold 或 staff 隐藏/软移除 | 保持 reversed |
| staff 恢复为完全可见 | 原始日期对应 `+1` |
| 当日首次签到 | 当日 `check_in +1`；重复请求 no-op |

内容不可用时，其上的正向反应也反向投影，避免用户保留来自违规内容的活跃度。主题隐藏、软删除
或归档会在同一事务停用主题、其全部评论以及主题/评论上的正向 vote；恢复只重新激活父主题可见、
评论自身可见且正向关系仍存在的 contribution，并使用内容或反应的原始日期。

## 日界线与范围

- canonical day 固定为 `Asia/Shanghai`，不随浏览器或服务器时区变化。
- 源 timestamp 为 UTC，projection 保存对应上海 `activity_date`。
- 默认返回截至今天的连续 365 天，请求最多 371 天；首页展示最近 20 周。
- 空日期显式返回零，客户端不自行推断或补时区缺口。

## 数据模型与不变量

### `activity.events`

Append-only event 保存：

- 全局唯一 `event_key`；
- 稳定 `source_key` 与重新激活时递增的 generation；
- account、kind (`thread/comment/like/check_in`)、delta（可撤销类型为 `+1/-1`，签到只有 `+1`）；
- 原始贡献的 activity date、occurred/created time；
- reversal 对应的唯一 `reverses_event_id`。

`(source_key, generation, delta)` 唯一。负事件必须且只能反向一条先前正事件。这里不保存正文、
邮箱、handle、IP 或 DM 内容。

### `activity.daily_counts`

每个 `(account_id, activity_date)` 一行，保存非负的 threads/comments/likes、0/1 check-ins、当前策略下
的 daily score 和更新时间。源业务
mutation、event 与 daily count 在同一 PostgreSQL transaction 中提交；读取不聚合 forum/reviews
源表。

### `activity.check_ins`、`activity.account_scores`

`check_ins` 以 `(account_id, activity_date)` 为主键，并约束 timestamp 映射到同一上海日期；
`account_scores` 保存 write-maintained lifetime qualifying score 及 score/trust policy version。读取等级进度
不扫描全量日表。普通贡献在共享 projection lock 下增量更新单日分数与账号 delta；策略发布使用同一
advisory key 的排他锁整体重投影，防止混合版本或不同日期并发更新丢分。

### `activity.score_policies`

策略 append-only，包含 version、四个 `0..1000` 权重、mandatory reason、changed by 和 created at。
`expectedVersion` 是 optimistic concurrency 输入。最新 version 立即生效并重新解释展示历史，
但不重写 raw daily counts；发布与 governance audit 在同一 transaction。

## 一致性

- source key 先 advisory lock，generation/reversal constraints 提供幂等边界。
- Forum 父子状态转换先锁 thread、再按 id 锁 comments；随后按 thread source、comment id、
  vote target/account 的固定顺序取得 activity source lock。comment 恢复必须在锁住 parent 后重读
  可见性，不能使用事务外快照。
- 重复创建、取消、隐藏、恢复和重试不能制造额外活跃度。
- 未匹配的 deactivation 是幂等 no-op，daily counters 不得为负。
- `check_in` 不允许走通用 deactivation；账号 purge 由 owner lifecycle 清除其事实与投影。
- policy update 锁定当前 version，stale `expectedVersion` 返回 conflict。
- calendar 使用 repeatable-read 读取同一 policy version 的 persisted daily score；策略变更与 contribution
  并发后最终 `daily_counts`、`account_scores` 和返回版本必须一致。
- Backfill 仅是 deployment coordination exception；runtime 仍通过 activity crate API 跨域写入。

## HTTP 与 UI 语义

- `GET /api/v2/me/activity?from=YYYY-MM-DD&to=YYYY-MM-DD` 返回 timezone、范围、policy version、
  weights 和连续 day entries。
- `GET /api/v2/me/check-in` 只读取当日状态；`POST /api/v2/me/check-in` 幂等创建当日事实，并返回
  `newlyCheckedIn`、连续/累计天数和下一次上海日界线。两者都要求登录。
- `GET /api/v2/me/trust-progress` 返回当前等级、有效活动分、下一等级阈值和策略版本，不把客户端
  计算结果当作权限依据。
- `GET/PUT /api/v2/admin/activity-policy` 与 history endpoint 需要 `activity.policy` capability。
- Public profile activity endpoint 不存在，直至 visibility policy 交付。
- 卡片标题固定为“活跃度”；五档 intensity 包含零值，颜色不是唯一信号。
- cell 可键盘聚焦并有可访问 label；tooltip 显示日期、score、thread/comment/like/check-in 原始数。
- 未登录显示解释性状态，不生成模拟数据；窄屏仍可访问签到、热力图与等级进度。
- Web 对本人当日签到状态使用 5 分钟 fresh cache，路由短时卸载/重挂不得重复请求；mutation 成功直接写入
  返回事实并刷新 activity/trust projection。`nextResetAt` 到达上海日界线后必须主动失效三类 cache，
  不能让 client cache 把前一天的已签到状态带到次日。
- 已签到主按钮只打开结果卡，不再提交 mutation。复制文案和 SVG 均从同一显式字段 allowlist 生成，
  不读取账号对象、当前 URL、头像或浏览器存储；导出不包含跨站资源或追踪参数。

具体 wire shape 以 `contract/openapi.yaml` 为准。

## 管理员修改权重

策略编辑器展示当前版本、公式、四个有界整数、样例日期预览、mandatory reason 和 cursor history。
保存使用 `expectedVersion`；成功策略立即成为 current，并明确提示历史 score 会按新权重重算展示。
后台不能编辑 raw daily counts，也不能用权重调整替代数据修复。

## 验收基线

- 上海日界线、连续日期和最大范围正确。
- duplicate、unlike、vote change、自动/人工隐藏、决定与恢复保持幂等计数；主题不可用时整棵
  contribution 子树归零，恢复只恢复仍合法的评论/vote。
- parent hide/delete/archive 与并发 comment restore/unhide/vote 不死锁，也不能留下错误 re-activation。
- down-vote、收到的赞和自互动不增加活跃度。
- 同一上海自然日的并发/重复签到只产生一个事实、一个正向事件和一个计分增量；签到不可通过
  通用 reversal 接口撤销，账号清除流程会一并删除签到事实与投影。
- 分享卡序列化测试必须向输入注入 email、account/device id 和签名 URL，并证明所有复制/图片输出均
  不包含这些字段。
- policy 版本冲突、权限、reason、即时生效和审计有集成测试。
- contribution 与策略发布并发后不能丢失账号总分或暴露混合 policy version；同一账号的不同日期
  并发贡献都必须保留。
- 自动升级每次至多一级；治理降级后的冷却期与“必须有新活动分”门槛、重复执行、重启接管和
  同日同分再次升级都要有行为测试。
- 读取只访问 projection，不在请求路径聚合源表。
- 首页不存在 trust-level 或数组位置派生的 placeholder cell。
- durable run/failure inventory、租约接管和单账号失败隔离属于上线基线；自动 projection drift
  dry-run/修复是后续运维增强，启用前必须补幂等与负向测试。

## 统一信任等级与活动分数

统一信任等级（1–6）由活动域基于 lifetime 有效活动分数自动评定，替代旧论坛 TL 手动规则。
等级映射为茶名：绿茶（Lv.1）、白茶（Lv.2）、黄茶（Lv.3）、
青茶（Lv.4）、红茶（Lv.5）、黑茶（Lv.6）。Lv.0 仅用于未注册访客的 UI 状态。

### 阈值与策略

默认阈值由 `activity.trust_level_policies` 管理（当前 30/120/400/1200/3000），
策略变更通过 `PUT /api/v2/admin/trust-policy` 版本化管理，包含 `expectedVersion`
optimistic lock、mandatory reason 与同事务 governance audit。

每日 like 每日上限（默认 20）在计分时不直接限制给予，而是按 `LEAST(likes_given * like_weight, cap)`
限制每日 like 对信任分数的贡献。基础公式为：

```text
qualifying_score = Σ (threads × thread_weight + comments × comment_weight
                      + LEAST(likes × like_weight, like_daily_cap)
                      + check_ins × check_in_weight)
```

### 自动升降

- **注册**：新注册账号从 Lv.1 起步，写入 registration event。
- **升级**：每次评估最多提升一级，不会跳跃多级。daily evaluator 使用 PostgreSQL durable
  run/lease 和上海自然日去重；`evaluate_account_tx` 读取同一版本政策与有效活动分，若目标等级高于
  当前则只前进一级。故障恢复或多实例接管不会在同一日重复执行该账号的 scheduled evaluation。
- **降级**：治理事件通过 `apply_governance_demotion_tx` 精确消耗一个 governance event id，
  最多降一级。同一 governance event 不会重复降级（唯一索引 `trust_level_events`
  的 `event_kind = 'demotion' AND governance_event_id IS NOT NULL` 保证）。降级后必须同时经过管理员
  配置的冷却期，并在降级基线之上取得新的活动分，才允许再次自动升级；不能靠旧分立即反弹。
- **手动覆盖**：管理员可通过 `PATCH /api/v2/admin/users/{id}/trust-level` 设置
  `override_level`。覆盖后自动评估只更新 `qualifying_score` 和 `policy_version`，
  不改写 trust level。降级事件仍可降低覆盖后的等级。

### 写限制与举报权重

| 等级 | 举报权重 |
|---|---:|
| Lv.1（绿茶）| 0.5 |
| Lv.2（白茶）| 1.0 |
| Lv.3（黄茶）| 1.5 |
| Lv.4–6（青/红/黑茶）| 2.0 |

同一内容 open report 总权重 ≥ 3.0 自动隐藏。等级 4–6 拥有相同最大举报权重，
不因等级差异产生额外安全漏洞。

### 审计

所有等级变动都写入 `activity.trust_level_events`（唯一 `event_key`），包括
registration、upgrade、demotion、manual_set、override_clear。
管理员的事件历史通过 `GET /api/v2/admin/users/{id}/trust-events` 提供 cursor 分页。
信任策略变更写入 `governance.audit_events`。

### 当前状态

统一信任等级后端与 Web 进度/管理面已交付。仍缺：
- 管理员信任策略变更的影响账号数预览。
- 每周 digest 中信任等级的展示。

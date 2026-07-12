# 每日活跃度计分

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Activity/Forum/Reviews maintainers、Community operations
>
> 最近核验：2026-07-11，`origin/main@33584db`

首页右侧方格图表示当前用户每天的社区活跃度，类似 contribution heatmap。它不是等级成长，
不授予信任等级，也不直接铸造或转移积分。

## 当前状态

### Current

- 独立 `activity` domain 拥有幂等 activation/reversal events、每日计数和版本化计分策略。
- 主题、评论、正向论坛 vote 和课评 like 计入；取消、状态变化和治理隐藏会反向投影。
- 管理员可查看/修改 thread/comment/like 权重并查看版本历史。
- 首页使用真实 `/me/activity` 数据，tooltip 展示日期、score 和原始计数。
- migration 会幂等 backfill 既有可见贡献。

### Partial

- 尚无可观察的 reconciliation worker、漂移指标和修复任务。
- 公开资料热力图尚未开放；必须先实现 activity visibility。
- 新贡献类型需要显式产品规则、migration、契约和策略版本，不能直接复用 `like`。

## 计分定义

初始策略为：

```text
score = threads * 10 + comments * 3 + likes * 1
```

权重是数据库中的 versioned policy，不是应用常量。`likes` 指用户主动给出的正向反应：

- forum thread/comment up-vote；
- course review like。

不计入：收到的赞、down-vote、浏览、编辑、登录、DM、举报、管理动作或积分变化。一个 source
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

内容不可用时，其上的正向反应也反向投影，避免用户保留来自违规内容的活跃度。主题隐藏、软删除
或归档会在同一事务停用主题、其全部评论以及主题/评论上的正向 vote；恢复只重新激活父主题可见、
评论自身可见且正向关系仍存在的 contribution，并使用内容或反应的原始日期。

## 日界线与范围

- canonical day 固定为 `Asia/Shanghai`，不随浏览器或服务器时区变化。
- 源 timestamp 为 UTC，projection 保存对应上海 `activity_date`。
- 默认返回截至今天的连续 365 天，请求最多 371 天。
- 空日期显式返回零，客户端不自行推断或补时区缺口。

## 数据模型与不变量

### `activity.events`

Append-only event 保存：

- 全局唯一 `event_key`；
- 稳定 `source_key` 与重新激活时递增的 generation；
- account、kind (`thread/comment/like`)、delta (`+1/-1`)；
- 原始贡献的 activity date、occurred/created time；
- reversal 对应的唯一 `reverses_event_id`。

`(source_key, generation, delta)` 唯一。负事件必须且只能反向一条先前正事件。这里不保存正文、
邮箱、handle、IP 或 DM 内容。

### `activity.daily_counts`

每个 `(account_id, activity_date)` 一行，保存非负的 threads/comments/likes 和更新时间。源业务
mutation、event 与 daily count 在同一 PostgreSQL transaction 中提交；读取不聚合 forum/reviews
源表。

### `activity.score_policies`

策略 append-only，包含 version、三个 `0..1000` 权重、mandatory reason、changed by 和 created at。
`expectedVersion` 是 optimistic concurrency 输入。最新 version 立即生效并重新解释展示历史，
但不重写 raw daily counts；发布与 governance audit 在同一 transaction。

## 一致性

- source key 先 advisory lock，generation/reversal constraints 提供幂等边界。
- Forum 父子状态转换先锁 thread、再按 id 锁 comments；随后按 thread source、comment id、
  vote target/account 的固定顺序取得 activity source lock。comment 恢复必须在锁住 parent 后重读
  可见性，不能使用事务外快照。
- 重复创建、取消、隐藏、恢复和重试不能制造额外活跃度。
- 未匹配的 deactivation 是幂等 no-op，daily counters 不得为负。
- policy update 锁定当前 version，stale `expectedVersion` 返回 conflict。
- Backfill 仅是 deployment coordination exception；runtime 仍通过 activity crate API 跨域写入。

## HTTP 与 UI 语义

- `GET /api/v2/me/activity?from=YYYY-MM-DD&to=YYYY-MM-DD` 返回 timezone、范围、policy version、
  weights 和连续 day entries。
- `GET/PUT /api/v2/admin/activity-policy` 与 history endpoint 需要 `activity.policy` capability。
- Public profile activity endpoint 不存在，直至 visibility policy 交付。
- 卡片标题固定为“活跃度”；五档 intensity 包含零值，颜色不是唯一信号。
- cell 可键盘聚焦并有可访问 label；tooltip 显示日期、score、thread/comment/like 原始数。
- 未登录显示解释性状态，不生成模拟数据；窄屏仍可访问热力图。

具体 wire shape 以 `contract/openapi.yaml` 为准。

## 管理员修改权重

策略编辑器展示当前版本、公式、三个有界整数、样例日期预览、mandatory reason 和 cursor history。
保存使用 `expectedVersion`；成功策略立即成为 current，并明确提示历史 score 会按新权重重算展示。
后台不能编辑 raw daily counts，也不能用权重调整替代数据修复。

## 验收基线

- 上海日界线、连续日期和最大范围正确。
- duplicate、unlike、vote change、自动/人工隐藏、决定与恢复保持幂等计数；主题不可用时整棵
  contribution 子树归零，恢复只恢复仍合法的评论/vote。
- parent hide/delete/archive 与并发 comment restore/unhide/vote 不死锁，也不能留下错误 re-activation。
- down-vote、收到的赞和自互动不增加活跃度。
- policy 版本冲突、权限、reason、即时生效和审计有集成测试。
- 读取只访问 projection，不在请求路径聚合源表。
- 首页不存在 trust-level 或数组位置派生的 placeholder cell。
- reconciliation worker 上线前必须补漂移告警、dry-run、修复和幂等测试。

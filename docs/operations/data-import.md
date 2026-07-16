# D1 选课快照导入

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Courses/Selection maintainers、Data migration maintainers
>
> 最近核验：2026-07-16，migration `0069`、projection preflight 与 selection sync lease fencing

本 runbook 只覆盖 Cloudflare D1 选课快照到 PostgreSQL `selection.pk_*` 的首次/恢复导入，再物化
`courses.*` 与 `selection.*`。它不是完整生产数据迁移，也不自动迁移历史课评、身份或钱包归属。

## 数据层

```text
D1 selection tables
  -> selection.pk_* raw snapshot
  -> courses.* catalogue projection
  -> selection.* normalized projection
```

- Raw 层尽量保留 D1 形状，供重放和差异检查。
- `courses.*` 与 `selection.*` 由 `backend/ops/materialize_*.sql` 幂等物化。
- `selection.courses` 每行是一个教学班：`id` 来自 raw course-detail/teaching-class id，`code` 是可重复的
  课程代码，`calendar_id` 是必须保留的学期归属。快照内详情和节次以该教学班 id 关联，不得用
  course code 取任意行。
- `courses.*` catalogue 和 `selection.*` teaching-class mirror 当前没有 typed authoritative bridge。
  物化成功不等于可以按 course code 安全实现“课程详情→某教学班”。
- `reviews.*` 的历史导入需要独立 identity/course mapping、privacy 和 moderation 决策，不属于本流程。

## 安全前提

- 仅在 local、isolated staging 或经过批准的恢复环境运行；不要对未知 main 数据库执行。
- D1 token 只要 read 权限，通过环境变量注入，不提交 `.db`、token、导出日志或真实数据样本。
- 目标 raw tables 必须为空；脚本为防快照叠加会拒绝非空目标。
- 先备份并记录 source snapshot/time、target database、operator、row counts 和 validation result。
- 两份 materializer 在首个写操作前都会锁定 raw tables，并要求最新 `selection.import_runs` 的 13 张表
  source/target counts、schema validation、completeness approval 与当前 live counts 完全一致；calendar、
  course nature、campus、faculty、major、teaching class、teacher、teacher timeslot、major binding、fetchlog
  等核心表任一为空，或无学期/课号教学班、孤立/越界时段都会 fail closed。

## 指定 backup 快照源

选课恢复导入必须使用无 FTS 特殊对象的 `jcourse-db-backup`，不得把生产 `jcourse-db` 临时替换进命令。
先验证当前 Cloudflare 身份确实能只读看到目标库，再导出到 Git 忽略范围外、mode `0600` 的临时目录：

```bash
wrangler whoami
wrangler d1 list

umask 077
wrangler d1 export jcourse-db-backup --remote \
  --output /tmp/jcourse-db-backup.sql
sqlite3 /tmp/jcourse-db-backup.sqlite3 < /tmp/jcourse-db-backup.sql
sha256sum /tmp/jcourse-db-backup.sql /tmp/jcourse-db-backup.sqlite3
```

也可以令 `CLOUDFLARE_D1_DATABASE_ID` 精确指向 backup database id 后使用仓库的
`tools/d1/d1_export.py`。无论哪条路径，都必须在 operator 记录中写 `jcourse-db-backup`，且不能把
database id、token、signed export URL 或快照文件写入仓库和日志附件。

导出器会逐表读取 D1 API，Cloudflare 不为这组请求提供跨表事务快照。文件落盘的原子性只保证读者不会拿到
半个本地文件，不代表源端各表来自同一事务时点。因此只能在 backup 刷新静止或明确维护窗口导出；若导出期间
`fetchlog` 或表计数变化，应丢弃该文件并重新导出，不能把其 manifest 当作一致性证明。

## 前置环境

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r tools/d1/requirements.txt

export CLOUDFLARE_ACCOUNT_ID=...
export CLOUDFLARE_D1_DATABASE_ID=...
export CLOUDFLARE_API_TOKEN=...
export DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj
```

按[本地环境](../development/local-development.md)启动 PostgreSQL，并通过 sqlx migration ledger
建立 schema。不要循环用裸 `psql` 重放全部 migration。

## 导出与导入

```bash
# 1. 校验 database id 的真实名称并原子导出到新文件；已有目标会被拒绝
python3 tools/d1/d1_export.py --output /tmp/jcourse-db-backup.sqlite3

# 2. 原子导入 selection raw tables，并落 import audit/manifest
python3 tools/d1/d1_import_pg.py \
  --source /tmp/jcourse-db-backup.sqlite3 \
  --source-database jcourse-db-backup \
  --snapshot-exported-at '<RFC3339 export time>' \
  --imported-by 'selection-import:on-call' \
  --compare-manifest /secure/approved-previous-manifest.json \
  --manifest-out /tmp/jcourse-db-backup-manifest.json

# 数据库只能从受控运维 shell 访问时，可输出 COPY stream
python3 tools/d1/d1_import_pg.py \
  --source /tmp/jcourse-db-backup.sqlite3 \
  --source-database jcourse-db-backup \
  --snapshot-exported-at '<RFC3339 export time>' \
  --imported-by 'selection-import:on-call' \
  --compare-manifest /secure/approved-previous-manifest.json \
  --emit-copy | psql "$DATABASE_URL"

# 3. 物化 catalogue 与 normalized selection
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/materialize_courses.sql
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/materialize_selection.sql
```

`--compare-manifest` 是默认必需的获批基线：schemaVersion/sourceDatabase、snapshot hash 与 13 张表完整
count key 必须有效，核心表任何下降都会在写 manifest/数据库前拒绝。确认上游确有合法下降时，必须显式附加
`--approve-count-decrease --approval-reason '<10-500 字符原因>'`；只有第一次建立可信基线可以改用
`--approve-unbaselined-snapshot --approval-reason '<10-500 字符原因>'`，且仍不能绕过核心表非空门禁。审批
mode、reason、baseline hash/counts 会进入 `selection.import_runs.validation`，materializer 会再次校验。

`--manifest-out` 是导入前的只读 source manifest，必须指向不存在的新文件；它不会覆盖旧证据。
`--imported-by` 必须显式使用 3–64 字符的小写 role/service label（例如
`selection-import:on-call`），不得填写姓名、邮箱、工号或其他直接身份标识。
`--source`、`--manifest-out` 与 `--compare-manifest` 不得解析到同一文件。比较与写新 manifest 同时启用时，
脚本先读取/报告旧 manifest，再原子创建新文件。只有事务内成功写入的
`selection.import_runs` 才表示 PostgreSQL 导入成功。脚本当前导入 13 张 `selection.pk_*` raw table。精确映射和参数以
[`tools/d1/README.md`](../../tools/d1/README.md) 与脚本 `--help` 为准，不在本文复制随代码变化的表清单。
`--source-database` 必须由 operator 显式填写，它是操作声明而非文件来源的密码学证明；只有紧接仓库 exporter
完成、且 hash/表计数连续记录的受控流程才可建立可信链。

## 失败与回滚

- COPY、row-count 验证或 `selection.import_runs` 写入失败时，导入事务会整体回滚；不要跳过 empty-table guard
  后重试，也不要手工补齐部分 raw rows。
- 已提交的 local/isolated 导入验证失败时，停止 worker 与读流量，保留 snapshot hash、manifest 和 audit 证据，
  然后丢弃该隔离数据库并从导入前备份重新建库、跑 migration、重新导入。
- 已提交的共享恢复环境只能按其已审批的数据库备份恢复流程回滚；不得直接 `TRUNCATE` raw 表。先恢复
  `selection.pk_*` 的一致快照，再重跑两份幂等 materialization，最后重建 search/cache 并完成本节验证。
- materialized projection 可以由已验证 raw snapshot 重建，但它不能替代 raw 数据库备份；Meilisearch 与 Redis
  也不是 PostgreSQL 回滚依据。

`materialize_selection.sql` 严格解析完整安排行中的星期、1-based inclusive 节次、周次与地点；无法解析的
周表达式保留 `weeksUnknown=true`，地点缺失独立保留 `locationUnknown=true`。若同一教学班同时包含可解析与
不可解析安排行，可解析事实继续使用，未覆盖的 day/section 从辅助 raw 表补齐，但整个教学班保持
`scheduleUnknown=true`，不能用部分成功推导“其余时间没有课”。

### Migration `0069` 升级与回退意图

`0069` 先从 raw teaching-class id 和受约束 calendar dimension 回填现有 `selection.courses.calendar_id`，再将
该列收紧为 `NOT NULL`。升级前必须确认每个既有 normalized offering 都能在获批 raw snapshot 中找到有效
calendar；任何剩余 NULL 会让 migration 整体失败，operator 应恢复可信 raw snapshot 后 forward-fix，不能用
当前学期或最大 id 猜值。周次范围同时要求两端均为空或均为合法范围，NULL 上游周次显式映射为 unknown。
旧 `selection.timeslots` 若缺少 weekday/start/end 或超出 1–7/1–20 有序范围，migration 会删除该条不可能安全
序列化的事实；新增 `scheduleUnknown=true` 会一直保留到可信 raw snapshot 重物化，不能把清理误报为“没有课”。
其余 day/slot 列随后收紧为 NOT NULL + range/order CHECK。

Migration 新建的空 `selection.import_runs` 不会替历史 raw 数据伪造 provenance。升级后第一次物化前，必须通过
本 runbook 的 exporter/importer 或获批数据库恢复流程建立匹配的 import run；不得手工插入 audit row 绕过
门禁。应用回退应保留新增列、约束、函数和审计表并停用新版 sync，随后 forward-fix；删除 `NOT NULL` 或完整性
门禁会重新允许跨学期/空快照破坏，因此不是安全回滚。

## Selection 搜索索引 schema cutover

`calendarId` 是 `selection_courses` 的 filterable attribute，也是每份教学班 document 的必需分区事实。
只部署新查询代码或只更新 filterable settings 都不够：旧 document 没有该字段，会让带 calendar filter 的
搜索稳定返回空结果。更新后的 admin selection sync 按以下顺序执行，并只在每个 Meilisearch task 确认
成功后进入下一步：

1. 物化 PostgreSQL `selection.*`，确认教学班数量和 calendar 分布非零且合理；
2. 设置 searchable/filterable attributes，并等待 settings task 成功；
3. 清空 `selection_courses` documents，并等待 clear task 成功；
4. 从 PostgreSQL 全量加入包含 `calendarId` 的 current documents，并等待 add task 成功；
5. 最后更新相关 Redis cache version，并记录完成日志中的 document count。

worker 在长耗时 SQL/Meilisearch 阶段每 60 秒续租，并在 catalogue/selection 物化及 Meilisearch
settings/clear/add 的每个破坏性动作前后，以 job id、lease token 与未过期时间重验所有权；任何一次失租都
取消旧 pipeline，旧 worker 不得继续写索引、推进进度或提交完成。该 fence 防止 lease 被新 worker 接管后旧
pipeline 继续覆盖结果，但没有把 Meilisearch 的 clear→add 变成原子操作。

现有 admin endpoint 返回 `202` 只代表进程内任务已排队。首次 calendar schema cutover 应在不接用户流量的
新 revision 实例上触发，且同一环境只运行一个 selection sync；clear 到 add 完成之间搜索必然有短暂空窗，
不得把当前流程描述为 versioned index 或原子切换。
Operator 必须看到 settings、clear、add 和 pipeline completion 日志，再用至少两个 calendar 的已知
教学班验证关键词/课号命中及跨学期不串数据，之后才能把新 backend 切入流量。任一 task 失败时停止
cutover；不要把空结果当成合法完成，也不要只重跑 add 掩盖未知 clear/settings 状态，应从完整 sync 重试。

## 验证

- Source export table/row counts 与 raw target counts 一致，记录允许的过滤项。
- Raw 关键主键无 duplicate，必要 foreign key mapping 无 unexpected null；记录教学班 id 在本次快照的唯一性，
  不在没有 lineage 规则时声称上游重编号后仍稳定。
- 最新 validated import run 的 source/target counts 与锁定后的 13 张 live raw tables 完全一致，并包含有效
  completeness approval；主动用缺失/旧格式 import run、核心表空值、未经批准的 count decrease 和改动一张
  live raw 表计数的 fixture 验证两份 materializer 都在任何写入前拒绝执行。
- `courses.courses`、`selection.courses`、teacher/timeslot/major relation 有非零且合理数量。
- 抽查多教师、同 course code 多教学班、跨学期重复 course code、课程性质缺失等 edge cases；
  记录 `weeksUnknown`、`locationUnknown`、`scheduleUnknown` 数量，并覆盖 NULL 周次、严格行解析、混合可解析/
  不可解析安排和辅助时段补齐，不手工伪造缺失事实。
- 选课列表/搜索 API 需要 `calendarId` 且不返回其他学期；详情和节次用 `teachingClassId`，
  同 course code 的平行班不串数据。Meilisearch full reindex 后课号/关键词可按 calendar 过滤；记录
  filterable attributes、完成的 clear/add task 和最终 document count，而不是只记录 enqueue response。
- Materialization rerun 结果稳定，不产生额外 duplicate 或删除人工维护数据。
- 同一教学班完全相同的 day/slot/week/location 事实不会因多教师 raw row 重复；单个教学班物化时段不超过
  API 的 100 条上限。多教师无法归属到某一条时段时 `teacherName` 必须为空，而不是任意挑一位教师。
- `selection.import_runs` 的 source/target row counts 一致，`rowCountsMatched=true`；`importedAt` 与
  `selection.fetchlog` 的上游 `updatedAt` 分开核对，不能用刚导入掩盖陈旧源数据。

### 2026-07-15 真实快照基线

本机隔离验证使用 2026-07-15 导出的 `jcourse-db-backup`：13 张表共 134,256 行，其中
`coursedetail=20,834`、`teacher=34,745`、`teacher_timeslots=12,880`、
`majorandcourse=64,521`。物化后有 20,834 个 offering、64,521 条专业绑定和 40,530 条去重时段；
`scheduleUnknown=3`、offering `weeksUnknown=8`、时段 `weeksUnknown=6`，非法节次、已知但空周集合、重复
时段事实均为 0，最大单 offering 51 条时段。第二次完整物化结果不变。

这些数字是本次 operator 按上述命令在仓库外隔离环境记录的可复现证据，不是 CI artifact、production telemetry
或线上 SLO 证明。

50 条确定周次时段通过真实 HTTP filter 与 PostgreSQL 真值逐条比对，结果为 50/50。快照本身在
2026-07-15 导入，但 `fetchlog` 最新上游时间仍为 2026-06-11，因此 `/selection/latest-update` 正确返回
`stale=true`；这是两个独立 freshness clock 的必要演练。

## 性能基线与全量物化决策

基线环境是 WSL2 本机、PostgreSQL 16.14、Redis 7.0.15、Meilisearch 1.10.3；使用上述真实快照、warm
buffer、prepared query、4 clients，每类 1,000 samples。它用于回归，不代表生产容量：

| 热路径 | 本机 p95 |
|---|---:|
| calendar→grade metadata SQL | 5.547 ms |
| major + grade offering page SQL | 0.958 ms |
| weekday/slot/week offering page SQL | 1.196 ms |
| 最大 offering timeslots SQL | 0.243 ms |
| loopback HTTP browse，100 个 cold cache keys | 7.558 ms |
| loopback HTTP Meili search + PG rehydrate，100 个 cold cache keys | 13.945 ms |

回归预算为 metadata/cache-hit p95 `<100 ms`、offering/search HTTP p95 `<300 ms`、单次加入前时段读取
p95 `<100 ms`。PR/CI 只能验证 plan 与功能；staging/production 必须从实际 telemetry 重新建立 p95，不能
拿本机数字宣称线上 SLO。

当前继续采用完整物化，不实施增量：真实快照首次/幂等重跑均在一个 advisory-lock transaction 内完成，
最慢约 15.1 秒，读者只会看到旧完整快照或新完整快照；规模和复杂度尚不支持引入删除检测、跨表增量
watermark 与部分失败恢复。只有当 staging 的完整物化持续超过 60 秒、锁等待触及告警或数据量显著增长，
才另立设计评审；届时必须先证明 source key、删除 tombstone、replay 和全量 reconciliation，再决定 go。

## 历史课评

`tools/d1/gen_reviews_sql.py` 只是迁移辅助，不等于可直接运行的生产方案。历史课评可能包含旧课程 id、
wallet hash、anonymous edit token、legacy like/report identity。运行前必须审批：

- course id mapping 和不可删除的 review foreign key；
- account attribution，无法证明时保持 nullable/legacy，不伪造平台用户；
- 任何要进入 wallet claim 的 `wallet_user_hash`/legacy link 必须先证明是 64 字符 lowercase hexadecimal
  SHA-256 canonical value；其他格式进入隔离清单，不能放宽在线 claim 输入或静默重编码猜测 owner；
- edit token、PII、report status 和 evidence retention；
- idempotency、冲突、rollback 和验证查询。

## 后台同步的边界

Admin selection sync 会创建持久 job id，按 catalogue/materialize/search/cache 四步记录进度，以
idempotency key 防重复，并使用单 active guard、lease fencing、八次有界退避、dead 状态和带 audit reason
的 retry。长耗时阶段每 60 秒续租，每个破坏性 SQL/Meilisearch 动作前后重验 job id、token 和 expiry，失租
就取消旧 pipeline。管理 UI 可以轮询与安全重试；它仍不替代 D1 export/raw first-load，worker 也仍随 API
进程运行，所以 operator 必须同时核对 job、database/import audit、search index 和 freshness。没有匹配
import run 或 live raw counts/关键关联发生漂移时，catalogue 与 selection materializer 均在首个写操作前
失败，job 不能把空投影或部分投影视为成功。

## 清理

导入完成后删除本地真实快照和临时 stream，清除 shell 中的 token，并确认文件从未进入 Git index。
如果验证失败，停止对目标数据库继续写入；从备份或隔离数据库重建，不用手工修几行掩盖 mapping 问题。

# D1 选课快照导入

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Courses/Selection maintainers、Data migration maintainers
>
> 最近核验：2026-07-15，D1 import tools、selection teaching-class contract 与 `backend/ops/materialize_selection.sql`

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
# 1. 导出 D1 到被 .gitignore 排除的本地 SQLite
python3 tools/d1/d1_export.py

# 2. 原子导入 selection raw tables
python3 tools/d1/d1_import_pg.py --source d1_export.db

# 数据库只能从受控运维 shell 访问时，可输出 COPY stream
python3 tools/d1/d1_import_pg.py --source d1_export.db --emit-copy | psql "$DATABASE_URL"

# 3. 物化 catalogue 与 normalized selection
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/materialize_courses.sql
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/materialize_selection.sql
```

脚本当前导入 13 张 `selection.pk_*` raw table。精确映射和参数以
[`tools/d1/README.md`](../../tools/d1/README.md) 与脚本 `--help` 为准，不在本文复制随代码变化的表清单。

当前 `materialize_selection.sql` 只从 raw timeslot 保留教学班、教师、星期和 1-based inclusive
节次，会把 `weeks` 和 `location` 写为 `NULL`。这是已知映射缺口，不能在验证报告中写成完整课表事实；
字段补齐前，Web/Flutter 会把节次重叠标记为 possible conflict 并要求用户显式覆盖。

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

现有 admin endpoint 返回 `202` 只代表进程内任务已排队。首次 calendar schema cutover 应在不接用户流量的
新 revision 实例上触发，且同一环境只运行一个 selection sync；clear 到 add 完成之间搜索必然有短暂空窗。
Operator 必须看到 settings、clear、add 和 pipeline completion 日志，再用至少两个 calendar 的已知
教学班验证关键词/课号命中及跨学期不串数据，之后才能把新 backend 切入流量。任一 task 失败时停止
cutover；不要把空结果当成合法完成，也不要只重跑 add 掩盖未知 clear/settings 状态，应从完整 sync 重试。

## 验证

- Source export table/row counts 与 raw target counts 一致，记录允许的过滤项。
- Raw 关键主键无 duplicate，必要 foreign key mapping 无 unexpected null；记录教学班 id 在本次快照的唯一性，
  不在没有 lineage 规则时声称上游重编号后仍稳定。
- `courses.courses`、`selection.courses`、teacher/timeslot/major relation 有非零且合理数量。
- 抽查多教师、同 course code 多教学班、跨学期重复 course code、课程性质缺失等 edge cases；
  记录 `weeks`/`location` 空值数量，不把已知全空列当作意外丢数据而手工伪造。
- 选课列表/搜索 API 需要 `calendarId` 且不返回其他学期；详情和节次用 `teachingClassId`，
  同 course code 的平行班不串数据。Meilisearch full reindex 后课号/关键词可按 calendar 过滤；记录
  filterable attributes、完成的 clear/add task 和最终 document count，而不是只记录 enqueue response。
- Materialization rerun 结果稳定，不产生额外 duplicate 或删除人工维护数据。

## 历史课评

`tools/d1/gen_reviews_sql.py` 只是迁移辅助，不等于可直接运行的生产方案。历史课评可能包含旧课程 id、
wallet hash、anonymous edit token、legacy like/report identity。运行前必须审批：

- course id mapping 和不可删除的 review foreign key；
- account attribution，无法证明时保持 nullable/legacy，不伪造平台用户；
- edit token、PII、report status 和 evidence retention；
- idempotency、冲突、rollback 和验证查询。

## 后台同步的边界

当前 admin selection sync 会触发物化、index settings、等待 clear/add 的 full reindex 和缓存版本更新；
任一 Meilisearch task 失败会停止 pipeline，不会提前记录 reindex complete 或更新 cache version。但它仍
没有持久 job status/progress/retry，也不替代 D1 export/raw import，不能作为未知状态恢复的唯一证据。
上线 durable job center 前，operator 需要从 server log、database 和 search 三处验证结果。

## 清理

导入完成后删除本地真实快照和临时 stream，清除 shell 中的 token，并确认文件从未进入 Git index。
如果验证失败，停止对目标数据库继续写入；从备份或隔离数据库重建，不用手工修几行掩盖 mapping 问题。

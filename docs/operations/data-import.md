# D1 选课快照导入

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Courses/Selection maintainers、Data migration maintainers
>
> 最近核验：2026-07-15，migration `0068`、真实 `jcourse-db-backup` 快照与本机 PostgreSQL 16

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
- `reviews.*` 的历史导入需要独立 identity/course mapping、privacy 和 moderation 决策，不属于本流程。

## 安全前提

- 仅在 local、isolated staging 或经过批准的恢复环境运行；不要对未知 main 数据库执行。
- D1 token 只要 read 权限，通过环境变量注入，不提交 `.db`、token、导出日志或真实数据样本。
- 目标 raw tables 必须为空；脚本为防快照叠加会拒绝非空目标。
- 先备份并记录 source snapshot/time、target database、operator、row counts 和 validation result。

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
  --imported-by '<bounded operator label>' \
  --manifest-out /tmp/jcourse-db-backup-manifest.json

# 数据库只能从受控运维 shell 访问时，可输出 COPY stream
python3 tools/d1/d1_import_pg.py \
  --source /tmp/jcourse-db-backup.sqlite3 \
  --source-database jcourse-db-backup \
  --snapshot-exported-at '<RFC3339 export time>' \
  --imported-by '<bounded operator label>' \
  --emit-copy | psql "$DATABASE_URL"

# 3. 物化 catalogue 与 normalized selection
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/materialize_courses.sql
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f backend/ops/materialize_selection.sql
```

`--manifest-out` 是导入前的只读 source manifest；只有事务内成功写入的
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

## 验证

- Source export table/row counts 与 raw target counts 一致，记录允许的过滤项。
- Raw 关键主键无 duplicate，必要 foreign key mapping 无 unexpected null。
- `courses.courses`、`selection.courses`、teacher/timeslot/major relation 有非零且合理数量。
- 抽查多教师、跨学期、课程性质缺失、重复教学班等 edge cases。
- 课程与选课 API 返回预期数据；Meilisearch reindex 后关键词/拼音/课号可检索。
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
- edit token、PII、report status 和 evidence retention；
- idempotency、冲突、rollback 和验证查询。

## 后台同步的边界

Admin selection sync 会创建持久 job id，按 catalogue/materialize/search/cache 四步记录进度，以
idempotency key 防重复，并使用单 active guard、lease fencing、八次有界退避、dead 状态和带 audit reason
的 retry。管理 UI 可以轮询与安全重试；它仍不替代 D1 export/raw first-load，worker 也仍随 API 进程运行，
所以 operator 必须同时核对 job、database/import audit、search index 和 freshness。

## 清理

导入完成后删除本地真实快照和临时 stream，清除 shell 中的 token，并确认文件从未进入 Git index。
如果验证失败，停止对目标数据库继续写入；从备份或隔离数据库重建，不用手工修几行掩盖 mapping 问题。

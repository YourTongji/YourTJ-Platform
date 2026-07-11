# D1 选课快照导入

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Courses/Selection maintainers、Data migration maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

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

## 验证

- Source export table/row counts 与 raw target counts 一致，记录允许的过滤项。
- Raw 关键主键无 duplicate，必要 foreign key mapping 无 unexpected null。
- `courses.courses`、`selection.courses`、teacher/timeslot/major relation 有非零且合理数量。
- 抽查多教师、跨学期、课程性质缺失、重复教学班等 edge cases。
- 课程与选课 API 返回预期数据；Meilisearch reindex 后关键词/拼音/课号可检索。
- Materialization rerun 结果稳定，不产生额外 duplicate 或删除人工维护数据。

## 历史课评

`tools/d1/gen_reviews_sql.py` 只是迁移辅助，不等于可直接运行的生产方案。历史课评可能包含旧课程 id、
wallet hash、anonymous edit token、legacy like/report identity。运行前必须审批：

- course id mapping 和不可删除的 review foreign key；
- account attribution，无法证明时保持 nullable/legacy，不伪造平台用户；
- edit token、PII、report status 和 evidence retention；
- idempotency、冲突、rollback 和验证查询。

## 后台同步的边界

当前 admin selection sync 会触发物化、搜索同步和缓存版本更新，但没有持久 job status/progress/retry。
它不替代 D1 export/raw import，也不能作为未知状态恢复的唯一证据。上线 durable job center 前，operator
需要从 server log、database 和 search 三处验证结果。

## 清理

导入完成后删除本地真实快照和临时 stream，清除 shell 中的 token，并确认文件从未进入 Git index。
如果验证失败，停止对目标数据库继续写入；从备份或隔离数据库重建，不用手工修几行掩盖 mapping 问题。

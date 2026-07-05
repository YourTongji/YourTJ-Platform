# D1 数据导出与本地导入

## 概述

生产数据运行在 Cloudflare D1 上，本地开发调试时需要一份一致的离线副本。
本文档记录：从 D1 导出 → 写入本地 PG 的完整流程、数据映射关系和常见问题。

## 三层数据架构

```
Layer 1: Raw (selection.pk_*)      — 1:1 D1 镜像，原始表结构
                ↕ 物化（SQL 脚本）
Layer 2: Normalized (selection.*)   — 选课标准表，匹配后端 API
                ↕ 物化（SQL 脚本）
Layer 3: Main (courses.*)           — 课程/教师/别名，供 review 前端
                (reviews.*)         — 评价/举报，供 review 前端
```

| 层 | Schema | 数据源 | 用途 |
|----|--------|--------|------|
| Raw | `selection.pk_*` | D1 1:1 | 排课模拟器原始数据，刷新用 |
| Normalized | `selection.*` | 从 pk_* 物化 | `GET /api/v2/selection/*` 后端 API |
| Main | `courses.*` | 从 pk_* 物化 | `GET /api/v2/courses/*` 后端 API |
| Main | `reviews.*` | 从 D1 直接导入 | `GET /api/v2/reviews/*` 后端 API |

## 完整导出流程

### 预备条件

```bash
# 启动本地服务
cd /path/to/YourTJ-Platform
docker compose up -d        # 启动 PostgreSQL + Redis + Meilisearch
# 或
brew services start postgresql@16
```

### 第 1 步：从 D1 导出 SQLite
```bash
python3 tools/d1/d1_export.py
# 输出: d1_export.db (~28 MB, 26 张表, ~200k 行)
```

脚本通过 Cloudflare D1 HTTP API 逐表导出，跳过 `_cf_KV`（Cloudflare 内部表）。
API key 需要 `d1:read` 权限。

### 第 2 步：创建 PG schema

```bash
# Migration 自动被 docker compose 的 entrypoint 执行
# 手动执行（如果已有 PG）：
DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj
for f in backend/migrations/*.sql; do
  psql "$DATABASE_URL" -f "$f" -v ON_ERROR_STOP=1
done
```

关键 migration：

| 文件 | 内容 |
|------|------|
| `0001_init.sql` | 核心 schema：identity, courses, reviews, credit, forum |
| `0002_escrow_selection.sql` | Credit escrow + selection 标准表 |
| `0009_selection_raw_pk.sql` | **Raw PK 镜像表**（selection.pk_*） |
| `0010_selection_raw_normalized.sql` | **缺失列 + 索引** |

### 第 3 步：导入 Raw PK 数据

```bash
python3 tools/d1/d1_import_pg.py

从 `d1_export.db` 读数据 → `INSERT INTO selection.pk_*`。
13 张表，约 141k 行。

### 第 4 步：物化 courses.* & selection.*

```bash
psql "$DATABASE_URL" -f backend/ops/materialize_courses.sql
psql "$DATABASE_URL" -f backend/ops/materialize_selection.sql
```

使用两个 SQL 脚本：

**`materialize_courses.sql`**（物化 `courses.*`）：
- `courses.teachers` — 从 `pk_teachers_raw` 按 name 去重
- `courses.courses` — 从 `pk_course_details` 按 `course_code` 聚合
- `courses.course_aliases` — 从 `course_code`/`code`/`new_course_code`/`new_code` 建别名

**`materialize_selection.sql`**（物化 `selection.*`）：
- `selection.calendars` — 直接映射
- `selection.campuses` — hash 转 id
- `selection.faculties`, `selection.majors` — 同理
- `selection.course_natures` — 合并 `pk_course_natures + pk_course_natures_by_calendar`
- `selection.courses` — 从 `pk_course_details` 展开（含 `DISTINCT ON` 去重）
- `selection.major_courses` — 从 `pk_major_courses` 映射
- `selection.timeslots` — 从 `pk_teacher_timeslots` 展开（含 `DISTINCT ON` 去重）

### 第 5 步：导入 Reviews 数据

```bash
python3 tools/d1/gen_reviews_sql.py | psql "$DATABASE_URL" -f -

由于 `reviews.reviews.id` 是 `GENERATED ALWAYS AS IDENTITY`，需 `OVERRIDING SYSTEM VALUE`。
由于 `account_id` 是 FK 指向 `identity.accounts`（D1 没有此表），导入时填 `NULL`。

## 数据映射对照

### D1 → Raw PK (1:1)

| D1 表 | PG Raw 表 | 说明 |
|-------|-----------|------|
| `calendar` | `selection.pk_calendars` | 学期历 |
| `language` | `selection.pk_languages` | 教学语言 |
| `coursenature` | `selection.pk_course_natures` | 课程性质 |
| `coursenature_by_calendar` | `selection.pk_course_natures_by_calendar` | 按学期课程性质 |
| `assessment` | `selection.pk_assessments` | 考核方式 |
| `campus` | `selection.pk_campuses` | 校区 |
| `faculty` | `selection.pk_faculties` | 开课学院 |
| `major` | `selection.pk_majors` | 专业 |
| `coursedetail` | `selection.pk_course_details` | 教学班明细 |
| `teacher` | `selection.pk_teachers_raw` | 教师分配 |
| `teacher_timeslots` | `selection.pk_teacher_timeslots` | 排课时间 |
| `majorandcourse` | `selection.pk_major_courses` | 专业-课程绑定 |
| `fetchlog` | `selection.pk_fetch_logs` | 抓取日志 |
| `categories` | `public.categories` | 课程类别 |
| `reviews` | `reviews.reviews` | 评价 |
| `review_likes` | `reviews.review_likes` | 评价点赞（暂未导入） |
| `review_reports` | `reviews.review_reports` | 评价举报（暂未完整导入） |
| `ai_summaries` | 暂无 | AI 摘要 |
| `settings` | 暂无 | 配置项 |
| `_cf_KV` | 无 | Cloudflare 内部，跳过 |

### Raw PK → Normalized (selection.*)

Raw PG 表与 selection 标准表不是 1:1：

- `pk_course_details` 多行（同一 `course_code` 有多个教学班）→ `selection.courses` 也用多行（每个教学班一行）
- `pk_teachers_raw` 多行同 `teaching_class_id`（一个教学班多位老师）→ `selection.courses.teacher_name` `DISTINCT ON (cd.id)` 取第一个
- `pk_course_natures_by_calendar` 补充 `course_natures` 缺失的 id

### 特殊处理

1. **PK 冲突**：`pk_course_details` LEFT JOIN `pk_teachers_raw` 会产生重复行（1:N）。解决方案是 `DISTINCT ON (cd.id)`。
2. **FK 确认**：`selection.courses` 的 `nature_id` 引用 `course_natures`，但 Raw 的 `courseLabelId` 可能只在 `coursenature_by_calendar` 中存在。需先合并。
3. **FK 绕过**：`reviews.reviews.account_id` 引用 `identity.accounts(id)`，本地无此数据，导入时填 `NULL`。
4. **identity 列**：`reviews.reviews.id` 是 `GENERATED ALWAYS`，需 `OVERRIDING SYSTEM VALUE` 或临时改为 `BY DEFAULT`。

## Appendix: 快速重建命令

### 决策记录

5. **`public.categories`**：D1 的 `categories` 表当前导入到 `public.categories`（无主 schema）。
   归属未定（大概率 courses 域）。后续迁移应将其移到 `courses.categories`，导入脚本同步调整。
   在此之前，`public.categories` 只作历史参考，不作为任何 API 数据源。
6. **`edit_token`**：D1 的无账号匿名编辑凭证已导入（`reviews.reviews.edit_token`），
   但 v2 后端**不**实现 `edit_token` 编辑功能 — 与 v2 账号体系冲突。列保留作历史参考。
```bash
# 1. 清理所有物化数据
psql "$DATABASE_URL" <<'EOSQL'
TRUNCATE selection.timeslots CASCADE;
TRUNCATE selection.major_courses CASCADE;
TRUNCATE selection.courses CASCADE;
TRUNCATE selection.calendars CASCADE;
TRUNCATE selection.campuses CASCADE;
TRUNCATE selection.faculties CASCADE;
TRUNCATE selection.majors CASCADE;
TRUNCATE selection.course_natures CASCADE;
TRUNCATE selection.fetchlog CASCADE;
TRUNCATE courses.course_aliases CASCADE;
TRUNCATE courses.courses CASCADE;
TRUNCATE courses.teachers CASCADE;
TRUNCATE reviews.review_reports CASCADE;
TRUNCATE reviews.review_likes CASCADE;
TRUNCATE reviews.reviews CASCADE;
EOSQL
# 2. 重新导入 Raw PK（如果已经清空）
python3 tools/d1/d1_import_pg.py

# 3. 物化 courses.*
psql "$DATABASE_URL" -f backend/ops/materialize_courses.sql

# 4. 物化 selection.*
psql "$DATABASE_URL" -f backend/ops/materialize_selection.sql

# 5. 导入 reviews
python3 tools/d1/gen_reviews_sql.py | psql "$DATABASE_URL" -f -
```

# 架构反思（基于 D1 导入实践）与端到端测试方案

> **Status:** Historical architecture review and proposed E2E plan; not a current implementation inventory
>
> **Owner:** Platform maintainers
>
> **Last classified:** 2026-07-11; verify every checklist item against current source and CI before use
>
> **Authoritative sources:** `AGENTS.md`, `contract/openapi.yaml`, numbered migrations, and current CI

> 触发点：[D1_LOCAL_IMPORT.md](D1_LOCAL_IMPORT.md) 记录了"生产 D1 → 本地 PG"的
> 完整导入流程。这次实践第一次把**真实生产数据**灌进了 v2 后端，暴露出的问题
> 比任何代码评审都真实。本文分两部分：
> §1–§4 重新审视架构并给出调整清单；§5–§9 给出验证"后端真的能 work"的
> 端到端测试完整方案。

---

## 1. D1 导入实践揭示了什么

设计文档假设的世界：v2 后端 + PolarDB 是权威数据源，选课数据由"同步任务"从
教务系统获取。**实际的世界**：

1. **生产数据的权威源今天在 Cloudflare D1 上**（老系统 YourTJCourse-Serverless
   持续抓教务写 D1）。v2 在 strangler 切流完成前，永远是 D1 的**下游**。
2. 数据形状由 D1 决定，且与 v2 schema 有真实落差：老点评没有 `account_id`
   （只有 `reviewer_name` / `wallet_user_hash`）、一个教学班多位教师、
   `courseLabelId` 跨表不一致 —— 这些都不是理论问题，是导入时踩到的。
3. 导入管道目前是**仓库外的手工流程**：Python 脚本（`d1_export.py` /
   `d1_import_pg.py` / `gen_reviews_sql.py`）不在版本控制里，物化 SQL 只进了一半
   （`materialize_courses.sql` 缺失），文档里引用的 `materialize.sql` 路径与实际
   文件名不符。今天离开这台机器，流程不可复现。

**总体判断：三层架构（Raw → Normalized → Main）本身是正确且值得升格的决定** ——
它就是标准的 ELT staging 模式：Raw 层 1:1 镜像上游、可整表刷新；Normalized/Main
层由幂等物化脚本推导。这解决了设计文档里一直悬空的"选课数据从哪来"问题，
而且答案（从 D1 拉，而不是重写教务爬虫）比原计划更现实。
需要调整的不是这个方向，而是它的**工程化程度**和暴露出的若干边界破损。

---

## 2. 必须立即修的（P0 — 正确性/安全）

### 2.1 历史点评当前全部不可见（真 bug）

`reviews.reviews.account_id` 本就是 NULLable（0001 的 DDL 如此，导入按此填了
NULL），但 `reviews/src/repo.rs` 的四处列表/详情查询全是
`JOIN identity.accounts a ON a.id = r.account_id`（INNER JOIN）——
**导入的约两万条历史点评一条都查不出来**。这正是端到端验证的价值：单元测试全绿，
因为测试数据永远有 account。

修法（0010 已加好 `reviewer_name` / `reviewer_avatar` 列，正好配套）：

```sql
LEFT JOIN identity.accounts a ON a.id = r.account_id
-- DTO: authorHandle = COALESCE(a.handle, r.reviewer_name, '匿名用户')
--      authorAvatar = COALESCE(a.avatar_url, NULLIF(r.reviewer_avatar, ''))
```

同时写路径 guard：`account_id IS NULL` 的点评不可被"本人编辑"、点赞 mint 的
受益人判定跳过 NULL 作者。

### 2.2 `materialize_selection.sql` 放错目录（会被自动执行）

它躺在 `backend/migrations/` 里，而这个目录有两个自动执行入口：
- `run.sh` glob `*.sql` 全量执行；
- `docker-compose.yml` 把整个目录挂到 `/docker-entrypoint-initdb.d`，Postgres
  initdb 按字母序执行所有 `.sql` **和 `.sh`**。

后果：每次全新起库都会在空 Raw 表上跑一遍物化（现在恰好无害，但它内含
`DELETE FROM selection.timeslots` 等清表语句，将来任何人在里面加一行就是事故）；
且 initdb 执行 `run.sh` 时因 `DATABASE_URL` 未设置会 `exit 1`，可能中断初始化。

**调整**：新建 `backend/ops/` 目录，物化脚本（selection + 待补的 courses 版）
移进去；`migrations/` 恢复"只有编号迁移"的纯净约定；`run.sh` 移到
`backend/scripts/` 或在 compose 挂载时排除。

### 2.3 数据与工具的仓库卫生（含 PIPL 风险）

- `d1_export.db`（40MB，含真实用户的 reviewer_name / wallet_user_hash /邮箱痕迹）
  躺在仓库根且**未被 .gitignore 覆盖** —— 一次 `git add -A` 就把生产 PII 提交进
  git 历史。立即加 ignore（`*.db`、`__pycache__/`），文件移出仓库目录。
- 三个 Python 脚本纳管进 `tools/d1/`（含 requirements 与 README），API key 走环境
  变量；`gen_reviews_sql.py` 缺失的 `materialize_courses.sql` 一并补齐入库。
- 0009/0010 两个迁移目前未提交 —— 它们是可复现性的地基，随本次调整一起进版本控制。

---

## 3. 结构性调整（P1 — 把管道变成产品的一部分）

### 3.1 同步管道收编进后端（替代手工五步）

现状是"文档 + 手工 psql"，正确形态是 `POST /api/v2/admin/selection/sync`
从 stub 变实，内部四步、可重复执行：

```text
1. pull   — 后端经 Cloudflare D1 HTTP API 逐表拉取（复刻 d1_export.py 逻辑，
            reqwest 实现；D1 read-only key 走 secret）
2. stage  — 事务内 TRUNCATE selection.pk_* → 批量 INSERT（COPY 语义）
3. mat    — 依次执行 ops/materialize_{courses,selection}.sql（include_str! 编译进
            二进制，整体一个事务，幂等）
4. post   — 刷 Meilisearch selection/courses 索引、bump 缓存版本、写 fetchlog
```

- 端点立即返回 202 + jobId，任务在受监督的 tokio task 里跑（与热榜 job 同一
  个调度基建）；`GET /admin/selection/sync/{jobId}` 查进度。
- Python 脚本降级为**逃生舱**（本地调试/后端不可用时的手动路径），保留在
  `tools/d1/`，但文档以后端端点为主路径。
- 这同时兑现了 F0 清单里"选课数据导入 job"和"admin sync 去 stub"两项。

### 3.2 物化脚本的确定性缺陷

当前 `materialize_selection.sql` 有两处会导致**重跑不稳定**：

1. `campuses` / `faculties` 用 `ROW_NUMBER() OVER (ORDER BY name)` 造 id +
   `ON CONFLICT DO NOTHING`：上游增删一个校区，后续所有 id 移位，与已发出去的
   客户端缓存/外键引用错位。**改为 natural-key upsert**（表加 `UNIQUE(name)`，
   id 只在首插时生成，冲突按 name 匹配更新）。
2. `is_current = (MAX(calendar_id))`：假设学期 id 单调递增。D1 侧历史上成立，
   但应改为读 `pk_fetch_logs` 最近抓取的学期，或 admin 显式设置 —— 至少在
   物化脚本里注释这个假设。

另有一处**已知数据丢失**要记录成显式决策：一个教学班多位教师时
`DISTINCT ON` 只取第一个（`selection.courses.teacher_name`）。短期接受（老系统
同样如此），中期给 `selection.courses` 加 `teacher_names TEXT[]`，物化时
`array_agg`，API DTO 加 `teacherNames` 字段。

### 3.3 legacy 身份的打通（把 NULL 变成资产）

0010 引入的 `wallet_user_hash` 与 identity 域已有的
`legacy_wallet_links.legacy_user_hash` 是同一个东西。顺势打通：

- 用户完成 `/wallet/claim`（老钱包认领）时，**同事务**执行
  `UPDATE reviews.reviews SET account_id = $me WHERE wallet_user_hash = $hash
  AND account_id IS NULL` —— 历史点评自动归位到新账号，编辑权、点赞收益随之恢复。
- 这一步跨域（identity → reviews），按边界规则走 reviews crate 公开 API
  `reviews::claim_legacy_reviews(tx, account_id, user_hash)`。

### 3.4 域边界的两处破损

- D1 的 `categories` 表被导入到 **`public.categories`** —— 无主 schema。判定归属
  （大概率 courses 域）或明确不导；`public` 里不允许落业务表。
- `edit_token`（D1 的无账号编辑凭证）导入了但 v2 不应支持这个机制——匿名可编辑
  与 v2 的账号体系冲突。列保留作历史参考，接口层面**不**实现 edit_token 编辑。

### 3.5 一个不调整的决策，记录在案

D1 实践可能诱发"既然生产在 Cloudflare，v2 干脆留在 Workers 栈"的想法。**不采纳**，
理由重申：ICP/PIPL 要求数据境内（D1 无境内区域）、积分账本需要 advisory lock +
事务性（D1/SQLite 弱）、Rust 后端与 5 个域已建成。D1 的正确定位就是 §3.1：
**切流过渡期的上游数据源**，切流完成（教务抓取器改写 PG）后 Raw 层的刷新来源
换成抓取器，三层架构本身不变。

---

## 4. 调整清单汇总

| # | 事项 | 级别 | 落点 |
|---|---|---|---|
| 1 | reviews 查询 INNER→LEFT JOIN + legacy 作者回退显示 | P0 bug | reviews crate |
| 2 | 物化脚本移出 migrations/ → `backend/ops/`；补 materialize_courses.sql | P0 | 仓库结构 |
| 3 | `.gitignore` 补 `*.db`/`__pycache__`；d1_export.db 出仓库 | P0 安全 | 根目录 |
| 4 | Python 工具入库 `tools/d1/`；0009/0010 提交 | P0 可复现 | 仓库结构 |
| 5 | admin selection sync 去 stub：pull→stage→mat→post 全链 | P1 | api/courses crate |
| 6 | campuses/faculties natural-key upsert；is_current 判定加固 | P1 | ops SQL |
| 7 | wallet claim 联动认领历史点评 | P1 | identity+reviews |
| 8 | public.categories 归属；edit_token 只存不用的决策注释 | P2 | migrations/文档 |
| 9 | 多教师 `teacher_names TEXT[]` | P2 | selection |

---

## 5. 端到端测试：总体设计

### 5.1 为什么现有测试不够

现有金字塔：单元测试（crate 内）+ 集成测试（testcontainers，每 crate 起独立
PG/Redis）。它们验证"每个域各自正确"，但 §2.1 的事故说明了盲区：
**真实数据形状**（NULL account_id）、**跨域接线**（通知钩子、mint、meili 同步）、
**进程级装配**（bootstrap 调度、中间件顺序）只有整机跑起来才暴露。

E2E 的目标命题：**用一个全新 clone + 一条命令，起完整栈、灌真实形状的数据、
以纯 HTTP 客户端身份走完所有用户旅程，并对数据库不变量做终态对账。**

### 5.2 环境拓扑

```text
docker compose --profile e2e up:
  postgres   (migrations 目录只含编号迁移 —— §2.2 修复后)
  redis
  meilisearch
  api        (backend/Dockerfile 构建的同一镜像，APP_ENV=e2e)
driver:
  crates/e2e (Rust bin：reqwest + tokio，直接复用各域 DTO 类型做反序列化断言)
  schemathesis (容器，跑契约层)
```

`APP_ENV=e2e` 时后端启用**测试后门**（编译期不裁剪、运行期按 env 门控，
生产环境该值非法即拒绝启动）：

| 后门 | 用途 |
|---|---|
| `POST /__test__/email-code/peek {email}` | 取回验证码明文（SMTP 旁路，解锁登录旅程） |
| `POST /__test__/mint {accountId, amount}` | 给测试账号注资（系统签名走正常 ledger，不破坏链） |
| 固定 `CREDIT_SYSTEM_PRIVATE_KEY` / `JWT_SECRET` 测试值 | 断言可复现 |

除这三项外**不加任何后门**——时间推进、状态构造一律走真实 API 或直连 SQL 断言。

### 5.3 数据集（三档）

| 档 | 内容 | 用途 |
|---|---|---|
| `fixtures/synthetic.sql` | 手工精选：3 板块 / 5 课程 / 若干账号 | PR 级快速套件 |
| `fixtures/d1_sample.sql` | 从 d1_export.db 抽样脱敏：~500 课程、~2000 条点评（**保留 NULL account_id、多教师、别名冲突等真实形状**），reviewer_name 假名化后 ~2MB 入库 | nightly 全量套件 |
| 全量 d1_export.db | 本地手动（按 D1_LOCAL_IMPORT.md 流程） | 发布前人工验收 |

抽样脚本 `tools/d1/make_sample.py` 与导入工具同目录维护——抽样规则本身要保证
"每种脏形状至少留 N 条"。

---

## 6. 测试套件（S1–S8）

### S1 契约一致性（自动生成，全端点覆盖）

schemathesis 读 `contract/openapi.yaml`，对全部路径做 property-based 探测：
- 响应状态码 ∈ 契约声明集合；响应体符合 schema（camelCase、错误信封）。
- 未认证访问受保护端点 → 一律 401 信封，绝无 500。
- 畸形输入（超长、负数、非法枚举）→ 4xx，绝无 500/DB 报错泄漏。

这层的价值是**兜住"契约与实现漂移"**——之前发现的 `/credit/bounty` 在契约却无
实现这类问题，S1 会天然抓住。

### S2 身份旅程

```text
request-code(非同济邮箱→400) → request-code(合法) → 限流(60s 内重发→429)
→ peek code → verify(错码 5 次→锁) → verify(对码)={tokens, account}
→ GET /me → PATCH /me(handle 冲突→409) → refresh(旧 refresh 复用→401)
→ wallet/bind(非法公钥→400；合法→204) → logout → 旧 access 过期语义验证
```

### S3 选课数据不变量（针对导入正确性，纯 SQL + API 双面断言）

导入/同步完成后，driver 直连 PG 跑对账清单，再从 API 侧抽查一致：

| # | 不变量 |
|---|---|
| 1 | `count(selection.courses)` = `count(DISTINCT id) from pk_course_details`（去重后教学班数） |
| 2 | 每个 `selection.courses.nature_id` 在 `course_natures` 存在（0 孤儿 FK） |
| 3 | `timeslots.weekday ∈ 1..7`、`start_slot ≤ end_slot`，0 越界行 |
| 4 | `courses.courses.code` 无重复；每个 code 至少 1 条 alias |
| 5 | `selection.majors` 无空名（TRIM 过滤生效） |
| 6 | API `GET /selection/courses/by-code/{任取10个code}` 与 pk 表行内容逐字段一致 |
| 7 | `GET /selection/latest-update` 与 fetchlog 最新时间一致 |
| 8 | 物化脚本**连跑两遍**，所有计数不变（幂等性回归） |

### S4 评课旅程（含 §2.1 回归）

```text
[legacy 可见性] 课程 X 含 account_id=NULL 的导入点评
  → GET /courses/X/reviews 必须返回它们，authorHandle=reviewer_name  ← 回归锚点
[发布] 登录 → POST review(带 Idempotency-Key) → 201；同 Key 重放 → 同一 id 不重复
  → 无 Key 同课程重复发 → 409 → rating=6 → 400 → 限流第 6 次/60s → 429
[聚合] 发布后 course.review_count/avg 增量值 == SQL 全量重算值
[点赞] like → 204 → 重复 like 幂等 → unlike → 计数归零；作者收到 vote 通知
[举报] report → admin/reports 队列出现 → resolve(uphold) → 点评隐藏 → 列表不可见
[编辑] 本人 patch → 200；他人 patch → 403；legacy(NULL author) patch → 403
```

### S5 论坛旅程（验证 F0/F1 接线）

```text
[发帖] TL0 新账号连发 3 主题 → 第 3 次 429（新手限额）
  → 帖含敏感词(block) → 400；含 queue 词 → 发布成功但 hidden 待审
[楼中楼] A 发帖，B 回复，C 回复 B → path 正确嵌套；B、A 各收到 reply 通知
  → 并发 10 评论同一楼 → path 无重复（唯一索引不炸）
[投票] B vote ↑ → thread.vote_count=1 → 改 ↓ → =-1 → 一人一票不叠加
[订阅] A watching 板块 → 他人板内发帖 → A 收 watching 通知；muted 后不再收
[已读] 读到 5 楼上报 → unread feed 反映余量；上报 3 楼（回退）→ 被拒（只前移）
[flag] 3 个 TL1 账号 flag 同一帖(权重和 3.0) → 自动 hidden + 作者收通知
  → mod uphold → 软删 + mod_actions 留痕 + 作者 flagged_upheld+1
[禁言] admin silence B → B 发帖/评论/vote 全 403；到期(endsAt=+2s)后恢复
[热榜] 新高互动帖 → ≤5min 出现在 feed=hot（调度真跑了）
```

### S6 积分高保证（treat as money）

```text
[初始] 新账号 balance=0 → 后门 mint 100（走真实 ledger）
[tip] A tip B 10 于 B 的帖子：正确 X-Wallet-Sig → 200，A=90 B=110
  → 错误签名 → 401；同 nonce 重放 → 拒绝；余额不足 tip 1000 → 402
  → **并发竞态**：A 同时发 20 笔 tip 各 10 → 恰好 9 笔成功 1 笔 402（无超扣、无死锁）
[escrow] 发任务(hold 50) → balance=40 且 hold 可见 → 接受 → submit → confirm
  → 赏金到账，escrow 清零；另开一单走 cancel → 全额返还
[合规红线] 断言不存在 recharge/withdraw/transfer 路由（openapi + 路由表双查）
[账本] 全旅程结束 → GET /wallet/ledger/verify → ok=true
  → driver 直连 SQL 篡改任意一行 amount → verify → ok=false 且指认 seq
  → 每个账号 balance == 全量 ledger 派生重算
[mint 钩子] B 的帖子净赞到 10 → 自动 mint 一次；再 +10 赞不重复 mint（幂等键）
```

### S7 搜索

```text
写路径同步：新发点评/帖子 → ≤10s 内 /search 可命中（轮询上限断言）
中文形状（用 d1_sample 真数据）：拼音 "gaoshu"、简称 "毛概"、课号前缀 → 首页命中
删除同步：软删的帖子 → 索引内消失
降级：停掉 meili 容器 → /search 返回优雅错误信封，其余端点不受影响 ← 韧性锚点
```

### S8 终态全局对账（每次全量套件收尾必跑）

| 断言 | 左边 | 右边 |
|---|---|---|
| 计数一致 | 各表 `*_count` 列 | SQL 全量重算 |
| 余额一致 | `credit.wallets.balance` | ledger 派生 |
| 链完整 | `/wallet/ledger/verify` | ok |
| 索引一致 | meili 文档数 | PG 可见行数 |
| 通知守恒 | notifications 行数 | 旅程期望产生数（防重复/丢失） |
| 无孤儿 | 全部跨域"软外键"（votes.post_id 等） | 0 孤儿 |

---

## 7. 执行编排

| 场景 | 套件 | 数据 | 时长预算 |
|---|---|---|---|
| 每 PR（CI） | S1 + S2 + S4 + S6(核心断言) | synthetic | ≤ 8 min |
| nightly（CI） | S1–S8 全量 | d1_sample | ≤ 30 min |
| 发布前（人工） | 全量 + §8 冒烟压测 | 全量 D1 导入 | 半天 |

- 本地一条命令：`make e2e`（compose up → 等 healthcheck → 灌 fixture →
  `cargo run -p e2e` → 汇总报告 → compose down）。
- CI 失败工件：api 容器日志、pg_dump、失败请求的 request/response 转储。
- driver 内每个 journey 独立账号命名空间（`e2e-s4-{run_id}@tongji.edu.cn`），
  套件间无共享可变状态，允许并行。

## 8. 性能冒烟（非基准，只设护栏）

用 `oha` 对三个最热端点各打 30s（nightly 数据集）：

| 端点 | 护栏 |
|---|---|
| `GET /forum/threads?feed=hot` | p95 < 100ms（走 Redis/边缘缓存路径） |
| `GET /courses/{id}/reviews` | p95 < 150ms |
| `GET /search?q=gaoshu` | p95 < 200ms |

超护栏不阻塞合并，只报警 —— 当前阶段"正确 > 快"，但要能看见劣化趋势。

## 9. 落地顺序

1. **先修 §2 的四个 P0**（否则 E2E 建立在会塌的地基上；#1 本身就是 S4 的回归锚点）。
2. 搭 `crates/e2e` 骨架 + compose e2e profile + 测试后门 → 跑通 S2（最短闭环，
   顺带逼出 SMTP 旁路设计）。
3. `tools/d1/make_sample.py` + fixture 入库 → S3/S4 上线（导入正确性从此可回归）。
4. S5/S6/S7 随 F0/F1 接线完成逐个点亮 —— **每接一条线，先写它的 E2E 再接**。
5. S1 契约层与 S8 对账收尾，接入 nightly。

> 维护约定：新增任何 HTTP 端点，PR 必须同时更新 openapi.yaml（既有 DoD）**并**
> 在对应套件加至少一条 happy-path + 一条拒绝路径断言；S8 的对账清单随新计数器/
> 新跨域引用同步扩充。E2E 不追求行覆盖率，追求**旅程覆盖率**：每个用户可感知的
> 功能，至少有一条从 HTTP 进、从数据库不变量出的完整验证。

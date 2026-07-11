# YourTJ v2 后端设计草案（A）

> **Status:** Historical architecture baseline; non-authoritative for current API, schema, governance,
> or implementation status.
>
> **Owner:** Platform maintainers
>
> **Last verified:** Partially reconciled through 2026-06 remediation notes; not a current inventory
>
> **Authoritative sources:** [`docs/README.md`](README.md), `contract/openapi.yaml`, numbered migrations,
> and current normative product/security documents
>
> 本文保留早期架构决策和演进背景。完整 DDL、示例路径、阶段状态可能已经过期；当前社区治理、
> 活跃度、资料/私信与权限规范请从 [`docs/README.md`](README.md) 进入。

> 目标架构：阿里云华东 · SAE 无状态容器（serverless）· PolarDB（A 单实例 / B 主从读写分离）· Meilisearch 搜索 · Redis 缓存/计数/限流/热榜 · OSS+CDN 媒体。
> 身份：校园邮箱验证码注册 + JWT/refresh；账号绑 Ed25519，仅资金操作签名。
> 本文覆盖：①API 约定 ②v2 OpenAPI 草案 ③PolarDB DDL ④搜索索引设计 ⑤缓存与失效策略。

---

## 0. API 约定（v2 conventions）

| 项 | 规则 |
|---|---|
| Base | `/api/v2` |
| 鉴权 | `Authorization: Bearer <access_jwt>`（15min）；refresh 走 `/auth/refresh` |
| 资金签名 | 资金/escrow 用户发起写操作先 `POST /api/v2/credit/signing-intents` 取得服务端规范化 `signingBytes`；提交写操作时必须同时带 `X-Wallet-Intent`、`X-Wallet-Sig`、同一个 `Idempotency-Key`。意图绑定 account/key/action/request/snapshot/TTL，并在同一事务中一次性消费。|
| 分页 | 游标优先：`?cursor=<opaque>&limit=20` → `{ items, next_cursor, has_more }`；管理后台允许 `page/limit` |
| 幂等 | 写操作支持 `Idempotency-Key` 头（发帖/转账/点赞防重放）|
| 错误 | `{ "error": { "code": "REVIEW_DUPLICATE", "message": "...", "details": {} } }`，HTTP 状态 + 稳定 code |
| 限流 | 响应带 `X-RateLimit-Limit/Remaining/Reset`；写接口 Redis 令牌桶 |
| 时间 | 全部 Unix 秒（int64），UTC；展示层本地化 |
| 防滥用 | 注册、发布、举报走 Captcha（Turnstile/TongjiCaptcha）+ 限流 |

---

## 1. v2 OpenAPI 草案（约定 + 代表性示例）

> **完整契约以 [`contract/openapi.yaml`](../contract/openapi.yaml) 为单一权威源**。本节只保留历史约定与代表性示例；路径、操作和 schema 数量不在文档中手工维护，改接口请先改契约。

```yaml
openapi: 3.1.0
info: { title: YourTJ v2 API, version: 2.0.0 }
servers: [{ url: https://api.yourtj.de/api/v2 }]

components:
  securitySchemes:
    bearer: { type: http, scheme: bearer, bearerFormat: JWT }
  schemas:
    Error:
      type: object
      properties:
        error:
          type: object
          required: [code, message]
          properties: { code: {type: string}, message: {type: string}, details: {type: object} }
    Account:
      type: object
      properties:
        id: {type: string}
        handle: {type: string}
        avatarUrl: {type: string, nullable: true}
        role: {type: string, enum: [user, mod, admin]}
        createdAt: {type: integer}
    Page:
      type: object
      properties:
        items: {type: array, items: {}}
        nextCursor: {type: string, nullable: true}
        hasMore: {type: boolean}

paths:
  # ---------- identity ----------
  /auth/email/request-code:
    post:
      summary: 发送邮箱验证码（@tongji.edu.cn）
      requestBody: { content: { application/json: { schema:
        { type: object, required: [email], properties: { email: {type: string, format: email} } } } } }
      responses: { '204': {description: sent}, '429': {description: rate limited} }
  /auth/email/verify:
    post:
      summary: 校验验证码并登录/注册
      requestBody: { content: { application/json: { schema:
        { type: object, required: [email, code], properties:
          { email: {type: string}, code: {type: string}, handle: {type: string, nullable: true} } } } } }
      responses:
        '200': { content: { application/json: { schema: { type: object, properties:
          { accessToken: {type: string}, refreshToken: {type: string},
            account: {$ref: '#/components/schemas/Account'} } } } } }
  /auth/refresh:   { post: { summary: 刷新令牌, responses: { '200': {description: ok} } } }
  /auth/logout:    { post: { security: [{bearer: []}], summary: 注销当前会话, responses: { '204': {} } } }
  /me:
    get:   { security: [{bearer: []}], summary: 当前账号, responses: { '200': {description: ok} } }
    patch: { security: [{bearer: []}], summary: 改昵称/头像, responses: { '200': {description: ok} } }

  # ---------- wallet（绑定 / 老钱包认领）----------
  /wallet:
    get: { security: [{bearer: []}], summary: 余额, responses: { '200': {description: ok} } }
  /wallet/bind:
    post:
      security: [{bearer: []}]
      summary: 绑定客户端生成的 Ed25519 公钥
      requestBody: { content: { application/json: { schema:
        { type: object, required: [publicKey], properties: { publicKey: {type: string} } } } } }
      responses: { '204': {} }
  /wallet/claim-challenge:
    get: { security: [{bearer: []}], summary: 取认领挑战串, responses: { '200': {description: '{challengeId, nonce}'} } }
  /wallet/claim:
    post:
      security: [{bearer: []}]
      summary: 认领旧钱包（重输学号+PIN 本地重算 userHash 对挑战签名）
      requestBody: { content: { application/json: { schema:
        { type: object, required: [legacyUserHash, challengeId, signature],
          properties: { legacyUserHash: {type: string}, challengeId: {type: string}, signature: {type: string} } } } } }
      responses: { '200': {description: 'merged balance & history'}, '409': {description: already claimed} }
  # 受控资金操作（无自由转账，见 §6.3）；均需 X-Wallet-Sig
  /credit/tip:
    post:
      security: [{bearer: []}]
      summary: 打赏（受控转移，绑定到具体内容）
      parameters: [{name: X-Wallet-Sig, in: header, required: true, schema: {type: string}}]
      requestBody: { content: { application/json: { schema:
        { type: object, required: [toAccountId, amount, targetType, targetId],
          properties: { toAccountId: {type: string}, amount: {type: integer},
            targetType: {type: string, enum: [review, thread, comment]}, targetId: {type: string} } } } } }
      responses: { '200': {description: ok}, '402': {description: insufficient} }
  /credit/bounty:
    post:
      security: [{bearer: []}]
      summary: 发布悬赏（积分托管 escrow_hold）
      parameters: [{name: X-Wallet-Sig, in: header, required: true, schema: {type: string}}]
      responses: { '201': {description: created}, '402': {description: insufficient} }
  /credit/bounty/{id}/confirm:
    post:
      security: [{bearer: []}]
      summary: 确认完成放款（escrow_release）
      parameters: [{name: X-Wallet-Sig, in: header, required: true, schema: {type: string}}]
      responses: { '200': {description: released} }

  # ---------- search（实时模糊，Meilisearch 支撑）----------
  /search:
    get:
      summary: 实时搜索（课程/老师/地点/简称/拼音/别名/点评内容）
      parameters:
        - { name: q, in: query, required: true, schema: {type: string} }
        - { name: type, in: query, schema: {type: string, enum: [course, teacher, review, all], default: all} }
        - { name: limit, in: query, schema: {type: integer, default: 10, maximum: 30} }
      responses: { '200': { content: { application/json: { schema:
        { type: object, properties: { courses: {type: array, items: {}}, reviews: {type: array, items: {}} } } } } } }

  # ---------- courses ----------
  /departments: { get: { summary: 院系列表, responses: { '200': {description: ok} } } }
  /courses:
    get:
      summary: 课程浏览/筛选（非实时搜索走这里，带缓存）
      parameters:
        - { name: dept, in: query, schema: {type: string} }
        - { name: sort, in: query, schema: {type: string, enum: [hot, rating, new]} }
        - { name: cursor, in: query, schema: {type: string} }
        - { name: limit, in: query, schema: {type: integer, default: 20} }
      responses: { '200': { content: { application/json: { schema: {$ref: '#/components/schemas/Page'} } } } }
  /courses/{id}:        { get: { summary: 课程详情, responses: { '200': {description: ok} } } }
  /courses/{id}/related:{ get: { summary: 相关课程, responses: { '200': {description: ok} } } }
  /courses/by-code/{code}: { get: { summary: 按课号查, responses: { '200': {description: ok} } } }

  # ---------- reviews ----------
  /courses/{id}/reviews:
    get:
      summary: 课程点评列表（分页 + 排序）
      parameters:
        - { name: sort, in: query, schema: {type: string, enum: [hot, new], default: hot} }
        - { name: cursor, in: query, schema: {type: string} }
      responses: { '200': { content: { application/json: { schema: {$ref: '#/components/schemas/Page'} } } } }
    post:
      security: [{bearer: []}]
      summary: 发布点评（Captcha + 幂等）
      parameters: [{name: Idempotency-Key, in: header, schema: {type: string}}]
      requestBody: { content: { application/json: { schema:
        { type: object, required: [rating], properties:
          { rating: {type: integer, minimum: 0, maximum: 5}, comment: {type: string},
            semester: {type: string}, score: {type: string} } } } } }
      responses: { '201': {description: created}, '409': {description: duplicate} }
  /reviews/{id}:
    patch: { security: [{bearer: []}], summary: 编辑本人点评, responses: { '200': {description: ok} } }
  /reviews/{id}/like:
    post:   { security: [{bearer: []}], summary: 点赞, responses: { '204': {} } }
    delete: { security: [{bearer: []}], summary: 取消点赞, responses: { '204': {} } }
  /reviews/{id}/report:
    post:
      security: [{bearer: []}]
      summary: 举报
      requestBody: { content: { application/json: { schema:
        { type: object, required: [reason], properties: { reason: {type: string} } } } } }
      responses: { '202': {description: queued} }

  # ---------- forum（Phase B，先占位契约）----------
  /forum/boards:                { get:  { summary: 板块列表 } }
  /forum/threads:               { get:  { summary: 主题流(hot/new/following) }, post: { security: [{bearer: []}], summary: 发帖 } }
  /forum/threads/{id}:          { get:  { summary: 主题详情 } }
  /forum/threads/{id}/comments: { get:  { summary: 楼层(支持楼中楼) }, post: { security: [{bearer: []}], summary: 评论 } }
  /forum/posts/{id}/vote:       { post: { security: [{bearer: []}], summary: 顶/踩 } }
  /notifications:               { get:  { security: [{bearer: []}], summary: 通知(游标) } }
```

---

## 2. PolarDB DDL（PostgreSQL 兼容版）

> 选 **PolarDB PostgreSQL**：JSONB、部分索引、CITEXT、枚举更顺手，且搜索外置 Meilisearch 不依赖库内中文分词。
> MySQL 变体：`CITEXT`→`VARCHAR + 唯一索引(lower())`，`BYTEA`→`VARBINARY`，枚举→`ENUM`/`CHECK`，`JSONB`→`JSON`。
> 单库多 schema，按域隔离。
>
> **Current addendum (2026-07-11):** the historical inline DDL below predates the governance rollout.
> Current structure is append-only in migrations: `0020_activity.sql` owns activity events/counts/policy,
> `0021_dm_moderation.sql` owns canonical DM/read/report state, `0022_governance.sql` owns central audit and
> invitations, `0023_review_moderation_decisions.sql` owns explicit review-report decisions,
> `0024_invitation_expiry.sql` owns invitation expiry/acceptance, and `0025_moderation_state.sql` owns
> automated-hide provenance plus system-issued sanctions. `0026_forum_flag_attempts.sql` preserves
> terminal report attempts, `0027_activity_backfill.sql` projects existing contributions, and
> `0028_review_course_restrict.sql` protects retained review history from course deletion.
> `0029_review_report_open_uniqueness.sql` preserves terminal cases while allowing later reports, and
> `0030_review_create_idempotency.sql` provides durable review-publication replay, and
> `0031_forum_board_thread_count_reconcile.sql` aligns historical board counters with the current
> visible-thread invariant. Do not copy
> this historical DDL to create a database; run the numbered migrations.

```sql
CREATE EXTENSION IF NOT EXISTS citext;
CREATE SCHEMA identity;  CREATE SCHEMA courses;
CREATE SCHEMA reviews;   CREATE SCHEMA credit;   CREATE SCHEMA forum;

-- ============ identity ============
CREATE TYPE identity.account_role   AS ENUM ('user','mod','admin');
CREATE TYPE identity.account_status AS ENUM ('active','suspended','deleted');

CREATE TABLE identity.accounts (
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  email         CITEXT UNIQUE NOT NULL,                 -- @tongji.edu.cn，落库可再加密
  email_verified_at TIMESTAMPTZ,
  handle        CITEXT UNIQUE NOT NULL,                 -- 公开昵称（前台匿名感）
  avatar_url    TEXT,
  role          identity.account_role   NOT NULL DEFAULT 'user',
  status        identity.account_status NOT NULL DEFAULT 'active',
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_active_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE identity.account_keys (              -- 资金/escrow 验签公钥
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  public_key  TEXT   NOT NULL UNIQUE,             -- base64 Ed25519
  algo        TEXT   NOT NULL DEFAULT 'ed25519',
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at  TIMESTAMPTZ,
  PRIMARY KEY (account_id, public_key)
);

CREATE TABLE identity.email_codes (               -- 验证码（短期）
  email       CITEXT NOT NULL,
  code_hash   TEXT   NOT NULL,
  expires_at  TIMESTAMPTZ NOT NULL,
  attempts    INT    NOT NULL DEFAULT 0,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON identity.email_codes (email, expires_at);

CREATE TABLE identity.sessions (                  -- refresh token（可吊销）
  id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  refresh_hash  TEXT   NOT NULL,
  user_agent    TEXT, ip INET,
  expires_at    TIMESTAMPTZ NOT NULL,
  revoked_at    TIMESTAMPTZ,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON identity.sessions (account_id);

CREATE TABLE identity.legacy_wallet_links (       -- 老钱包认领映射
  legacy_user_hash TEXT PRIMARY KEY,
  account_id       BIGINT REFERENCES identity.accounts(id),
  claimed_at       TIMESTAMPTZ
);

-- ============ courses ============
CREATE TABLE courses.teachers (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  tid TEXT, name TEXT NOT NULL, title TEXT, department TEXT,
  name_pinyin TEXT, name_initials TEXT          -- 由同步任务预计算（拼音/首字母）
);
CREATE TABLE courses.courses (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  code TEXT NOT NULL, name TEXT NOT NULL,
  credit REAL DEFAULT 0, department TEXT,
  teacher_id BIGINT REFERENCES courses.teachers(id),
  review_count INT  NOT NULL DEFAULT 0,         -- 增量维护，禁止读时重算
  review_avg   REAL NOT NULL DEFAULT 0,
  name_pinyin     TEXT,                          -- gaodengshuxue
  name_initials   TEXT,                          -- gdsx
  search_keywords TEXT,                          -- 归一化关键词冗余
  is_legacy INT DEFAULT 0, is_icu INT DEFAULT 0
);
CREATE INDEX ON courses.courses (code);
CREATE INDEX ON courses.courses (department);
CREATE TABLE courses.course_aliases (            -- 别名/简称：高数→高等数学
  course_id BIGINT NOT NULL REFERENCES courses.courses(id),
  alias TEXT NOT NULL,
  PRIMARY KEY (course_id, alias)
);
-- 选课(PK) 一系统镜像表 → 独立 `selection` schema（calendars/campuses/faculties/majors/
-- course_natures/courses/major_courses/timeslots/fetchlog），见 migration 0002_escrow_selection.sql

-- ============ reviews ============
CREATE TYPE reviews.review_status AS ENUM ('visible','hidden','pending');
CREATE TABLE reviews.reviews (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  course_id  BIGINT NOT NULL REFERENCES courses.courses(id) ON DELETE CASCADE,
  account_id BIGINT REFERENCES identity.accounts(id),     -- 取代 wallet_user_hash
  rating  INT NOT NULL CHECK (rating BETWEEN 0 AND 5),
  comment TEXT, score TEXT, semester TEXT,
  approve_count    INT NOT NULL DEFAULT 0,
  disapprove_count INT NOT NULL DEFAULT 0,
  status reviews.review_status NOT NULL DEFAULT 'visible',
  is_legacy INT DEFAULT 0, is_icu INT DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON reviews.reviews (course_id, status, created_at DESC);   -- 列表热路径
CREATE INDEX ON reviews.reviews (account_id);

CREATE TABLE reviews.review_likes (
  review_id  BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (review_id, account_id)
);
CREATE TABLE reviews.review_reports (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  review_id   BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  reporter_account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',
  admin_note TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  resolved_at TIMESTAMPTZ,
  UNIQUE (review_id, reporter_account_id)
);

-- D1 的 client_id 不是平台 account_id；先原样保存，禁止伪造账户关系。
CREATE TABLE reviews.legacy_review_likes (
  review_id BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  client_id TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  PRIMARY KEY (review_id, client_id)
);
CREATE TABLE reviews.legacy_review_reports (
  id BIGINT PRIMARY KEY,
  review_id BIGINT NOT NULL REFERENCES reviews.reviews(id) ON DELETE CASCADE,
  client_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  status TEXT NOT NULL,
  admin_note TEXT,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL,
  resolved_at BIGINT
);

-- ============ credit（Web2.5：中心账本 + Ed25519 签名 + 哈希链）============
-- 账本 credit.ledger 是唯一权威，append-only、单调 seq、prev_hash 链接、每条带签名。
-- 余额是账本的派生缓存（wallets.balance），事务内更新 + 夜间对账，禁止直接改余额。
CREATE TABLE credit.ledger (
  seq          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- 全局单调序号
  tx_id        TEXT UNIQUE NOT NULL,                              -- 业务幂等 id
  type         TEXT NOT NULL,                -- mint/transfer/escrow_hold/escrow_release/admin_adjust
  from_account BIGINT REFERENCES identity.accounts(id),           -- mint 为 NULL（系统增发）
  to_account   BIGINT REFERENCES identity.accounts(id),
  amount       BIGINT NOT NULL CHECK (amount > 0),
  nonce        TEXT NOT NULL,                                     -- 防重放
  metadata     JSONB,
  signer       TEXT NOT NULL,                -- 'system' 或 发起账号的 Ed25519 公钥
  signature    TEXT NOT NULL,                -- Ed25519(canonical(payload))
  prev_hash    TEXT NOT NULL,                -- 上一条 hash（创世为 64 个 0）
  hash         TEXT NOT NULL,                -- SHA256(canonical(payload) || prev_hash)
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON credit.ledger (from_account);
CREATE INDEX ON credit.ledger (to_account);
-- append 时取 advisory lock 串行化，保证链线性（当前 QPS 下零压力）

CREATE TABLE credit.wallets (                  -- 派生余额缓存（权威是 ledger）
  account_id BIGINT PRIMARY KEY REFERENCES identity.accounts(id),
  balance    BIGINT NOT NULL DEFAULT 0,
  last_seq   BIGINT NOT NULL DEFAULT 0,        -- 已结算到的账本 seq，便于增量对账
  last_active_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE credit.checkpoints (              -- 可选：定期账本快照（透明/公信）
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  up_to_seq BIGINT NOT NULL,
  merkle_root TEXT NOT NULL,                    -- 截至 up_to_seq 的根
  system_sig  TEXT NOT NULL,                    -- 系统私钥签名（默认不上真链）
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- tasks/products/purchases（escrow 状态机）：状态用枚举，资金动作落 ledger（hold/release）。
-- 完整 DDL 见 migration 0002_escrow_selection.sql（credit.tasks / credit.products / credit.purchases）

-- ============ forum（Phase B 占位）============
CREATE TABLE forum.boards   (id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY, slug TEXT UNIQUE, name TEXT);
CREATE TABLE forum.threads  (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  board_id BIGINT REFERENCES forum.boards(id),
  author_id BIGINT REFERENCES identity.accounts(id),
  title TEXT NOT NULL, body TEXT,
  reply_count INT DEFAULT 0, vote_count INT DEFAULT 0,
  hot_score DOUBLE PRECISION DEFAULT 0,           -- 定时任务刷新，进 Redis ZSET
  status TEXT DEFAULT 'visible',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_activity_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.threads (board_id, last_activity_at DESC);
CREATE TABLE forum.comments (                     -- 楼中楼：parent_id + 物化路径
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  thread_id BIGINT REFERENCES forum.threads(id),
  parent_id BIGINT REFERENCES forum.comments(id),
  path      TEXT,                                 -- 形如 '0003.0007.0012' 便于排序/取子树
  author_id BIGINT REFERENCES identity.accounts(id),
  body TEXT, vote_count INT DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.comments (thread_id, path);
CREATE TABLE forum.notifications (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id BIGINT REFERENCES identity.accounts(id),
  type TEXT, payload JSONB, read_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.notifications (account_id, created_at DESC);
```

---

## 3. 搜索索引设计（Meilisearch + 拼音/简称/别名归一化）

**语料只有几十 MB，全部进内存**。两个索引：`courses`、`reviews`（实时搜索同时打两个）。

### 3.1 index `courses` 文档结构
```jsonc
{
  "id": 1234,
  "code": "1234567",
  "name": "高等数学(上)",
  "name_pinyin": "gaodengshuxue",     // 全拼无声调
  "name_initials": "gdsx",            // 首字母
  "teacher_name": "张三",
  "teacher_pinyin": "zhangsan",
  "teacher_initials": "zs",
  "department": "数学科学学院",
  "aliases": ["高数", "高数上", "微积分"],   // 来自 course_aliases
  "credit": 5.0,
  "review_count": 312, "review_avg": 4.2
}
```

### 3.2 Meilisearch 设置
- `searchableAttributes`（顺序即权重）：`name` > `aliases` > `teacher_name` > `name_pinyin` > `name_initials` > `teacher_pinyin` > `teacher_initials` > `code` > `department`
- `filterableAttributes`：`department`, `credit`, `review_count`
- `sortableAttributes`：`review_count`, `review_avg`
- `rankingRules`：`["words","typo","proximity","attribute","sort","exactness","review_count:desc"]`（热门做兜底排序）
- `typoTolerance`：开启（自动容错"高等树学"→"高等数学"）
- `synonyms`：`{"高数":["高等数学"], "大物":["大学物理"], "线代":["线性代数"], "概率论":["概率论与数理统计"]}`

### 3.3 拼音归一化流水线（同步任务里预计算，不在查询时算）
1. 中文字段 → 全拼（无声调，去空格）+ 首字母两份，分别入 `*_pinyin` / `*_initials`。Rust 用 `pinyin` crate；同步 job 一次算好写库 + 推 Meili。
2. 多音字：课程/老师名固定，取常用读音即可；少量错配用 `synonyms` 兜。
3. 查询直接把原始 q 丢给 Meili 跨上述字段；纯首字母（如 `gdsx`）命中 `name_initials`，前缀/typo 由 Meili 处理。

### 3.4 index `reviews`（点评内容检索）
`{ id, course_id, course_name, comment, rating, created_at, like_count }`
- searchable：`comment` > `course_name`；filter：`course_id`, `rating`；sort：`created_at`, `like_count`

### 3.5 实时同步
- 发布/编辑/隐藏点评 → 异步 upsert reviews 文档 + 刷 course 的 `review_count/avg`。
- 课程统计变化 → upsert course 文档。
- 每日全量 reindex 兜底。

---

## 4. 缓存与失效策略（核心：绝不读时全表扫，写时精准失效）

### 4.1 分层
| 层 | 内容 | TTL / 策略 |
|---|---|---|
| L1 客户端 | debounce 200–250ms + AbortController 取消 + LRU(query→结果, ~50 条) | 60s |
| L2 边缘/CDN | `GET /courses`、`/courses/{id}`、`/courses/{id}/reviews` | 短 TTL + stale-while-revalidate |
| L3 应用/Redis | 课程详情+统计、搜索热词结果、热榜 ZSET、计数器、限流桶 | 见下 |
| 搜索 | Meilisearch 自身亚毫秒；仅对热词加 Redis 30–60s | — |

### 4.2 失效模型：**版本号 key + 标签清除**
- 每个可缓存对象有版本：Redis `ver:course:{id}`（整数）。缓存 key 内嵌版本：`course:{id}:v{n}`。
- 写操作 → `INCR ver:course:{id}`，旧 key 自然随 TTL 过期，**无需显式删除**，避免缓存击穿/漏删。
- 边缘层用 `Cache-Tag: course-{id}`；阿里云 CDN 按 URL/目录刷新，标签能力有限时退化为"短 TTL + URL 带版本号"。

### 4.3 统计与计数（关键性能点）
- `review_count/review_avg`：**增量维护**（发/删/隐藏点评时 +/-），读时直接取，**永不 `AVG()` 全表重算**；夜间任务对账兜底。
- 点赞/举报计数：Redis `INCR/DECR` 即时返回，定时 flush 回库；读取 = 库基线 + Redis 增量。
- 论坛热榜：定时任务算时间衰减分写 Redis ZSET，读直接取榜，不实时排序。

### 4.4 写路径 → 失效映射
| 写操作 | 库 | 缓存失效 | 搜索 | 计数 |
|---|---|---|---|---|
| 发布点评 | insert review；course.review_count/avg 增量 | `INCR ver:course:{cid}`；purge tag `course-{cid}` | 异步 upsert review+course 文档 | — |
| 点赞/取消 | （延迟落库）| 该课点评列表靠短 TTL 自然刷新 | — | Redis INCR like |
| 举报 | insert report（进审核队列）| 无公开缓存影响 | — | — |
| 管理隐藏 | review.status=hidden；重算该课统计 | `INCR ver:course:{cid}` | upsert/删除 review 文档 | — |
| 转账/结算 | transactions + wallets（事务）| 钱包余额 key 失效 | — | — |

### 4.5 限流（Redis 令牌桶）
- 写接口：发布点评 `account:5/min`、举报 `account:10/min`、转账 `account:20/min`。
- 搜索：`ip:30/10s` 防刷（叠加 L1 debounce，正常用户碰不到）。
- 登录/验证码：`email:1/60s` + `ip:5/10min`。

---

## 5. 落地顺序（对应路线图 P0–P2）
1. identity（accounts/keys/sessions/email_codes）+ 邮箱验证码 + JWT。
2. courses + reviews 平迁（含 account_id 映射、统计增量化）。
3. Meilisearch 接入 + 拼音同步任务 + `/search`。
4. Redis 缓存/计数/限流 + 边缘版本号失效。
5. credit 收编（account_keys 签名、老钱包认领、ledger 哈希链）。
6. forum（Phase B）。

---

## 6. 积分 Web2.5：账本设计与合规

### 6.1 形态（确认：中心账本 + Ed25519 签名 + 哈希链）
- 权威账本 = `credit.ledger`，append-only、单调 `seq`、`prev_hash` 链接，**每条带发起方 Ed25519 签名** → 可审计、防篡改、不可抵赖。
- **不上真实公链、无 Gas、无共识**；余额是账本派生的缓存（`wallets.balance`）。
- mint（系统增发，发点评/被点赞/发帖/悬赏完成）用**系统私钥**签名；transfer/打赏/悬赏用**用户私钥**签名（`X-Wallet-Sig`），服务端用 `account_keys` 公钥验签。

### 6.2 哈希链与校验
- payload = 规范化 JSON `{tx_id,type,from,to,amount,nonce,metadata}`；`hash = SHA256(canonical(payload) || prev_hash)`。
- append 取 advisory lock 串行化，保证链线性（低 QPS 零压力）。
- 校验接口 `GET /api/v2/wallet/ledger/verify`：重算全链 hash + 逐条验签，返回 `{ ok, latest_seq, latest_hash }`；可对外做透明页。
- 可选 `credit.checkpoints`：定期把截至某 seq 的 Merkle root 系统签名存证（**默认不锚定真实公链以避免合规风险**）。

### 6.3 ⚠️ 合规红线（闭环虚拟权益，非法律意见，建议正式咨询）
- **无充值入口、无提现、不与法币双向兑换**——否则构成支付中介（二清），需牌照。
- **不开放无理由自由转账**：积分流转只在 打赏 / 悬赏 / 商品托管 等受控场景内发生，防止形成影子货币 / 二级市场 / 洗钱赌博。
- **积分纯靠贡献赚取**，只花在平台内虚拟权益与人际打赏/悬赏 → 是"积分/荣誉"，不是"代币"。
- 因此 `credit` **不设** recharge/withdraw/自由 transfer 接口；对外只暴露 `mint(系统)`、`escrow hold/release`、受控 `tip/bounty`、`ledger/verify`。

---

## 7. 仓库结构（新建 monorepo，客户端各自独立）

```
yourtj-platform/                      # 新仓库（Cargo workspace + web）
├─ backend/
│  ├─ crates/  api/                   # Axum 网关（MVP 可先 Hono/TS）
│  │           identity/ courses/ reviews/ credit/ forum/ shared/
│  ├─ migrations/                     # PolarDB DDL + 增量迁移
│  └─ Dockerfile                      # 无状态镜像 → SAE，后续可换 SLB+ECS
├─ web/                               # React v2 前端
├─ contract/  openapi.yaml            # 单一契约源 → 生成 TS / Swift / Dart 类型
├─ infra/                             # SAE / Terraform / CI / 部署
└─ docs/                              # 本设计文档迁入
```

- iOS / Flutter **保持各自独立仓库**，只消费 `contract/` 生成的类型。
- 老仓库 `YourTJCourse-Serverless`(course) + `YourTJ-Credit-Serverless`(credit) **继续跑生产**，strangler 逐域切流，切完归档。
- 切流顺序对齐第 5 节：identity → reviews → courses → credit → forum。

---

## 8. Remediation Changes (2026-06)

### 8.1 Migration 0004 — New tables and columns

Added `backend/migrations/0004_review_remediation.sql`:

| Table/Column | Schema | Purpose |
|---|---|---|
| `wallet_claim_challenges` | `identity` | Challenge-response for legacy wallet claim (id, account_id, nonce, expires_at, used_at) |
| `legacy_public_key`, `legacy_balance`, `imported_metadata` (added to `legacy_wallet_links`) | `identity` | Store legacy Ed25519 key and pending balance for import on claim |
| `votes` | `forum` | One-vote-per-user (`PRIMARY KEY (post_type, post_id, account_id)`) |
| `forum_comments_thread_path_unique` | `forum` | Partial unique index on `(thread_id, path) WHERE path IS NOT NULL` — prevents concurrent comment path duplicates |

### 8.2 Credit signing — real Ed25519 system signatures

Previously all system-originated ledger entries used literal `"system-signed"` as signature. Now:
- `CREDIT_SYSTEM_PRIVATE_KEY` (hex-encoded 32-byte seed) loaded from the environment
- `derive_system_key()` in `api/src/bootstrap.rs` derives the Ed25519 keypair at startup
- System public key is stored in `AppState.system_public_key_b64`
- All `mint`, `escrow_hold`, `escrow_release` entries are signed with real Ed25519
- `verify_full_ledger` verifies both user and system signatures against stored public keys
- Batch preload of `identity.account_keys` reduces N+1 queries during full verification

### 8.3 Wallet claim flow

`GET /api/v2/wallet/claim-challenge`:
- Returns `{ challengeId, nonce }` valid for 10 minutes

`POST /api/v2/wallet/claim`:
- Accepts `{ legacyUserHash, challengeId, signature }` with bearer auth
- One transaction: locks challenge + legacy link, verifies Ed25519 signature over canonical JSON, marks challenge used, links legacy wallet, mints balance via system-signed ledger entry

### 8.4 Atomic purchases and escrow

- `append_ledger_entry_tx()` — transaction-aware variant that accepts `&mut PgConnection`
- `purchase_product`, `action_task`, `action_purchase` wrap ledger + state-table changes in single transactions
- All user value-moving writes (tip, task create, task confirm/cancel, purchase, purchase action) verify `X-Wallet-Sig` header

### 8.5 Forum votes and comment paths

- Forum votes use UPSERT on `forum.votes` with one-vote-per-user enforcement (PRIMARY KEY)
- Comment path generation uses `FOR UPDATE` row locks inside a transaction
- Unique partial index on `(thread_id, path)` prevents concurrent path collisions

### 8.6 OSS upload trust boundary

- `POST /api/v2/media/upload-credentials` creates a 15-minute account-bound upload intent and returns Alibaba STS credentials whose policy permits `oss:PutObject` only for that intent's exact object key.
- OSS callbacks accept only HTTPS public-key URLs on `gosspublic.alicdn.com`, disable redirects, verify RSA PKCS#1 v1.5 with MD5 over the OSS canonical path/body, and atomically consume the upload intent with upload-row creation.
- Missing OSS configuration fails media routes closed; long-lived access keys and role secrets are loaded only from environment variables.

### 8.7 Domain boundaries

| Crate | Owns |
|---|---|
| `identity` | Auth, accounts, sessions, Ed25519 key management, wallet claim |
| `credit` | Ledger, wallets, wallets balance, tasks, products, purchases |
| `forum` | Boards, threads, comments, votes, notifications (moved from identity) |
| `activity` | Idempotent contribution events, daily counts, versioned display weights |
| `governance` | Append-only cross-domain staff/system audit events |
| `courses` | Catalogue, teachers, departments, selection (选课) mirror, admin course CRUD (moved from api) |
| `shared` | Config, JWT primitives (no DB queries), AppState, error types, pagination, cache, rate limiting |
| `api` | Router composition, startup wiring, platform routes, admin stubs (selection sync, review reindex) |

### 8.8 Rate limits (Redis token bucket)

| Operation | Rate |
|---|---|
| Review creation | 5 per 60s per account |
| Thread creation | 5 per 60s per account |
| Comment creation | 20 per 60s per account |
| Transfer/tip/task ops | 20 per 60s per account |
| Email code | 1 per 60s per email |
| IP code | 5 per 10min per IP |

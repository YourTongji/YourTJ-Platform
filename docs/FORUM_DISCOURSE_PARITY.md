# 论坛体系对齐 Discourse — 完整设计（Forum Parity Plan）

> **Status:** Historical parity plan; its gap matrix and phase labels are not a current implementation
> inventory or current governance authority.
>
> **Owner:** Forum maintainers
>
> **Last verified:** Written before the current F1/F2/F3 implementation; retained for rationale
>
> **Authoritative sources:** [`docs/README.md`](README.md),
> [`product/community-governance.md`](product/community-governance.md),
> [`product/profile-and-messaging.md`](product/profile-and-messaging.md),
> [`security/rbac-and-audit.md`](security/rbac-and-audit.md), OpenAPI, and numbered migrations
>
> 本文保留 Discourse 对齐时的产品思考，但其中 ✅/🟡/❌ 与 F0–F3 状态已过期。不要据此判断
> 当前接口、数据库或已上线能力。

> 目标：以 Discourse 的论坛功能体系为基准，逐项对照 YourTJ forum 域的现状，
> 给出**内容功能完整、详尽、好用**的对齐方案。本文是 forum 域 Phase B → Phase C
> 的权威设计，落地时按 §9 分期执行；改 HTTP 面先改 `contract/openapi.yaml`。
>
> 阅读前置：[REWRITE_V2_DESIGN.md](REWRITE_V2_DESIGN.md)（总体架构、缓存模型、合规红线）。

---

## 0. 对齐原则

1. **功能对齐，形态本地化。** Discourse 是通用社区产品；我们是校园垂直平台。
   对齐的是"能力"（审核、信任、订阅、已读、编辑历史……），不是照抄它的交互形态。
2. **体系裁剪，不做的明说。** 多站点、插件/主题系统、群组权限矩阵、email-in 等
   与校园单站场景无关的能力明确不做（见 §8），避免范围蔓延。
3. **差异化保留。** 我们有 Discourse 没有的东西：Web2.5 积分账本、评课/选课联动、
   校园实名底座 + 匿名前台。对齐方案里凡是 Discourse 用"虚荣心"（badge/TL 特权）
   驱动的地方，我们可以叠加**积分 mint** 驱动 —— 但严守合规红线（只 mint，不新增
   转账形态）。
4. **沿用既有基建。** 缓存 = 版本号失效（design doc §4），限流 = Redis 令牌桶，
   搜索 = Meilisearch，通知 = `forum.notifications` 表。不引入新中间件；
   实时性用 SSE，不上 WebSocket 集群。
5. **域边界不破。** forum crate 拥有 `forum.*` 表；信任等级/封禁属于 identity；
   徽章跨域属于 platform；媒体上传新建 media 域。跨域只走对方 crate 的公开 API。

---

## 1. 差距总览矩阵

图例：✅ 已有　🟡 部分有/未接线　❌ 缺失　➖ 明确不做

| # | Discourse 能力 | 我们现状 | 目标 | 阶段 |
|---|---|---|---|---|
| 1 | Categories（层级板块、板块权限） | 🟡 扁平 boards，无描述/排序/锁定 | 两级板块 + 描述/排序 + 发帖门槛 | F1 |
| 2 | Tags（标签、标签筛选） | ❌ | 标签 + 按标签筛选主题流 | F1 |
| 3 | Topic 状态（置顶/关闭/归档/隐藏） | 🟡 只有 `status` 一个 TEXT | pinned/closed/archived/deleted 全状态机 | F1 |
| 4 | Post 编辑 + 修订历史 | ❌ 无编辑端点 | 编辑 + `post_revisions` 全历史 + 编辑窗口 | F1 |
| 5 | 删除/软删/恢复 | ❌ | 作者软删 + mod 删除/恢复，删除留痕 | F1 |
| 6 | Likes / post actions | 🟡 有 ±1 votes（一人一票） | 保留 votes 为主反馈；表情回应 F3 | ✅/F3 |
| 7 | 楼中楼 / replies | ✅ materialized path + 并发安全 | 维持 | ✅ |
| 8 | 已读追踪（unread/new、回到上次位置） | ❌ | `thread_reads` + 未读数 + last-read 定位 | F1 |
| 9 | Watching/Tracking/Muted 订阅 | ❌（模块注释里承诺了 follows） | 板块级 + 主题级三档订阅 | F1 |
| 10 | 通知体系（回复/提及/引用/点赞/系统） | 🟡 表和端点在，**无产生调用点** | 全类型通知 + 聚合 + 偏好开关 | F0+F1 |
| 11 | @提及 | ❌ | 解析 @handle → 通知 | F1 |
| 12 | 引用回复（quote） | ❌ | 客户端引用块 + 被引通知 | F2 |
| 13 | 草稿 | ❌ | 服务端草稿（跨设备） | F2 |
| 14 | 图片/附件上传 | ❌ 无任何媒体能力 | media 域 + OSS 直传 + 审核态 | F2 |
| 15 | Onebox 链接预览 | ❌ | 白名单域名 SSR 卡片 | F2 |
| 16 | 全文搜索 | 🟡 meili 有 courses/reviews 索引，forum 无 | `forum_threads` 索引 + `/search?type=thread` | F0+F1 |
| 17 | Trust Levels（TL0–4） | ❌ | 简化 TL0–3 校园版 + 新手限制 | F1 |
| 18 | Flag 体系（阈值自动隐藏） | ❌ 论坛完全没有举报 | 通用 flags + 信任加权阈值自动隐藏 | F1 |
| 19 | Review Queue（统一审核队列） | 🟡 reviews 域有自己的 reports | 论坛 flags 队列 + 管理端点（Web 层聚合两域） | F1 |
| 20 | Mod 工具（silence/suspend/staff log） | ❌ | identity.sanctions + forum.mod_actions | F1 |
| 21 | Watched words（敏感词） | ❌ | block/censor/queue 三档词表 | F1 |
| 22 | 限流 / 新用户限制 | 🟡 有令牌桶，无按 TL 分档 | 限流 × TL 分档 + 新手外链/图片限制 | F1 |
| 23 | Badges | ❌ | platform.badges + 授予任务 + 积分 mint 联动 | F2 |
| 24 | 书签 | ❌ | thread/comment 书签 + 列表 | F1 |
| 25 | 用户主页/活动流 | 🟡 只有 `/me` | 公开主页：主题/评论/徽章/统计 | F2 |
| 26 | 屏蔽用户（ignore/mute user） | ❌ | 个人级屏蔽（内容过滤 + 通知抑制） | F2 |
| 27 | 实时更新（MessageBus） | ❌ | SSE：新通知 + 主题新回复提示 | F2 |
| 28 | 私信（PM/群 PM） | ❌ | 1:1 私信（群聊不做） | F3 |
| 29 | 投票贴（poll） | ❌ | 单选/多选投票贴 | F3 |
| 30 | 邮件摘要 / email 通知 | ❌（SMTP 尚未接） | 登录码复用同通道；每周摘要 | F0/F3 |
| 31 | Topic timers（定时关闭等） | ❌ | 自动归档陈旧主题（单一场景） | F3 |
| 32 | Solved（采纳答案） | ❌ | 问答板块采纳 + 与悬赏 confirm 联动 | F3 |
| 33 | 热榜 | 🟡 算法和 ZSET 写好，**无调度** | spawn 定时任务 | F0 |
| 34 | Email-in（邮件发帖） | — | ➖ 不做 | — |
| 35 | 多站点/插件/主题/i18n | — | ➖ 不做 | — |
| 36 | 群组（groups）与权限矩阵 | — | ➖ 不做（role: user/mod/admin 够用） | — |
| 37 | Chat（实时聊天室） | — | ➖ 不做 | — |

**F0 = 接线修复**（现有代码有函数无调用点，先让既有承诺兑现）；**F1 = 核心对齐**；
**F2 = 体验对齐**；**F3 = 增强**。分期详情见 §9。

---

## 2. 数据模型（migration `0005_forum_parity.sql` + `0006`）

> 全部 append-only 新迁移。F1 所需入 `0005`，F2/F3 所需入 `0006`（届时再定稿）。
> 以下 DDL 是 0005 的定稿草案。

### 2.1 板块：层级、元数据、门槛

```sql
ALTER TABLE forum.boards
  ADD COLUMN parent_id         BIGINT REFERENCES forum.boards(id),  -- 两级封顶，应用层校验
  ADD COLUMN description       TEXT,
  ADD COLUMN position          INT NOT NULL DEFAULT 0,              -- 展示排序
  ADD COLUMN is_locked         BOOLEAN NOT NULL DEFAULT FALSE,      -- 仅 mod/admin 可发主题（公告板）
  ADD COLUMN min_trust_to_post SMALLINT NOT NULL DEFAULT 0,         -- 发帖最低 TL
  ADD COLUMN thread_count      INT NOT NULL DEFAULT 0;              -- 增量维护，不读时算
```

### 2.2 主题：完整状态机

```sql
ALTER TABLE forum.threads
  ADD COLUMN pinned_at        TIMESTAMPTZ,          -- 板内置顶
  ADD COLUMN pinned_globally  BOOLEAN NOT NULL DEFAULT FALSE,  -- 全站置顶
  ADD COLUMN closed_at        TIMESTAMPTZ,          -- 关闭：可读不可回
  ADD COLUMN archived_at      TIMESTAMPTZ,          -- 归档：只读且不进流
  ADD COLUMN deleted_at       TIMESTAMPTZ,          -- 软删：作者或 mod
  ADD COLUMN deleted_by       BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN edited_at        TIMESTAMPTZ,
  ADD COLUMN hidden_at        TIMESTAMPTZ;          -- flag 阈值自动隐藏（待审）
-- status TEXT 保留兼容读，写路径以上述时间戳列为准，最终由 DTO 折叠成一个枚举。
```

**状态优先级**（DTO 折叠规则）：`deleted > hidden > archived > closed > visible`。
置顶与状态正交。

### 2.3 评论：软删与编辑

```sql
ALTER TABLE forum.comments
  ADD COLUMN deleted_at TIMESTAMPTZ,
  ADD COLUMN deleted_by BIGINT REFERENCES identity.accounts(id),
  ADD COLUMN edited_at  TIMESTAMPTZ,
  ADD COLUMN hidden_at  TIMESTAMPTZ;
```

软删的评论**保留占位**（"该楼层已删除"）以维持楼层号与 path 树完整 —— 与
Discourse 行为一致。

### 2.4 标签

```sql
CREATE TABLE forum.tags (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug         TEXT UNIQUE NOT NULL,
  name         TEXT NOT NULL,
  description  TEXT,
  thread_count INT NOT NULL DEFAULT 0,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE forum.thread_tags (
  thread_id BIGINT NOT NULL REFERENCES forum.threads(id) ON DELETE CASCADE,
  tag_id    BIGINT NOT NULL REFERENCES forum.tags(id)    ON DELETE CASCADE,
  PRIMARY KEY (thread_id, tag_id)
);
CREATE INDEX ON forum.thread_tags (tag_id);
```

规则：标签由 admin 预建（校园场景不放开自由建标签，防碎片化）；每主题 ≤ 3 个。

### 2.5 修订历史（编辑留痕）

```sql
CREATE TABLE forum.post_revisions (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  post_type  TEXT NOT NULL CHECK (post_type IN ('thread', 'comment')),
  post_id    BIGINT NOT NULL,
  seq        INT NOT NULL,                         -- 每帖单调递增
  editor_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  old_title  TEXT,                                 -- 仅 thread 有
  old_body   TEXT NOT NULL,                        -- 存全量旧文本，不存 diff
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (post_type, post_id, seq)
);
```

编辑规则（对齐 Discourse 的 grace period / edit window 思想，做校园简化）：
- 发布后 **5 分钟内**的编辑不产生修订记录（打字纠错宽限）。
- 之后每次编辑：写一条 revision（旧文本）→ 更新正文 → `edited_at = now()`。
- 修订历史作者本人 + mod 可见（`GET .../revisions`），普通用户只见"已编辑"标记。
- 帖子被他人回复/引用后，thread 标题仍可编辑，正文编辑不受限但留痕 ——
  纠纷仲裁以 revisions 为准。

### 2.6 已读追踪

```sql
CREATE TABLE forum.thread_reads (
  account_id           BIGINT NOT NULL REFERENCES identity.accounts(id),
  thread_id            BIGINT NOT NULL REFERENCES forum.threads(id) ON DELETE CASCADE,
  last_read_comment_id BIGINT,          -- NULL = 只读了主楼
  updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, thread_id)
);
```

- 客户端滚动到楼层即上报（防抖 5s，批量端点）；服务端只允许**前移**。
- 未读数 = `thread.reply_count` 与 last_read 位置的差，列表页按需批量计算
  （一次 JOIN，绝不 N+1）。
- 对齐 Discourse 的 unread/new 分栏：`new` = 订阅范围内、创建于上次访问后且从未读；
  `unread` = 读过但有新回复。

### 2.7 订阅（watching / tracking / muted）

```sql
CREATE TABLE forum.subscriptions (
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  target_type TEXT NOT NULL CHECK (target_type IN ('board', 'thread')),
  target_id   BIGINT NOT NULL,
  level       TEXT NOT NULL CHECK (level IN ('watching', 'tracking', 'muted')),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, target_type, target_id)
);
CREATE INDEX ON forum.subscriptions (target_type, target_id, level);
```

语义对齐 Discourse：
- **watching**：每条新回复都通知。
- **tracking**：不推送通知，只累计未读数（默认态 = 自己发的/回过的主题自动 tracking）。
- **muted**：不通知、不进 feed、不计未读。
- 主题级设置覆盖板块级；`/forum/threads?feed=following` 即"我 watching/tracking
  的主题按活跃排序"（读时聚合 + Redis 缓存，**不做写扩散**，见 crate 头注释约定）。

### 2.8 举报（flags）与审核

```sql
CREATE TABLE forum.flags (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  target_type TEXT NOT NULL CHECK (target_type IN ('thread', 'comment')),
  target_id   BIGINT NOT NULL,
  reporter_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  reason      TEXT NOT NULL CHECK (reason IN ('spam', 'abuse', 'off_topic', 'illegal', 'other')),
  note        TEXT,                                -- reason=other 必填
  weight      REAL NOT NULL DEFAULT 1.0,           -- 举报人 TL 加权，见 §5.2
  status      TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'upheld', 'rejected', 'ignored')),
  handled_by  BIGINT REFERENCES identity.accounts(id),
  handled_at  TIMESTAMPTZ,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (target_type, target_id, reporter_id)     -- 一人一票
);
CREATE INDEX ON forum.flags (status, created_at DESC);
```

### 2.9 敏感词表

```sql
CREATE TABLE forum.watched_words (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  word       TEXT UNIQUE NOT NULL,                 -- 小写归一化存储
  action     TEXT NOT NULL CHECK (action IN ('block', 'censor', 'queue')),
  created_by BIGINT REFERENCES identity.accounts(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- **block**：发布直接 400（提示含违规内容，不回显命中词）。
- **censor**：发布成功，展示时替换为 `▇▇`。
- **queue**：发布成功但 `hidden_at = now()`，进审核队列。
- 词表全量加载进进程内 `ArcSwap<AhoCorasick>`，admin 改词后 bump 版本号重载 ——
  命中判定在写路径，零额外查询。

### 2.10 管理操作留痕（staff action log）

```sql
CREATE TABLE forum.mod_actions (
  id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  actor_id    BIGINT NOT NULL REFERENCES identity.accounts(id),
  action      TEXT NOT NULL,        -- pin/unpin/close/reopen/archive/delete/restore/hide/unhide/resolve_flag/...
  target_type TEXT NOT NULL,
  target_id   BIGINT NOT NULL,
  reason      TEXT,
  metadata    JSONB,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON forum.mod_actions (created_at DESC);
```

所有 mod/admin 对论坛内容的写操作**必须**同事务写一条 mod_action —— 对齐
Discourse 的 staff action log，这是校园平台公信力的底线设施。

### 2.11 书签

```sql
CREATE TABLE forum.bookmarks (
  account_id  BIGINT NOT NULL REFERENCES identity.accounts(id),
  target_type TEXT NOT NULL CHECK (target_type IN ('thread', 'comment')),
  target_id   BIGINT NOT NULL,
  note        TEXT,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account_id, target_type, target_id)
);
CREATE INDEX ON forum.bookmarks (account_id, created_at DESC);
```

### 2.12 论坛用户统计（信任等级的数据源）

```sql
CREATE TABLE forum.user_stats (
  account_id       BIGINT PRIMARY KEY REFERENCES identity.accounts(id),
  threads_created  INT NOT NULL DEFAULT 0,
  comments_created INT NOT NULL DEFAULT 0,
  votes_cast       INT NOT NULL DEFAULT 0,
  votes_received   INT NOT NULL DEFAULT 0,
  flags_upheld     INT NOT NULL DEFAULT 0,   -- 举报被采纳次数（信任加分）
  flagged_upheld   INT NOT NULL DEFAULT 0,   -- 被举报成立次数（信任减分）
  last_posted_at   TIMESTAMPTZ,
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

写路径增量维护（与 `review_count` 同一模式），绝不读时聚合。

### 2.13 identity 域：信任等级与处罚（identity crate 所有）

```sql
ALTER TABLE identity.accounts
  ADD COLUMN trust_level SMALLINT NOT NULL DEFAULT 0;   -- 0..3，晋升任务维护

CREATE TABLE identity.sanctions (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  account_id BIGINT NOT NULL REFERENCES identity.accounts(id),
  kind       TEXT NOT NULL CHECK (kind IN ('silence', 'suspend')),
  reason     TEXT NOT NULL,
  issued_by  BIGINT NOT NULL REFERENCES identity.accounts(id),
  starts_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  ends_at    TIMESTAMPTZ,                  -- NULL = 永久
  revoked_at TIMESTAMPTZ,
  revoked_by BIGINT REFERENCES identity.accounts(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON identity.sanctions (account_id, ends_at);
```

- **silence**（禁言）：可登录可浏览，全平台禁写（forum + reviews + credit 市场描述类）。
- **suspend**（封禁）：拒绝登录（refresh 失效，access 到期即断）。
- 生效判定进 `identity::auth_middleware`：登录时查 suspend；写请求鉴权扩展一个
  `require_can_post()`，内部查 silence（结果进 Redis 缓存 60s，处罚变更时精准失效）。
- forum/reviews 通过 `identity` crate 公开函数查询，不直接摸 `identity.sanctions` 表。

### 2.14 通知偏好（forum 域）

```sql
CREATE TABLE forum.notification_prefs (
  account_id BIGINT PRIMARY KEY REFERENCES identity.accounts(id),
  prefs      JSONB NOT NULL DEFAULT '{}'::jsonb,   -- {"reply":true,"mention":true,"vote":false,...}
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

缺省全开（vote 类默认聚合）。类型枚举见 §5.4。

### 2.15 F2/F3 预告（入 `0006`，此处只定方向）

- `media.uploads`（新 schema + 新 crate）：`id, account_id, kind, oss_key, url,
  bytes, mime, sha256, status(pending/clean/blocked), created_at`。
- `platform.badges` + `platform.account_badges`：徽章定义与授予。
- `forum.drafts`：`(account_id, draft_key) PK, payload JSONB`。
- `forum.dm_conversations / dm_participants / dm_messages`：1:1 私信。
- `forum.polls / poll_options / poll_votes`：投票贴。
- `forum.user_ignores`：`(account_id, ignored_account_id) PK`。

---

## 3. API 面（`contract/openapi.yaml` 增量）

> 约定沿用平台规范：`/api/v2` 前缀、camelCase、Unix 秒、`Page<T>` 游标分页、
> 错误信封。以下按用户侧 / 管理侧列出。**契约先行**：实现前先落 openapi。

### 3.1 用户侧

```text
# 板块与标签
GET    /forum/boards                          # 含层级、描述、threadCount、我的订阅级别
GET    /forum/tags
GET    /forum/threads?board=&tag=&feed=hot|new|following|unread&cursor=&limit=

# 主题
POST   /forum/threads                         # + tags[]，敏感词/TL/禁言校验
GET    /forum/threads/{id}                    # 含我的订阅级别、last_read 位置、状态
PATCH  /forum/threads/{id}                    # 作者编辑（标题/正文/标签），留痕
DELETE /forum/threads/{id}                    # 作者软删
GET    /forum/threads/{id}/revisions          # 作者/mod
POST   /forum/threads/{id}/read               # {lastReadCommentId} 已读上报（幂等前移）

# 评论
GET    /forum/threads/{id}/comments?cursor=   # 软删占位返回
POST   /forum/threads/{id}/comments           # 支持 parentId（楼中楼）、@提及解析
PATCH  /forum/comments/{id}                   # 作者编辑，留痕
DELETE /forum/comments/{id}                   # 作者软删
GET    /forum/comments/{id}/revisions

# 互动
POST   /forum/posts/{id}/vote                 # 既有，{value: 1|-1|0}（0=撤销）
POST   /forum/posts/{id}/flag                 # {reason, note?} → 202
PUT    /forum/posts/{id}/bookmark             # {note?}
DELETE /forum/posts/{id}/bookmark
GET    /forum/bookmarks?cursor=

# 订阅
PUT    /forum/subscriptions                   # {targetType, targetId, level}
DELETE /forum/subscriptions                   # 回到默认态
GET    /forum/subscriptions?type=board|thread

# 通知
GET    /notifications?cursor=&unread=true     # 既有，加 unread 过滤
POST   /notifications/read                    # 既有，{ids?}（缺省全部已读）
GET    /notifications/unread-count
GET    /notifications/stream                  # SSE（F2）
GET    /me/notification-prefs
PUT    /me/notification-prefs

# 用户公开面（F2）
GET    /users/{handle}                        # 公开主页：TL、统计、徽章（不暴露 email）
GET    /users/{handle}/threads?cursor=
GET    /users/{handle}/comments?cursor=
PUT    /me/ignores/{accountId}   DELETE 同    # 屏蔽用户（F2）

# 草稿 / 上传 / 私信 / 投票贴（F2/F3，契约届时落）
```

### 3.2 管理侧（mod/admin，全部写 mod_actions）

```text
# 板块与标签管理
POST/PATCH/DELETE  /admin/forum/boards[/{id}]
POST/PATCH/DELETE  /admin/forum/tags[/{id}]

# 内容处置
POST   /admin/forum/threads/{id}/pin          # {globally?} / unpin
POST   /admin/forum/threads/{id}/close        # / reopen
POST   /admin/forum/threads/{id}/archive
POST   /admin/forum/threads/{id}/move         # {boardId}
DELETE /admin/forum/threads/{id}              # mod 删除（软删，deleted_by=mod）
POST   /admin/forum/threads/{id}/restore
POST   /admin/forum/posts/{id}/hide           # / unhide（解除 flag 隐藏）
DELETE /admin/forum/comments/{id}             # / restore

# 审核队列
GET    /admin/forum/flags?status=open&cursor=  # 按目标聚合：一个目标一行，含加权分
POST   /admin/forum/flags/{id}/resolve         # {action: uphold|reject|ignore, note?}
                                               # uphold → 目标隐藏/删除 + 被举报人计数
# 用户处置（identity 域路由，forum 管理台聚合展示）
POST   /admin/users/{id}/silence              # {reason, endsAt?}
POST   /admin/users/{id}/suspend              # {reason, endsAt?}
POST   /admin/users/{id}/unsanction           # {sanctionId}
GET    /admin/users/{id}/sanctions

# 词表与日志
GET/POST/DELETE  /admin/forum/watched-words[/{id}]
GET    /admin/forum/mod-actions?cursor=        # staff log，只读
```

---

## 4. 内容组织与阅读体验（对齐细节）

### 4.1 主题流（对齐 Discourse 的 latest/top/unread/new）

`GET /forum/threads` 的 `feed` 参数：

| feed | 语义 | 数据源 |
|---|---|---|
| `hot` | 热榜 | Redis ZSET（F0 接上调度后） |
| `new` | 最新活跃 | `last_activity_at DESC` 索引（已有） |
| `following` | 我订阅的（watching+tracking） | subscriptions JOIN，Redis 缓存 60s |
| `unread` | 登录用户未读 | thread_reads 差集，读时算 |

全站置顶主题在任何 feed 首屏前置；板内置顶只在该板前置。muted 的板块/主题
从所有 feed 剔除。

### 4.2 热榜算法（把已写好的 `refresh_hot_rank` 接入调度）

- `api/bootstrap.rs` spawn 一个受监督的循环任务：每 5 分钟调
  `forum::repo::refresh_hot_rank`；panic 记录并重启，不拖垮主进程。
- 分数公式维持现实现（时间衰减 + votes + replies），后续只调参不改结构。

### 4.3 排版能力

- 正文为 Markdown 子集（CommonMark，无 raw HTML），**服务端只存原文**，
  渲染在客户端；服务端做长度上限（thread 64KB / comment 16KB）与敏感词处理。
- @提及：写路径用 `@[\p{L}\p{N}_-]+` 提取 → 批量查 handle → 对存在的账号发
  `mention` 通知（单帖最多通知 10 人，防 @全楼滥用）。
- 引用（F2）：客户端插入 `> quote` 块并带 `quotedCommentId` 字段，服务端据此发
  `quote` 通知。
- Onebox（F2）：白名单域名（校内站点、B 站、GitHub 等）服务端抓 OG 标签，
  结果按 URL 缓存 7 天；非白名单一律纯链接。

---

## 5. 信任、反滥用与审核（Discourse 的精髓，重点对齐）

### 5.1 信任等级（TL0–3 校园简化版）

| TL | 名称 | 晋升条件（全自动） | 权益 |
|---|---|---|---|
| 0 | 新人 | 注册即是 | 限制见下表 |
| 1 | 成员 | 注册 ≥ 2 天 且 发帖+评论 ≥ 3 且 阅读 ≥ 10 主题 | 解除新人限制 |
| 2 | 熟客 | 注册 ≥ 15 天 且 天数访问 ≥ 8 且 获赞 ≥ 10 且 无 upheld 被举报 | 举报权重 1.5、可发外链无审、编辑窗口放宽 |
| 3 | 资深 | 注册 ≥ 60 天 且 近 60 天活跃 ≥ 20 天 且 获赞 ≥ 50 且 flags_upheld ≥ 3 | 举报权重 2.0、其 flag 直接触发隐藏阈值的 50% |

- 晋升由**每日任务**扫描 `forum.user_stats` + `identity.accounts` 计算，只升不自动降；
  TL2/TL3 在被 uphold 举报后由任务降一级（留 mod_action 记录）。
- TL 存于 `identity.accounts.trust_level`，forum 通过 identity 公开 API 读取
  （带 Redis 缓存，处罚/晋升时失效）。
- **不做** Discourse TL4（版主级）：mod 是 role，不与 TL 混合。

**TL0 新人限制**（对齐 Discourse new user restrictions）：

| 限制项 | TL0 | TL1+ |
|---|---|---|
| 发主题 | 2/天 | 令牌桶常规（5/60s） |
| 发评论 | 5/天 | 20/60s |
| 外链 | ≤ 1 条/帖，命中即 `queue`（隐藏待审） | 不限（TL1 仍计敏感词） |
| 图片（F2 后） | 不可 | TL1 ≤ 4 张/帖 |
| @提及 | ≤ 2 人/帖 | ≤ 10 人/帖 |
| 举报权重 | 0.5 | 1.0 / 1.5 / 2.0 |

### 5.2 Flag 阈值自动隐藏（对齐 Discourse flag threshold）

- 目标（thread/comment）的**加权举报分** = Σ reporter 的 TL 权重。
- `score ≥ 3.0` → 自动 `hidden_at = now()` + 进审核队列 + 通知作者
  （"内容被社区暂时隐藏，等待审核"）。
- mod `uphold`：内容转软删、被举报人 `flagged_upheld += 1`、举报人各
  `flags_upheld += 1`；`reject`：解除隐藏、举报分清零；`ignore`：解除隐藏但不奖惩。
- 同一作者 24h 内被自动隐藏 ≥ 2 次 → 自动 silence 24h（系统 mod_action，
  actor 记 system，mod 可复核撤销）。这是 Discourse "spam auto-silence" 的等价物。

### 5.3 审核队列（review queue）

- 入队来源：flag 达阈值、watched_words `queue` 命中、TL0 外链帖。
- `GET /admin/forum/flags` 按目标聚合展示：内容摘录、举报人列表与理由分布、
  加权分、作者近况（TL、历史 upheld 数 —— 经 identity API 取）。
- 与 reviews 域的 `review_reports` **不合表**（域边界），Web 管理台并列两个 tab；
  处置动作语义对齐（uphold/reject/ignore），前端可复用组件。

### 5.4 通知类型全集（§F0 接线 + F1 扩类型）

| type | 触发点 | 聚合规则 |
|---|---|---|
| `reply` | 我的主题/评论被回复 | 同主题 10 分钟窗口聚合为一条 |
| `mention` | 被 @ | 不聚合 |
| `quote` | 被引用（F2） | 不聚合 |
| `vote` | 我的帖被顶 | 每帖每日聚合（"你的帖子获得了 N 个赞"） |
| `watching` | watching 的主题有新回复 | 同主题聚合 |
| `flag_hidden` | 我的内容被自动隐藏 | 不聚合 |
| `mod_action` | 我的内容被处置 / 我被处罚 | 不聚合 |
| `badge` | 获得徽章（F2） | 不聚合 |
| `credit` | 收到打赏 / mint 到账（credit 域经 forum 公开 API 写入） | 每日聚合 |
| `dm` | 新私信（F3） | 同会话聚合 |

实现：`notification_hooks::create_notification` 已存在，F0 把调用点接到
create_comment / vote / claim 等写路径（**同事务**或紧随其后，失败只 warn 不回滚
业务写）；聚合 = 插入前查同 (account, type, 聚合键) 未读旧条目则 UPDATE payload
计数。偏好关闭的类型直接跳过。

### 5.5 限流总表（扩展 design doc §4.5）

| 操作 | TL0 | TL1+ | 既有 |
|---|---|---|---|
| 发主题 | 2/天 | 5/60s | ✅（补 TL 分档） |
| 发评论 | 5/天 | 20/60s | ✅（补 TL 分档） |
| vote | 30/60s | 60/60s | ❌ 新增 |
| flag | 5/天 | 15/天 | ❌ 新增 |
| 编辑 | — | 10/60s | ❌ 新增 |
| 已读上报 | — | 60/60s（批量端点） | ❌ 新增 |
| 订阅变更 | — | 30/60s | ❌ 新增 |

---

## 6. 与积分/评课体系的联动（我们的差异化，Discourse 做不到的）

严守合规红线：以下只有 **系统 mint** 与既有 tip/bounty 流转，不新增转账形态。

| 事件 | 动作 |
|---|---|
| 主题首次获得 10 个净赞 | mint 5 分（每主题一次，幂等键 = `mint:thread_vote:{id}`） |
| 评论首次获得 10 个净赞 | mint 2 分 |
| 举报被 uphold | mint 1 分（封顶 5 分/日）—— 把 Discourse 的"信任荣誉"变成可用权益 |
| 优质主题被 mod 加精（F2 徽章） | mint 10 分 + badge |
| 悬赏帖（credit.tasks）与问答主题互链 | 采纳答案（F3 solved）时提示发起 `tasks/{id}/action confirm` 放款 |

实现路径：forum 写路径不直接摸 `credit.ledger`，调用 `credit` crate 公开的
`mint_for_contribution(pool, account_id, amount, idempotency_key, metadata)`
（系统签名、幂等、走 advisory lock —— 该函数 F0 与 reviews 域共用）。

---

## 7. 搜索、缓存与实时性

### 7.1 Meilisearch `forum_threads` 索引

- 文档：`{id, title, body_excerpt(前 2KB), board, tags[], author_handle,
  reply_count, vote_count, created_at, status}`；只索引 `visible` 状态。
- 同步：写路径（create/edit/状态变更/软删）挂 `sync_thread_to_meili`（软删=删除文档）；
  admin `POST /admin/forum/reindex` 全量重建（复用既有 reviews reindex 落地方式，F0
  一并把 stub 变实）。
- 查询：`GET /search?type=thread` 并入现有 `/search` 端点；评论不单独建索引
  （校园量级下命中主题足够，避免索引膨胀）。

### 7.2 缓存失效映射（扩展 design doc §4.4）

| 写操作 | 失效 |
|---|---|
| 发/编/删主题、状态变更 | `forum:board:{id}:v++`、`forum:thread:{id}:v++`、hot ZSET 惰性 |
| 发/编/删评论 | `forum:thread:{id}:v++` |
| vote | 计数走 Redis 增量，thread 版本不 bump（容忍 60s 陈旧） |
| 订阅变更 | `forum:following:{account}:v++` |
| 处罚/晋升 | `identity:sanction:{account}` 精准删除 |
| 词表变更 | 进程内 ArcSwap 重载广播（Redis pub/sub） |

### 7.3 实时性（F2，SSE 而非 MessageBus）

- `GET /notifications/stream`：SSE 长连接，事件 = 未读数变化 + 新通知摘要；
  SAE 单实例内用 `tokio::sync::broadcast`，多实例经 Redis pub/sub 扇出。
- 主题页"有 N 条新回复"提示：客户端 30s 轮询 `reply_count`（走边缘缓存），
  不为此建推送 —— 校园量级下轮询成本可忽略，符合"不做写扩散"的约定。

---

## 8. 明确不做（及理由）

| Discourse 能力 | 不做理由 |
|---|---|
| Email-in（邮件回帖/发帖） | 客户端是自研 App，无邮件工作流场景；SMTP 只用于验证码/摘要单向发送 |
| 多站点（multisite） | 单校区单站 |
| 插件 / 主题系统 | 客户端自研，扩展走 domain crate 演进，不需要运行时插件面 |
| Groups 与类目权限矩阵 | `user/mod/admin` + 板块 `min_trust_to_post`/`is_locked` 已覆盖校园场景；权限矩阵是复杂度陷阱 |
| Chat / 实时聊天室 | 与论坛+私信重叠；确有需求也是独立产品决策，不混入 forum 域 |
| TL4 与 Leader 自治 | 校园管理链路短，mod 任命走 admin，不需要社区自治晋升 |
| i18n 多语言 | 中文单语；DTO 文案由客户端管理 |
| Akismet 等外部反垃圾 | 校内邮箱实名底座 + TL + 词表 + flag 加权已构成闭环；数据不出境（PIPL） |

---

## 9. 分期路线图与验收（DoD）

### F0 — 接线修复（前置，不含新功能）
1. SMTP 通道（阿里云邮件推送）：验证码真实发送。
2. `notification_hooks` 接入 reply/vote 写路径（§5.4 前 4 类）。
3. `refresh_hot_rank` 进 bootstrap 调度（§4.2）。
4. Meili 索引 setup + 写路径同步 + admin reindex 去 stub（courses/reviews/forum 三索引）。
5. `credit::mint_for_contribution` 公开 API + reviews/forum 首批 mint 钩子（§6）。

**DoD**：新用户可用真实邮箱登录 → 发帖 → 被回复收到通知 → 被赞满 10 获得 mint
→ 搜索能搜到帖子 → 热榜 5 分钟内出现。

### F1 — 核心对齐（本文主体）
- migration 0005 全量落地（§2.1–2.14）。
- 编辑/软删/修订历史；tags；主题状态机 + 置顶；已读追踪；订阅三档 + following/unread feed；
  书签；@提及；flags + 阈值隐藏 + 审核队列；watched words；silence/suspend；
  TL0–3 晋升任务 + 新人限制 + 限流分档；管理端 §3.2 全量；mod_actions 留痕。

**DoD**：§1 矩阵中标 F1 的行全部 ✅；`cargo test --all` 覆盖：修订留痕、并发
已读前移、flag 阈值触发隐藏、TL 晋升边界、禁言拦截写路径、mod 操作必留痕。
契约（openapi.yaml）与实现同步。

### F2 — 体验对齐
- media crate + OSS 直传（STS 临时凭证）+ 图片审核态；草稿；引用通知；onebox；
  用户公开主页；屏蔽用户；SSE 通知流；badges + 加精 + mint 联动；通知偏好页。

### F3 — 增强
- 1:1 私信；投票贴；每周邮件摘要；陈旧主题自动归档；问答板 solved + 悬赏联动。

---

## 10. 工程约束提醒

- 每期改动遵守 AGENTS.md §3 Definition of Done；迁移 append-only；
  合规红线（§5/credit）任何时候优先于本文。
- forum crate 体量将显著增长：按 §2 的表拆 `repo/` 子模块
  （`repo/threads.rs`、`repo/flags.rs`、`repo/subscriptions.rs`…），
  handlers 同构拆分；`lib.rs` 只留路由表。
- 所有新写路径：限流 → 处罚检查 → 敏感词 → 业务事务（含计数增量 + mod_action/
  revision）→ 通知/搜索/缓存副作用（失败不回滚业务，warn + 补偿任务）。

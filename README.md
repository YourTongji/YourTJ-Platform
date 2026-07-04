# YourTJ Platform

同济大学校园社区平台（论坛为主；选课、评课、积分为子业务）的统一后端 monorepo。

- **后端**：Rust（Axum 0.8 + Tokio），Cargo workspace，按域拆 crate
- **数据库**：PolarDB（PostgreSQL 兼容），单库多 schema，append-only migration
- **检索**：Meilisearch（拼音 / 简称 / 别名）。**缓存 / 计数 / 限流 / 热榜**：Redis。**媒体**：OSS + CDN
- **部署**：阿里云华东（ICP 备案）· SAE 无状态容器，后续可换 SLB + ECS
- **身份**：校园邮箱验证码 + JWT；账号绑 Ed25519，仅资金操作签名
- **积分**：Web2.5 闭环虚拟权益 —— 中心账本 + Ed25519 签名 + 哈希链；**无充值 / 提现 / 自由转账**

## 目录结构

```
yourtj-platform/
├─ backend/                     # Rust workspace
│  ├─ crates/
│  │  ├─ api/                   # Axum 网关（进程入口，组合各域路由、后台任务）
│  │  ├─ identity/              # 账号 / 邮箱认证 / 会话 / Ed25519 公钥 / 禁言
│  │  ├─ courses/               # 课程目录 / 选课镜像 / 搜索 / Meilisearch
│  │  ├─ reviews/               # 评课 / 点赞 / 举报 / 审核
│  │  ├─ credit/                # Web2.5 积分账本（哈希链）/ escrow 市场
│  │  ├─ media/                 # OSS 上传、审核、回调
│  │  ├─ forum/                 # 论坛（板块 / 主题 / 楼中楼 / 投票 / 搜索）
│  │  │  ├─ handlers/           # route handlers（boards, threads, comments, votes, polls, DMs, 等）
│  │  │  ├─ repo/               # DB 查询层（每域一文件）
│  │  │  ├─ admin/              # 管理端路由
│  │  │  ├─ badges.rs           # 徽章自动颁发 + 积分 mint 桥接
│  │  │  ├─ digest.rs           # 每周邮件摘要
│  │  │  ├─ sanctions.rs        # 违禁词过滤
│  │  │  ├─ sse.rs              # 实时通知 SSE 推送
│  │  │  └─ trust_levels.rs     # 信任等级升降级
│  │  └─ shared/                # 配置 / 错误 / 分页 / 认证/ SSE 类型
│  ├─ migrations/               # PolarDB DDL（append-only，当前 8 个 migration）
│  └─ Dockerfile
├─ contract/openapi.yaml        # API 契约 → 生成 TS / Swift / Dart 类型
├─ infrastructure/              # SAE / Terraform / 部署
└─ docs/
   ├─ REWRITE_V2_DESIGN.md      # 全量 API 约定、DDL、缓存策略、积分合规
   └─ FORUM_DISCOURSE_PARITY.md # Discourse 功能对齐清单（F0-F4）
```

## 本地开发

```bash
# 启动依赖服务
docker compose up -d

# 后端
cd backend
cp .env.example .env
cargo run --bin api           # http://localhost:8080/health
cargo fmt --all               # 格式化
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --lib        # 单元测试
```

## API 概览

所有路由以 `/api/v2` 为前缀。完整契约见 `contract/openapi.yaml`。

### 认证（公开）

```bash
# 请求验证码
curl -X POST http://localhost:8080/api/v2/auth/email/request-code \
  -H "Content-Type: application/json" \
  -d '{"email":"student@tongji.edu.cn"}'

# 验证码登录
curl -X POST http://localhost:8080/api/v2/auth/email/verify \
  -H "Content-Type: application/json" \
  -d '{"email":"student@tongji.edu.cn","code":"123456"}'

# 刷新令牌（在 header 中传 refresh token）
curl -X POST http://localhost:8080/api/v2/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token":"..."}'

# 登出
curl -X POST http://localhost:8080/api/v2/auth/logout \
  -H "Authorization: Bearer <access_token>"
```

### 身份（需认证）

```bash
# 个人信息
curl http://localhost:8080/api/v2/me \
  -H "Authorization: Bearer <access_token>"

# 修改昵称 / 头像
curl -X PATCH http://localhost:8080/api/v2/me \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"handle":"my_new_handle"}'

# 用户公开资料
curl http://localhost:8080/api/v2/users/my_new_handle

# 绑定 Ed25519 公钥
curl -X POST http://localhost:8080/api/v2/wallet/bind \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"publicKey":"<base64_ed25519>"}'

# 查看积分余额
curl http://localhost:8080/api/v2/wallet \
  -H "Authorization: Bearer <access_token>"

# 草稿（自动保存）
curl http://localhost:8080/api/v2/me/drafts \
  -H "Authorization: Bearer <access_token>"

# 忽略用户
curl -X PUT http://localhost:8080/api/v2/me/ignores/2 \
  -H "Authorization: Bearer <access_token>"
```

### 课程 & 选课（公开）

```bash
# 课程列表
curl "http://localhost:8080/api/v2/courses?dept=数学科学学院&sort=hot&limit=20"

# 课程详情
curl http://localhost:8080/api/v2/courses/1

# 搜索
curl "http://localhost:8080/api/v2/search?q=高等数学&type=course&limit=10"

# 选课日历
curl http://localhost:8080/api/v2/selection/calendars
```

### 点评

```bash
# 课程点评列表
curl "http://localhost:8080/api/v2/courses/1/reviews?sort=hot"

# 发布点评（需认证）
curl -X POST http://localhost:8080/api/v2/courses/1/reviews \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"rating":4,"comment":"很好的课程","semester":"2024-2025-1"}'

# 点赞
curl -X POST http://localhost:8080/api/v2/reviews/1/like \
  -H "Authorization: Bearer <access_token>"
```

### 积分（需认证 + X-Wallet-Sig）

```bash
# 打赏
curl -X POST http://localhost:8080/api/v2/credit/tip \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Wallet-Sig: <base64_signature>" \
  -H "Content-Type: application/json" \
  -d '{"toAccountId":"2","amount":10,"targetType":"review","targetId":"1"}'

# 查看账本
curl "http://localhost:8080/api/v2/wallet/ledger" \
  -H "Authorization: Bearer <access_token>"

# 验证账本完整性（公开）
curl http://localhost:8080/api/v2/wallet/ledger/verify
```

### 论坛

```bash
# 板块列表
curl http://localhost:8080/api/v2/forum/boards

# 主题流（hot / new / unread / following）
curl "http://localhost:8080/api/v2/forum/threads?board=1&sort=hot"
curl "http://localhost:8080/api/v2/forum/threads?sort=unread" \
  -H "Authorization: Bearer <access_token>"

# 发帖（需认证）
curl -X POST http://localhost:8080/api/v2/forum/threads \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"boardId":"1","title":"Hello","body":"First post!"}'

# 评论（楼中楼，需认证）
curl -X POST http://localhost:8080/api/v2/forum/threads/1/comments \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"body":"Great post!","parentId":null}'

# 顶/踩
curl -X POST http://localhost:8080/api/v2/forum/posts/1/vote \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"value":"up"}'

# 订阅（watching / tracking / muted）
curl -X PUT http://localhost:8080/api/v2/forum/subscriptions \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"targetType":"thread","targetId":"1","level":"watching"}'

# 举报
curl -X POST http://localhost:8080/api/v2/forum/posts/1/flag \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"reason":"spam"}'

# 收藏
curl -X PUT http://localhost:8080/api/v2/forum/posts/1/bookmark \
  -H "Authorization: Bearer <access_token>"

# 通知
curl http://localhost:8080/api/v2/notifications \
  -H "Authorization: Bearer <access_token>"

# 实时通知（SSE）
curl -N http://localhost:8080/api/v2/notifications/stream \
  -H "Authorization: Bearer <access_token>"
```

## 设计文档

[`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md) — 全量 API 约定、DDL、搜索索引、缓存策略、积分合规。

## 路线图

- ✅ P0：Cargo workspace + 脚手架 + CI
- ✅ P1：Identity（邮箱验证码 + JWT + 会话 / 禁言 / 信任等级）
- ✅ P2：Courses / Selection（课程目录 + 选课 mirror）+ Reviews（点评 CRUD + 审核）+ Credit（账本 + 打赏 + escrow 市场 / 徽章 mint 桥接）
- ✅ P3：Forum（板块 / 主题 / 楼中楼 / 投票 / 搜索 / 通知 / 实时推送 / 收藏 / 订阅 / DM / 举报 / 草稿 / 忽略用户 / poll / 徽章自动颁发 / 积分桥接 / 站点设置 / 违禁词 / 邮件摘要）
- ✅ **[Forum Discourse 对齐](docs/FORUM_DISCOURSE_PARITY.md)**：P0-F4 全部完工
- 🔜 P4：Redis 缓存层（生产配置）、SMTP 就绪、Captcha 集成、一键部署

---

> 开发规范见 [`AGENTS.md`](AGENTS.md)。设计细节见 [`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md)。
> Forum 功能对齐清单：[`docs/FORUM_DISCOURSE_PARITY.md`](docs/FORUM_DISCOURSE_PARITY.md)。

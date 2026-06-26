# YourTJ Platform

同济大学校园社区平台（论坛为主业；选课、评课、积分为子业务）的统一后端 + Web 前端 monorepo。

- **后端**：Rust（Axum 0.8 + Tokio），Cargo workspace，按域拆 crate。
- **数据库**：PolarDB（PostgreSQL 兼容），单库多 schema。
- **检索**：Meilisearch（拼音/简称/别名）。**缓存/计数/限流**：Redis。**媒体**：OSS+CDN。
- **部署**：阿里云华东（ICP 备案）· SAE 无状态容器（serverless），后续可换 SLB+ECS。
- **身份**：校园邮箱验证码 + JWT；账号绑 Ed25519，仅资金操作签名。
- **积分**：Web2.5 闭环虚拟权益 —— 中心账本 + Ed25519 签名 + 哈希链；**无充值/提现/自由转账**。

## 目录结构

```
yourtj-platform/
├─ backend/                 # Rust workspace
│  ├─ crates/
│  │  ├─ api/               # Axum 网关二进制（进程入口，组合各域路由）
│  │  ├─ identity/          # 账号 / 邮箱认证 / 会话 / Ed25519 公钥 / 通知
│  │  ├─ courses/           # 课程目录 / 选课镜像 / 搜索 / Meilisearch
│  │  ├─ reviews/           # 评课 / 点赞 / 举报 / 审核
│  │  ├─ credit/            # Web2.5 积分账本（哈希链）/ escrow 市场
│  │  ├─ forum/             # 论坛（boards, threads, 楼中楼评论, votes）
│  │  └─ shared/            # 配置 / 错误 / 分页 / 认证
│  ├─ migrations/           # PolarDB DDL（append-only）
│  └─ Dockerfile
├─ web/                     # React v2 前端（占位）
├─ contract/openapi.yaml    # 单一 API 契约源 → 生成 TS/Swift/Dart 类型
├─ infrastructure/          # SAE / Terraform / 部署
└─ docs/                    # 设计文档
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

# 修改昵称
curl -X PATCH http://localhost:8080/api/v2/me \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"handle":"my_new_handle"}'

# 绑定 Ed25519 公钥
curl -X POST http://localhost:8080/api/v2/wallet/bind \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"publicKey":"<base64_ed25519>"}'

# 查看积分余额
curl http://localhost:8080/api/v2/wallet \
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

# 主题流
curl "http://localhost:8080/api/v2/forum/threads?board=1&sort=hot"

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
```

## 设计文档

[`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md) — 全量 API 约定、DDL、搜索索引、缓存策略、积分合规。

## 路线图

- ✅ P0：Cargo workspace + 脚手架 + CI
- ✅ P1：Identity（邮箱验证码 + JWT + 会话）
- ✅ P2：Courses / Selection（课程目录 + 选课 mirror）+ Reviews（点评 CRUD + 审核）+ Credit（账本 + 打赏 + escrow 市场）
- ✅ P3：Forum（板块 / 主题 / 楼中楼 / 投票）+ Notifications + Platform + 搜索
- 🔜 P4：Redis 缓存层、SMTP 发送、Captcha、Meilisearch 生产索引、一键部署

---

> 开发规范见 [`AGENTS.md`](AGENTS.md)。设计细节见 [`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md)。

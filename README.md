<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-F74C00?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Axum-0.8-0055FF?style=flat-square" alt="Axum">
  <img src="https://img.shields.io/badge/PostgreSQL-PolarDB-336791?style=flat-square&logo=postgresql&logoColor=white" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/Redis-caching%20%2B%20rate%20limit-DC382D?style=flat-square&logo=redis&logoColor=white" alt="Redis">
  <img src="https://img.shields.io/badge/Search-Meilisearch-FF5C83?style=flat-square" alt="Meilisearch">
  <img src="https://img.shields.io/badge/Edition-2021-555?style=flat-square" alt="Edition">
  <img src="https://img.shields.io/badge/license-Proprietary-lightgrey?style=flat-square" alt="License">
</p>

# YourTJ Platform

同济大学校园社区平台——论坛、选课、评课、积分，统一后端 monorepo。

[本地开发](#local-development) · [API 文档](docs/API_REFERENCE.md) · [设计文档](docs/REWRITE_V2_DESIGN.md) · [论坛对齐](docs/FORUM_DISCOURSE_PARITY.md)

> **YourTJ 产品矩阵** · [iOS（原生客户端）](https://github.com/YourTongji/YourTJCourse-iOS) · [Flutter（跨端版）](https://github.com/YourTongji/YourTJCourse-Flutter) · [Serverless（旧版 API）](https://github.com/YourTongji/YourTJCourse-Serverless) · [HomePage](https://github.com/YourTongji/YourTJ-HomePage)

---

## Features

| 模块 | 功能 | 状态 |
|------|------|------|
| **论坛** | 板块 / 主题流（hot·new·unread·following）/ 楼中楼 / 投票 / 搜索 / 通知（SSE 实时推送）/ 收藏 / 订阅（watching·tracking·muted）/ DM / 举报 / 草稿 / 忽略用户 | `✅ Stable` |
| **身份** | 校园邮箱验证码登录 / JWT / Ed25519 资金签名 / 信任等级 / 禁言 | `✅ Stable` |
| **课程** | 目录浏览 / 选课镜像 / 搜索（拼音·简称·别名）/ 院系列表 | `✅ Stable` |
| **评课** | 点评 CRUD / 点赞·取消 / 举报·审核 / 统计增量维护 / 旧版匿名点评兼容 | `✅ Stable` |
| **积分** | 中心账本（哈希链 + Ed25519 签名）/ 打赏·悬赏·托管市场 / 链上验证 / 徽章 mint 桥接 | `✅ Stable` |
| **媒体** | OSS 直传 STS / 回调·审核·封禁 / CDN 签名 URL | `✅ Stable` |
| **管理** | 选课同步 / 点评重索引 / 论坛·媒体审核 / 站点设置 / 违禁词管理 | `✅ Stable` |

---

## Architecture

```
┌─────────────────────────────────────────────┐
│  api  (Axum 网关)                            │
│  进程入口 · 路由组合 · 后台任务 · 启动引导     │
├─────────────────────────────────────────────┤
│  Domain Crates                               │
│  ┌──────────┬──────────┬──────────┬────────┐ │
│  │ identity │ courses  │ reviews  │ credit │ │
│  │ 账户·认证│ 课程·选课│ 评课·审核│ 积分账本│ │
│  ├──────────┼──────────┼──────────┼────────┤ │
│  │  forum   │  media   │          │        │ │
│  │ 论坛     │ OSS·上传 │          │        │ │
│  └──────────┴──────────┴──────────┴────────┘ │
├─────────────────────────────────────────────┤
│  shared                                      │
│  配置 · AppError · 分页 · JWT · 缓存 · 限流   │
└─────────────────────────────────────────────┘
```

**Key design decisions:**

- **按域拆 crate** — 每个 domain crate 拥有自己的表。跨域访问走 crate 的 public API，禁止直接从外部 SQL 触碰。
- **Append-only migrations** — `backend/migrations/NNNN_name.sql`，永不修改已应用的迁移。
- **增量统计** — `review_count` / `review_avg` 在写路径实时更新，从不全表 `AVG()` 重算。
- **版本号缓存失效** — Redis `INCR ver:key` 而非盲删，旧 key 随 TTL 自然过期。
- **Meilisearch 实时搜索** — 拼音·首字母·别名预计算推送，禁止 `LIKE %q%` 走 DB。

---

## Local Development

### Prerequisites

- Rust 1.80+（`rustup`）
- Docker（PostgreSQL、Redis、Meilisearch）

### Setup

```bash
# 启动依赖服务
docker compose up -d

# 后端
cd backend
cp .env.example .env
cargo run --bin api           # http://localhost:8080/health
```

### Definition of Done

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

> 集成测试需要 testcontainers（自动拉起临时 Postgres / Redis 容器）。

---

## Project Structure

```
yourtj-platform/
├── backend/
│   ├── crates/
│   │   ├── api/              # Axum 网关（路由组合 · 后台任务 · 管理端点）
│   │   ├── identity/         # 账户 · 邮箱认证 · JWT 会话 · Ed25519
│   │   ├── courses/          # 课程目录 · 选课镜像 · Meilisearch
│   │   ├── reviews/          # 评课 · 点赞 · 举报 · 审核队列
│   │   ├── credit/           # 积分账本（哈希链）· 托管市场
│   │   ├── media/            # OSS 上传 · 审核 · 回调
│   │   ├── forum/            # 论坛（板块·主题·楼中楼·投票·DM·通知）
│   │   │   ├── handlers/     # 路由处理器
│   │   │   ├── repo/         # 数据访问层
│   │   │   └── admin/        # 管理端路由
│   │   └── shared/           # 配置 · 错误 · 分页 · JWT · 缓存 · 限流
│   ├── migrations/           # PolarDB DDL（append-only）
│   ├── ops/                  # 运维脚本（选课物化等）
│   └── Dockerfile
├── contract/openapi.yaml     # API 契约 → 生成客户端类型
├── docs/
│   ├── REWRITE_V2_DESIGN.md  # 全量 API 约定 · DDL · 缓存 · 积分合规
│   ├── FORUM_DISCOURSE_PARITY.md  # Discourse 功能对齐清单
│   ├── API_REFERENCE.md      # API 参考（curl 示例）
│   ├── ARCH_REVIEW_AND_E2E_PLAN.md  # 架构审查 & E2E 计划
│   └── D1_LOCAL_IMPORT.md    # D1 数据导入指南
├── tools/d1/                 # D1 导入工具链
├── docker-compose.yml
├── AGENTS.md                 # 编码规范（AI agent + human）
└── README.md
```

---

## Tech Stack

| 层 | 选择 |
|----|------|
| **语言** | Rust 2021 edition（稳定版） |
| **Web 框架** | Axum 0.8 + Tower（middleware） |
| **数据库** | PolarDB PostgreSQL（sqlx 0.8, 单库多 schema） |
| **搜索** | Meilisearch（拼音·首字母·别名预计算） |
| **缓存·计数·限流·热榜** | Redis（deadpool-redis） |
| **媒体存储** | OSS + CDN |
| **身份认证** | 校园邮箱验证码 + JWT（HS256） |
| **资金签名** | Ed25519（ring crate） |
| **密码哈希** | Argon2 |
| **部署** | 阿里云华东 SAE（无状态容器）· Docker |
| **CI/CD** | GitHub Actions（fmt + clippy + test + build） |

---

## Domain Invariants

### 积分 Web2.5 — 合规红线

闭环虚拟权益，**不是**虚拟货币：
- **无充值·无提现·不与法币双向兑换**
- **无自由转账** — 价值仅通过 `mint`（系统增发）/ `escrow`（托管）/ `tip`（打赏）/ `bounty`（悬赏）流动
- 积分**纯靠贡献赚取**（发帖·评课·被点赞·悬赏完成）
- 账本 `credit.ledger` 只追加、哈希链连接、每条 Ed25519 签名
- 余额 `credit.wallets.balance` 是**派生缓存**，权威来自账本

### 隐私

- 公开 handle 展示，真实 email 仅服务端可见
- 最小化 PII 存储，支持账号删除
- 无设备指纹采集

---

## Documentation

| 文档 | 内容 |
|------|------|
| [`AGENTS.md`](AGENTS.md) | 编码规范（注释·命名·错误处理·测试·依赖·Git） |
| [`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md) | API 约定 · DDL · 搜索索引 · 缓存策略 · 积分设计 |
| [`docs/FORUM_DISCOURSE_PARITY.md`](docs/FORUM_DISCOURSE_PARITY.md) | Discourse 功能对齐清单（F0–F4） |
| [`docs/API_REFERENCE.md`](docs/API_REFERENCE.md) | API 参考 & curl 示例 |
| [`docs/ARCH_REVIEW_AND_E2E_PLAN.md`](docs/ARCH_REVIEW_AND_E2E_PLAN.md) | 架构审查 & E2E 测试计划 |
| [`docs/D1_LOCAL_IMPORT.md`](docs/D1_LOCAL_IMPORT.md) | D1 旧版数据导入指南 |
| [`contract/openapi.yaml`](contract/openapi.yaml) | OpenAPI 3.1 完整契约（单一权威源） |

---

## License

© 2026 YourTJ. All rights reserved.

# YourTJ Platform

同济大学校园社区平台（论坛为主业；选课、评课、积分为子业务）的统一后端 + Web 前端 monorepo。

- **后端**：Rust（Axum + Tokio），Cargo workspace，按域拆 crate。
- **数据库**：PolarDB（PostgreSQL 兼容），单库多 schema。
- **检索**：Meilisearch（拼音/简称/别名）。**缓存/计数/限流**：Redis。**媒体**：OSS+CDN。
- **部署**：阿里云华东（ICP 备案）· SAE 无状态容器（serverless），后续可换 SLB+ECS。
- **身份**：校园邮箱验证码 + JWT；账号绑 Ed25519，仅资金操作签名。
- **积分**：Web2.5 闭环虚拟权益 —— 中心账本 + Ed25519 签名 + 哈希链；**无充值/提现/自由转账**。

> 设计细节见 [`docs/REWRITE_V2_DESIGN.md`](docs/REWRITE_V2_DESIGN.md)。开发规范见 [`AGENTS.md`](AGENTS.md)。

## 目录结构

```
yourtj-platform/
├─ backend/                 # Rust workspace
│  ├─ crates/
│  │  ├─ api/               # Axum 网关二进制（进程入口，组合各域路由）
│  │  ├─ identity/          # 账号 / 邮箱认证 / 会话 / Ed25519 公钥
│  │  ├─ courses/           # 课程目录 / 选课镜像 / 搜索
│  │  ├─ reviews/           # 评课 / 点赞 / 举报 / 审核
│  │  ├─ credit/            # Web2.5 积分账本（哈希链）
│  │  ├─ forum/             # 论坛（Phase B）
│  │  └─ shared/            # 配置 / 错误 / 分页
│  ├─ migrations/           # PolarDB DDL（append-only）
│  └─ Dockerfile
├─ web/                     # React v2 前端（占位）
├─ contract/openapi.yaml    # 单一 API 契约源 → 生成 TS/Swift/Dart 类型
├─ infra/                   # SAE / Terraform / 部署
└─ docs/
```

iOS / Flutter 客户端保持各自独立仓库，只消费 `contract/` 生成的类型。

## 本地开发

```bash
cd backend
cp .env.example .env          # 按需填写 DATABASE_URL / REDIS_URL / MEILI_URL
cargo run --bin api           # 启动后访问 http://localhost:8080/health
cargo fmt --all               # 格式化
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

依赖服务（Postgres / Redis / Meilisearch）建议用本地容器起；接线在后续阶段补 `docker-compose`。

## 路线图

P0 地基（本脚手架）→ P1 身份统一 → P2 评课/选课/积分平迁 + 客户端切 v2 → P3 论坛 MVP → P4 打磨。

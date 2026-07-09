<p align="center">
  <img src="https://raw.githubusercontent.com/YourTongji/YourTJCourse-iOS/master/icon.png" width="96" alt="YourTJ">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-F74C00?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Axum-0.8-0055FF?style=flat-square" alt="Axum">
  <img src="https://img.shields.io/badge/PostgreSQL-PolarDB-336791?style=flat-square&logo=postgresql&logoColor=white" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/Redis-DC382D?style=flat-square&logo=redis&logoColor=white" alt="Redis">
  <img src="https://img.shields.io/badge/Search-Meilisearch-FF5C83?style=flat-square" alt="Meilisearch">
  <img src="https://img.shields.io/badge/license-Proprietary-lightgrey?style=flat-square" alt="License">
</p>

# YourTJ Platform

同济大学校园社区平台——论坛、选课、评课、积分，统一后端 monorepo。

> **YourTJ 产品矩阵** &nbsp; [iOS 客户端](https://github.com/YourTongji/YourTJCourse-iOS) · [Flutter](https://github.com/YourTongji/YourTJCourse-Flutter) · [Serverless（旧版）](https://github.com/YourTongji/YourTJCourse-Serverless) · [HomePage](https://github.com/YourTongji/YourTJ-HomePage)

## 项目结构

```
yourtj-platform/
├── backend/
│   ├── crates/
│   │   ├── api/              # Axum 网关（路由组合 · 管理端点 · 后台任务）
│   │   ├── identity/         # 账户 · 邮箱认证 · JWT · Ed25519
│   │   ├── courses/          # 课程目录 · 选课镜像 · Meilisearch
│   │   ├── reviews/          # 评课 CRUD · 点赞 · 举报 · 审核
│   │   ├── credit/           # 积分账本（哈希链）· 托管市场
│   │   ├── media/            # OSS 上传 · 审核 · 回调
│   │   ├── forum/            # 论坛（板块·主题·楼中楼·投票·DM）
│   │   │   ├── handlers/     # 路由处理器
│   │   │   ├── repo/         # 数据访问层
│   │   │   └── admin/        # 管理路由
│   │   └── shared/           # 配置 · 错误 · 分页 · 缓存 · 限流
│   ├── migrations/           # PolarDB DDL（append-only）
│   ├── ops/                  # 运维脚本（选课物化等）
│   └── Dockerfile
├── contract/openapi.yaml     # API 契约 → 生成客户端类型
├── docs/                     # 设计文档 · API 参考 · 对齐清单
├── tools/d1/                 # D1 旧版数据导入工具链
├── docker-compose.yml
└── AGENTS.md                 # 编码规范
```

## 本地开发

```bash
# 启动依赖服务
docker compose up -d

# 后端
cd backend
cp .env.example .env
cargo run --bin api           # http://localhost:8080/health
```

### 提交前检查

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

---

© 2026 YourTJ. All rights reserved.

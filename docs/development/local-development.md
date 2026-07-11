# 本地环境

> 文档类型：开发指南
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

## 前置工具

- Docker with Compose
- Rust stable toolchain，workspace MSRV 为 1.80；`backend/rust-toolchain.toml` 安装 rustfmt/clippy
- Node.js 22
- pnpm 11.11.0
- PostgreSQL client/sqlx-cli（运行 migration 与 CI-parity integration tests 时）

```bash
cargo install sqlx-cli --no-default-features --features postgres --locked
corepack enable
corepack prepare pnpm@11.11.0 --activate
```

## 建立安全工作区

不要直接在 `main` 开发。已有 checkout 有未提交内容时，优先新 worktree：

```bash
git fetch origin main
git worktree add -b codex/<topic> ../YourTJ-Platform-<topic> origin/main
cd ../YourTJ-Platform-<topic>
```

分支和提交规则见[Pull Request 指南](pull-requests.md)。

## 启动依赖

```bash
docker compose up -d
docker compose ps
```

Compose 启动 local PostgreSQL、Redis 和 Meilisearch。Schema 由 backend 的 sqlx migrator 维护；
PostgreSQL initdb 不再裸执行 migration 文件，避免与 `_sqlx_migrations` ledger 重放冲突。

首次复制 backend 配置：

```bash
cd backend
cp .env.example .env
```

`.env.example` 提供仅供 local 的 dev signing seed，并让 `MEILI_MASTER_KEY` 与 Compose 对齐。真实环境
必须替换所有 signing/JWT/email/OSS keys，且不能提交 `.env`。

## 启动 backend

```bash
cd backend
set -a
source .env
set +a
cargo run --bin api
```

API 在 `http://localhost:8080`，健康检查为：

```bash
curl --fail http://localhost:8080/api/v2/health
```

应用不会自动解析 `.env`，因此当前 shell 必须先 export 上述变量。Startup 会应用未运行的 sqlx
migrations。Migration 失败时先读具体错误，不要手工修改
`_sqlx_migrations` 或编辑已经应用的 SQL 文件。

## 启动 Web

```bash
cd web
pnpm install --frozen-lockfile
pnpm run generate:api
pnpm run dev
```

Vite 默认把 `/api` 代理到 `http://localhost:8080`。需要其他 gateway 时使用
`VITE_API_BASE_URL`；captcha endpoint 用 `VITE_CAPTCHA_URL` 覆盖。

## Local provider 行为

- Email 默认 `log`，只记录 redacted metadata，不发送真实 code。
- OSS 未配置时 media routes fail closed；本地 UI 不应假装上传成功。
- Captcha 默认访问配置的 YourTJCaptcha；离线开发受保护写入会失败，不能在生产代码加绕过。
- Meilisearch 使用 Compose 的 `dev-master-key`；backend `.env` 必须一致。

## 测试数据库

Integration tests 会清理共享表。绝不能指向个人开发库、staging 或 production。首次创建独立库：

```bash
docker compose exec postgres createdb -U yourtj yourtj_test
cd backend
DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj_test \
  sqlx migrate run --source migrations
```

精确测试命令见[测试策略与命令](testing.md)。

## 清理

```bash
docker compose down
```

只有确认 local volume 可以永久删除时才运行：

```bash
docker compose down -v
```

这会删除 local PostgreSQL、Redis 和 Meilisearch 数据，不可恢复。

## 常见问题

- **API startup 拒绝 signing key**：确认使用更新后的 `.env.example`，不要把 production key 放本地。
- **Meili 401**：确认 `.env` 与 Compose master key 一致。
- **Migration already exists**：旧 local volume 可能来自 initdb 裸 SQL；备份需要的数据后重建 local volume，
  不篡改 migration ledger。
- **Integration test 相互干扰**：使用 `yourtj_test` 并按 CI 串行运行。
- **生成类型有 diff**：contract 或 generated schema 漂移；提交正确生成文件，不手改 schema.ts。

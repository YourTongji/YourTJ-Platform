<p align="center">
  <img src="https://raw.githubusercontent.com/YourTongji/YourTJCourse-iOS/master/icon.png" width="96" alt="YourTJ">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-F74C00?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Axum-0.8-0055FF?style=flat-square" alt="Axum">
  <img src="https://img.shields.io/badge/PostgreSQL-PolarDB-336791?style=flat-square&logo=postgresql&logoColor=white" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/Redis-DC382D?style=flat-square&logo=redis&logoColor=white" alt="Redis">
  <img src="https://img.shields.io/badge/Search-Meilisearch-FF5C83?style=flat-square" alt="Meilisearch">
  <img src="https://img.shields.io/badge/Flutter-3.44.6-02569B?style=flat-square&logo=flutter&logoColor=white" alt="Flutter 3.44.6">
  <img src="https://img.shields.io/badge/license-Proprietary-lightgrey?style=flat-square" alt="License">
</p>

# YourTJ Platform

同济大学校园社区平台：论坛为核心，选课、评课与闭环积分共享身份、数据和治理能力。

> **YourTJ 产品矩阵**： [当前 Flutter Android/iOS 客户端](mobile/) ·
> [历史 iOS 选课客户端](https://github.com/YourTongji/YourTJCourse-iOS) ·
> [历史 Flutter 选课客户端](https://github.com/YourTongji/YourTJCourse-Flutter) ·
> [旧版 Serverless](https://github.com/YourTongji/YourTJCourse-Serverless) ·
> [HomePage](https://github.com/YourTongji/YourTJ-HomePage)

## 仓库结构

```text
backend/
  crates/
    api/          Axum gateway、startup 与 router composition
    identity/     账号、邮箱/密码认证、session、keys
    courses/      课程目录、选课镜像与课程搜索
    reviews/      课评、互动、举报与审核
    credit/       闭环积分 ledger 与受控 escrow flows
    forum/        板块、主题、评论、互动、通知与私信
    media/        OSS upload intent、callback 与 quarantine
    activity/     每日贡献 projection 与计分策略
    governance/   跨域 staff/system audit
    platform/     公告、用户 receipt、首页推广与 runtime settings
    shared/       配置、错误、auth primitive、分页、缓存与限流
    e2e/          旅程测试 harness
  migrations/    Append-only PostgreSQL migrations
  ops/           可重放物化脚本
web/              React + TypeScript Web
mobile/           Flutter Android/iOS 客户端
contract/         OpenAPI wire contract
docs/             产品、架构、开发、运维与安全规范
tools/d1/         Cloudflare D1 选课快照导入工具
.agents/skills/   仓库级 Codex 工作流
```

`mobile/` 是 proprietary clean-room 实现。FluxDO 和历史 YourTJ 客户端只用于观察产品需求与交互，
它们的源码、资产、生成文件和 Git 历史不会复制进本仓库；任何未来代码复用都必须先有明确的许可证与
版权授权记录。移动端当前状态和 Web 对齐范围见
[Flutter 移动端产品规范](docs/product/mobile-client.md)。

## 文档

- [文档中心](docs/README.md)
- [成熟社区能力模型](docs/product/community-capability-model.md)
- [当前缺口与路线图](docs/product/current-state-and-roadmap.md)
- [开发入口](docs/development/README.md)
- [测试命令](docs/development/testing.md)
- [Pull Request 流程](docs/development/pull-requests.md)

开发前还必须阅读 [AGENTS.md](AGENTS.md) 和需求对应的产品/安全规范。

## Local quick start

```bash
docker compose up -d

cd backend
cp .env.example .env
set -a
source .env
set +a
cargo run --bin api
```

另一个 terminal：

```bash
cd web
pnpm install --frozen-lockfile
pnpm run generate:api
pnpm run dev
```

启动 Flutter 客户端（Flutter 3.44.6 stable / Dart 3.12.2）：

```bash
cd mobile
flutter pub get --enforce-lockfile
../scripts/generate_mobile_api.sh
flutter run
```

生成脚本固定并校验 OpenAPI Generator，产物位于 `mobile/packages/yourtj_api`。客户端仍为 `Partial`；
主要普通/管理旅程已接真实 API，但逐项后端契约与 golden/integration/device/release 证据仍以移动端
产品规范的 19 项矩阵为准。

详细前置工具、测试数据库和 provider 行为见[本地环境](docs/development/local-development.md)。

## 提交前

```bash
python3 scripts/check_docs.py

cd backend
cargo fmt --all --check
cargo clippy --all-targets --all-features -- --deny warnings
cargo test --lib

cd ../web
pnpm run generate:api
pnpm run test:run
pnpm run lint
pnpm run typecheck
pnpm run build

cd ../mobile
flutter pub get --enforce-lockfile
git diff --exit-code -- pubspec.lock
../scripts/generate_mobile_api.sh
git diff --exit-code -- packages/yourtj_api
dart format --output=none --set-exit-if-changed lib test
flutter analyze --fatal-infos --fatal-warnings
flutter test
flutter build apk --debug
flutter build apk --release
# macOS/Xcode only
flutter build ios --debug --no-codesign
flutter build ios --release --no-codesign
```

Database integration tests 需要专用测试库并串行运行；iOS build 需要 macOS/Xcode。完整 CI-parity
命令见测试文档。

---

© 2026 YourTJ. All rights reserved.

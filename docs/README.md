# YourTJ 文档中心

> 文档类型：文档索引
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

这里是 YourTJ 产品、架构、开发、运维与安全规范的统一入口。Git 历史承担归档职责；
已经失真的阶段计划、PR 交付清单和重复的 API/DDL 快照不在当前文档树中长期保留。

## 如何判断事实来源

不同问题由不同来源负责，不建立一个会混淆“目标”和“现状”的总排序：

| 问题 | 权威来源 |
|---|---|
| 产品应该如何工作 | `docs/product/` |
| 安全、隐私、合规硬约束 | `AGENTS.md` 与 `docs/security/` |
| HTTP 请求和响应结构 | `contract/openapi.yaml` |
| 已部署数据库结构 | `backend/migrations/` 中按编号追加的 migration |
| 当前代码实际行为 | 源码、自动化测试和部署版本 |
| 开发、测试和 PR 流程 | `docs/development/` |
| 部署、导入和故障处置 | `docs/operations/` |

这些来源不一致时，不应选择一个方便的版本继续开发。应把差异视为缺陷，在同一个 PR 中
修正契约、实现、测试和相关文档，或明确记录为 `Partial`。

## 实现状态词

产品文档只使用以下四种实现状态：

- `Current`：用户可达的必要链路、后端约束和相应验证均存在。
- `Partial`：只有部分层完成；必须写明缺少 Web、API、schema、worker、运营流程或测试中的哪一层。
- `Planned`：目标业务规则已形成，但尚未交付，不能在界面或宣传中声称可用。
- `Decision needed`：数据模型、权限或产品方向仍需负责人决策；决策前不得擅自实现。

状态标注作用于具体、可验证的行为，不作用于整个大领域。一个领域可以同时有 `Current` 的后端
基础和 `Partial` 的端到端产品链路；前者不能被用来宣称整个功能已经完成。

文档自身使用 `Active`、`Draft`、`Deprecated` 表示生命周期。它与功能实现状态是两件事。
禁止使用合并后立即失真的 PR-relative “本次已交付/以后再做”标签作为长期状态。

## 目录

### 产品

- [产品愿景与原则](product/vision-and-principles.md)
- [成熟社区能力模型](product/community-capability-model.md)
- [当前能力、缺口与路线图](product/current-state-and-roadmap.md)
- [设计系统与无障碍](product/design-system-and-accessibility.md)
- [身份、登录与账号生命周期](product/identity-and-access.md)
- [个人资料、社交图与隐私](product/social-profile-and-privacy.md)
- [内容、媒体与发现](product/content-media-and-discovery.md)
- [通知、公告与私信](product/notifications-announcements-and-messaging.md)
- [信任安全、治理与管理后台](product/trust-safety-and-administration.md)
- [每日活跃度计分](product/activity-scoring.md)

### 架构

- [系统概览与域边界](architecture/system-overview.md)
- [契约、数据与派生投影](architecture/contracts-and-data.md)

### 开发

- [开发入口](development/README.md)
- [本地环境](development/local-development.md)
- [测试策略与命令](development/testing.md)
- [分支、提交与 Pull Request](development/pull-requests.md)
- [文档治理](development/documentation.md)

### 运维与安全

- [部署与 PR Preview](operations/deployment-and-previews.md)
- [邮件发送](operations/email-delivery.md)
- [OSS 媒体存储](operations/media-storage.md)
- [D1 选课快照导入](operations/data-import.md)
- [授权与审计](security/authorization-and-audit.md)
- [隐私与数据生命周期](security/privacy-and-data-lifecycle.md)

## 推荐阅读路径

- 产品规划：愿景与原则 → 成熟社区能力模型 → 当前能力与路线图 → 对应领域规范。
- 业务开发：`AGENTS.md` → 开发入口 → 对应产品规范 → 架构规范 → 测试与 PR 规范。
- API 变更：对应产品规范 → `contract/openapi.yaml` → 契约与数据规范 → 测试规范。
- 线上操作：对应运维 runbook；不要从历史 PR、聊天记录或本地笔记执行生产操作。

## 维护底线

- 不在 prose 中复制完整 OpenAPI、DDL 或随代码变化的端点数量。
- 不新增根目录临时报告；内容必须进入上述分类，或保留在 PR/Issue 中。
- 每个 PR 必须更新受影响文档，或在 PR 中写明 `Docs impact: none` 及理由。
- 删除过期文档前先把仍有效的规则迁入当前规范；Git 历史已经是可追溯的归档。
- 运行 `python3 scripts/check_docs.py` 检查目录、元数据、状态词和本地链接。

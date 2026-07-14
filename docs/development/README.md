# 开发入口

> 文档类型：开发指南
>
> 状态：Active
>
> 负责人：Platform maintainers
>
> 最近核验：2026-07-14，Flutter Android/iOS monorepo workflow 与 CI gates

任何 backend、Web、Flutter、契约、migration、CI 或文档变更都从这里开始。`AGENTS.md` 保存仓库硬约束，本目录
保存可执行流程；不要从历史 PR 或聊天记录复制开发步骤。

## 开始前

1. 阅读根目录 `AGENTS.md`、[文档索引](../README.md)和与需求直接相关的产品/安全规范。
2. 确认请求是只读分析、实现变更，还是还明确授权 commit/push/开 PR。
3. 检查 branch、worktree 和未提交内容；不得覆盖或提交他人改动。
4. 从 `origin/main` 创建 feature/fix/docs branch；Codex 默认使用 `codex/<topic>`。
5. 写出 change impact：backend、Web、Flutter Android/iOS、HTTP、schema、auth/PII、credit、
   search/cache、media、deploy、docs。

仓库级 `$yourtj-development` skill 位于 `.agents/skills/yourtj-development`，用于统一上述流程、
验证和 PR 交付。

## 标准工作流

```text
需求与产品语义
  -> 影响与风险边界
  -> contract/migration（如需要）
  -> owner domain 实现
  -> 已落库的 Web/Dart generated clients 与实际 consumers
  -> focused tests
  -> scope-wide CI-parity checks
  -> 文档影响与 diff review
  -> commit/push/PR（仅在明确授权后）
  -> CI + preview 验证
```

## 详细指南

- [本地环境](local-development.md)
- [测试策略与命令](testing.md)
- [Flutter 移动端产品规范](../product/mobile-client.md)
- [分支、提交与 Pull Request](pull-requests.md)
- [文档治理](documentation.md)
- [契约、数据与派生投影](../architecture/contracts-and-data.md)

## 完成定义

- 产品语义、权限、失败/恢复、隐私与保留没有未说明的空白。
- 代码在正确 domain，OpenAPI、Web/Dart 生成类型、migration 与实现一致；两个 client generator 都应
  regenerate 后保持 worktree clean，不得以手写 DTO 掩盖。
- Flutter 改动同时维持 Android 与 iOS，按 scope 通过 format、analyze、unit/widget 和两平台 build；
  golden、integration 或商店发布未运行时明确记录。
- 相关测试和 CI-parity checks 真实运行并记录结果。
- 受影响的产品、架构、安全或运维文档同步更新；无影响则说明理由。
- Diff 只包含本任务内容，无 secret、PII、generated garbage 或本地 artifact。
- 如果开 PR，reviewer 能从 PR body 理解行为、风险、验证、preview 和 rollback。

## 移动端来源边界

`mobile/` 是 proprietary clean-room 实现。FluxDO 和历史 YourTJ iOS/Flutter 客户端可以帮助确认用户
任务、信息层级和选课领域问题，但不得复制源码、资产、生成文件或 Git 历史。新增 package 从官方
registry/source 获取并审查许可证；任何参考项目代码复用都必须先形成明确的版权与分发决定。

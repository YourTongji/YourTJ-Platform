## 需求与摘要

<!-- 解决什么用户/业务问题？关联 Issue/需求；没有则写 N/A。 -->

## 产品行为与不变量

<!-- 说明参与者、权限、状态转换、失败/恢复行为和明确不做的内容。 -->

## 影响矩阵

- [ ] 产品行为或 Web UI
- [ ] `contract/openapi.yaml` 与 Web 生成类型
- [ ] PostgreSQL migration、backfill 或兼容窗口
- [ ] 身份、授权、PII、隐私、保留或审计
- [ ] 积分合规、签名、防重放或 ledger 完整性
- [ ] Media/OSS、search、cache、counter、notification 或 background job
- [ ] 依赖、配置、CI、部署或外部 provider

逐项说明勾选内容，以及高风险但未勾选项目为何不受影响：

## 文档影响

<!-- 链接受影响文档；如无，写 `Docs impact: none` 并解释为何行为、契约、schema、安全、运维和开发流程都未改变。 -->

## 验证

<!-- 列出实际命令与结果；未运行或 skip 的检查也要写原因。 -->

- [ ] `python3 scripts/check_docs.py`
- [ ] Backend focused 与 CI-parity checks（如适用）
- [ ] Web generate/lint/typecheck/build（如适用）
- [ ] Fresh migration 与数据/并发检查（如适用）
- [ ] Desktop/mobile/accessibility 人工验收（如适用）

## Migration、部署与回滚

<!-- Forward compatibility、backfill、preview/main 配置、外部副作用、smoke journey 和 rollback/roll-forward。 -->

## Preview 与证据

<!-- Preview URL、验证的路由/账号/状态、截图或录屏。不得包含 secret 或 PII。 -->

## 已知限制与 Review 重点

<!-- 哪些内容明确后置？Reviewer 应重点检查什么？ -->

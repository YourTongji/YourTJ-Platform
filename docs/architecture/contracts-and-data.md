# 契约、数据与派生投影

> 文档类型：架构规范
>
> 状态：Active
>
> 负责人：Platform maintainers、Domain maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

本规范说明产品规则如何落实为 HTTP 契约、migration、domain API、事务和可重建投影。它不复制
完整 OpenAPI 或 DDL。

## 从需求到实现的顺序

1. 在对应产品文档确定参与者、状态、权限、失败/恢复、隐私和验收。
2. HTTP surface 先改 `contract/openapi.yaml`，包括 request/response、错误、分页和安全要求。
3. Schema 只新增下一个编号 migration，说明现有数据、并发、回滚意图和部署兼容性。
4. 在 owner crate 实现 repository/service/handler；`api` 只 merge routes 和 wiring。
5. 重新生成 Web 类型，接入所有需要的客户端与管理 UI。
6. 从 focused tests 开始，再跑对应 CI-parity checks。
7. 同步 current-state、产品、安全或运维文档，并在 PR 写明影响。

## HTTP 契约

- `contract/openapi.yaml` 是 wire contract 唯一事实源；prose 解释业务语义但不复制完整 schema。
- 路由版本为 `/api/v2`；Axum 0.8 path param 使用 `{id}`。
- DTO 显式命名并使用 camelCase；timestamp 传 Unix seconds，日期类字段按产品规范。
- 错误统一 `{ "error": { "code", "message" } }`；客户端依赖稳定 code，而不是数据库消息。
- 列表使用有界 cursor/page envelope；不能返回无界数组。
- Viewer-specific state 必须显式建模，不能让 Web 根据计数或缺失字段猜测。
- 兼容变更优先 additive；破坏性变化需要版本/双写/迁移计划，不能静默改变历史字段语义。
- OpenAPI 变化后从 `web/` 运行 `pnpm run generate:api` 并提交生成的 schema。

## Migration 与数据所有权

- migration append-only；已在任何环境运行的文件不得修改。
- 每张表有一个 owner domain；外域不通过手写 SQL 触及其私有表。
- DB row 与 API DTO 分离；PII/内部状态不能因 `Serialize` 直接暴露。
- 多行 invariant、money、状态转换、counter 和 audit 在事务中提交。
- SQL 全部使用 bound parameters；输入先做长度、范围、枚举和 ownership 校验。
- 新 nullable/默认值必须说明 backfill、读旧写新和最终收紧策略。
- 删除语义区分软删除、匿名化、retention purge 与法律保留。

Fresh database 必须只通过 sqlx migration ledger 建立。普通启动、CI 和运维不能同时用裸 psql
重复执行同一组文件；开发流程见[本地环境](../development/local-development.md)。

## 状态机与幂等

- 业务状态使用受约束 enum/check 和显式转换，不以多个模糊 boolean 代替。
- 外部可重试写入使用 idempotency key 或稳定 source key；同 key 不同 payload 返回 conflict。
- “已经是目标状态”是幂等成功还是冲突，由产品规范明确，不交给每个 handler 自由决定。
- 反向动作追加 reversal/history，不覆盖需要审计的原事件。
- 并发转换使用 row/advisory lock、unique constraint 或 compare-and-set，不只依赖前端禁用按钮。

## Outbox 与后台任务

业务事务需要可靠触发搜索、通知、媒体或其他跨域副作用时，写入 transactional outbox。consumer：

- 通过 event id/source key 幂等；
- 有 queued/running/succeeded/failed 状态、重试上限和 dead-letter；
- 记录不含 PII/secret 的 bounded error；
- 支持 reconciliation 比较事实源和 projection；
- 失败不伪装成成功，也不让 API 请求无限等待外部供应商。

当前部分索引/通知路径仍使用 `tokio::spawn`，属于迁移目标，不是推荐的新模式。

## 搜索、缓存与计数

- PostgreSQL 是权限、内容和当前状态事实源；Meilisearch 文档可全部删除重建。
- 索引只包含搜索所需最小字段；返回前应用 status、visibility、privacy、block/mute policy。
- Full reindex 等待 clear task 成功后再 add，并观察 add 结果。
- Hot/search counter 使用增量/投影，读路径避免全表聚合；定期 reconciliation 纠偏。
- Redis cache key 版本化或短 TTL，mutation 精确 bump 相关 version；缓存故障不改变业务写入事实。
- 不使用 `LIKE %q%` 作为热点中文聚合搜索降级，除非产品/性能测试定义了严格有界范围。

## 内容与媒体契约

- 内容携带 `contentFormat`；legacy `plain_v1` 不自动解释为 `markdown_v1`。
- 服务端验证 canonical source，客户端 preview 不构成安全边界。
- Media credential 只允许 account-bound exact object key；callback 原子消费 intent。
- 业务保存 asset/reference，不保存任意 URL；访问 URL 是带权限和到期的派生值。
- asset status、binding、owner、target、alt、variants 与 retention 由 media/domain API 协作维护。

## 身份、隐私与审计

- 公开 handle 与内部 account id 可跨域使用，校园邮箱只在 identity 的目的限定接口中处理。
- 新 PII migration 同时更新[隐私与数据生命周期](../security/privacy-and-data-lifecycle.md)。
- Staff write 记录 actor kind/id/role、action、target、reason、result 和 correlation；metadata 最小化。
- Secrets、code、token、signature-as-credential、raw email、完整请求 body 和任意 DM 不进入日志/审计。
- Evidence read 本身是敏感动作，需要 capability、purpose 和 audit。

## 积分不变量

- `credit.ledger` 是 append-only 权威；wallet balance 是可重建 projection。
- Append 序列化，验证 prev hash、canonical payload、signature、nonce 和 signing intent。
- System mint 和用户受控操作使用明确 signer；私钥从 runtime secret 注入。
- 不新增 recharge、withdraw、fiat conversion 或 free transfer；冲突需求必须停止并升级确认。

## Change impact matrix

| 变更 | 同一 PR 的必需产物 |
|---|---|
| HTTP | 产品语义、OpenAPI、实现、生成类型、客户端、contract/handler tests |
| Schema | 新 migration、owner code、fresh-up 验证、兼容/回填说明、相关架构/产品文档 |
| 权限/治理 | capability、负向测试、reason/audit/notification、产品与安全规范 |
| PII/保留 | data purpose、visibility、retention/export/delete、privacy review |
| Search/cache/counter | 事实源、投影写入、失效、reindex/reconcile 和隐私过滤测试 |
| Media | asset state/binding、OSS policy、URL authorization、cleanup 和安全测试 |
| Credit | 合规确认、签名/重放/边界测试、ledger verification |

精确验证命令见[测试策略与命令](../development/testing.md)。

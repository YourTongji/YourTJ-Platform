# 文档治理

> 文档类型：开发流程
>
> 状态：Active
>
> 负责人：Platform maintainers、Product owner
>
> 最近核验：2026-07-11，`origin/main@33584db`

文档的目标是让产品规则、当前实现和可执行流程各有一个事实来源。Git 历史保存旧方案；当前树
不保留已经失真的 PR 计划和重复快照。

## 分类

| 路径 | 内容 | 不应出现 |
|---|---|---|
| `docs/product/` | 用户行为、业务规则、状态机、权限语义、验收 | 完整 OpenAPI/DDL、部署命令 |
| `docs/architecture/` | 域边界、数据流、跨切面工程决策 | PR 完成清单、产品宣传 |
| `docs/development/` | 本地开发、测试、PR、文档维护 | 生产 secret、一次性诊断 |
| `docs/operations/` | 可执行部署/导入/故障 runbook | 未落地目标架构冒充 current |
| `docs/security/` | 授权、隐私、合规和威胁边界 | 任意 credential、无 owner 愿望清单 |
| colocated `README.md` | 单个目录/工具的最短入口 | 重复领域规范 |

根目录只保留 `README.md` 和 `AGENTS.md`。临时 QA 报告、截图索引和阶段计划放在 PR/Issue；
可复用结论迁入正确分类。

## 文件与元数据

- 文件名使用 lowercase kebab-case；`docs/README.md` 是唯一例外。
- 每个 `docs/` 下的 canonical 文档只有一个 H1，开头包含：`文档类型`、`状态`、`负责人`、`最近核验`。
- `最近核验` 写日期和 commit/ref；不要写“本 PR”或随合并失效的相对描述。
- 元数据中的 `状态` 只描述文档生命周期，使用 `Active`、`Draft`、`Deprecated`；Deprecated 必须链接
  替代文档并尽快删除。不要把功能是否上线写进这项元数据。
- 文档正文中的能力实现状态只用 `Current`、`Partial`、`Planned`、`Decision needed`；不要用这些词
  替代文档生命周期。
- 命令、字段和状态使用 code formatting；产品/运维 prose 默认中文，代码标识保持原语言。

## 一份领域文档应回答什么

按相关性覆盖：目标与 non-goals、actors/capabilities、用户旅程、状态机、数据/API owner、可见性、
滥用与治理、通知/副作用、失败与恢复、保留/删除、指标、当前状态、未决决策和验收基线。

不必机械复制全部标题，但不能因为“以后再说”而漏掉权限、失败、隐私或删除。

## PR 文档影响

| 变更 | 必须同步 |
|---|---|
| 用户行为/UI flow | 对应产品规范；交付后更新 current-state 状态 |
| HTTP | OpenAPI、生成类型、产品语义与测试说明 |
| Schema | migration、架构/产品 data ownership 与 rollout |
| 权限/治理/PII | 产品 + security 的 capability、audit、retention、negative cases |
| Config/provider/deploy | `.env.example` 和对应 operations runbook |
| Test/CI/dev workflow | `docs/development/`、AGENTS/skill/CI 如相关 |
| Internal refactor | 可写 `Docs impact: none`，但必须说明无行为/contract/schema/security/ops 影响 |

每个 PR 必须更新文档或解释 none。不能为了满足检查机械修改日期，也不能把 Planned 改成 Current
而没有用户主流程、必要后台和验证。

## 避免重复与失真

- 不在 prose 复制完整 OpenAPI path/schema；直接链接 `contract/openapi.yaml` 并解释语义。
- 不复制 migration DDL、表/端点总数或会频繁漂移的 line number。
- Colocated README 只说明如何运行该目录工具，并链接 canonical product/operations doc。
- 历史方案中的仍有效规则先迁入 Active 文档，然后删除旧文件；Git history 可追溯来源。
- 一次性视觉 QA 证据放 PR，不引用不可复现的 `/tmp` 文件。
- `TODO` 属于 Issue/roadmap；Active 文档用 Planned/Decision needed 说明业务状态。

## 图表与链接

- 只有关系、状态或依赖用 prose 难以表达时才使用 Mermaid。
- Mermaid 添加 `accTitle` 和 `accDescr`，不使用主题 init 或 inline style。
- 相对链接指向 repository 文件；外部链接优先官方/原始来源。
- 删除/移动文档时用 `rg` 更新源码注释、README、AGENTS 和其他 docs 引用。

## 自动检查

```bash
python3 scripts/check_docs.py
```

检查范围包括分类、文件名、元数据、单 H1、本地链接和过期 PR-relative 状态词。CI 也运行该命令。
自动检查不能判断内容正确性；领域 owner 仍需核对 code/OpenAPI/migration 和产品语义。

## Review checklist

- 读者能区分目标、当前实现和未决决策。
- 权限、错误、恢复、隐私与保留没有只写 happy path。
- 链接和命令可执行，source/ref 真实存在。
- 没有 secret、真实 PII、无界数据样例或临时本地路径。
- 没有第二份 API/DDL/状态事实源。

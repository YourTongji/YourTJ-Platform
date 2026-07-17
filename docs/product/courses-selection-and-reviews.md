# 课程、选课、课表与评课

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Courses/Reviews/Web/Mobile maintainers、Product owner
>
> 最近核验：2026-07-17，真实 D1 快照物化、历史评分聚合、搜索 readiness 与 Web/Flutter 课表

本规范定义课程目录、教学班镜像、本机课表和评课之间的事实边界。四者服务同一个校园旅程，
但不是同一张表或同一种标识；客户端不得用名称、数组位置或非唯一课程代码把它们静默拼接。

## 当前能力

### Current

- `courses` 域拥有长期课程目录和教师、课程别名、聚合评课指标；`selection` 是按学期导入、可重建的
  教学班镜像。课程代码用于目录关联和检索，不是教学班唯一键。
- `SelectionCourse.id` 承载上游 teaching-class id，是当前快照和平台 API 中的教学班标识；`code` 是可能
  跨学期、教师或平行班重复的课程代码。教学班详情和节次都按教学班标识查询；不得再用
  `WHERE code = ...` 取任意一行。上游重编号后的 lineage 尚未定义。
- 年级、专业下教学班、课程性质下教学班和教学班搜索都由明确的 `calendarId` 限定。返回项携带
  `calendarId`，客户端切换学期后重新查询，不把另一学期结果留在当前选择器中。
- 节次使用 1-based、两端包含的 `startSlot..endSlot`；`TimeSlot.courseId` 与教学班标识一致。
- 物化会从完整匹配的安排文本保留周次集合和地点，并识别无前缀、`教师名(工号)`、`(工号)` 以及
  多身份前缀。无法唯一归属的多身份前缀只保留时段而不猜教师；无法解析、单边周次或混合成功/失败的安排
  均保留 `weeksUnknown/locationUnknown/scheduleUnknown`，不能用已解析的一部分冒充完整课表。
- 教学班携带历史公开评分聚合。优先匹配当前教师工号/姓名，无法精确匹配时才使用课程别名级参考，并通过
  `reviewScope=teacher|course|none` 明示口径；零样本必须返回 `reviewAvg=null`，Web/Flutter 不显示伪造的
  `0.0` 分。D1 旧评课正文、作者、点赞和举报不在该公开聚合迁移内。
- Web 与 Flutter 的本机课表按 API environment、账号（匿名使用独立 principal）和学期分区。
  登入、登出或切换学期不会自动合并另一 scope 的本地选择。
- Web 与 Flutter 都先按星期和 inclusive 节次区间找冲突候选。双方周次可解析且有交集时是
  confirmed conflict，可解析但周次无交集时不冲突；任一方周次缺失或无法解析时是
  possible conflict。只有 possible conflict 可在明示不确定性后由用户显式覆盖。
- 同一 `teachingClassId` 在同一课表 scope 内幂等加入；同一 course code 下的不同教学班不会
  被错误去重。
- Web 可导出和严格校验后恢复不含账号资料的课表 JSON；Flutter 可导出/分享同一类本机备份。
  这些文件只含当前 scope 的教学班和节次，不是账号数据导出或正式选课凭证。
- 评课创建、编辑、点赞/取消点赞、举报和管理处置由 `reviews` 域拥有；课程目录只消费其公开聚合。
- 公开列表和精确详情都由 Reviews 回表重验当前可见性；服务端返回 `viewerLiked/canEdit/canReport`，
  Web/Flutter 只按这些事实提供取消点赞、本人编辑和非本人举报。搜索结果按 review id 进入精确详情，
  隐藏或不存在的评课返回 not found。

### Partial

- 当前解析器只覆盖已由真实快照证明的上游安排格式；新增或畸形格式会保守标记 unknown。统一 contract 仍未提供
  容量、可信停开课状态和变更序列，因此 possible conflict 与 unknown 提示仍是正常状态，不能推断无冲突。
- 课程目录 id 与教学班 id 已在客户端分开使用，但 wire contract 尚无明确的
  `canonicalCourseId ↔ teachingClassId` bridge。因此“课程详情直接加入某教学班课表”、从课表进入准确
  课程/评课，以及换班/变更检测仍为 `Partial`；客户端不得按 course code 猜唯一关系。
- 选课搜索在 Meilisearch 候选阶段按 calendar 过滤，随后由 `courses` owner 按同一 calendar 批量回表，
  丢弃过期、已删、非法 id 或跨学期候选并保持候选相关性顺序。PostgreSQL 持久记录 catalogue/selection
  source generation、document count 和 readiness；进程启动及周期 reconciliation 会从空索引或外部丢失中
  全量恢复，未 ready 时搜索返回 unavailable 而不是伪装成零结果。全量 rebuild 仍是 clear→add，有短暂
  空窗，尚不是版本化索引原子切换。
- 评课仍缺 Reviews→Identity/Media 的 typed reviewer avatar；历史身份映射、课程合并/归档后的 deep-link
  保留政策和真实 Android/iOS device journey 也未闭环。
- 本机课表没有 server owner、跨设备同步、服务端删除编排或 canonical enrollment 事实；本机 JSON
  备份不改变这一边界。它不是正式选课结果，也不代表教务系统状态。
- 后端 route/DB 与客户端单元/widget 测试已覆盖教学班标识、calendar 分区和冲突分类，但仍没有
  真实浏览器 E2E 或 Android/iOS 真机的“选择教学班→冲突确认/覆盖→重启恢复”证据。

## 标识与查询不变量

```text
课程目录 course id ──长期内容/评课聚合
       │
       └── course code ──可关联但不唯一
                            │
学期 calendar id ── 教学班 teaching-class id ──节次/教师/地点
```

- 任何返回教学班集合或可用于筛选该集合的专业/课程性质发现查询都必须先确定学期；字典行可以在数据库
  复用，但 API 只返回该学期实际出现的选项。
- 教学班详情不存在时返回 not found；无效标识是 bad request。不能让数据库的 `fetch_optional` 在非唯一
  course code 上承担产品选择。
- 搜索索引只是候选投影。calendar 过滤必须在索引查询阶段生效，最终教学班事实应由 owner domain 回表确认。
- 课表本地 key 必须含版本和完整 scope。旧的未分区 key 不自动迁入登录账号，避免跨账号或跨学期泄漏。

具体字段和参数由 [`contract/openapi.yaml`](../../contract/openapi.yaml) 负责，本文件不复制接口定义。

## 冲突语义

- 同一天且节次区间重叠是冲突候选，不等于已确认冲突。
- 双方都有可解析周次且存在交集时才是 confirmed conflict；任一方周次缺失或无法解析时是 possible
  conflict，UI 必须说明不确定性并允许用户显式覆盖。
- confirmed conflict 不提供覆盖入口；possible conflict 的覆盖只保存本机选择，不改写上游事实。
- 不同教学班即使课程代码相同也不能去重；同一教学班标识在同一 scope 内幂等加入。
- 数据刷新后若教师、节次、周次或状态变化，未来变更检测必须显示差异并让用户确认，不能静默改写本机课表。

## 评课权限与互动

- viewer state、编辑资格和举报资格由服务端基于当前账号、review status 和制裁/权限计算，Web/Flutter
  只按 typed 字段展示动作。
- 点赞和取消点赞都幂等；客户端可以乐观显示，但失败必须回滚并以服务端响应或重新读取校正。
- 作者编辑使用 owner-domain 规则；本人内容不显示举报入口。管理能力不让 staff 绕过 no-self、层级或审计。
- 搜索结果若暴露 review id，就必须能确定地定位该 review，同时重新验证课程和 review 可见性。

## 数据导入与发布

- D1 snapshot、raw tables 和 materialized selection 的运维步骤由
  [D1 选课快照导入](../operations/data-import.md)负责。
- 教学班标识和 calendar scope 的 contract 升级不改写已有业务数据；Web 和 Flutter generated client 必须
  与服务端在同一 release family 更新。旧客户端不能继续用 course code 详情路径。
- 发布前抽查同课程代码的多个教学班、跨学期重复课程、无周次和无地点记录，并验证各学期查询互不串行。

## Decision needed

- 课程目录与教学班的 authoritative bridge、上游重编号处理和跨学期 lineage。
- 正式课表是否有 server owner、跨设备同步、服务端导出/删除、容量/停开课和变更通知；在此之前保持本机数据。
- 上游周次和教室字段的 authoritative 来源、格式清单、异常格式监控与回填/重物化策略。
- 历史评课正文、旧作者/匿名身份与互动的隐私迁移，review deep-link 保留期，以及课程被合并/归档后的
  显示语义；公开 aggregate 已恢复不代表这些 identity-bearing 记录获准迁移。

## 验收基线

- 同一 course code 下至少两个教学班、至少两个 calendar 的测试证明详情、节次、专业/性质和搜索不串数据。
- 真实安排 fixture 覆盖 `教师名(工号)`、`(工号)`、多身份前缀和无法解析行；前两者保留可证明教师，
  多身份只保留时段且不任意归属。真实快照重物化后 unknown 数量和时段总数稳定。
- 历史评分同时覆盖教师精确匹配、课程 fallback、无评分和重复物化；公开课程聚合只纳入唯一可证明的课程
  映射，未映射样本不猜归属。清空 Meilisearch document 后 reconciliation 能重建，未 ready 请求返回 503。
- Web/Flutter 切账号和切学期后只看到对应本机课表；匿名 scope 不自动并入账号。
- Web JSON 恢复拒绝错误 schema、其他学期/scope、重复教学班及越界节次；Flutter 导出不包含账号标识或 token。
- 可解析且有交集、可解析但无交集、周次缺失、两侧相同但无法解析都有 Web/Flutter 回归；
  周次未知不被渲染为“确定冲突”或“确定无冲突”，教学班 id 不被课程代码替代，合法第 20 节不被丢弃。
- 评课 like/unlike、owner edit、self-report 拒绝和搜索 deep link 有 owner-domain 与客户端关键旅程测试。

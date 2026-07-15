# 课程、选课、课表与评课

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Courses/Reviews/Web/Mobile maintainers、Product owner
>
> 最近核验：2026-07-15，selection teaching-class contract、Web/Flutter 课表与 reviews API

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
- Web 与 Flutter 的本机课表按 API environment、账号（匿名使用独立 principal）和学期分区。
  登入、登出或切换学期不会自动合并另一 scope 的本地选择。
- Web 与 Flutter 都先按星期和 inclusive 节次区间找冲突候选。双方周次可解析且有交集时是
  confirmed conflict，可解析但周次无交集时不冲突；任一方周次缺失或无法解析时是
  possible conflict。只有 possible conflict 可在明示不确定性后由用户显式覆盖。
- 同一 `teachingClassId` 在同一课表 scope 内幂等加入；同一 course code 下的不同教学班不会
  被错误去重。
- 评课创建、编辑、点赞/取消点赞、举报和管理处置由 `reviews` 域拥有；课程目录只消费其公开聚合。
- 公开列表和精确详情都由 Reviews 回表重验当前可见性；服务端返回 `viewerLiked/canEdit/canReport`，
  Web/Flutter 只按这些事实提供取消点赞、本人编辑和非本人举报。搜索结果按 review id 进入精确详情，
  隐藏或不存在的评课返回 not found。

### Partial

- 当前 `materialize_selection.sql` 会把物化节次的 `weeks` 和 `location` 写为 `NULL`，且统一
  contract 尚未提供教学语言、容量、停开课状态和变更序列。因此真实快照中的节次重叠目前大多
  只能落入 possible conflict；算法会要求显式覆盖，但不代表上游周次事实已补齐。
- 课程目录 id 与教学班 id 已在客户端分开使用，但 wire contract 尚无明确的
  `canonicalCourseId ↔ teachingClassId` bridge。因此“课程详情直接加入某教学班课表”、从课表进入准确
  课程/评课，以及换班/变更检测仍为 `Partial`；客户端不得按 course code 猜唯一关系。
- 选课搜索在 Meilisearch 候选阶段按 calendar 过滤，随后由 `courses` owner 按同一 calendar 批量回表，
  丢弃过期、已删、非法 id 或跨学期候选并保持候选相关性顺序；可靠更新/reconciliation 仍未完成，
  因而新鲜教学班可能暂时漏搜，但旧索引事实不会直接作为 API 结果返回。
- 评课仍缺 Reviews→Identity/Media 的 typed reviewer avatar；历史身份映射、课程合并/归档后的 deep-link
  保留政策和真实 Android/iOS device journey 也未闭环。
- 本机课表没有 server owner、跨设备同步、导出/删除编排或 canonical enrollment 事实；它不是正式选课
  结果，也不代表教务系统状态。
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

- 任何返回教学班集合的查询都必须先确定学期；课程性质字典本身可以跨学期复用。
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
- 正式课表是否有 server owner、跨设备同步、导出/删除、容量/停开课和变更通知；在此之前保持本机数据。
- 上游周次和教室字段的 authoritative 来源、格式清单、异常格式监控与回填/重物化策略。
- 历史评课与旧课程/匿名身份的映射、review deep-link 保留期，以及课程被合并/归档后的显示语义。

## 验收基线

- 同一 course code 下至少两个教学班、至少两个 calendar 的测试证明详情、节次、专业/性质和搜索不串数据。
- Web/Flutter 切账号和切学期后只看到对应本机课表；匿名 scope 不自动并入账号。
- 可解析且有交集、可解析但无交集、周次缺失、两侧相同但无法解析都有 Web/Flutter 回归；
  周次未知不被渲染为“确定冲突”或“确定无冲突”，教学班 id 不被课程代码替代。
- 评课 like/unlike、owner edit、self-report 拒绝和搜索 deep link 有 owner-domain 与客户端关键旅程测试。

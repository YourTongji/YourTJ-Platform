# Flutter 移动端产品规范

> 文档类型：产品体验规范
>
> 状态：Active
>
> 负责人：Product owner、Mobile maintainers、Security maintainers
>
> 最近核验：2026-07-14，Web routes/components、OpenAPI 与移动端参考客户端快照

YourTJ 移动端是 Web 之外的正式一等客户端，不是选课工具的换皮，也不是把 desktop 页面缩进手机。
它以相同账号、相同业务事实、相同权限和同一 `contract/openapi.yaml` 为基础，在手机和平板上用适合
触控、窄屏和系统生命周期的布局完成同一批用户旅程。Flutter 工程位于 monorepo 的 `mobile/`，首个
受支持的分发目标是 Android 与 iOS。

本文定义产品目标、跨端对齐标准、信息架构、安全边界和验收门槛。HTTP 字段仍以 OpenAPI 为准，
安全与合规硬边界仍以 `AGENTS.md` 和 `docs/security/` 为准，本文不复制接口或 DDL。

## 为什么不直接复制现有 Flutter 客户端

2026-07-14 对两个参考仓库做了只读研究：

| 参考 | 核验快照 | 可借鉴内容 | 不得直接复用的内容 |
|---|---|---|---|
| `Lingyan000/fluxdo` | `917c921ec577652535612cd8d28c91ac2b4a13ad` | 自适应 shell、持久 tab、平板 master-detail、账号切换时取消旧请求 | GPL-3.0 源码、布局数值、资产、本地 packages、Cookie/WebView/DOH 网络栈和不安全的存储降级 |
| `YourTongji/YourTJCourse-Flutter` | `35b601852e9e0d59032098c866c0b6e040c0a423` | 教学班、周次冲突、换班与变更检测的领域研究，移动查课与课表交互 | 未授权源码/资产、旧 Cloudflare API、匿名 client identity、硬编码视觉与本地自更新机制 |

FluxDO 是 GPL-3.0 项目，而 YourTJ Platform 是 proprietary 产品；旧选课客户端也没有允许任意复制的
公开许可证。因此移动端必须 clean-room 重写：参考客户端只提供需求观察，不进入 git history，不复制
实现、测试、文案、视觉数值、图标、字体或图片。共同使用 Riverpod、Dio 等上游开源依赖本身不构成
代码复用，但依赖必须直接从其官方来源取得并独立核验许可证。若以后要复用任何 proprietary 旧客户端
代码，必须先取得可记录的内部版权授权；若要复用 FluxDO 代码，必须先完成单独的许可证与分发法务决策。

## 产品目标与非目标

### 目标

- 同一账号在 Web、Android 和 iOS 看到相同的业务事实、权限、治理状态和失败语义。
- 用户可以只使用移动端完成 Web 当前所有普通用户旅程；具备 staff capability 的账号也能完成对应
  的紧急治理和运营动作。
- 视觉使用 Web semantic token、组件语义和内容层级；移动端只改变布局方式，不另造品牌。
- 网络不稳定、应用切后台、账号切换、token 轮换和重复点击不会产生串号、泄露或重复 mutation。
- OpenAPI 生成类型是 wire contract 的唯一移动端事实源；页面不得手写另一套宽松 JSON 模型。
- Android 与 iOS 在同一功能矩阵下交付；平台限制必须显式标记，不能静默缺功能。

### 非目标

- 不在 Flutter 中复制 Web DOM、CSS breakpoint 或 hover-only 交互。
- 不把 Flutter Web、Windows、macOS 或 Linux 作为首个发布目标。
- 不新增绕过统一身份、治理、积分或媒体边界的移动专用后端。
- 不做充值、提现、法币兑换、自由转账或任何跨越积分合规红线的入口。
- 不把离线缓存当业务事实源；首个版本不是 offline-first 产品。
- 后端尚无系统推送投递能力时，不用本地伪通知声称已实现后台实时推送。

## 跨端对齐的定义

“对齐 Web”同时包含六层，不能只核对页面名称：

1. **业务语义**：相同输入、权限和服务器事实得到相同结果；状态名、计数和可恢复性一致。
2. **能力可达**：Web 的用户功能与 capability-gated staff 功能在移动端有可发现入口。
3. **信息层级**：标题、主操作、次级操作、风险解释和空/错/加载状态保持同一优先级。
4. **设计语言**：颜色、圆角、排版、图标语义、motion 速度和 light/dark 层级来自共享 token。
5. **契约与安全**：同一 OpenAPI、请求头、幂等、签名 intent、隐私和审计要求。
6. **可访问性**：语义标签、动态字体、对比度、键盘/读屏和 reduced motion 达到相同标准。

像素相同不是目标。Desktop 的三栏必须在手机上重排为单栏与分层导航；右栏能力必须移入主内容，
不能隐藏。反过来，移动端 bottom sheet、下拉刷新、系统返回和 safe area 也不要求 Web 生搬硬套。

## 信息架构与自适应布局

### 手机

登录后的一级 bottom navigation 固定为五项：

| 一级入口 | 内容与次级入口 |
|---|---|
| 首页 | 签到、活跃度、等级进度、最新/热门/关注/订阅 feed、站内推广 |
| 社区 | 板块、主题列表、标签筛选、发帖、主题/评论详情与互动 |
| 排课 | 学期/专业/性质筛选、课程检索、本机课表与冲突提示 |
| 评课 | 课程检索、课程详情、AI 摘要、评课列表/创作/互动 |
| 积分 | 钱包、贡献任务、商品/购买、ledger/verify、tip/escrow |

这五项与 Web 的一级产品导航一一对应。顶部保留全局搜索、通知/私信未读入口与账号头像；手机宽度
不足时，搜索保留 icon，通知和私信合并为带分项 badge 的消息页，头像菜单承载公告、收藏、申诉、
设置与 capability-gated 管理入口。每个 tab 使用独立导航栈并保留滚动位置。发主题是社区场景的
明确主操作，可使用 FAB；不能把所有跨域创建动作塞进同一个含义不清的全局 `+`。详情、编辑、筛选
和确认使用全屏 route 或可恢复的 bottom sheet；深层 route 的系统返回必须回到原 tab 与原列表位置。

### 平板与横屏

- 可用宽度小于 600 logical pixels：五项 bottom navigation，单栏内容。
- 600–839：NavigationRail 与单栏/有限双栏，表单保持可读宽度。
- 840 及以上：NavigationRail + master-detail；列表选择与详情并排，但 canonical route 不变。
- 任何宽度都尊重 safe area、IME、屏幕旋转和分屏；不得用设备型号判断布局。
- 内容正文有最大可读宽度；管理数据表在窄屏改为字段分组卡片，不用水平滚动藏住关键动作。

### 匿名、登录与 onboarding

公开首页、公开论坛、课程、评课、公开资料和搜索允许匿名浏览。触发关注、发帖、评论、私信、积分、
收藏、通知或 staff 操作时进入登录，并在成功后回到原 intent。首次账号进入 focused onboarding；
recovery-only credential 只能进入恢复流程，不能借由 deep link 访问普通业务。

## 功能对齐矩阵

表内状态描述移动端目标，不改变对应后端/Web 的当前状态。只有用户可达、错误/恢复、权限和测试同时
存在时，移动端行为才可标为 `Current`。

| 领域 | 移动端目标 | 关键布局与验收 |
|---|---|---|
| App shell | `Planned` | 五项主导航、独立导航栈、匿名/登录守卫、deep link、手机/平板自适应 |
| 首页 | `Planned` | 窄屏先呈现签到/成长，再呈现 feed；签到同步刷新活跃度与等级事实 |
| 身份 | `Planned` | 密码、验证码、注册、找回、onboarding、登出、session replacement 与防枚举语义对齐 Web |
| 公开资料与社交 | `Planned` | 资料、内容 tabs、关注/粉丝、follow/remove、block/mute、隐私拒绝与媒体临期刷新 |
| 论坛列表 | `Planned` | latest/hot/subscriptions/following、板块与 tag、稳定游标、刷新与加载更多 |
| 论坛创作 | `Planned` | plain/Markdown、预览、CAS 草稿/发布、图片状态恢复、投票、版本冲突保留本地输入 |
| 论坛详情 | `Planned` | 主题、评论、回复、点赞、投票、收藏、订阅、修订历史、举报与 canonical viewer state |
| 聚合搜索 | `Planned` | 六类 typed 结果、all 与单类续页、安全高亮、建议词和局部失败保真 |
| 课程与评课 | `Planned` | 浏览、检索、详情、AI 摘要、关联课程、评课列表/创作/互动/举报；筛选语义与 Web 一致 |
| 选课与课表 | `Planned` | 学期/年级/专业/性质筛选、课程级本机课表、冲突提示、按账号与学期隔离 |
| 通知 | `Planned` | 分类、分页、已读、全部已读、target deep link、偏好；SSE 仅作前台刷新提示 |
| 公告 | `Planned` | audience/presentation、全局未看弹层、seen/dismiss/ack receipt 与公告历史 |
| 私信 | `Planned` | canonical 1:1、请求箱、接受/拒绝/撤回/举报、未读、搜索、mute、archive/delete/recover |
| 积分 | `Planned` | 钱包、任务、商品/购买、ledger/verify、tip/escrow；所有写入使用 exact signing bytes |
| 申诉 | `Planned` | 可申诉对象、提交、历史、终态和权限/期限失败；不泄露内部审核证据 |
| 设置与生命周期 | `Planned` | 资料/隐私/通知偏好、sessions、改密、导出、停用/删除/恢复和不可逆影响说明 |
| 管理入口 | `Planned` | capability 驱动，不按客户端写死 role；无权限时入口与 deep link 均拒绝 |
| 管理治理 | `Planned` | 账号、论坛、评课、申诉、通知死信、审核原因/层级/recusal/recent-auth/审计语义 |
| 管理运营 | `Planned` | 板块、公告、推广、媒体、活动/信任策略、认证、成就、积分 reconciliation、搜索重建 |

### Staff 页面原则

管理功能不是一级 tab。`我的 → 管理中心` 只显示服务端返回 capability 允许的模块；每次 mutation 仍由
服务端重新授权。手机使用“队列卡片 → 证据详情 → 明确确认”的纵向流程，平板可用双栏。所有 staff
操作必须显示 target、影响、reason、可恢复性和冲突状态；recent-auth、no-self、hierarchy、append-only
审计与 credit 禁止项不能因移动布局而弱化。大型批处理在移动端展示状态和安全重试，不提供无界客户端
循环。

## 选课与课表的正确性边界

旧选课客户端展示了教学班选择、单双周冲突、换班和变更检测等有价值的产品方向，但当前统一平台
还不能可靠提供这些事实：选课镜像以教学班为行，公开 API 却以 canonical course code 查询；同一课程
跨学期或多教师班可能歧义，且 contract 缺少教学班号、学期归属、完整周次、语言和状态。

因此分成两个明确能力层：

- `Current` Web 语义对应的课程级本机课表是首个移动端对齐范围；移动端不得假装用户选择了一个
  可验证的教学班，也不得承诺停开课/时间变更监测。
- teaching-class scheduler 为 `Planned` 的跨端升级。交付顺序必须是 backend 数据物化与唯一标识、
  OpenAPI、Web、移动端、迁移/回填与跨端测试一起完成；不能只在 Flutter 中猜测。

在 teaching-class 升级前，节次统一按平台 API 的 1-based inclusive `startSlot..endSlot` 展示，课表画布
至少容纳 1–13；冲突依据同一天节次区间重叠判断。`weeks` 缺失时只能提示“周次未知并按可能冲突
处理”，不能当作全周或无冲突。课程级课表存在本机，必须以 `accountId + calendarId` 分区；匿名数据
单独分区，登录后不自动并入账号。跨设备同步属于 `Decision needed`，没有服务端 owner 前不发明
移动专用云表。

## 设计系统

### Token 来源

`web/src/styles/index.css` 的 semantic variables 是当前运行事实，Flutter 以相同语义生成 `ColorScheme`
和 `ThemeExtension`，不从页面抽颜色：

| 语义 | Light | Dark |
|---|---|---|
| background | `#F8FAF8` | `#0C1E1B` |
| foreground | `#191C1B` | `#D8EDEA` |
| card | `#F2F4F2` | `#132922` |
| primary | `#009688` | `#2ECFB2` |
| secondary surface | `#F0FDFA` | `#1A3832` |
| secondary foreground | `#00796B` | `#A8D9D0` |
| muted | `#ECEEEC` | `#1A3832` |
| muted foreground | `#596562` | `#79AAA2` |
| accent | `#ECEEEC` | `#1E4039` |
| border | `#E1E3E1` | `rgba(46, 207, 178, 0.14)` |
| input | `#BCC9C6` | `rgba(46, 207, 178, 0.14)` |
| destructive | `#D4183D` | `#F04060` |

基础圆角为 12 logical pixels，对应 Web `0.75rem` 的语义；组件可以使用已定义的 compact/large 变体，
页面不得散布自定义 radius。直接操作、页面/状态过渡和低频强调分别使用 120/200/320 ms；系统开启
reduced motion 时取消非必要位移与缩放。

### 排版与图标

- 跟随系统 CJK fallback，不把 proprietary 字体打包进 App；Android/iOS 使用平台可用中文字体。
- 支持系统动态字体，正文不小于可读基线；200% 字体下关键操作仍可达且不裁字。
- 使用 Material Symbols/平台标准图标表达稳定语义；状态同时有文本或读屏 label。
- 数字、日期、积分、等级和未读数不只靠颜色。相对时间必须能访问精确时间。

### 组件状态

所有网络页面覆盖 loading、empty、error/retry、permission-denied、stale/refetch 和 content；所有 mutation
覆盖 submitting、成功后的服务端校正、重复点击、幂等重试和冲突。Skeleton 尺寸接近真实内容。
Toast 只作反馈，不能是结果或恢复入口的唯一载体。

## 内容与媒体

- `plain_v1` 与 `markdown_v1` 必须使用跨客户端共享 conformance corpus。支持产品定义的 GFM 子集；
  禁用 raw HTML、任意远程图片、危险 scheme 和自动链接抓取。
- 正文图片只解析 `yourtj-asset:<id>`，再通过 owner-authorized media projection 获取短期 signed URL。
- URL 过期或加载失败只刷新该授权 projection；账号切换时清空内存 delivery cache。
- 上传遵循 Media intent → exact-key OSS V4 upload → callback → processing/publication → binding，不能让
  SDK 自选 key、跳过 callback 或把 Ingest URL 当展示 URL。
- pending/processing/blocked 状态对 owner 可恢复，对普通 viewer 不扩大可见性。
- captcha 用服务端 puzzle/verify 协议原生渲染；不嵌入含账号凭证的通用 WebView。

## 身份、密钥与本地数据

### Session

- access token 只保存在内存；refresh token 只写入 iOS Keychain/Android Keystore 支持的安全存储。
- 安全存储不可用时 fail closed，不降级到 SharedPreferences、SQLite、文件或日志。
- refresh 必须 single-flight；session generation 改变后取消旧请求并丢弃迟到响应。
- 认证 header 只发送到配置的 API origin；OSS/CDN/captcha/外链请求绝不携带 JWT。
- 账号切换原子清理旧账号的 token、签名 URL、敏感内存状态和 pending mutation。

### 钱包

- Ed25519 private seed 只存在 OS secure storage 的 account-scoped entry，禁止云备份、日志、导出和
  analytics；服务器只接收 public key。
- 钱包写入先请求服务端 signing intent，再对服务端返回的 exact bytes 签名；客户端不得重组或
  “等价序列化” payload。
- nonce、intent expiry、幂等 key 和 mutation 生命周期作为同一个不可重放操作管理；通用网络 retry
  不得自动重放 tip/escrow/purchase。
- 生物识别可以作为本机解锁增强，但不能改变服务器验证和恢复模型。密钥迁移/恢复必须使用平台签名
  challenge，不导入服务器保存的 secret。

### 非敏感缓存

公开 feed、课程、板块元数据和本机课表可以用有界缓存改善冷启动；按账号和环境分区并有 schema
version/TTL。通知、私信正文、治理证据、refresh token、wallet seed 和短期签名 URL 不进入普通缓存。
缓存只提供 stale-while-revalidate 展示，mutation 成功必须以服务器回读校正。

## Realtime、后台与系统集成

- 前台登录状态连接 notifications/DM SSE；事件只表示“相关事实可能变化”，收到后重新拉 canonical API。
- 应用切后台时关闭或暂停连接，回前台根据 last refresh 重新同步；不把断线显示成业务错误。
- 后端推送 token、投递、偏好和审计闭环交付前，系统级后台 push 为 `Planned`。可以展示 App 内未读，
  不能注册无 owner 的 device token 表或依赖轮询常驻后台。
- `yourtj://` 与受控 HTTPS universal/app links 使用 allowlist route table。外部 URL 只允许 `https`，打开前
  明示离开 YourTJ；任何 token、email、signature 或 object key 不进入 URL。

## 无障碍与本地化

- Android TalkBack、iOS VoiceOver 的主导航、列表、表单、对话框、编辑器、课表和图表必须有可理解
  的语义顺序与 label。
- 触控目标至少 44×44 logical pixels；焦点不被 bottom sheet、IME 或 keyboard trap。
- Light/dark 对比度、系统 high contrast、dynamic type、横竖屏和 reduced motion 都进入 widget/golden
  验证矩阵。
- 首个语言是简体中文，但用户文案集中管理，不散布在网络/领域层；日期、数字和时区显式处理。
- 服务端时间使用 Unix seconds 或契约声明的格式；校园日历/签到边界继续以产品定义的时区计算，
  客户端本地时区只负责展示。

## API 与工程验收

移动端依赖由 `contract/openapi.yaml` 生成的 Dart package。生成器版本和配置必须固定，生成命令可重放，
CI 检查 regenerate 后无 diff。任何 HTTP 修改仍遵循“OpenAPI → Web types → handler → Dart client →
消费者 → tests”的顺序；不得在移动端以动态 map 掩盖 contract 漂移。

每个用户旅程至少有以下证据：

- repository/mapper contract tests，包括错误 envelope、nullable、unknown enum 和时间格式。
- controller/provider tests，包括 refresh、分页、取消、账号切换、冲突和重复 mutation。
- 手机与平板 widget tests，包括 loading/empty/error/permission、动态字体和 semantics。
- 设计系统 golden，覆盖 light/dark、320/390/840 宽度；golden 不是唯一行为测试。
- 关键 integration journeys：匿名浏览→登录返回、发帖/草稿恢复、选课→课表冲突、通知 deep link、
  私信请求、钱包签名、设置/账号生命周期和 capability-gated staff action。
- Android 与 iOS 编译；release signing、证书和 secret 只来自 CI secret/environment，不进仓库。

PR CI 强制运行 format、analyze（warning 为失败）、unit/widget/golden、OpenAPI drift 和 Android debug build。
iOS build 在可用 macOS runner 执行。App Store/应用市场发布、签名和 rollout 是独立的受控发布动作，
不能因合并代码而自动声称已上线。

## 实施依赖与退出条件

实现按依赖而非页面数量推进，每层达到退出条件后才能把上层状态改为 `Current`：

1. **契约健康**：OpenAPI 可被标准工具验证和生成；Rust DTO、Web 类型与 selection wire schema 漂移
   清零；生成器版本、配置和 drift check 固定。
2. **客户端底座**：设计 token、自适应 shell、typed routing、统一 API、secure session、账号隔离、
   error/loading 组件和测试 harness 完成。
3. **公开与普通用户旅程**：匿名浏览、身份、首页、论坛、课程/评课、课程级课表、搜索、资料/社交、
   通知/公告、私信、收藏和设置逐域接入真实 API，不留假数据或成功占位。
4. **签名与高风险旅程**：钱包、媒体上传、账号生命周期、申诉和 recent-auth 使用独立负向/重放/
   恢复测试证明安全边界。
5. **Capability 旅程**：移动管理中心覆盖 Web capability surface；复杂表格转换为卡片/双栏，但不删
   授权、理由、证据、审计、冲突与重试语义。
6. **发布质量**：Android/iOS 编译、OpenAPI drift、跨客户端内容/签名向量、无障碍、golden、关键
   integration journeys、依赖许可证与 secret scan 全部通过，再进入受控商店发布流程。

任一层发现后端事实不足，应先回到 owner domain/contract 修复或把行为保持为 `Partial`；不允许用
客户端猜测跨过依赖。这个顺序同时是 code review 的切分边界，避免把所有领域堆进无法审查的单模块。

## 完成判定

移动端整体只有同时满足以下条件才能从 `Partial` 升为 `Current`：

1. 功能矩阵的所有 Web 普通用户能力可达，staff 能力按 capability 可达且服务端拒绝被正确呈现。
2. 业务语义、权限、幂等、错误、深链、媒体和钱包签名通过跨端契约/旅程验证。
3. 手机与平板在 light/dark、动态字体、读屏、横屏和断网/恢复下均无唯一入口丢失。
4. Android 与 iOS CI 构建成功，生成 client 无 drift，依赖许可证与 secret/日志扫描通过。
5. 当前已知缺口不被占位 UI、假数据或本地状态伪装为服务端能力。

以下事实不阻止 Web-parity 客户端交付，但必须保持显式状态：teaching-class scheduler、系统后台 push、
跨设备课表同步、私信附件、Web 尚未消费的 Onebox 和目标生产商店发布均为 `Planned` 或
`Decision needed`；它们不能在产品页显示成已完成。

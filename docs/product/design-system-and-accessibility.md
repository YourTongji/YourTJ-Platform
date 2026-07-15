# 设计系统与无障碍

> 文档类型：产品体验规范
>
> 状态：Active
>
> 负责人：Product design、Web maintainers、Mobile maintainers
>
> 最近核验：2026-07-15，Web 运行 token、Flutter Android/iOS 自适应基线与共享无障碍要求

YourTJ Community Web 的 Figma 第三版是当前跨端品牌与视觉方向来源：
[Figma node 106:2](https://www.figma.com/design/dndgylImv8ZuVAXg9uWU3y/YourTJ-Community-Web?node-id=106-2)。
Figma 说明目标体验，`web/src/styles/index.css` 与实际 Web component 是当前运行事实；Flutter 通过同名
semantic token 和原生自适应 component 表达同一层级，不把 Web DOM/CSS 当作像素模板。三者不一致时
在同一产品/代码 PR 解决，不用一次性 QA 报告建立第四份真相。移动信息架构与功能矩阵见
[Flutter 移动端产品规范](mobile-client.md)。

## 体验原则

- 校园社区界面平静、清晰、信息密度适中，不用增长型视觉制造紧迫或成瘾反馈。
- 真实数据优先：无数据展示空态，不复制 Figma sample avatar、照片、等级或帖子伪装 live content。
- 课程、论坛、私信和后台共享 token/component，但允许按任务密度调整布局。
- Light/dark 都保持可读层级；颜色、icon 和 motion 不是唯一状态信号。
- Desktop、tablet、mobile 是同一用户旅程，不隐藏唯一入口或核心解释。
- Web、Android、iOS 对齐业务能力、状态、文案层级和安全边界；平台原生导航、返回、键盘和生命周期
  可以不同，但不得借适配之名删掉错误恢复、权限说明或高风险确认。

## 当前 shell 基线

| 项目 | 当前实现 |
|---|---|
| Header | 64 px sticky header |
| Content max width | 1280 px |
| Desktop left rail | 256 px |
| Home center/right | `minmax(0, 640px)` + 320 px |
| Desktop shell breakpoint | 1240 px，避免 1280 viewport 被 scrollbar 挤回 mobile shell |
| Gutters | mobile 16 px、small 24 px、1360+ 为 32 px |
| Mobile navigation | 288 px left sheet |
| Minimum supported width | 320 px |

Home right-rail capability 在窄于 1240 px 时必须进入主内容可达位置；不能仅 `display:none` 丢失信息。
当前 desktop 右栏成长卡的首个主操作是“每日签到”，活跃度热力图在其下方；等级进度是信息与次级
链接，不再把“查看等级任务”伪装为每日主操作。窄屏在 feed 之前复用同一签到状态和近 20 周热力图，
不要求打开 desktop sidebar。新增 sidebar/推广位时验证 1240、1280、1360 与常见 mobile 宽度。

## Flutter 自适应 shell

Flutter shell 当前为 `Partial`：monorepo 工程、主题与导航底座可以独立交付，但以下目标只有对应真实
journey、状态和测试接通后才能标为 `Current`。

| Window class | 导航与布局目标 |
|---|---|
| Compact `<600` | 底部五项一级导航：首页、社区、排课、评课、积分；搜索、通知、私信和账号留在顶栏/更多页 |
| Medium `600–839` | `NavigationRail` 与单栏/有限双栏，表单保持可读宽度 |
| Expanded `≥840` | `NavigationRail` + master-detail；达到 1240 时可按任务恢复 Web 的宽内容与辅助栏，但 route 语义不变 |

登录、账号恢复、受限申诉和 onboarding 使用 focused shell，不显示主导航。首页/profile 的 desktop 右栏
在 compact 布局进入主滚动区；私信使用手机 master-detail、平板双栏；排课竖屏以日期/日程为主，完整
周网格只在横屏或平板作为增强。桌面表格、hover、浏览器下载和大型 dialog 分别适配为卡片/分段详情、
显式操作、系统分享和 bottom sheet/full-screen route，不能等比例缩小。

签到按钮必须覆盖匿名登录引导、loading、error/retry、submitting、当日已签到和未签到状态；当日已签到
显示连续天数并把同一控件变为“打开分享结果”，不得重复提交。Mutation 成功后同时刷新签到、活跃度与
等级进度事实并打开单一结果 Dialog，不用仅更新按钮的局部动画冒充投影已更新。复制/保存操作有可见
文字标签；分享图中的日期、签到天数和成长等级不能只靠颜色表达，也不能包含账号或设备标识。

## 当前视觉 token

CSS variables 是 Web 当前 token 事实源。核心 light token：

| Token | Value | 用途 |
|---|---|---|
| background | `#f8faf8` | 页面背景 |
| foreground | `#191c1b` | 主文字 |
| card | `#f2f4f2` | surface/card |
| primary | `#009688` | 主操作与品牌色 |
| muted/accent | `#eceeec` | 选中/弱背景 |
| muted foreground | `#596562` | 次要文字 |
| border | `#e1e3e1` | 分隔 |
| input | `#bcc9c6` | 表单边界 |
| destructive | `#d4183d` | 高风险动作 |
| base radius | `0.75rem` | card/control 圆角基线 |

Dark token 也在同一 CSS 文件定义。新增颜色先建立 semantic token，不在页面散布 hex；数据图表使用
chart token，并为每个 series 提供 label/pattern/文本值。

Flutter 以这些 semantic variables 生成 `ColorScheme` 与 `ThemeExtension`；CSS variable 名和移动 token
应能逐项追踪，但允许按平台生成符合对比度的 pressed/disabled overlay。基础圆角为 12 logical pixels，
动效继续使用 120/200/320 ms。当前 Flutter token 映射为 `Partial`；完成 light/dark golden、动态字体和
Android/iOS 真机核验后才能视为跨端 `Current`。

## Typography 与内容

- Sans/display stack：HarmonyOS Sans SC → PingFang SC → Noto Sans SC → Microsoft YaHei → system。
- 中文界面优先使用用户能理解的词；技术状态只在需要时附英文 code。
- Heading 层级按信息结构，不用字号代替语义；正文保持可缩放，不锁死浏览器 font size。
- 数字、时间、计数和状态有明确 label；截断内容能访问完整值。
- 破坏性术语统一：禁言、封禁、隐藏、软移除、清除分别对应不同业务状态。

## Component 状态

每个可交互 component/页面至少设计：default、hover、focus-visible、active/selected、disabled、loading、
empty、error、success 和 permission-denied。Mutation：

- 防重复提交并显示进度；失败保留用户输入和可重试路径。
- Optimistic update 有 rollback/服务端校正，不把局部 toast 当持久成功事实。
- 作者编辑器遇到版本冲突时保留本地输入，以可朗读 alert 同时提供“载入线上版本”和“基于最新版重试”；
  refetch 不得自动替换正在编辑的内容。
- Destructive action 展示 target、影响、可恢复性和 reason；确认不能只靠红色。
- Skeleton 与最终布局尺寸接近，避免大幅 layout shift。

## Motion 基线

- 全局 motion token 由 `web/src/styles/index.css` 维护：fast 120 ms、normal 200 ms、slow 320 ms，
  分别用于直接操作反馈、页面/状态过渡和低频强调。
- 路由页面按需加载，切换时使用结构接近正文的可朗读 loading state；进入动画只做轻微透明度与
  纵向位移，不以长动画阻塞阅读或操作。
- Button、导航和其他直接操作 surface 复用 `motion-interactive`；状态 icon 可以使用低幅
  `motion-pop`，内容区不做持续漂浮、弹跳或自动播放。
- `prefers-reduced-motion: reduce` 时关闭位移反馈并将 animation/transition 缩短到近乎即时；
  功能和状态表达不得依赖动画是否播放。

## 无障碍基线

- 所有功能可键盘完成，focus order 与视觉顺序一致，focus-visible 清晰。
- Icon-only button 有可理解 accessible name；form control 有 label、description 和关联 error。
- Dialog/sheet 管理初始 focus、focus trap、Escape 和关闭后 focus restoration。
- 图片灯箱的缩略图必须是 Dialog trigger；支持关闭按钮、空白区、Escape 与多图左右方向键，关闭后焦点
  返回原缩略图。处于导航链接内的 Markdown 图片保留链接语义，不嵌套第二个 button。
- 导航头像在 profile/delivery 尚未确定时使用中性 skeleton，不短暂泄露或闪烁账号首字母；URL 更新时
  保留已成功加载的旧图直到新图加载成功。
- 文本/关键 UI 对比度满足 WCAG AA；颜色之外使用文字、icon shape 或 pattern。
- Heatmap/chart cell 可聚焦并朗读日期、数值和原始构成。
- 首页活跃度按 20 周 × 7 日 grid 表达；方向键按日期移动，Home/End 到首尾可用格。每格 label 同时朗读
  日期、总分、发帖/评论/点赞/签到构成和强度；颜色不是唯一信息来源。
- 动画尊重 `prefers-reduced-motion`；不使用闪烁、强制连续 autoplay 或阻断式装饰动效。
- Touch target、缩放、横向 scroll、长中文/英文/URL 和 screen-reader reading order 需要移动端验证。
- Flutter 触控目标至少 44×44 logical pixels，Material 主交互优先 48×48；TalkBack/VoiceOver、系统
  dynamic type、high contrast、IME、安全区、横竖屏和系统返回必须纳入 Android/iOS 验证。

## Design QA 流程

1. PR 链接 Figma node/variant 和要验证的 state，不复制不可追踪 screenshot 为长期规范。
2. 在实际 API 的 authenticated/unauthenticated、loading/empty/error/filled/permission 状态验收。
3. Web 至少验证 desktop 1280/1360、shell breakpoint 1240、mobile 320/375/430；Flutter 至少验证
   320/390/840、Android/iOS、动态字体和横竖屏。
4. 检查 overflow、sticky、deep link、keyboard、screen reader labels、contrast、reduced motion。
5. 截图/录屏放 PR artifact，并记录浏览器/viewport/route/state；不提交 `/tmp` 路径清单。
6. 查看 console/network 和 API truth；不把 mock/sample 数据与真实功能混淆。

## 文档与实现同步

- 全局 token、shell breakpoint、导航 IA 或共享 component 行为变化时更新本规范。
- 单个功能的 user journey/业务状态更新对应领域文档，不在本文件复制。
- Figma 变化但代码未交付标 `Planned`；代码变化偏离 Figma 时记录有意决定，而非悄悄漂移。
- Web 已有 Vitest + Testing Library + axe-core 的最小 component/a11y harness；Flutter 的 format、
  analyzer、unit/widget 与 Android/iOS build gate 已接入 CI。跨端 golden、真实读屏和完整 browser/device
  journey coverage 仍为 `Planned`，见[测试策略](../development/testing.md)。

## 验收基线

- 页面没有合成等级、徽章、互动或内容冒充服务端事实。
- 核心功能在 320 px 到 desktop 可达，无水平页面溢出或被 breakpoint 永久隐藏。
- Keyboard、focus、label、contrast、reduced motion 和 loading/error 状态经过验收。
- Light/dark 使用 semantic token，没有不可解释的局部颜色分叉。
- PR evidence 可复现，长期规范不依赖本地临时截图。

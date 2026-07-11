# 设计系统与无障碍

> 文档类型：产品体验规范
>
> 状态：Active
>
> 负责人：Product design、Web maintainers
>
> 最近核验：2026-07-11，`origin/main@ed8a06c`

YourTJ Community Web 的 Figma 第三版是当前视觉方向来源：
[Figma node 106:2](https://www.figma.com/design/dndgylImv8ZuVAXg9uWU3y/YourTJ-Community-Web?node-id=106-2)。
Figma 说明目标体验，`web/src/styles/index.css` 与实际 component 是运行事实；两者不一致时在同一
产品/代码 PR 解决，不用一次性 QA 报告建立第三份真相。

## 体验原则

- 校园社区界面平静、清晰、信息密度适中，不用增长型视觉制造紧迫或成瘾反馈。
- 真实数据优先：无数据展示空态，不复制 Figma sample avatar、照片、等级或帖子伪装 live content。
- 课程、论坛、私信和后台共享 token/component，但允许按任务密度调整布局。
- Light/dark 都保持可读层级；颜色、icon 和 motion 不是唯一状态信号。
- Desktop、tablet、mobile 是同一用户旅程，不隐藏唯一入口或核心解释。

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

Home right-rail capability在窄于 1240 px 时必须进入主内容可达位置，例如当前活跃度卡片；不能仅
`display:none` 丢失信息。新增 sidebar/推广位时验证 1240、1280、1360 与常见 mobile 宽度。

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
- 文本/关键 UI 对比度满足 WCAG AA；颜色之外使用文字、icon shape 或 pattern。
- Heatmap/chart cell 可聚焦并朗读日期、数值和原始构成。
- 动画尊重 `prefers-reduced-motion`；不使用闪烁、强制连续 autoplay 或阻断式装饰动效。
- Touch target、缩放、横向 scroll、长中文/英文/URL 和 screen-reader reading order 需要移动端验证。

## Design QA 流程

1. PR 链接 Figma node/variant 和要验证的 state，不复制不可追踪 screenshot 为长期规范。
2. 在实际 API 的 authenticated/unauthenticated、loading/empty/error/filled/permission 状态验收。
3. 至少验证 desktop 1280/1360、shell breakpoint 1240、mobile 320/375/430；功能需要时加 tablet。
4. 检查 overflow、sticky、deep link、keyboard、screen reader labels、contrast、reduced motion。
5. 截图/录屏放 PR artifact，并记录浏览器/viewport/route/state；不提交 `/tmp` 路径清单。
6. 查看 console/network 和 API truth；不把 mock/sample 数据与真实功能混淆。

## 文档与实现同步

- 全局 token、shell breakpoint、导航 IA 或共享 component 行为变化时更新本规范。
- 单个功能的 user journey/业务状态更新对应领域文档，不在本文件复制。
- Figma 变化但代码未交付标 `Planned`；代码变化偏离 Figma 时记录有意决定，而非悄悄漂移。
- Web 已有 Vitest + Testing Library + axe-core 的最小 component/a11y harness；browser E2E 和完整
  journey coverage 仍未建立，见[测试策略](../development/testing.md)。

## 验收基线

- 页面没有合成等级、徽章、互动或内容冒充服务端事实。
- 核心功能在 320 px 到 desktop 可达，无水平页面溢出或被 breakpoint 永久隐藏。
- Keyboard、focus、label、contrast、reduced motion 和 loading/error 状态经过验收。
- Light/dark 使用 semantic token，没有不可解释的局部颜色分叉。
- PR evidence 可复现，长期规范不依赖本地临时截图。

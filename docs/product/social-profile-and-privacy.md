# 个人资料、社交图与隐私

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Forum/Identity/Web maintainers、Privacy owner、Product owner
>
> 最近核验：2026-07-11，`origin/main@33584db`

本规范定义公开资料、用户关注、板块/主题订阅、block、mute、资料隐私和徽章语义。目标是
建立可解释的社交关系，而不是把所有关系都命名为 “following” 或 “屏蔽”。

## 当前状态

### Current

- 公开资料包含 handle、头像、角色、信任等级、徽章、主题/回复/获赞统计和分页内容列表。
- board/thread 支持 watching、tracking、muted subscription。
- `user_ignores` 会从当前用户部分 feed 隐藏对方，并在任意方向存在时禁止私信发送。

### Partial

- 没有用户 follow、粉丝/关注列表、计数或 relationship API。
- UI 把 `ignore` 称为 block，但资料、搜索、回复、提及和投票没有完整 block 语义。
- 资料缺 display name、bio、banner、受控链接、关系统计和 activity/media/likes tabs。
- 头像仍可写任意 URL；handle 无历史、冷却和旧链接跳转。
- profile、activity、关系列表、DM 和 discoverability 没有用户隐私设置。

## 四种关系不得混用

| 关系 | 方向 | 主要作用 | 是否通知对方 |
|---|---|---|---|
| follow | 单向，可有 pending | 用户 following feed、关系计数、DM/mention policy | 接受或建立时按偏好通知 |
| subscription | 用户到 board/thread | 内容更新通知和订阅 feed | 不通知内容作者 |
| mute | 单向私密 | 降低 feed、通知或会话可见性 | 否 |
| block | 双向安全边界 | 阻止 follow、私信和直接互动，并统一可见规则 | 不主动通知，但操作结果不可伪装 |

现有名为 `following` 的 forum feed 必须在用户关注上线前改成能表达 subscription 的名称。

## Follow 状态机

公开资料的基本状态为 `not_following -> following -> not_following`。如果未来支持需要审批的
资料，则增加 `pending -> accepted | rejected | cancelled`。规则包括：

- 不能关注自己；block 任意方向存在时不能关注或批准请求。
- 并发重复 follow/unfollow 幂等，计数在同一事务中更新或从关系事实可靠投影。
- 列表使用稳定游标并在读取时应用 block、账号状态和可见性规则。
- 删除/停用账号解除待处理请求；已接受关系的保留/删除由账号生命周期策略处理。
- 用户可以 remove follower；它删除对方→本人的关系但不自动 block，对方能否再次 follow 取决于
  profile/request policy。

relationship 读取应一次返回：`following`、`followedBy`、`requestState`、`blocked`、`muted`、
`canDm`、`canMention` 与相关操作能力，避免页面下载整张关系列表来推断单个用户状态。

## Block 与 mute

目标语义：

- mute 是当前用户私有过滤，不阻止对方继续访问公共板块内容或互动。
- block 在任意方向存在时阻止 follow、DM、回复到对方内容、直接 mention 和对对方内容的反应。
- 创建 block 在同一 transaction 删除双方 accepted follow 和 pending request，并校正关系计数；解除
  block 不恢复这些关系。
- 公共板块原讨论仍由板块可见性决定；block 对历史公开串采用“折叠/提示/不主动推荐”的方式，
  不伪造不存在，也不破坏其他参与者的回复上下文。
- profile、搜索、通知、feed、DM 和互动端点共享同一 relationship policy，不各自解释。
- 解除 block 不自动恢复 follow 或订阅关系。

最终的公开资料可见规则仍需产品负责人确认；在确认前，UI 不得承诺“双方完全不可见”。

## 资料模型

目标公开资料字段：

- 不可变 account id、handle、display name、avatar asset、banner asset、bio。
- 有协议/域名白名单和数量限制的外部链接。
- join time、公开角色/认证、信任等级、成就徽章。
- followers、following、主题、回复、获赞等准确计数。
- tabs：posts、replies；likes、media 和 activity 是否公开由隐私设置决定。

校园邮箱、制裁证据、设备、内部风险分和私信绝不属于公开资料。Staff console 按 capability
获取必要运营字段，也不因为“管理员”而默认显示邮箱或任意 DM。

## 隐私矩阵

目标设置至少包含：

| 设置 | 建议选项 | 推荐默认值 |
|---|---|---|
| profile visibility | public / campus / only_me | campus，待最终确认 |
| activity visibility | public / campus / only_me | only_me |
| follower list visibility | public / campus / followers / only_me | campus |
| following list visibility | public / campus / followers / only_me | campus |
| DM policy | everyone / following / nobody | following |
| mention policy | everyone / following / nobody | everyone，受 block 限制 |
| discoverability | on / off | on for campus |

公共论坛主题的可见性由 board policy 控制，不由作者 profile visibility 反向改变。若未来要做
followers-only 内容，应建立独立内容类型和授权模型。

## 徽章、认证与角色标识

三者必须分开：

- **成就徽章**：由贡献或规则授予，例如首帖、优质作者，可自动获得。
- **身份认证**：证明组织、官方账号或经核实身份，有签发、到期、撤销、证据和审计。
- **角色标识**：说明 moderator/admin 的当前平台职责，不作为成就或身份认证。

特殊认证应记录 type、issuer、issued/expiry/revoked time、reason、display priority 和私有证据引用。
前台只展示允许公开的 label；后台提供创建、授予、撤销和历史，不允许通过通用 string setting 伪造。

## API 与数据所有权

用户关系、block/mute、privacy 和公开资料投影由明确的 domain owner 持有；当前推荐 forum 拥有
关系，identity 拥有账号与 owner-editable profile 核心字段。HTTP 至少需要 follow/unfollow、
followers/following、relationship 和 privacy settings，具体 shape 以 OpenAPI 为准。

头像和 banner 只保存 clean media asset reference。服务端生成可用 URL，客户端不得提交任意
第三方 URL 作为权威字段。

## Decision needed

- public/campus 的全站默认范围，以及搜索引擎是否索引公开 profile。
- block 对历史公开内容和资料直链的具体呈现。
- 是否需要私密账号请求；公共论坛第一阶段建议不做。
- 关系列表默认可见性、handle 释放期和认证账号改名流程。

## 验收基线

- follow/unfollow/request 并发安全，计数、列表和 relationship 结果一致。
- follow、subscription、mute、block 的文案、API 与行为不混用。
- block/mute 在 profile、feed、search、notification、DM 和互动中使用同一 policy。
- 隐私设置对匿名、校园成员、关系用户、本人和 staff 有矩阵化授权测试。
- 所有公开 profile 响应不含邮箱或内部治理字段，媒体来自平台 asset。
- 成就、认证和角色标识可以独立授予/撤销并留下审计。

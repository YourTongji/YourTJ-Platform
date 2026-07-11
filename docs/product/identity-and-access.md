# 身份、登录与账号生命周期

> 文档类型：产品领域规范
>
> 状态：Active
>
> 负责人：Identity maintainers、Security owner、Product owner
>
> 最近核验：2026-07-11，`origin/main@33584db`

身份域证明“谁在使用平台”和“是否具备校园资格”，但不把校园邮箱变成公开身份。本规范定义
登录、注册、密码、会话、onboarding、handle 与账号生命周期的目标语义。

## 当前状态

### Current

- 校园邮箱验证码可登录或注册，账号使用 JWT access 与可撤销 refresh session。
- 后端已有密码登录、忘记密码、重置密码和修改密码，密码使用 Argon2id。
- 账号状态、角色、禁言/封禁会参与受保护请求判断。
- 校园邮箱不进入公开 profile DTO；支持加密和 blind index 配置。

### Partial

- Web 只提供混合的验证码登录/注册表单，没有暴露密码登录和完整找回流程。
- 验证码没有 purpose 和 `used_at`，成功验证不是严格的原子一次性消费。
- 密码重置或修改后没有撤销其他 refresh session。
- 账号不存在、无密码和密码错误的响应可被用来枚举账号。
- 未填写 handle 时可能使用邮箱前缀，形成公开身份关联风险。
- 验证码已实现 purpose 绑定与原子消费（AUTH-001）。
- access 与 refresh token 都保存在 localStorage；富文本上线前必须重新评估 XSS 后果。
- 没有设备中心、recent-auth、条款版本确认、onboarding、导出或自助删除。

## 登录与注册体验

Web 必须把以下旅程分开，不能让用户猜测同一个表单当前处于什么状态：

1. **密码登录**：校园邮箱 + 密码，支持显示密码、错误恢复和登录后返回原页面。
2. **验证码登录**：校园邮箱 + 单次 code，适合无密码或临时登录。
3. **注册**：校园邮箱验证后选择不泄露邮箱信息的 handle，确认规则/隐私版本，再创建账号。
4. **忘记密码**：始终返回中性结果；只有合法且已设置密码的账号在后台收到 reset code。
5. **重置密码**：reset-purpose code + 新密码；成功后撤销旧会话并创建明确的安全通知。

登录页面至少有加载、发送失败、倒计时、可重发、code 过期、限流、账号被暂停和网络重试
状态。密码规则在输入时可理解，不能只在提交后返回模糊错误。

## 验证码状态机

每个验证码必须绑定：

- 规范化邮箱 blind index；
- purpose，例如 `login`、`registration`、`password_reset`；
- code hash、到期时间、尝试次数、发送请求 id；
- nullable `used_at`，以及必要的 provider acceptance 状态。

验证在单个数据库事务中锁定最新有效记录，检查 purpose/到期/尝试次数，constant-time 验证，
然后写入 `used_at`。成功、并发重放和跨 purpose 使用都只能有一个成功结果。provider 未接受邮件时，
code 不可使用。

## 密码与会话

- 密码登录、忘记密码和验证码请求对“账号是否存在”提供同等级别的外部响应。
- 修改密码要求当前密码或 recent-auth；重置密码依赖 reset-purpose code。
- 密码修改、重置、角色变化和 suspend 撤销现有 refresh session；产品需决定是否保留当前设备。
- refresh token rotation 必须识别重放；设备中心显示有限的设备/时间信息，不暴露精确历史 IP。
- 支持撤销单设备、撤销其他设备和全设备注销。
- Web refresh credential 的长期目标是 `HttpOnly + Secure + SameSite` cookie；在迁移决策完成前，
  任何富文本和第三方脚本变更都必须把 localStorage token 风险列入安全评审。
- 角色、永久制裁、PII reveal、账号删除等高风险操作需要最近认证或独立 challenge。
- Moderator/admin 目标要求 phishing-resistant MFA（优先 WebAuthn/passkey）与受审计 recovery；在
  交付前 staff 账号安全仍为 `Partial`，不能把普通 JWT recent-auth 当完整二次验证。

## Onboarding

首次注册完成后按可跳过、可恢复的步骤引导：

1. 选择 handle，说明其公开性、修改冷却和受保护名称。
2. 上传平台 OSS 头像；允许跳过，不能要求填写外部 URL。
3. 选择兴趣板块和通知基础偏好。
4. 阅读并确认当前社区规则、隐私政策和积分边界版本。
5. 解释 follow 与板块 subscription 的差异；在 follow graph 完成后提供建议关注。

Onboarding 进度不应阻止必要的账号安全操作，也不能把营销同意与服务必要条款捆绑。

## Handle 生命周期

- account id 永久稳定；handle 是可变公开标识。
- 新 handle 不得从邮箱前缀、学号或姓名自动派生。
- 修改需要冷却期、保留名称检查和审计；旧 handle 在保护期内跳转到当前资料页。
- 认证账号、管理员和高曝光账号改名时保留可理解的历史。
- 删除账号后旧 handle 的释放与防冒用期限由隐私和安全负责人共同确定。

## 账号生命周期

目标状态机：

```text
active -> deactivated -> deleted -> mutable identity purged
   ^           |
   +-----------+ 允许在恢复窗内重新激活

active + silence = 可认证和读取，但受保护社区写入被拒绝
active + suspend = 登录、refresh 和受保护请求被拒绝
```

- `deactivated` 是用户可恢复停用，不等同于处罚。
- `deleted` 开始跨域删除/匿名化编排；恢复窗内仍可撤回。
- `purged` 移除可变 PII，但保留法律/安全允许的最小审计和不可改写积分账本 tombstone。
- 毕业或校园邮箱失效不应自动抹除社区历史；恢复政策仍为 `Decision needed`。

## 数据与接口所有权

Identity crate 拥有 accounts、email codes、password hash、sessions、account keys 和 handle history。
其他域通过 identity 的公开 API 获取最小身份/角色视图，不直接查询或返回校园邮箱。HTTP 结构以
`contract/openapi.yaml` 为准，本文不复制字段清单。

## Decision needed

- 密码登录是否只接受校园邮箱，还是也接受 handle；推荐只接受邮箱以降低改名和枚举复杂度。
- 新注册是否必须设置密码；推荐允许验证码账号稍后设置，但 onboarding 明确提示恢复能力。
- 密码改变后是否保留当前设备；推荐仅在 recent-auth 修改时保留当前设备，reset 全部撤销。
- 毕业用户的资格、恢复和邮箱换绑政策。
- refresh cookie 的跨端迁移、CSRF 防护和旧 token 撤销计划。
- Staff WebAuthn/passkey、recovery code 和 break-glass 的注册、丢失与撤销政策。

## 验收基线

- 密码与验证码两种登录都能从 Web 完成，注册和找回路径互不混淆。
- code 跨 purpose、并发重放、过期、超尝试次数均失败且无状态竞争。
- 外部响应不能可靠区分不存在账号、无密码和错误密码。
- 密码重置、角色变化和 suspend 后旧 refresh token 不可继续使用。
- 公共 API、日志、通知和审计不泄露邮箱、code、password hash 或 refresh secret。
- handler→repo→PostgreSQL 集成测试覆盖上述正向与负向旅程。

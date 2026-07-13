# 邮件发送

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Platform maintainers、Identity maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

Production 目标 provider 为 Cloudflare Email Sending。SMTP 仅作显式 fallback；local/test/PR preview
使用 redacted `log` provider，不发送真实邮件。Identity 中用于完成当前身份证明的 code 和已提交
安全事实的事后通知使用不同失败语义。

## Provider

| Provider | 必要配置 | 环境 |
|---|---|---|
| `cloudflare` | `EMAIL_FROM`、account id、API token | Main/production secret store |
| `smtp` | `EMAIL_FROM`、host；可选 username/password | 受控 fallback |
| `log` | 无 | Local、test、PR preview |

Cloudflare endpoint 与响应 envelope 以官方
[Email Sending API](https://developers.cloudflare.com/api/resources/email_sending/methods/send/) 为准。
应用只在 provider 表示成功、无 API error/permanent bounce 且返回非空 message id 时视为 accepted。

## 业务语义

- 登录/注册 code：provider 接受邮件后 API 才返回成功；失败时使新 code 无效并返回可重试
  `SERVICE_UNAVAILABLE`，不能告诉用户“已发送”。
- 忘记密码始终返回中性 204，避免用 provider 失败枚举账号；provider 未接受时 reset code 仍失效，
  运维通过不含收件人/code 的 error rate 告警。客户端使用中性文案提示“若账号可重置将收到邮件”。
- recent-auth 和 password-reset code 同样先等 provider accepted；reset code 还绑定签发时凭据版本。
- 密码 set/change/reset 和管理员邀请：业务 transaction 只写 `account_id + kind` 的 durable job，不等 provider。
  worker 事后读取当前加密邮箱、渲染固定模板并投递；provider 失败不回放密码 mutation 或重复邀请账号。
- Refresh replay 当前只产生可导出的有界 security fact，没有直接发邮件，避免攻击者用已泄露旧 token
  重复触发邮件轰炸。若以后增加通知，必须先有 family/window 去重。
- Forum digest 尚未纳入这条 Identity durable delivery 链路；bounce/complaint webhook 和 dead-letter 管理员重排界面也未交付，
  不得因安全通知 worker 上线而宣称它们已完成。
- 治理/安全邮件即使用户关闭互动邮件也不能被错误抑制；仍需保留站内事实通知。
- Provider accepted 不等于最终 delivered；bounce、complaint 和 retry 状态需要后续 operational model。

## Secret 边界

- Token 只存 deployment secret store/权限受限环境文件，不进入 `.env.example`、GitHub workflow、
  PR、Issue、截图、日志或 generated client。
- 只赋予目标 account 的最小 email-sending permission，并限制 verified sender。
- Main backend 才能读取生产 token；PR preview 必须没有该 secret。
- 日志不记录邮箱、验证码、正文、token 或 raw upstream response，只记录 provider、purpose、结果类别、
  latency 和 opaque request/message id。
- Durable job 表不存 recipient、subject、text/html、code、provider message id 或 response；worker 日志只使用 job id、
  template kind、attempt 和有界 error code，不使用 account id/email/body 作 label。

## Durable worker

- Claim 使用 `FOR UPDATE SKIP LOCKED`，每次生成唯一 lease token；provider I/O 期间不持有 PostgreSQL lock。
- Lease 5 分钟后可回收；provider 失败最多 8 次，从 30 秒指数退避且最长 1 小时，耗尽进入 `dead`。
- 账号邮箱加密材料/数据库短暂不可用时记 `identity_unavailable` 并重试；只有账号不存在或已 purge
  才以 `recipient_unavailable` 永久终止。
- Provider accepted 到 job 完成之间崩溃可能产生重复安全通知；模板必须是无害、幂等的事实通知，不能包含
  单次消费链接或 code。
- `succeeded` job 保留 30 天，`dead` 保留 90 天；安全事实保留 365 天。worker 每小时运行有界 retention。

## 配置与验证

1. 在 main secret store 配置 `EMAIL_PROVIDER=cloudflare`、verified `EMAIL_FROM`、
   `CLOUDFLARE_EMAIL_ACCOUNT_ID` 和 `CLOUDFLARE_EMAIL_API_TOKEN`。GitHub Actions 只负责把 main repository secrets
   注入服务器环境；PR preview 不得引用这些 secret。
2. 保持官方 HTTPS API base；非 HTTPS override 只允许 loopback integration test。
3. 用受控测试邮箱验证 accepted envelope；不要在命令历史/日志打印 token 或 code。
4. 通过平台 API 请求一次 login code 并确认收到。
5. 注入 provider failure，确认 login/registration/recent-auth code API 返回 503、forgot API 保持中性 204，
   所有未 accepted code 均不可使用且日志无 PII。
6. 触发一次密码 change，确认 job 从 `queued -> running -> succeeded`；注入 provider 503 时业务仍成功、job
   回到 `queued` 且只出现 `provider_unavailable`。
7. 确认 preview 仍为 `log` provider，且无法使用生产 token。

## 故障处理

1. 查看 provider status、应用 accepted/error rate 与 latency，不打印 credential。
2. 检查 sender verification、token scope/expiry 和 account id。
3. 判断是身份 code（fail closed）还是已提交事实的安全通知（queue/retry）。不要为了重发通知而重放密码 mutation。
4. 若切换 SMTP，按 deployment change 评审 secret、TLS、sender 和回滚。
5. 疑似泄露时先创建新 token、更新 secret、验证发送，再撤销旧 token并审计影响。

禁止把 production 切到 `log` provider；那会产生 API 成功但用户永远收不到 code 的假健康状态。
当前没有对 email dead letter 的管理员 requeue API；运维只能根据去标识化指标告警和修复 provider/密钥，
不应通过临时 SQL 绕过下一次受审计的管理界面。

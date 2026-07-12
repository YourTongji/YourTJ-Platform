# 邮件发送

> 文档类型：运维 runbook
>
> 状态：Active
>
> 负责人：Platform maintainers、Identity maintainers
>
> 最近核验：2026-07-11，`origin/main@33584db`

Production 目标 provider 为 Cloudflare Email Sending。SMTP 仅作显式 fallback；local/test/PR preview
使用 redacted `log` provider，不发送真实邮件。

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
- 邀请与 digest：主业务 mutation 不因外部邮件失败回滚；写 retry/outbox 并发出不含收件人的告警。
- 治理/安全邮件即使用户关闭互动邮件也不能被错误抑制；仍需保留站内事实通知。
- Provider accepted 不等于最终 delivered；bounce、complaint 和 retry 状态需要后续 operational model。

## Secret 边界

- Token 只存 deployment secret store/权限受限环境文件，不进入 `.env.example`、GitHub workflow、
  PR、Issue、截图、日志或 generated client。
- 只赋予目标 account 的最小 email-sending permission，并限制 verified sender。
- Main backend 才能读取生产 token；PR preview 必须没有该 secret。
- 日志不记录邮箱、验证码、正文、token 或 raw upstream response，只记录 provider、purpose、结果类别、
  latency 和 opaque request/message id。

## 配置与验证

1. 在 main secret store 配置 `EMAIL_PROVIDER=cloudflare`、verified `EMAIL_FROM`、account id 和 token。
2. 保持官方 HTTPS API base；非 HTTPS override 只允许 loopback integration test。
3. 用受控测试邮箱验证 accepted envelope；不要在命令历史/日志打印 token 或 code。
4. 通过平台 API 请求一次 login code 并确认收到。
5. 注入 provider failure，确认 login/registration API 返回 503、forgot API 保持中性 204，所有新
   code 均不可使用且日志无 PII。
6. 确认 preview 仍为 `log` provider，且无法使用生产 token。

## 故障处理

1. 查看 provider status、应用 accepted/error rate 与 latency，不打印 credential。
2. 检查 sender verification、token scope/expiry 和 account id。
3. 判断是身份 code（fail closed）还是 best-effort notification（queue/retry）。
4. 若切换 SMTP，按 deployment change 评审 secret、TLS、sender 和回滚。
5. 疑似泄露时先创建新 token、更新 secret、验证发送，再撤销旧 token并审计影响。

禁止把 production 切到 `log` provider；那会产生 API 成功但用户永远收不到 code 的假健康状态。

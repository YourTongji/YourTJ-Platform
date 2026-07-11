# Outbound email delivery

> **Status:** DELIVERED IN THIS PR — Cloudflare transport, verification delivery semantics, and
> production secret isolation
>
> **Owner:** Platform maintainers
>
> **Last verified:** 2026-07-11 against `contract/openapi.yaml`, shared/identity source, and the main
> deployment configuration
>
> **Authoritative sources:** `AGENTS.md`, `contract/openapi.yaml`, `backend/crates/shared/src/email.rs`,
> `backend/crates/shared/src/config.rs`, `backend/crates/identity/src/handlers.rs`

## Provider model

Production uses Cloudflare Email Sending's account-scoped REST endpoint. The shared email transport
also retains SMTP as an explicit fallback. Local development and tests default to the `log` provider,
which records only redacted delivery metadata and never sends a message.

| Provider | Required configuration | Intended environment |
|---|---|---|
| `cloudflare` | `EMAIL_FROM`, `CLOUDFLARE_EMAIL_ACCOUNT_ID`, `CLOUDFLARE_EMAIL_API_TOKEN` | Production |
| `smtp` | `EMAIL_FROM`, `SMTP_HOST`; optional username/password pair | Controlled fallback |
| `log` | None | Local development and tests only |

`CLOUDFLARE_EMAIL_API_BASE_URL` normally remains the official API base URL. A non-HTTPS override is
accepted only for loopback integration tests. `SMTP_FROM` remains a compatibility alias for
`EMAIL_FROM`.

Cloudflare's request and response contract is documented in the official
[Email Sending API reference](https://developers.cloudflare.com/api/resources/email_sending/methods/send/).
The application treats a response as accepted only when the envelope is successful, has no API
errors or permanent bounces, and includes a non-empty message identifier. Delivered and queued counts
are retained as redacted operational metadata; the live API can accept a message before either list is
populated.

## Delivery semantics

Login and password-reset endpoints return success only after the provider accepts the message. When
the provider is unavailable or rejects the request, the newly generated code is invalidated and the
endpoint returns `503` with error code `SERVICE_UNAVAILABLE`. The client may retry; it must not tell
the user that a code was sent after this response.

Invitation and scheduled digest email is best-effort. Their primary operation continues if email
delivery fails, and the failure is logged without recipient addresses, message content, credentials,
or raw upstream responses.

## Secret boundary

- Store the API token in a root-owned or deployment-user-owned environment file with mode `0600`.
- Inject that file into the main backend container only. PR preview code must never receive production
  email credentials.
- Never put account credentials in `.env.example`, workflow variables committed to Git, issue text,
  PR descriptions, screenshots, logs, or generated client code.
- Limit the token to the minimum Cloudflare email-sending permission and the intended account.
- Rotate the token after suspected disclosure. Update the server secret, verify one delivery, then
  revoke the old token.

## Deployment and verification

1. Configure `EMAIL_PROVIDER=cloudflare`, the verified sender in `EMAIL_FROM`, the account identifier,
   API token, and the default API base URL in the main-only secret file.
2. Validate the deployment script with `bash -n` and confirm the preview deployment path does not load
   the secret file.
3. Send one controlled test message and verify a successful envelope with a non-empty message
   identifier and no permanent bounce.
4. Redeploy the main backend and request a login code through the public API.
5. Confirm success without logging the recipient or code. During an induced provider failure, confirm
   the API returns `503` and the generated code cannot be consumed.

## Incident response

For elevated `SERVICE_UNAVAILABLE` rates, first check Cloudflare status and the application warning
count. Verify sender authorization and token scope without printing the token. If fallback SMTP is
enabled, changing providers is a deployment configuration change and must follow the same secret and
verification controls. Do not switch production to `log`: that would report successful requests
without delivering mail.

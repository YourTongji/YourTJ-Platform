# YourTJ web

React + TypeScript frontend for YourTJ Platform v2. It consumes
`../contract/openapi.yaml` through generated TypeScript schema types and targets the
Rust backend under `/api/v2`.

## Development

```bash
pnpm install
pnpm generate:api
pnpm dev
pnpm lint
pnpm typecheck
pnpm build
```

Set `VITE_API_BASE_URL` to override the API base; by default the Vite dev server
proxies `/api` to `http://localhost:8080`.

Set `VITE_CAPTCHA_URL` to override the TongjiCaptcha service base. It defaults to
`https://captcha.07211024.xyz`; the browser calls `/api/captcha` for a challenge and
`/api/verify` for the single-use pass token sent to protected backend writes.

See the repository [development guide](../docs/development/README.md),
[testing guide](../docs/development/testing.md), and
[current product inventory](../docs/product/current-state-and-roadmap.md). The source and generated
OpenAPI types are authoritative for current route coverage; do not maintain a second feature matrix here.

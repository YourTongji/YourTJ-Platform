# infra

Deployment and infrastructure-as-code (placeholder).

Target: Aliyun 华东 (Shanghai/Hangzhou), ICP-filed domain.

- App: SAE serverless containers (the `backend/Dockerfile` image). Same image runs
  on SLB + ECS when Phase B needs it.
- DB: PolarDB (PostgreSQL). Single instance in Phase A; primary + read replica in Phase B.
- Cache: Redis/Tair. Search: Meilisearch. Media: OSS + CDN. Async: RocketMQ (Phase B).

Add Terraform / SAE config and the GitHub Actions deploy workflow here.

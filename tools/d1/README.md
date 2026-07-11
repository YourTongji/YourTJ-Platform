# tools/d1 — D1 selection snapshot import toolchain

Scripts for importing the Cloudflare D1 selection snapshot into isolated PostgreSQL Raw PK tables.
This is not a complete production-data migration. Read the canonical
[`docs/operations/data-import.md`](../../docs/operations/data-import.md) before running it.

## Prerequisites

```bash
pip install -r requirements.txt
export CLOUDFLARE_ACCOUNT_ID=...
export CLOUDFLARE_D1_DATABASE_ID=...
export CLOUDFLARE_API_TOKEN=...           # needs d1:read
export DATABASE_URL=postgres://yourtj:yourtj@localhost:5432/yourtj
```

## Scripts

| Script | Purpose |
|--------|---------|
| `d1_export.py` | Export all tables from D1 → local SQLite (`d1_export.db`) |
| `d1_import_pg.py` | First-load `selection.pk_*` with explicit D1→PG column mapping |
| `gen_reviews_sql.py` | Generate `OVERRIDING SYSTEM VALUE` INSERTs for `reviews.*` |

## Full import workflow (manual)

```bash
# 1. Export D1 to SQLite
python3 d1_export.py

# 2. Import raw PK tables
python3 d1_import_pg.py --source d1_export.db

# 3. Materialize courses
psql "$DATABASE_URL" -f ../../backend/ops/materialize_courses.sql

# 4. Materialize selection
psql "$DATABASE_URL" -f ../../backend/ops/materialize_selection.sql

```

`d1_import_pg.py` requires every target Raw table to be empty and imports all 13 tables in one
transaction. For a database reachable only through an operational shell, emit an atomic psql stream:

```bash
python3 d1_import_pg.py --source d1_export.db --emit-copy | psql "$DATABASE_URL"
```

Historical reviews, likes, reports, wallet hashes, and anonymous edit tokens are not part of the
selection first-load. Do not run `gen_reviews_sql.py` against a shared environment without the
identity, course-mapping, moderation, privacy, idempotency, and rollback decisions in the operations
runbook.

## Admin sync boundary

`POST /api/v2/admin/selection/sync` triggers materialization/search/cache work for data already present in
PostgreSQL. It does not export D1 or first-load the Raw tables, and the current job has no durable
progress/retry record. These scripts remain the explicit local/recovery import path.

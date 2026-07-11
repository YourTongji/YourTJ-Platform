# tools/d1 — D1 data import toolchain

Scripts for importing production data from Cloudflare D1 into local PostgreSQL.

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
| `make_sample.py` | Sample rows from d1_export.db preserving edge-case shapes |

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
selection first-load. They require a separate identity, foreign-key, and privacy migration.

## Automated workflow (via admin API)

After `POST /api/v2/admin/selection/sync` is implemented, the backend handles
steps 1–4 internally (pull→stage→mat→post). These scripts remain as the
escape-hatch for local debugging and disaster recovery.

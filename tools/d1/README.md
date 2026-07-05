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
| `d1_import_pg.py` | Import `selection.pk_*` from SQLite into PG |
| `gen_reviews_sql.py` | Generate `OVERRIDING SYSTEM VALUE` INSERTs for `reviews.*` |
| `make_sample.py` | Sample rows from d1_export.db preserving edge-case shapes |

## Full import workflow (manual)

```bash
# 1. Export D1 to SQLite
python3 d1_export.py

# 2. Import raw PK tables
python3 d1_import_pg.py

# 3. Materialize courses
psql "$DATABASE_URL" -f ../../backend/ops/materialize_courses.sql

# 4. Materialize selection
psql "$DATABASE_URL" -f ../../backend/ops/materialize_selection.sql

# 5. Import reviews
python3 gen_reviews_sql.py | psql "$DATABASE_URL" -f -
```

## Automated workflow (via admin API)

After `POST /api/v2/admin/selection/sync` is implemented, the backend handles
steps 1–4 internally (pull→stage→mat→post). These scripts remain as the
escape-hatch for local debugging and disaster recovery.

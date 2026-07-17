# tools/d1 — D1 selection snapshot import toolchain

Scripts for importing the Cloudflare D1 selection snapshot into isolated PostgreSQL raw tables.
The pipeline loads 13 selection tables plus the legacy teachers/courses/course-aliases used to
restore public rating aggregates. It is not a complete production-data migration. Read the canonical
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
| `d1_export.py` | Verify `jcourse-db-backup`, then atomically export to a fresh mode-0600 SQLite file |
| `d1_import_pg.py` | First-load `selection.pk_*` and `courses.pk_legacy_*` with explicit D1→PG mapping |
| `gen_reviews_sql.py` | Generate `OVERRIDING SYSTEM VALUE` INSERTs for `reviews.*` |

## Full import workflow (manual)

```bash
# 1. Point CLOUDFLARE_D1_DATABASE_ID at jcourse-db-backup; its API name is verified
python3 d1_export.py --output /tmp/jcourse-db-backup.sqlite3

# 2. Import raw PK tables and persist a bounded import audit
python3 d1_import_pg.py \
  --source /tmp/jcourse-db-backup.sqlite3 \
  --source-database jcourse-db-backup \
  --snapshot-exported-at '<RFC3339 export time>' \
  --imported-by 'selection-import:on-call' \
  --compare-manifest /secure/approved-previous-manifest.json \
  --manifest-out /tmp/jcourse-db-backup-manifest.json

# 3. Materialize courses
psql "$DATABASE_URL" -f ../../backend/ops/materialize_courses.sql

# 4. Materialize selection
psql "$DATABASE_URL" -f ../../backend/ops/materialize_selection.sql

```

`d1_import_pg.py` requires every target raw table to be empty and imports all 16 tables plus both
snapshot-bound audit rows in one transaction. For a database reachable only through an operational
shell, emit an atomic psql stream:

```bash
python3 d1_import_pg.py \
  --source /tmp/jcourse-db-backup.sqlite3 \
  --source-database jcourse-db-backup \
  --snapshot-exported-at '<RFC3339 export time>' \
  --imported-by 'selection-import:on-call' \
  --compare-manifest /secure/approved-previous-manifest.json \
  --emit-copy | psql "$DATABASE_URL"
```

The required `--source-database` value is an explicit operator attestation, not proof that an arbitrary
SQLite file came from that D1 database. Trust it only when the file is produced immediately by the
name-verifying exporter in the same controlled workflow. The importer accepts only
`jcourse-db-backup`, hashes the SQLite snapshot, validates the source schema, and records source/target
counts in `selection.import_runs` and `courses.legacy_import_runs`. The optional `--manifest-out` is written atomically with mode `0600`
as a pre-import source manifest and refuses to replace an existing path. `--source`, `--manifest-out`,
and `--compare-manifest` must resolve to three different files, including through symlinks/hard links.
The approved comparison manifest must have the same schema/source and all 16 exact count keys. Every
essential selection and legacy-course table must remain non-empty, and any count decrease fails closed. A reviewed legitimate decrease
requires `--approve-count-decrease --approval-reason '<10-500 chars>'`. Only the first trusted snapshot may
omit a baseline, using `--approve-unbaselined-snapshot --approval-reason '<10-500 chars>'`; this does not
bypass the non-empty guard. Validation and comparison happen before a new manifest is created or any target
write begins. The two transactional audit rows record the approval mode, baseline evidence, and shared
snapshot hash; import time and the upstream `selection.fetchlog` clock are intentionally separate.

Both materializers lock all raw tables and compare each live table count, required teaching-class
relationship, non-empty essential set, and completeness approval metadata with both validated audit rows
before writing. A legacy raw snapshot without that evidence must be re-established through the approved
importer/recovery flow, not blessed with a hand-written row.

Run the importer safety tests from the repository root:

```bash
python3 -m unittest discover -s tools/d1/tests
```

Only uniquely mapped public historical count/average values are materialized. Historical review bodies,
authors, likes, reports, wallet hashes, and anonymous edit tokens are not part of this first-load. Do not
run `gen_reviews_sql.py` against a shared environment without the
identity, course-mapping, moderation, privacy, idempotency, and rollback decisions in the operations
runbook.

## Admin sync boundary

`POST /api/v2/admin/selection/sync` enqueues durable catalogue/materialization/search/cache work for data
already present in PostgreSQL. It exposes bounded progress, lease-fenced retries, dead-job recovery, and
audit, but does not export D1 or first-load the Raw tables. These scripts remain the explicit
local/recovery import path.

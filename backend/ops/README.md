# ops — Operational SQL scripts

Idempotent materialization scripts that transform Raw PK data into Normalized/Main schemas.

The canonical export/import procedure and safety checks live in
[`docs/operations/data-import.md`](../../docs/operations/data-import.md).

These are **not** database migrations — do not place them in `backend/migrations/`.
Migrations are append-only, numbered DDL that the init system executes automatically.

## Scripts

| Script | Purpose |
|--------|---------|
| `materialize_selection.sql` | Materialize `selection.*` from `selection.pk_*` |
| `materialize_courses.sql` | Materialize `courses.*` from `selection.pk_*` |

## Execution order

1. Raw PK data is imported into `selection.pk_*` tables and recorded in
   `selection.import_runs`
2. `materialize_courses.sql` — populates `courses.*`
3. `materialize_selection.sql` — populates `selection.*`

Both scripts are wrapped in `BEGIN...COMMIT`, take the same advisory lock, and hold `SHARE` locks on all
Raw PK tables. Before their first write they require the latest validated import run to match all 13 live
raw table counts, carry valid baseline/completeness approval metadata, and keep every essential source table
non-empty; structurally invalid teaching-class snapshots are also rejected. Missing/stale audit evidence
therefore fails before any projection deletion or catalogue update. Do not bypass the guard or insert a
synthetic import run; restore/re-import the approved snapshot through the canonical toolchain.

The scripts are safe to re-run after that preflight. Validate row counts and API/search output after every
operational execution; a committed transaction does not prove the source snapshot or mapping was correct.

`materialize_selection.sql` is intentionally a full transactional reconcile at the current data size.
It retains stable dimension IDs, replaces dependent offering facts under an advisory lock, and folds
identical day/slot/week/location rows repeated once per teacher into one schedule fact. Incremental
materialization is not an implicit optimization; it requires a separate deletion/replay design and the
go/no-go thresholds in the import runbook.

`materialize_courses.sql` only upserts catalogue rows and aliases. It deliberately retains entries absent
from the latest selection snapshot; retirement needs a separate Courses-owned lifecycle and cannot query
Reviews-private tables from operational SQL.

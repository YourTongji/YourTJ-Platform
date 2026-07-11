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

1. Raw PK data is imported into `selection.pk_*` tables
2. `materialize_courses.sql` — populates `courses.*`
3. `materialize_selection.sql` — populates `selection.*`

All scripts are wrapped in `BEGIN...COMMIT` and are designed to be safe to re-run. Validate row counts
and API/search output after every operational execution; a committed transaction does not prove the
source snapshot or mapping was correct.

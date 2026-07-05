# ops — Operational SQL scripts

Idempotent materialization scripts that transform Raw PK data into Normalized/Main schemas.

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

All scripts are wrapped in `BEGIN...COMMIT` and are safe to re-run (idempotent).

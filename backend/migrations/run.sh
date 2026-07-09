#!/bin/bash
# run.sh — apply all migrations in order against the database specified
# by DATABASE_URL. Requires `psql` on PATH.
#
# Usage: DATABASE_URL=postgres://... ./run.sh
#
# When placed in a Postgres Docker container's /docker-entrypoint-initdb.d/,
# the entrypoint runs this script AFTER all .sql files have been applied
# automatically. In that context there is nothing left to do — the guard
# below detects the scenario and exits early.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Docker init context guard — all .sql files already applied by Postgres
if [ -f "/.dockerenv" ] && [ -z "${DATABASE_URL:-}" ]; then
    echo "Detected Docker init context — migrations already applied. Skipping run.sh."
    exit 0
fi

if [ -z "${DATABASE_URL:-}" ]; then
    echo "ERROR: DATABASE_URL environment variable is not set."
    exit 1
fi

echo "Applying migrations from ${SCRIPT_DIR}..."

for f in "$SCRIPT_DIR"/*.sql; do
    if [ -f "$f" ]; then
        echo "  Running $(basename "$f")..."
        psql "$DATABASE_URL" -f "$f" -v ON_ERROR_STOP=1
    fi
done

echo "All migrations applied."

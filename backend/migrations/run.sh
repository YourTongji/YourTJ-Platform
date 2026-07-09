#!/bin/bash
# run.sh — apply all migrations in order against the database specified
# by DATABASE_URL. Requires `psql` on PATH.
#
# Usage: DATABASE_URL=postgres://... ./run.sh
#
# This script is an external management tool for applying migrations to a
# remote database. The whole migrations directory is also bind-mounted into
# the Postgres container's /docker-entrypoint-initdb.d, where the entrypoint
# runs every *.sql file and then this script. In that init context DATABASE_URL
# is not set (and the .sql files were already applied by the entrypoint), so
# the guard below detects the Docker init scenario and exits cleanly instead of
# failing the container startup.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Docker init context guard — the entrypoint has already applied every .sql
# file, and no DATABASE_URL is provided, so there is nothing left to do.
if [ -f "/.dockerenv" ] && [ -z "${DATABASE_URL:-}" ]; then
    echo "Detected Postgres Docker init context — migrations already applied by the entrypoint. Skipping run.sh."
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

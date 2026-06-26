#!/bin/bash
# run.sh — apply all migrations in order against the database specified
# by DATABASE_URL. Requires `psql` on PATH.
#
# Usage: DATABASE_URL=postgres://... ./run.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

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
